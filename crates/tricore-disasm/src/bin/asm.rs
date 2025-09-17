use anyhow::{anyhow, Result};
use clap::Parser;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about = "Tiny TriCore assembler (subset)")]
struct Opts {
    /// Input assembly file (one instruction or directive per line)
    #[arg(short, long)]
    input: PathBuf,
    /// Output binary file (little-endian)
    #[arg(short, long)]
    output: PathBuf,
    /// Start address (used for label resolution and PC-relative encodings)
    #[arg(long, default_value_t = 0u32)]
    start: u32,
}

#[derive(Debug, Clone)]
enum Item {
    Label(String),
    Instr(Inst),
    Dir(Dir),
}

#[derive(Debug, Clone)]
enum Inst {
    MovU { d: u32, imm16: u32 },   // 32-bit MOV.U
    Mov16 { d: u32, imm4: u32 },   // 16-bit MOV (const4)
    J { target: Target },          // 32-bit J (relative)
    Call { target: Target },       // 32-bit CALL (relative)
    CallA { ea: u32 },             // 32-bit CALLA absolute EA
    CallI { a: u32 },              // 32-bit CALLI A[a]
    // New ops for examples and simple programs
    AddRR { rd: u32, ra: u32, rb: u32 },   // 32-bit ADD rr
    SubRR { rd: u32, ra: u32, rb: u32 },   // 32-bit SUB rr
    LdBuOff16 { rd: u32, ab: u32, off16: u32 }, // 32-bit LD.BU D[rd], A[ab], off16
    StBOff16 { ab: u32, rs: u32, off16: u32 },  // 32-bit ST.B A[ab], off16, D[rs]
    JneRR { a: u32, b: u32, target: Target },   // 32-bit JNE D[a], D[b], disp15
    JeqRR { a: u32, b: u32, target: Target },   // 32-bit JEQ D[a], D[b], disp15
    JgeURR { a: u32, b: u32, target: Target },  // 32-bit JGE.U D[a], D[b], disp15
    JltURR { a: u32, b: u32, target: Target },  // 32-bit JLT.U D[a], D[b], disp15
    LeaAb { rd: u32, rb: u32, off: i32 },       // 32-bit LEA A[rd], A[rb], off10
    Word { val: u32 },             // .word
    Byte { val: u8 },              // .byte
}

#[derive(Debug, Clone)]
enum Dir { Word(u32), Byte(u8) }

#[derive(Debug, Clone)]
enum Target { Label(String), Abs(u32) }

fn parse_reg_d(s: &str) -> Option<u32> { s.strip_prefix('d').and_then(|r| r.parse::<u32>().ok()) }
fn parse_reg_a(s: &str) -> Option<u32> { s.strip_prefix('a').and_then(|r| r.parse::<u32>().ok()) }

fn parse_num(s: &str) -> Option<u32> {
    let t = s.trim();
    if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16).ok()
    } else { t.parse::<u32>().ok() }
}

fn parse_line(line: &str) -> Result<Option<Item>> {
    // Treat lines starting with '#' (after leading spaces) or ';' anywhere as comments.
    let ls = line.trim_start();
    if ls.starts_with('#') { return Ok(None); }
    let mut s = line;
    if let Some(p) = s.find(';') { s = &s[..p]; }
    let s = s.trim();
    if s.is_empty() { return Ok(None); }
    if s.ends_with(':') {
        let name = s[..s.len()-1].trim().to_string();
        return Ok(Some(Item::Label(name)));
    }
    // directive
    if let Some(rest) = s.strip_prefix(".word") { 
        let v = parse_num(rest.trim()).ok_or_else(|| anyhow!("bad .word: {}", line))?;
        return Ok(Some(Item::Dir(Dir::Word(v))));
    }
    if let Some(rest) = s.strip_prefix(".byte") { 
        let v = parse_num(rest.trim()).ok_or_else(|| anyhow!("bad .byte: {}", line))?;
        return Ok(Some(Item::Dir(Dir::Byte((v & 0xFF) as u8))));
    }
    // instr tokens
    let mut parts = s.split_whitespace();
    let mn = parts.next().unwrap().to_lowercase();
    let rest = parts.collect::<Vec<_>>().join(" ");
    let rest = rest.trim();
    let parse_imm = |imm: &str| -> Result<u32> { parse_num(imm).ok_or_else(|| anyhow!("bad imm: {}", imm)) };
    let comma = |s: &str| s.split(',').map(|x| x.trim().to_string()).collect::<Vec<String>>();

    let item = match mn.as_str() {
        "movu" => {
            // movu dX, #imm16
            let p = comma(rest);
            if p.len() != 2 { return Err(anyhow!("movu syntax: movu dX, #imm16")); }
            let d = parse_reg_d(&p[0]).ok_or_else(|| anyhow!("bad reg: {}", p[0]))?;
            let imm = p[1].trim_start_matches('#');
            let imm16 = parse_imm(imm)? & 0xFFFF;
            Item::Instr(Inst::MovU { d, imm16 })
        }
        "mov" => {
            // mov dX, #imm (0..15 => 16-bit, else movu)
            let p = comma(rest);
            if p.len() != 2 { return Err(anyhow!("mov syntax: mov dX, #imm")); }
            let d = parse_reg_d(&p[0]).ok_or_else(|| anyhow!("bad reg: {}", p[0]))?;
            let imm = parse_imm(p[1].trim_start_matches('#'))?;
            if imm <= 0xF { Item::Instr(Inst::Mov16 { d, imm4: imm }) } else { Item::Instr(Inst::MovU { d, imm16: imm & 0xFFFF }) }
        }
        "add" => {
            // add dC, dA, dB
            let p = comma(rest);
            if p.len() != 3 { return Err(anyhow!("add syntax: add dC, dA, dB")); }
            let rd = parse_reg_d(&p[0]).ok_or_else(|| anyhow!("bad reg: {}", p[0]))?;
            let ra = parse_reg_d(&p[1]).ok_or_else(|| anyhow!("bad reg: {}", p[1]))?;
            let rb = parse_reg_d(&p[2]).ok_or_else(|| anyhow!("bad reg: {}", p[2]))?;
            Item::Instr(Inst::AddRR { rd, ra, rb })
        }
        "sub" => {
            // sub dC, dA, dB
            let p = comma(rest);
            if p.len() != 3 { return Err(anyhow!("sub syntax: sub dC, dA, dB")); }
            let rd = parse_reg_d(&p[0]).ok_or_else(|| anyhow!("bad reg: {}", p[0]))?;
            let ra = parse_reg_d(&p[1]).ok_or_else(|| anyhow!("bad reg: {}", p[1]))?;
            let rb = parse_reg_d(&p[2]).ok_or_else(|| anyhow!("bad reg: {}", p[2]))?;
            Item::Instr(Inst::SubRR { rd, ra, rb })
        }
        "ld.bu" => {
            // ld.bu dA, [aB+off]
            let p = comma(rest);
            if p.len() != 2 { return Err(anyhow!("ld.bu syntax: ld.bu dA, [aB+off]")); }
            let rd = parse_reg_d(&p[0]).ok_or_else(|| anyhow!("bad reg: {}", p[0]))?;
            let mem = p[1].trim();
            let (ab, off) = parse_mem_ab_off(mem)?;
            Item::Instr(Inst::LdBuOff16 { rd, ab, off16: off & 0xFFFF })
        }
        "st.b" => {
            // st.b [aB+off], dA
            let p = comma(rest);
            if p.len() != 2 { return Err(anyhow!("st.b syntax: st.b [aB+off], dA")); }
            let (ab, off) = parse_mem_ab_off(&p[0])?;
            let rs = parse_reg_d(&p[1]).ok_or_else(|| anyhow!("bad reg: {}", p[1]))?;
            Item::Instr(Inst::StBOff16 { ab, rs, off16: off & 0xFFFF })
        }
        "lea" => {
            // lea aC, [aB+off]
            let p = comma(rest);
            if p.len() != 2 { return Err(anyhow!("lea syntax: lea aC, [aB+off]")); }
            let rd = parse_reg_a(&p[0]).ok_or_else(|| anyhow!("bad areg: {}", p[0]))?;
            let (rb, off) = parse_mem_ab_off(&p[1])?;
            // off10 signed range will be enforced at encode time
            Item::Instr(Inst::LeaAb { rd, rb, off: off as i32 })
        }
        "j" => {
            // j <label|abs>
            let t = rest;
            let target = if let Some(v) = parse_num(t) { Target::Abs(v) } else { Target::Label(t.to_string()) };
            Item::Instr(Inst::J { target })
        }
        "jeq" => {
            let p = comma(rest);
            if p.len() != 3 { return Err(anyhow!("jeq syntax: jeq dA, dB, <label|abs>")); }
            let a = parse_reg_d(&p[0]).ok_or_else(|| anyhow!("bad reg: {}", p[0]))?;
            let b = parse_reg_d(&p[1]).ok_or_else(|| anyhow!("bad reg: {}", p[1]))?;
            let target = if let Some(v) = parse_num(&p[2]) { Target::Abs(v) } else { Target::Label(p[2].to_string()) };
            Item::Instr(Inst::JeqRR { a, b, target })
        }
        "jne" => {
            let p = comma(rest);
            if p.len() != 3 { return Err(anyhow!("jne syntax: jne dA, dB, <label|abs>")); }
            let a = parse_reg_d(&p[0]).ok_or_else(|| anyhow!("bad reg: {}", p[0]))?;
            let b = parse_reg_d(&p[1]).ok_or_else(|| anyhow!("bad reg: {}", p[1]))?;
            let target = if let Some(v) = parse_num(&p[2]) { Target::Abs(v) } else { Target::Label(p[2].to_string()) };
            Item::Instr(Inst::JneRR { a, b, target })
        }
        "jge.u" => {
            let p = comma(rest);
            if p.len() != 3 { return Err(anyhow!("jge.u syntax: jge.u dA, dB, <label|abs>")); }
            let a = parse_reg_d(&p[0]).ok_or_else(|| anyhow!("bad reg: {}", p[0]))?;
            let b = parse_reg_d(&p[1]).ok_or_else(|| anyhow!("bad reg: {}", p[1]))?;
            let target = if let Some(v) = parse_num(&p[2]) { Target::Abs(v) } else { Target::Label(p[2].to_string()) };
            Item::Instr(Inst::JgeURR { a, b, target })
        }
        "jlt.u" => {
            let p = comma(rest);
            if p.len() != 3 { return Err(anyhow!("jlt.u syntax: jlt.u dA, dB, <label|abs>")); }
            let a = parse_reg_d(&p[0]).ok_or_else(|| anyhow!("bad reg: {}", p[0]))?;
            let b = parse_reg_d(&p[1]).ok_or_else(|| anyhow!("bad reg: {}", p[1]))?;
            let target = if let Some(v) = parse_num(&p[2]) { Target::Abs(v) } else { Target::Label(p[2].to_string()) };
            Item::Instr(Inst::JltURR { a, b, target })
        }
        "call" => {
            let t = rest;
            let target = if let Some(v) = parse_num(t) { Target::Abs(v) } else { Target::Label(t.to_string()) };
            Item::Instr(Inst::Call { target })
        }
        "calla" => {
            let ea = parse_imm(rest)?;
            Item::Instr(Inst::CallA { ea })
        }
        "calli" => {
            let a = parse_reg_a(rest).ok_or_else(|| anyhow!("calli aN"))?;
            Item::Instr(Inst::CallI { a })
        }
        ".word" => {
            let v = parse_imm(rest)?;
            Item::Instr(Inst::Word { val: v })
        }
        ".byte" => {
            let v = parse_imm(rest)?;
            Item::Instr(Inst::Byte { val: (v & 0xFF) as u8 })
        }
        _ => return Err(anyhow!("unknown mnemonic: {}", mn)),
    };
    Ok(Some(item))
}

fn parse_mem_ab_off(s: &str) -> Result<(u32, u32)> {
    // Expect "[aN+imm]" or "[aN]"
    let st = s.trim();
    if !st.starts_with('[') || !st.ends_with(']') { return Err(anyhow!("expected memory operand like [aN+imm]: {}", s)); }
    let inner = &st[1..st.len()-1];
    let mut parts = inner.split('+').map(|t| t.trim());
    let base = parts.next().ok_or_else(|| anyhow!("bad mem"))?;
    let ab = parse_reg_a(base).ok_or_else(|| anyhow!("bad base reg in {}", s))?;
    let off = if let Some(off_s) = parts.next() { parse_num(off_s).ok_or_else(|| anyhow!("bad offset in {}", s))? } else { 0 };
    Ok((ab, off))
}

fn width_of(item: &Item) -> usize {
    match item {
        Item::Label(_) => 0,
        Item::Dir(Dir::Word(_)) | Item::Instr(Inst::Word{..}) => 4,
        Item::Dir(Dir::Byte(_)) | Item::Instr(Inst::Byte{..}) => 1,
        Item::Instr(Inst::Mov16{..}) => 2,
        Item::Instr(Inst::MovU{..}) => 4,
        Item::Instr(Inst::J{..}) => 4,
        Item::Instr(Inst::Call{..}) => 4,
        Item::Instr(Inst::CallA{..}) => 4,
        Item::Instr(Inst::CallI{..}) => 4,
        Item::Instr(Inst::AddRR{..}) => 4,
        Item::Instr(Inst::SubRR{..}) => 4,
        Item::Instr(Inst::LdBuOff16{..}) => 4,
        Item::Instr(Inst::StBOff16{..}) => 4,
        Item::Instr(Inst::JneRR{..}) | Item::Instr(Inst::JeqRR{..}) | Item::Instr(Inst::JgeURR{..}) | Item::Instr(Inst::JltURR{..}) => 4,
        Item::Instr(Inst::LeaAb{..}) => 4,
    }
}

fn encode(items: &[Item], start: u32) -> Result<Vec<u8>> {
    // Pass 1: labels
    let mut pc = start;
    let mut labels: HashMap<String, u32> = HashMap::new();
    for it in items {
        match it {
            Item::Label(name) => { labels.insert(name.clone(), pc); }
            _ => pc = pc.wrapping_add(width_of(it) as u32),
        }
    }
    // Pass 2: encode
    let mut out = Vec::new();
    pc = start;
    for it in items {
        match it {
            Item::Label(_) => {}
            Item::Dir(Dir::Word(v)) | Item::Instr(Inst::Word{ val: v }) => { out.extend_from_slice(&v.to_le_bytes()); pc += 4; }
            Item::Dir(Dir::Byte(b)) | Item::Instr(Inst::Byte{ val: b }) => { out.push(*b); pc += 1; }
            Item::Instr(Inst::MovU{ d, imm16 }) => {
                let raw = ((d & 0xF) << 28) | ((imm16 & 0xFFFF) << 12) | 0xBB;
                out.extend_from_slice(&raw.to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::AddRR{ rd, ra, rb }) => {
                // op1=0x0B, op2=0x00
                let raw = ((*rd & 0xF) << 28) | ((*rb & 0xF) << 16) | (0x00 << 20) | ((*ra & 0xF) << 8) | 0x0B;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::SubRR{ rd, ra, rb }) => {
                // op1=0x0B, op2=0x08
                let raw = ((*rd & 0xF) << 28) | ((*rb & 0xF) << 16) | (0x08 << 20) | ((*ra & 0xF) << 8) | 0x0B;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::LdBuOff16{ rd, ab, off16 }) => {
                // op1=0x39; fields: [31:28]=off[9:6], [22:27]=off[15:10], [16:21]=off[5:0], [12:15]=A[b], [8:11]=D[a]
                let off = off16 & 0xFFFF;
                let off_hi4 = (off >> 6) & 0xF;
                let off_mid6 = (off >> 10) & 0x3F;
                let off_lo6 = off & 0x3F;
                let raw = (off_hi4 << 28) | (off_mid6 << 22) | (off_lo6 << 16) | ((*ab & 0xF) << 12) | ((*rd & 0xF) << 8) | 0x39;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::StBOff16{ ab, rs, off16 }) => {
                // op1=0xE9; fields mirror LD.B base+off16
                let off = off16 & 0xFFFF;
                let off_hi4 = (off >> 6) & 0xF;
                let off_mid6 = (off >> 10) & 0x3F;
                let off_lo6 = off & 0x3F;
                let raw = (off_hi4 << 28) | (off_mid6 << 22) | (off_lo6 << 16) | ((*ab & 0xF) << 12) | ((*rs & 0xF) << 8) | 0xE9;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::JeqRR{ a, b, target }) | Item::Instr(Inst::JneRR{ a, b, target }) => {
                let tgt = match target { Target::Abs(v) => *v, Target::Label(l) => *labels.get(l).ok_or_else(|| anyhow!("unknown label: {}", l))? };
                let off = (tgt as i64) - (pc as i64) - 4;
                let disp15 = (off >> 1) as i32;
                if disp15 < -(1<<14) || disp15 >= (1<<14) { return Err(anyhow!("JEQ/JNE target out of range")); }
                let d15 = (disp15 as u32) & 0x7FFF;
                let cond = if matches!(it, Item::Instr(Inst::JneRR{..})) { 0x01 } else { 0x00 };
                let raw = (cond << 30) | (d15 << 15) | ((*b & 0xF) << 12) | ((*a & 0xF) << 8) | 0x5F;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::JgeURR{ a, b, target }) => {
                let tgt = match target { Target::Abs(v) => *v, Target::Label(l) => *labels.get(l).ok_or_else(|| anyhow!("unknown label: {}", l))? };
                let off = (tgt as i64) - (pc as i64) - 4;
                let disp15 = (off >> 1) as i32;
                if disp15 < -(1<<14) || disp15 >= (1<<14) { return Err(anyhow!("JGE.U target out of range")); }
                let d15 = (disp15 as u32) & 0x7FFF;
                // unsigned marker cond==0x01 per decoder
                let raw = (0x01 << 30) | (d15 << 15) | ((*b & 0xF) << 12) | ((*a & 0xF) << 8) | 0x7F;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::JltURR{ a, b, target }) => {
                let tgt = match target { Target::Abs(v) => *v, Target::Label(l) => *labels.get(l).ok_or_else(|| anyhow!("unknown label: {}", l))? };
                let off = (tgt as i64) - (pc as i64) - 4;
                let disp15 = (off >> 1) as i32;
                if disp15 < -(1<<14) || disp15 >= (1<<14) { return Err(anyhow!("JLT.U target out of range")); }
                let d15 = (disp15 as u32) & 0x7FFF;
                let raw = (0x01 << 30) | (d15 << 15) | ((*b & 0xF) << 12) | ((*a & 0xF) << 8) | 0x3F;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::LeaAb { rd, rb, off }) => {
                // op1 = 0x49, op2 field [27:22] must be 0x28; off10 signed
                let off10 = *off as i32;
                if off10 < -(1<<9) || off10 >= (1<<9) { return Err(anyhow!("lea off10 out of range")); }
                let u = (off10 as u32) & 0x3FF;
                let off_upper4 = (u >> 6) & 0xF;
                let off_lower6 = u & 0x3F;
                let raw = (off_upper4 << 28) | (0x28 << 22) | (off_lower6 << 16) | ((*rb & 0xF) << 12) | ((*rd & 0xF) << 8) | 0x49;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::Mov16{ d, imm4 }) => {
                let raw16: u16 = (((imm4 & 0xF) as u16) << 12) | (((d & 0xF) as u16) << 8) | 0x82u16;
                out.extend_from_slice(&raw16.to_le_bytes()); pc += 2;
            }
            Item::Instr(Inst::J { target }) => {
                let tgt = match target { Target::Abs(v) => *v, Target::Label(l) => *labels.get(l).ok_or_else(|| anyhow!("unknown label: {}", l))? };
                let off = (tgt as i64) - (pc as i64) - 4; // relative to next PC
                let disp24 = (off >> 1) as i32;
                if disp24 < -(1<<23) || disp24 >= (1<<23) { return Err(anyhow!("J target out of range")); }
                let d24 = (disp24 as u32) & 0xFF_FFFF;
                let raw = ((d24 & 0xFFFF) << 16) | ((d24 >> 16) << 8) | 0x1D;
                out.extend_from_slice(&raw.to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::Call { target }) => {
                let tgt = match target { Target::Abs(v) => *v, Target::Label(l) => *labels.get(l).ok_or_else(|| anyhow!("unknown label: {}", l))? };
                let off = (tgt as i64) - (pc as i64) - 4;
                let disp24 = (off >> 1) as i32;
                if disp24 < -(1<<23) || disp24 >= (1<<23) { return Err(anyhow!("CALL target out of range")); }
                let d24 = (disp24 as u32) & 0xFF_FFFF;
                let raw = ((d24 & 0xFFFF) << 16) | ((d24 >> 16) << 8) | 0x6D;
                out.extend_from_slice(&raw.to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::CallA { ea }) => {
                // Invert EA mapping: disp24 = {ea[31:28], ea[20:1]}
                let top4 = (ea >> 28) & 0xF;
                let low20 = (ea >> 1) & 0xFFFFF;
                let d24 = (top4 << 20) | low20;
                let raw = (((d24 & 0xFFFF) << 16) | (((d24 >> 16) & 0xFF) << 8) | 0xED) as u32;
                out.extend_from_slice(&raw.to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::CallI { a }) => {
                let raw = (((a & 0xF) << 8) | 0x2D) as u32;
                out.extend_from_slice(&raw.to_le_bytes()); pc += 4;
            }
        }
    }
    Ok(out)
}

fn main() -> Result<()> {
    let opts = Opts::parse();
    let text = fs::read_to_string(&opts.input)?;
    let mut items = Vec::new();
    for (i, line) in text.lines().enumerate() {
        match parse_line(line)? {
            None => {}
            Some(it) => items.push(it),
        }
    }
    let bin = encode(&items, opts.start)?;
    fs::write(&opts.output, &bin)?;
    Ok(())
}
