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
    LdBOff16 { rd: u32, ab: u32, off16: u32 },  // 32-bit LD.B D[rd], A[ab], off16
    LdHOff16 { rd: u32, ab: u32, off16: u32 },  // 32-bit LD.H D[rd], A[ab], off16
    LdHuOff16 { rd: u32, ab: u32, off16: u32 }, // 32-bit LD.HU D[rd], A[ab], off16
    LdWOff16 { rd: u32, ab: u32, off16: u32 },  // 32-bit LD.W D[rd], A[ab], off16
    StHOff16 { ab: u32, rs: u32, off16: u32 },  // 32-bit ST.H A[ab], off16, D[rs]
    StWOff16 { ab: u32, rs: u32, off16: u32 },  // 32-bit ST.W A[ab], off16, D[rs]
    // Absolute addressing variants (18-bit ABS encoding)
    LdBAbs { rd: u32, ea: u32 },
    LdBuAbs { rd: u32, ea: u32 },
    LdHAbs { rd: u32, ea: u32 },
    LdHuAbs { rd: u32, ea: u32 },
    LdWAbs { rd: u32, ea: u32 },
    StBAbs { rs: u32, ea: u32 },
    StHAbs { rs: u32, ea: u32 },
    StWAbs { rs: u32, ea: u32 },
    // Address-register helpers
    MovHAa { rd: u32, imm16: u32 },        // movh.a aC, #imm16
    LeaAbs { rd: u32, ea: u32 },           // lea aC, [abs]
    AddihA { rd: u32, ra: u32, imm16: u32 }, // addih.a aC, aA, #imm16
    JneRR { a: u32, b: u32, target: Target },   // 32-bit JNE D[a], D[b], disp15
    JeqRR { a: u32, b: u32, target: Target },   // 32-bit JEQ D[a], D[b], disp15
    JgeURR { a: u32, b: u32, target: Target },  // 32-bit JGE.U D[a], D[b], disp15
    JltURR { a: u32, b: u32, target: Target },  // 32-bit JLT.U D[a], D[b], disp15
    CmpRR { a: u32, b: u32, unsigned: bool },
    CmpRI { a: u32, imm: u32, unsigned: bool },
    ShRR { rd: u32, ra: u32, rb: u32, kind: u8 },     // 0=shl,1=shr,2=sar,3=ror
    ShRI { rd: u32, ra: u32, imm: u32, kind: u8 },
    AndnRR { rd: u32, ra: u32, rb: u32 },
    AndnRI { rd: u32, ra: u32, imm: u32 },
    NotR { rd: u32, ra: u32 },
    MinRR { rd: u32, ra: u32, rb: u32, unsigned: bool },
    MinRI { rd: u32, ra: u32, imm: u32, unsigned: bool },
    MaxRR { rd: u32, ra: u32, rb: u32, unsigned: bool },
    MaxRI { rd: u32, ra: u32, imm: u32, unsigned: bool },
    MulRR { rd: u32, ra: u32, rb: u32, unsigned: bool },
    MulRI { rd: u32, ra: u32, imm: u32, unsigned: bool },
    DivRR { rd: u32, ra: u32, rb: u32, unsigned: bool },
    // Flag-based branches
    BFlag { kind: u8, target: Target }, // 0=beq,1=bne,2=bge,3=blt,4=bge.u,5=blt.u
    // Add-with-carry / Add-extended
    AddcRR { rd: u32, ra: u32, rb: u32 },
    AddcRI { rd: u32, ra: u32, imm: u32 },
    AddxRR { rd: u32, ra: u32, rb: u32 },
    AddxRI { rd: u32, ra: u32, imm: u32 },
    // JEQ/JNE with const4 immediate and label/abs target
    JeqImm { ra: u32, imm4: u32, target: Target },
    JneImm { ra: u32, imm4: u32, target: Target },
    // JGE/JLT with const4 or reg-reg
    JgeRR { a: u32, b: u32, target: Target, unsigned: bool },
    JltRR { a: u32, b: u32, target: Target, unsigned: bool },
    JgeI  { a: u32, imm4: u32, target: Target, unsigned: bool },
    JltI  { a: u32, imm4: u32, target: Target, unsigned: bool },
    // 16-bit SRR logical RR (rd==ra)
    AndRR16 { ra: u32, rb: u32 },
    OrRR16  { ra: u32, rb: u32 },
    XorRR16 { ra: u32, rb: u32 },
    // 32-bit RR mov
    MovRR { rd: u32, rb: u32 },
    // Register extend macros (expand to real ops at encode)
    ZextB { rd: u32, ra: u32 },
    ZextH { rd: u32, ra: u32 },
    SextB { rd: u32, ra: u32 },
    SextH { rd: u32, ra: u32 },
    // Logical RC (const9) immediate forms
    AndRI { rd: u32, ra: u32, imm9: u32 },
    OrRI  { rd: u32, ra: u32, imm9: u32 },
    XorRI { rd: u32, ra: u32, imm9: u32 },
    // A-register branches (32-bit BRR)
    JeqARR { ra: u32, rb: u32, target: Target },
    JneARR { ra: u32, rb: u32, target: Target },
    JzAR   { ra: u32, target: Target },
    JnzAR  { ra: u32, target: Target },
    // P[b] addressing (bit-reverse and circular) loads/stores
    LdBPbr { rd: u32, pb: u32 },
    LdBUPbr { rd: u32, pb: u32 },
    LdHPbr { rd: u32, pb: u32 },
    LdHUPbr { rd: u32, pb: u32 },
    LdWPbr { rd: u32, pb: u32 },
    LdBPcir { rd: u32, pb: u32, off10: i32 },
    LdBUPcir { rd: u32, pb: u32, off10: i32 },
    LdHPcir { rd: u32, pb: u32, off10: i32 },
    LdHUPcir { rd: u32, pb: u32, off10: i32 },
    LdWPcir { rd: u32, pb: u32, off10: i32 },
    StBPbrP { pb: u32, rs: u32 },
    StHPbrP { pb: u32, rs: u32 },
    StWPbrP { pb: u32, rs: u32 },
    StBPcirP { pb: u32, rs: u32, off10: i32 },
    StHPcirP { pb: u32, rs: u32, off10: i32 },
    StWPcirP { pb: u32, rs: u32, off10: i32 },
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
            if p.len() != 2 { return Err(anyhow!("ld.bu syntax: ld.bu dA, [aB+off|0xADDR]")); }
            let rd = parse_reg_d(&p[0]).ok_or_else(|| anyhow!("bad reg: {}", p[0]))?;
            let mem = p[1].trim();
            if mem.starts_with('[') && mem.contains('a') { let (ab, off) = parse_mem_ab_off(mem)?; Item::Instr(Inst::LdBuOff16 { rd, ab, off16: off & 0xFFFF }) }
            else {
                let ea = parse_mem_abs(mem)?;
                Item::Instr(Inst::LdBuAbs { rd, ea })
            }
        }
        "ld.b" => {
            // ld.b dA, [aB+off]
            let p = comma(rest);
            if p.len() != 2 { return Err(anyhow!("ld.b syntax: ld.b dA, [aB+off|0xADDR]")); }
            let rd = parse_reg_d(&p[0]).ok_or_else(|| anyhow!("bad reg: {}", p[0]))?;
            let mem = p[1].trim();
            if mem.starts_with('[') && mem.contains('a') { let (ab, off) = parse_mem_ab_off(mem)?; Item::Instr(Inst::LdBOff16 { rd, ab, off16: off & 0xFFFF }) }
            else { let ea = parse_mem_abs(mem)?; Item::Instr(Inst::LdBAbs { rd, ea }) }
        }
        "ld.h" => {
            // ld.h dA, [aB+off]
            let p = comma(rest);
            if p.len() != 2 { return Err(anyhow!("ld.h syntax: ld.h dA, [aB+off|0xADDR]")); }
            let rd = parse_reg_d(&p[0]).ok_or_else(|| anyhow!("bad reg: {}", p[0]))?;
            let mem = p[1].trim();
            if mem.starts_with('[') && mem.contains('a') { let (ab, off) = parse_mem_ab_off(mem)?; Item::Instr(Inst::LdHOff16 { rd, ab, off16: off & 0xFFFF }) }
            else { let ea = parse_mem_abs(mem)?; Item::Instr(Inst::LdHAbs { rd, ea }) }
        }
        "ld.hu" => {
            // ld.hu dA, [aB+off]
            let p = comma(rest);
            if p.len() != 2 { return Err(anyhow!("ld.hu syntax: ld.hu dA, [aB+off|0xADDR]")); }
            let rd = parse_reg_d(&p[0]).ok_or_else(|| anyhow!("bad reg: {}", p[0]))?;
            let mem = p[1].trim();
            if mem.starts_with('[') && mem.contains('a') { let (ab, off) = parse_mem_ab_off(mem)?; Item::Instr(Inst::LdHuOff16 { rd, ab, off16: off & 0xFFFF }) }
            else { let ea = parse_mem_abs(mem)?; Item::Instr(Inst::LdHuAbs { rd, ea }) }
        }
        "ld.w" => {
            // ld.w dA, [aB+off]
            let p = comma(rest);
            if p.len() != 2 { return Err(anyhow!("ld.w syntax: ld.w dA, [aB+off|0xADDR]")); }
            let rd = parse_reg_d(&p[0]).ok_or_else(|| anyhow!("bad reg: {}", p[0]))?;
            let mem = p[1].trim();
            if mem.starts_with('[') && mem.contains('a') { let (ab, off) = parse_mem_ab_off(mem)?; Item::Instr(Inst::LdWOff16 { rd, ab, off16: off & 0xFFFF }) }
            else { let ea = parse_mem_abs(mem)?; Item::Instr(Inst::LdWAbs { rd, ea }) }
        }
        "st.b" => {
            // st.b [aB+off], dA
            let p = comma(rest);
            if p.len() != 2 { return Err(anyhow!("st.b syntax: st.b [aB+off|0xADDR], dA")); }
            let mem = p[0].trim();
            let rs = parse_reg_d(&p[1]).ok_or_else(|| anyhow!("bad reg: {}", p[1]))?;
            if mem.starts_with('[') && mem.contains('a') { let (ab, off) = parse_mem_ab_off(mem)?; Item::Instr(Inst::StBOff16 { ab, rs, off16: off & 0xFFFF }) }
            else { let ea = parse_mem_abs(mem)?; Item::Instr(Inst::StBAbs { rs, ea }) }
        }
        "st.h" => {
            // st.h [aB+off], dA
            let p = comma(rest);
            if p.len() != 2 { return Err(anyhow!("st.h syntax: st.h [aB+off|0xADDR], dA")); }
            let mem = p[0].trim();
            let rs = parse_reg_d(&p[1]).ok_or_else(|| anyhow!("bad reg: {}", p[1]))?;
            if mem.starts_with('[') && mem.contains('a') { let (ab, off) = parse_mem_ab_off(mem)?; Item::Instr(Inst::StHOff16 { ab, rs, off16: off & 0xFFFF }) }
            else { let ea = parse_mem_abs(mem)?; Item::Instr(Inst::StHAbs { rs, ea }) }
        }
        "st.w" => {
            // st.w [aB+off], dA
            let p = comma(rest);
            if p.len() != 2 { return Err(anyhow!("st.w syntax: st.w [aB+off|0xADDR], dA")); }
            let mem = p[0].trim();
            let rs = parse_reg_d(&p[1]).ok_or_else(|| anyhow!("bad reg: {}", p[1]))?;
            if mem.starts_with('[') && mem.contains('a') { let (ab, off) = parse_mem_ab_off(mem)?; Item::Instr(Inst::StWOff16 { ab, rs, off16: off & 0xFFFF }) }
            else { let ea = parse_mem_abs(mem)?; Item::Instr(Inst::StWAbs { rs, ea }) }
        }
        // P[b] addressing loads: bit-reverse forms ld.* dA, [pB]
        "ld.b" | "ld.bu" | "ld.h" | "ld.hu" | "ld.w" if p[1].trim().starts_with("[p") && !rest.contains(',') => {
            let rd = parse_reg_d(&comma(rest)[0]).ok_or_else(|| anyhow!("bad dreg in {}", rest))?;
            let pb = parse_reg_p(&comma(rest)[1])?;
            match mn.as_str() {
                "ld.b" => Item::Instr(Inst::LdBPbr { rd, pb }),
                "ld.bu" => Item::Instr(Inst::LdBUPbr { rd, pb }),
                "ld.h" => Item::Instr(Inst::LdHPbr { rd, pb }),
                "ld.hu" => Item::Instr(Inst::LdHUPbr { rd, pb }),
                "ld.w" => Item::Instr(Inst::LdWPbr { rd, pb }),
                _ => unreachable!(),
            }
        }
        // P[b] circular loads: ld.* dA, [pB], off
        "ld.b" | "ld.bu" | "ld.h" | "ld.hu" | "ld.w" if comma(rest).len()==3 && comma(rest)[1].trim().starts_with("[p") => {
            let parts = comma(rest);
            let rd = parse_reg_d(&parts[0]).ok_or_else(|| anyhow!("bad dreg: {}", parts[0]))?;
            let pb = parse_reg_p(&parts[1])?;
            let off = parse_num(parts[2].trim()).ok_or_else(|| anyhow!("bad off10: {}", parts[2]))? as i32;
            match mn.as_str() {
                "ld.b" => Item::Instr(Inst::LdBPcir { rd, pb, off10: off }),
                "ld.bu" => Item::Instr(Inst::LdBUPcir { rd, pb, off10: off }),
                "ld.h" => Item::Instr(Inst::LdHPcir { rd, pb, off10: off }),
                "ld.hu" => Item::Instr(Inst::LdHUPcir { rd, pb, off10: off }),
                "ld.w" => Item::Instr(Inst::LdWPcir { rd, pb, off10: off }),
                _ => unreachable!(),
            }
        }
        // P[b] addressing stores: st.* [pB], dA  or  st.* [pB], dA, off
        "st.b" | "st.h" | "st.w" if comma(rest).len()==2 && comma(rest)[0].trim().starts_with("[p") => {
            let parts = comma(rest);
            let pb = parse_reg_p(&parts[0])?;
            let rs = parse_reg_d(&parts[1]).ok_or_else(|| anyhow!("bad dreg: {}", parts[1]))?;
            match mn.as_str() {
                "st.b" => Item::Instr(Inst::StBPbrP { pb, rs }),
                "st.h" => Item::Instr(Inst::StHPbrP { pb, rs }),
                "st.w" => Item::Instr(Inst::StWPbrP { pb, rs }),
                _ => unreachable!(),
            }
        }
        "st.b" | "st.h" | "st.w" if comma(rest).len()==3 && comma(rest)[0].trim().starts_with("[p") => {
            let parts = comma(rest);
            let pb = parse_reg_p(&parts[0])?;
            let rs = parse_reg_d(&parts[1]).ok_or_else(|| anyhow!("bad dreg: {}", parts[1]))?;
            let off = parse_num(parts[2].trim()).ok_or_else(|| anyhow!("bad off10: {}", parts[2]))? as i32;
            match mn.as_str() {
                "st.b" => Item::Instr(Inst::StBPcirP { pb, rs, off10: off }),
                "st.h" => Item::Instr(Inst::StHPcirP { pb, rs, off10: off }),
                "st.w" => Item::Instr(Inst::StWPcirP { pb, rs, off10: off }),
                _ => unreachable!(),
            }
        }
        "movh.a" => {
            // movh.a aC, #imm16
            let p = comma(rest);
            if p.len() != 2 { return Err(anyhow!("movh.a syntax: movh.a aC, #imm16")); }
            let rd = parse_reg_a(&p[0]).ok_or_else(|| anyhow!("bad areg: {}", p[0]))?;
            let imm = parse_num(p[1].trim_start_matches('#')).ok_or_else(|| anyhow!("bad imm: {}", p[1]))? & 0xFFFF;
            Item::Instr(Inst::MovHAa { rd, imm16: imm })
        }
        "addih.a" => {
            // addih.a aC, aA, #imm16
            let p = comma(rest);
            if p.len() != 3 { return Err(anyhow!("addih.a syntax: addih.a aC, aA, #imm16")); }
            let rd = parse_reg_a(&p[0]).ok_or_else(|| anyhow!("bad areg: {}", p[0]))?;
            let ra = parse_reg_a(&p[1]).ok_or_else(|| anyhow!("bad areg: {}", p[1]))?;
            let imm = parse_num(p[2].trim_start_matches('#')).ok_or_else(|| anyhow!("bad imm: {}", p[2]))? & 0xFFFF;
            Item::Instr(Inst::AddihA { rd, ra, imm16: imm })
        }
        "lea" => {
            // Overload existing lea for abs: lea aC, [abs]
            // If it's [aB+off], existing rule earlier handles; here accept [0xADDR]
            let p = comma(rest);
            if p.len() != 2 { return Err(anyhow!("lea syntax: lea aC, [aB+off|0xADDR]")); }
            if let Some(rd) = parse_reg_a(&p[0]) {
                let mem = p[1].trim();
                if mem.starts_with('[') && mem.contains('a') {
                    // Already handled above by lea aC, [aB+off]
                    // Fall back to default path by reusing existing parser
                    let (rb, off) = parse_mem_ab_off(mem)?;
                    Item::Instr(Inst::LeaAb { rd, rb, off: off as i32 })
                } else {
                    let ea = parse_mem_abs(mem)?;
                    Item::Instr(Inst::LeaAbs { rd, ea })
                }
            } else { return Err(anyhow!("lea: bad areg {}", p[0])); }
        }
        "mov.a" => {
            // mov.a aC, aA  => encode as lea aC, [aA+0]
            let p = comma(rest);
            if p.len() != 2 { return Err(anyhow!("mov.a syntax: mov.a aC, aA")); }
            let rd = parse_reg_a(&p[0]).ok_or_else(|| anyhow!("bad areg: {}", p[0]))?;
            let ra = parse_reg_a(&p[1]).ok_or_else(|| anyhow!("bad areg: {}", p[1]))?;
            Item::Instr(Inst::LeaAb { rd, rb: ra, off: 0 })
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
        "cmp" => {
            let p = comma(rest);
            if p.len() != 2 { return Err(anyhow!("cmp syntax: cmp dA, (dB|#imm)")); }
            let a = parse_reg_d(&p[0]).ok_or_else(|| anyhow!("bad reg: {}", p[0]))?;
            if let Some(b) = parse_reg_d(&p[1]) { Item::Instr(Inst::CmpRR { a, b, unsigned: false }) }
            else {
                let imm = parse_num(p[1].trim_start_matches('#')).ok_or_else(|| anyhow!("bad imm: {}", p[1]))?;
                Item::Instr(Inst::CmpRI { a, imm, unsigned: false })
            }
        }
        "cmp.u" => {
            let p = comma(rest);
            if p.len() != 2 { return Err(anyhow!("cmp.u syntax: cmp.u dA, (dB|#imm)")); }
            let a = parse_reg_d(&p[0]).ok_or_else(|| anyhow!("bad reg: {}", p[0]))?;
            if let Some(b) = parse_reg_d(&p[1]) { Item::Instr(Inst::CmpRR { a, b, unsigned: true }) }
            else {
                let imm = parse_num(p[1].trim_start_matches('#')).ok_or_else(|| anyhow!("bad imm: {}", p[1]))?;
                Item::Instr(Inst::CmpRI { a, imm, unsigned: true })
            }
        }
        "shl" | "shr" | "sar" | "ror" => {
            let kind = match mn.as_str() { "shl" => 0u8, "shr" => 1, "sar" => 2, "ror" => 3, _ => unreachable!() };
            let p = comma(rest);
            if p.len() != 3 { return Err(anyhow!("{} syntax: {} dC, dA, (dB|#imm)", mn, mn)); }
            let rd = parse_reg_d(&p[0]).ok_or_else(|| anyhow!("bad reg: {}", p[0]))?;
            let ra = parse_reg_d(&p[1]).ok_or_else(|| anyhow!("bad reg: {}", p[1]))?;
            if let Some(rb) = parse_reg_d(&p[2]) {
                Item::Instr(Inst::ShRR { rd, ra, rb, kind })
            } else {
                let imm = parse_num(p[2].trim_start_matches('#')).ok_or_else(|| anyhow!("bad imm: {}", p[2]))? & 31;
                Item::Instr(Inst::ShRI { rd, ra, imm, kind })
            }
        }
        "andn" => {
            let p = comma(rest);
            if p.len() != 3 { return Err(anyhow!("andn syntax: andn dC, dA, (dB|#imm)")); }
            let rd = parse_reg_d(&p[0]).ok_or_else(|| anyhow!("bad reg: {}", p[0]))?;
            let ra = parse_reg_d(&p[1]).ok_or_else(|| anyhow!("bad reg: {}", p[1]))?;
            if let Some(rb) = parse_reg_d(&p[2]) { Item::Instr(Inst::AndnRR { rd, ra, rb }) }
            else { let imm = parse_num(p[2].trim_start_matches('#')).ok_or_else(|| anyhow!("bad imm: {}", p[2]))?; Item::Instr(Inst::AndnRI { rd, ra, imm }) }
        }
        "not" => {
            let p = comma(rest);
            if p.len() != 2 { return Err(anyhow!("not syntax: not dC, dA")); }
            let rd = parse_reg_d(&p[0]).ok_or_else(|| anyhow!("bad reg: {}", p[0]))?;
            let ra = parse_reg_d(&p[1]).ok_or_else(|| anyhow!("bad reg: {}", p[1]))?;
            Item::Instr(Inst::NotR { rd, ra })
        }
        "min" | "min.u" | "max" | "max.u" => {
            let is_min = mn.starts_with("min");
            let unsigned = mn.ends_with(".u");
            let p = comma(rest);
            if p.len() != 3 { return Err(anyhow!("{} syntax: {} dC, dA, (dB|#imm)", mn, mn)); }
            let rd = parse_reg_d(&p[0]).ok_or_else(|| anyhow!("bad reg: {}", p[0]))?;
            let ra = parse_reg_d(&p[1]).ok_or_else(|| anyhow!("bad reg: {}", p[1]))?;
            if let Some(rb) = parse_reg_d(&p[2]) {
                if is_min { Item::Instr(Inst::MinRR { rd, ra, rb, unsigned }) } else { Item::Instr(Inst::MaxRR { rd, ra, rb, unsigned }) }
            } else {
                let imm = parse_num(p[2].trim_start_matches('#')).ok_or_else(|| anyhow!("bad imm: {}", p[2]))?;
                if is_min { Item::Instr(Inst::MinRI { rd, ra, imm, unsigned }) } else { Item::Instr(Inst::MaxRI { rd, ra, imm, unsigned }) }
            }
        }
        "mul" | "mul.u" => {
            let unsigned = mn.ends_with(".u");
            let p = comma(rest);
            if p.len() != 3 { return Err(anyhow!("{} syntax: {} dC, dA, (dB|#imm)", mn, mn)); }
            let rd = parse_reg_d(&p[0]).ok_or_else(|| anyhow!("bad reg: {}", p[0]))?;
            let ra = parse_reg_d(&p[1]).ok_or_else(|| anyhow!("bad reg: {}", p[1]))?;
            if let Some(rb) = parse_reg_d(&p[2]) { Item::Instr(Inst::MulRR { rd, ra, rb, unsigned }) }
            else { let imm = parse_num(p[2].trim_start_matches('#')).ok_or_else(|| anyhow!("bad imm: {}", p[2]))?; Item::Instr(Inst::MulRI { rd, ra, imm, unsigned }) }
        }
        "div" | "div.u" => {
            let unsigned = mn.ends_with(".u");
            let p = comma(rest);
            if p.len() != 3 { return Err(anyhow!("{} syntax: {} dC, dA, dB", mn, mn)); }
            let rd = parse_reg_d(&p[0]).ok_or_else(|| anyhow!("bad reg: {}", p[0]))?;
            let ra = parse_reg_d(&p[1]).ok_or_else(|| anyhow!("bad reg: {}", p[1]))?;
            let rb = parse_reg_d(&p[2]).ok_or_else(|| anyhow!("bad reg: {}", p[2]))?;
            Item::Instr(Inst::DivRR { rd, ra, rb, unsigned })
        }
        "addc" | "addx" => {
            let is_addx = mn == "addx";
            let p = comma(rest);
            if p.len() != 3 { return Err(anyhow!("{} syntax: {} dC, dA, (dB|#imm)", mn, mn)); }
            let rd = parse_reg_d(&p[0]).ok_or_else(|| anyhow!("bad reg: {}", p[0]))?;
            let ra = parse_reg_d(&p[1]).ok_or_else(|| anyhow!("bad reg: {}", p[1]))?;
            if let Some(rb) = parse_reg_d(&p[2]) {
                if is_addx { Item::Instr(Inst::AddxRR { rd, ra, rb }) } else { Item::Instr(Inst::AddcRR { rd, ra, rb }) }
            } else {
                let imm = parse_num(p[2].trim_start_matches('#')).ok_or_else(|| anyhow!("bad imm: {}", p[2]))?;
                if is_addx { Item::Instr(Inst::AddxRI { rd, ra, imm }) } else { Item::Instr(Inst::AddcRI { rd, ra, imm }) }
            }
        }
        "jeq" | "jne" if rest.contains('#') => {
            // jeq dA, #imm4, <label|abs>
            let p = comma(rest);
            if p.len() != 3 { return Err(anyhow!("{} syntax: {} dA, #imm4, <label|abs>", mn, mn)); }
            let ra = parse_reg_d(&p[0]).ok_or_else(|| anyhow!("bad reg: {}", p[0]))?;
            let imm4 = parse_num(p[1].trim_start_matches('#')).ok_or_else(|| anyhow!("bad imm: {}", p[1]))? & 0xF;
            let tgt = if let Some(v) = parse_num(p[2]) { Target::Abs(v) } else { Target::Label(p[2].to_string()) };
            if mn == "jeq" { Item::Instr(Inst::JeqImm { ra, imm4, target: tgt }) } else { Item::Instr(Inst::JneImm { ra, imm4, target: tgt }) }
        }
        // jge/jlt signed/unsigned with const4 or reg-reg
        "jge" | "jge.u" | "jlt" | "jlt.u" => {
            let unsigned = mn.ends_with(".u");
            let p = comma(rest);
            if p.len() == 3 {
                // reg, reg, target
                let a = parse_reg_d(&p[0]).ok_or_else(|| anyhow!("bad dreg: {}", p[0]))?;
                let b = parse_reg_d(&p[1]).ok_or_else(|| anyhow!("bad dreg: {}", p[1]))?;
                let tgt = if let Some(v) = parse_num(p[2]) { Target::Abs(v) } else { Target::Label(p[2].to_string()) };
                if mn.starts_with("jge") { Item::Instr(Inst::JgeRR { a, b, target: tgt, unsigned }) } else { Item::Instr(Inst::JltRR { a, b, target: tgt, unsigned }) }
            } else if p.len() == 3 && p[1].trim().starts_with('#') {
                // reg, #imm4, target
                let a = parse_reg_d(&p[0]).ok_or_else(|| anyhow!("bad dreg: {}", p[0]))?;
                let imm4 = parse_num(p[1].trim_start_matches('#')).ok_or_else(|| anyhow!("bad imm4: {}", p[1]))? & 0xF;
                let tgt = if let Some(v) = parse_num(p[2]) { Target::Abs(v) } else { Target::Label(p[2].to_string()) };
                if mn.starts_with("jge") { Item::Instr(Inst::JgeI { a, imm4, target: tgt, unsigned }) } else { Item::Instr(Inst::JltI { a, imm4, target: tgt, unsigned }) }
            } else {
                return Err(anyhow!("{} syntax: {} dA, dB, <label|abs> | {} dA, #imm4, <label|abs>", mn, mn, mn));
            }
        }
        // and/or/xor immediate const9: and dC, dA, #imm9 etc.
        "and" | "or" | "xor" if rest.contains('#') => {
            let p = comma(rest);
            if p.len() != 3 { return Err(anyhow!("{} syntax: {} dC, dA, #imm9", mn, mn)); }
            let rd = parse_reg_d(&p[0]).ok_or_else(|| anyhow!("bad dreg: {}", p[0]))?;
            let ra = parse_reg_d(&p[1]).ok_or_else(|| anyhow!("bad dreg: {}", p[1]))?;
            let imm9 = parse_num(p[2].trim_start_matches('#')).ok_or_else(|| anyhow!("bad imm: {}", p[2]))? & 0x1FF;
            match mn.as_str() {
                "and" => Item::Instr(Inst::AndRI { rd, ra, imm9 }),
                "or"  => Item::Instr(Inst::OrRI  { rd, ra, imm9 }),
                "xor" => Item::Instr(Inst::XorRI { rd, ra, imm9 }),
                _ => unreachable!(),
            }
        }
        // and/or/xor RR using 16-bit SRR when rd==ra: and dA, dA, dB
        "and" | "or" | "xor" => {
            let p = comma(rest);
            if p.len() != 3 { return Err(anyhow!("{} syntax: {} dC, dA, dB", mn, mn)); }
            let rd = parse_reg_d(&p[0]).ok_or_else(|| anyhow!("bad dreg: {}", p[0]))?;
            let ra = parse_reg_d(&p[1]).ok_or_else(|| anyhow!("bad dreg: {}", p[1]))?;
            let rb = parse_reg_d(&p[2]).ok_or_else(|| anyhow!("bad dreg: {}", p[2]))?;
            if rd != ra { return Err(anyhow!("{} RR requires rd==ra for 16-bit form", mn)); }
            match mn.as_str() {
                "and" => Item::Instr(Inst::AndRR16 { ra, rb }),
                "or"  => Item::Instr(Inst::OrRR16  { ra, rb }),
                "xor" => Item::Instr(Inst::XorRR16 { ra, rb }),
                _ => unreachable!(),
            }
        }
        // mov dC, dB (32-bit RR)
        "mov" if comma(rest).len()==2 && parse_reg_d(&comma(rest)[0]).is_some() && parse_reg_d(&comma(rest)[1]).is_some() => {
            let p = comma(rest);
            let rd = parse_reg_d(&p[0]).ok_or_else(|| anyhow!("bad dreg: {}", p[0]))?;
            let rb = parse_reg_d(&p[1]).ok_or_else(|| anyhow!("bad dreg: {}", p[1]))?;
            Item::Instr(Inst::MovRR { rd, rb })
        }
        // Extend helpers: zext.b/.h, sext.b/.h
        "zext.b" | "zext.h" | "sext.b" | "sext.h" => {
            let p = comma(rest);
            if p.len() != 2 { return Err(anyhow!("{} syntax: {} dC, dA", mn, mn)); }
            let rd = parse_reg_d(&p[0]).ok_or_else(|| anyhow!("bad dreg: {}", p[0]))?;
            let ra = parse_reg_d(&p[1]).ok_or_else(|| anyhow!("bad dreg: {}", p[1]))?;
            match mn.as_str() {
                "zext.b" => Item::Instr(Inst::ZextB { rd, ra }),
                "zext.h" => Item::Instr(Inst::ZextH { rd, ra }),
                "sext.b" => Item::Instr(Inst::SextB { rd, ra }),
                "sext.h" => Item::Instr(Inst::SextH { rd, ra }),
                _ => unreachable!(),
            }
        }
        // A-register branches with label/abs target (32-bit BRR)
        "jeq.a" | "jne.a" => {
            let p = comma(rest);
            if p.len() != 3 { return Err(anyhow!("{} syntax: {} aA, aB, <label|abs>", mn, mn)); }
            let ra = parse_reg_a(&p[0]).ok_or_else(|| anyhow!("bad areg: {}", p[0]))?;
            let rb = parse_reg_a(&p[1]).ok_or_else(|| anyhow!("bad areg: {}", p[1]))?;
            let tgt = if let Some(v) = parse_num(p[2]) { Target::Abs(v) } else { Target::Label(p[2].to_string()) };
            if mn == "jeq.a" { Item::Instr(Inst::JeqARR { ra, rb, target: tgt }) } else { Item::Instr(Inst::JneARR { ra, rb, target: tgt }) }
        }
        "jz.a" | "jnz.a" => {
            let p = comma(rest);
            if p.len() != 2 { return Err(anyhow!("{} syntax: {} aA, <label|abs>", mn, mn)); }
            let ra = parse_reg_a(&p[0]).ok_or_else(|| anyhow!("bad areg: {}", p[0]))?;
            let tgt = if let Some(v) = parse_num(p[1]) { Target::Abs(v) } else { Target::Label(p[1].to_string()) };
            if mn == "jz.a" { Item::Instr(Inst::JzAR { ra, target: tgt }) } else { Item::Instr(Inst::JnzAR { ra, target: tgt }) }
        }
        "beq" | "bne" | "bge" | "blt" | "bge.u" | "blt.u" => {
            let kind = match mn.as_str() { "beq" => 0u8, "bne" => 1, "bge" => 2, "blt" => 3, "bge.u" => 4, "blt.u" => 5, _ => 0 };
            let t = rest.trim();
            let target = if let Some(v) = parse_num(t) { Target::Abs(v) } else { Target::Label(t.to_string()) };
            Item::Instr(Inst::BFlag { kind, target })
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

fn parse_mem_abs(s: &str) -> Result<u32> {
    let st = s.trim();
    if !st.starts_with('[') || !st.ends_with(']') { return Err(anyhow!("expected absolute mem operand like [0xADDR]: {}", s)); }
    let inner = &st[1..st.len()-1];
    parse_num(inner).ok_or_else(|| anyhow!("bad absolute addr: {}", s))
}

fn abs_off18_fields(ea: u32, sel: Option<u32>) -> (u32, u32, u32, u32) {
    // off18 = {ea[31:28], ea[13:0]}
    let top4 = (ea >> 28) & 0xF;
    let low14 = ea & 0x3FFF;
    let off18 = (top4 << 14) | low14;
    let off17_14 = (off18 >> 14) & 0xF;
    let off13_10 = (off18 >> 10) & 0xF;
    let off9_6 = sel.unwrap_or((off18 >> 6) & 0xF);
    let off5_0 = off18 & 0x3F;
    // Return split fields: used at positions [15:12], [25:22], [31:28], [21:16]
    (off17_14, off13_10, off9_6, off5_0)
}

fn parse_reg_p(s: &str) -> Result<u32> {
    let st = s.trim();
    if !st.starts_with('[') || !st.ends_with(']') { return Err(anyhow!("expected [pN]: {}", s)); }
    let inner = &st[1..st.len()-1];
    if !inner.starts_with('p') { return Err(anyhow!("expected [pN]: {}", s)); }
    let n: u32 = inner[1..].parse().map_err(|_| anyhow!("bad p-reg: {}", s))?;
    Ok(n & 0xF)
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
        Item::Instr(Inst::LdBOff16{..}) => 4,
        Item::Instr(Inst::LdHOff16{..}) => 4,
        Item::Instr(Inst::LdHuOff16{..}) => 4,
        Item::Instr(Inst::LdWOff16{..}) => 4,
        Item::Instr(Inst::StHOff16{..}) => 4,
            Item::Instr(Inst::StWOff16{..}) => 4,
        Item::Instr(Inst::LdBAbs{..}) | Item::Instr(Inst::LdBuAbs{..}) | Item::Instr(Inst::LdHAbs{..}) | Item::Instr(Inst::LdHuAbs{..}) | Item::Instr(Inst::LdWAbs{..}) => 4,
        Item::Instr(Inst::StBAbs{..}) | Item::Instr(Inst::StHAbs{..}) | Item::Instr(Inst::StWAbs{..}) => 4,
        Item::Instr(Inst::JneRR{..}) | Item::Instr(Inst::JeqRR{..}) | Item::Instr(Inst::JgeURR{..}) | Item::Instr(Inst::JltURR{..}) => 4,
        Item::Instr(Inst::LeaAb{..}) => 4,
        Item::Instr(Inst::CmpRR{..}) => 4,
        Item::Instr(Inst::CmpRI{..}) => 4,
        Item::Instr(Inst::ShRR{..}) => 4,
        Item::Instr(Inst::ShRI{..}) => 4,
        Item::Instr(Inst::AndnRR{..}) | Item::Instr(Inst::AndnRI{..}) => 4,
        Item::Instr(Inst::NotR{..}) => 4,
        Item::Instr(Inst::MinRR{..}) | Item::Instr(Inst::MaxRR{..}) | Item::Instr(Inst::MinRI{..}) | Item::Instr(Inst::MaxRI{..}) => 4,
        Item::Instr(Inst::MulRR{..}) | Item::Instr(Inst::MulRI{..}) | Item::Instr(Inst::DivRR{..}) => 4,
        Item::Instr(Inst::BFlag{..}) => 4,
        Item::Instr(Inst::AddcRR{..}) | Item::Instr(Inst::AddcRI{..}) | Item::Instr(Inst::AddxRR{..}) | Item::Instr(Inst::AddxRI{..}) => 4,
        Item::Instr(Inst::JeqImm{..}) | Item::Instr(Inst::JneImm{..}) => 4,
        Item::Instr(Inst::AndRI{..}) | Item::Instr(Inst::OrRI{..}) | Item::Instr(Inst::XorRI{..}) => 4,
        Item::Instr(Inst::JeqARR{..}) | Item::Instr(Inst::JneARR{..}) | Item::Instr(Inst::JzAR{..}) | Item::Instr(Inst::JnzAR{..}) => 4,
        Item::Instr(Inst::AndRR16{..}) | Item::Instr(Inst::OrRR16{..}) | Item::Instr(Inst::XorRR16{..}) => 2,
        Item::Instr(Inst::MovRR{..}) => 4,
        Item::Instr(Inst::ZextB{..}) | Item::Instr(Inst::ZextH{..}) | Item::Instr(Inst::SextB{..}) | Item::Instr(Inst::SextH{..}) => 12, // worst-case 3 x 4B
        Item::Instr(Inst::JgeRR{..}) | Item::Instr(Inst::JltRR{..}) | Item::Instr(Inst::JgeI{..}) | Item::Instr(Inst::JltI{..}) => 4,
        // P[b] addressing widths
        Item::Instr(Inst::LdBPbr{..}) | Item::Instr(Inst::LdBUPbr{..}) | Item::Instr(Inst::LdHPbr{..}) | Item::Instr(Inst::LdHUPbr{..}) | Item::Instr(Inst::LdWPbr{..}) => 4,
        Item::Instr(Inst::LdBPcir{..}) | Item::Instr(Inst::LdBUPcir{..}) | Item::Instr(Inst::LdHPcir{..}) | Item::Instr(Inst::LdHUPcir{..}) | Item::Instr(Inst::LdWPcir{..}) => 4,
        Item::Instr(Inst::StBPbrP{..}) | Item::Instr(Inst::StHPbrP{..}) | Item::Instr(Inst::StWPbrP{..}) => 4,
        Item::Instr(Inst::StBPcirP{..}) | Item::Instr(Inst::StHPcirP{..}) | Item::Instr(Inst::StWPcirP{..}) => 4,
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
            Item::Instr(Inst::LdBOff16{ rd, ab, off16 }) => {
                let off = off16 & 0xFFFF;
                let off_hi4 = (off >> 6) & 0xF;
                let off_mid6 = (off >> 10) & 0x3F;
                let off_lo6 = off & 0x3F;
                let raw = (off_hi4 << 28) | (off_mid6 << 22) | (off_lo6 << 16) | ((*ab & 0xF) << 12) | ((*rd & 0xF) << 8) | 0x79;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::LdHOff16{ rd, ab, off16 }) => {
                let off = off16 & 0xFFFF;
                let off_hi4 = (off >> 6) & 0xF;
                let off_mid6 = (off >> 10) & 0x3F;
                let off_lo6 = off & 0x3F;
                let raw = (off_hi4 << 28) | (off_mid6 << 22) | (off_lo6 << 16) | ((*ab & 0xF) << 12) | ((*rd & 0xF) << 8) | 0xC9;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::LdHuOff16{ rd, ab, off16 }) => {
                let off = off16 & 0xFFFF;
                let off_hi4 = (off >> 6) & 0xF;
                let off_mid6 = (off >> 10) & 0x3F;
                let off_lo6 = off & 0x3F;
                let raw = (off_hi4 << 28) | (off_mid6 << 22) | (off_lo6 << 16) | ((*ab & 0xF) << 12) | ((*rd & 0xF) << 8) | 0xB9;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::LdWOff16{ rd, ab, off16 }) => {
                let off = off16 & 0xFFFF;
                let off_hi4 = (off >> 6) & 0xF;
                let off_mid6 = (off >> 10) & 0x3F;
                let off_lo6 = off & 0x3F;
                let raw = (off_hi4 << 28) | (off_mid6 << 22) | (off_lo6 << 16) | ((*ab & 0xF) << 12) | ((*rd & 0xF) << 8) | 0x19;
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
            Item::Instr(Inst::StHOff16{ ab, rs, off16 }) => {
                let off = off16 & 0xFFFF;
                let off_hi4 = (off >> 6) & 0xF;
                let off_mid6 = (off >> 10) & 0x3F;
                let off_lo6 = off & 0x3F;
                let raw = (off_hi4 << 28) | (off_mid6 << 22) | (off_lo6 << 16) | ((*ab & 0xF) << 12) | ((*rs & 0xF) << 8) | 0xF9;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::StWOff16{ ab, rs, off16 }) => {
                let off = off16 & 0xFFFF;
                let off_hi4 = (off >> 6) & 0xF;
                let off_mid6 = (off >> 10) & 0x3F;
                let off_lo6 = off & 0x3F;
                let raw = (off_hi4 << 28) | (off_mid6 << 22) | (off_lo6 << 16) | ((*ab & 0xF) << 12) | ((*rs & 0xF) << 8) | 0x59;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::LdBAbs { rd, ea }) => {
                let (off17_14, off13_10, _off9_6_sel, off5_0) = abs_off18_fields(*ea, Some(0x0));
                let raw = (0x0 << 28) | (off13_10 << 22) | (off5_0 << 16) | (off17_14 << 12) | (((*rd & 0xF) as u32) << 8) | 0x05;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::LdBuAbs { rd, ea }) => {
                let (off17_14, off13_10, _sel, off5_0) = abs_off18_fields(*ea, Some(0x1));
                let raw = (0x1 << 28) | (off13_10 << 22) | (off5_0 << 16) | (off17_14 << 12) | (((*rd & 0xF) as u32) << 8) | 0x05;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::LdHAbs { rd, ea }) => {
                let (off17_14, off13_10, _sel, off5_0) = abs_off18_fields(*ea, Some(0x2));
                let raw = (0x2 << 28) | (off13_10 << 22) | (off5_0 << 16) | (off17_14 << 12) | (((*rd & 0xF) as u32) << 8) | 0x05;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::LdHuAbs { rd, ea }) => {
                let (off17_14, off13_10, _sel, off5_0) = abs_off18_fields(*ea, Some(0x3));
                let raw = (0x3 << 28) | (off13_10 << 22) | (off5_0 << 16) | (off17_14 << 12) | (((*rd & 0xF) as u32) << 8) | 0x05;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::LdWAbs { rd, ea }) => {
                let (off17_14, off13_10, _sel, off5_0) = abs_off18_fields(*ea, Some(0x0));
                let raw = (0x0 << 28) | (off13_10 << 22) | (off5_0 << 16) | (off17_14 << 12) | (((*rd & 0xF) as u32) << 8) | 0x85;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::StBAbs { rs, ea }) => {
                let (off17_14, off13_10, _sel, off5_0) = abs_off18_fields(*ea, Some(0x0));
                let raw = (0x0 << 28) | (off13_10 << 22) | (off5_0 << 16) | (off17_14 << 12) | (((*rs & 0xF) as u32) << 8) | 0x25;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::StHAbs { rs, ea }) => {
                let (off17_14, off13_10, _sel, off5_0) = abs_off18_fields(*ea, Some(0x2));
                let raw = (0x2 << 28) | (off13_10 << 22) | (off5_0 << 16) | (off17_14 << 12) | (((*rs & 0xF) as u32) << 8) | 0x25;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::StWAbs { rs, ea }) => {
                let (off17_14, off13_10, _sel, off5_0) = abs_off18_fields(*ea, Some(0x0));
                let raw = (0x0 << 28) | (off13_10 << 22) | (off5_0 << 16) | (off17_14 << 12) | (((*rs & 0xF) as u32) << 8) | 0xA5;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::MovHAa { rd, imm16 }) => {
                // op1=0x91; rd in [31:28], imm16 in [27:12]
                let raw = (((*rd & 0xF) as u32) << 28) | (((*imm16 & 0xFFFF) as u32) << 12) | 0x91;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::AddihA { rd, ra, imm16 }) => {
                // op1=0x11; rd [31:28], ra [11:8], imm16 [27:12]
                let raw = (((*rd & 0xF) as u32) << 28) | (((*imm16 & 0xFFFF) as u32) << 12) | (((*ra & 0xF) as u32) << 8) | 0x11;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::LeaAbs { rd, ea }) => {
                // op1=0xC5; off18 from EA
                let (off17_14, off13_10, _sel, off5_0) = abs_off18_fields(*ea, None);
                // off fields placed across 31:28, 25:22, 21:16 in tc16; this op combines via helper used above in decoder
                let raw = (off13_10 << 22) | (off5_0 << 16) | (off17_14 << 12) | (((*rd & 0xF) as u32) << 8) | 0xC5;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            // P[b] addressing encoders (loads)
            Item::Instr(Inst::LdBPbr { rd, pb }) => { let raw = (0x00 << 22) | (((*pb & 0xF) as u32) << 12) | (((*rd & 0xF) as u32) << 8) | 0x29; out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4; }
            Item::Instr(Inst::LdBUPbr { rd, pb }) => { let raw = (0x01 << 22) | (((*pb & 0xF) as u32) << 12) | (((*rd & 0xF) as u32) << 8) | 0x29; out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4; }
            Item::Instr(Inst::LdHPbr { rd, pb }) => { let raw = (0x02 << 22) | (((*pb & 0xF) as u32) << 12) | (((*rd & 0xF) as u32) << 8) | 0x29; out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4; }
            Item::Instr(Inst::LdHUPbr { rd, pb }) => { let raw = (0x03 << 22) | (((*pb & 0xF) as u32) << 12) | (((*rd & 0xF) as u32) << 8) | 0x29; out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4; }
            Item::Instr(Inst::LdWPbr { rd, pb }) => { let raw = (0x04 << 22) | (((*pb & 0xF) as u32) << 12) | (((*rd & 0xF) as u32) << 8) | 0x29; out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4; }
            Item::Instr(Inst::LdBPcir { rd, pb, off10 }) => {
                let u = ((*off10 as i32) as i32) as i32; let u = u as i32; // keep sign
                let v = (*off10 as i32) as i32;
                let off = (*off10 as i32) as i32;
                let u10 = ((*off10 as i32) & 0x3FF) as u32;
                let hi4 = (u10 >> 6) & 0xF; let lo6 = u10 & 0x3F;
                let raw = (hi4 << 28) | (0x10 << 22) | (lo6 << 16) | (((*pb & 0xF) as u32) << 12) | (((*rd & 0xF) as u32) << 8) | 0x29; out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::LdBUPcir { rd, pb, off10 }) => { let u10 = ((*off10 as i32) & 0x3FF) as u32; let hi4=(u10>>6)&0xF; let lo6=u10&0x3F; let raw=(hi4<<28)|(0x11<<22)|(lo6<<16)|(((*pb&0xF)as u32)<<12)|(((*rd&0xF)as u32)<<8)|0x29; out.extend_from_slice(&(raw as u32).to_le_bytes()); pc+=4; }
            Item::Instr(Inst::LdHPcir { rd, pb, off10 }) => { let u10 = ((*off10 as i32) & 0x3FF) as u32; let hi4=(u10>>6)&0xF; let lo6=u10&0x3F; let raw=(hi4<<28)|(0x12<<22)|(lo6<<16)|(((*pb&0xF)as u32)<<12)|(((*rd&0xF)as u32)<<8)|0x29; out.extend_from_slice(&(raw as u32).to_le_bytes()); pc+=4; }
            Item::Instr(Inst::LdHUPcir { rd, pb, off10 }) => { let u10 = ((*off10 as i32) & 0x3FF) as u32; let hi4=(u10>>6)&0xF; let lo6=u10&0x3F; let raw=(hi4<<28)|(0x13<<22)|(lo6<<16)|(((*pb&0xF)as u32)<<12)|(((*rd&0xF)as u32)<<8)|0x29; out.extend_from_slice(&(raw as u32).to_le_bytes()); pc+=4; }
            Item::Instr(Inst::LdWPcir { rd, pb, off10 }) => { let u10 = ((*off10 as i32) & 0x3FF) as u32; let hi4=(u10>>6)&0xF; let lo6=u10&0x3F; let raw=(hi4<<28)|(0x14<<22)|(lo6<<16)|(((*pb&0xF)as u32)<<12)|(((*rd&0xF)as u32)<<8)|0x29; out.extend_from_slice(&(raw as u32).to_le_bytes()); pc+=4; }
            // P[b] stores
            Item::Instr(Inst::StBPbrP { pb, rs }) => { let raw=(0x00<<22)|(((*pb&0xF)as u32)<<12)|(((*rs&0xF)as u32)<<8)|0xA9; out.extend_from_slice(&(raw as u32).to_le_bytes()); pc+=4; }
            Item::Instr(Inst::StHPbrP { pb, rs }) => { let raw=(0x02<<22)|(((*pb&0xF)as u32)<<12)|(((*rs&0xF)as u32)<<8)|0xA9; out.extend_from_slice(&(raw as u32).to_le_bytes()); pc+=4; }
            Item::Instr(Inst::StWPbrP { pb, rs }) => { let raw=(0x04<<22)|(((*pb&0xF)as u32)<<12)|(((*rs&0xF)as u32)<<8)|0xA9; out.extend_from_slice(&(raw as u32).to_le_bytes()); pc+=4; }
            Item::Instr(Inst::StBPcirP { pb, rs, off10 }) => { let u10=(((*off10 as i32)&0x3FF)as u32); let hi4=(u10>>6)&0xF; let lo6=u10&0x3F; let raw=(hi4<<28)|(0x10<<22)|(lo6<<16)|(((*pb&0xF)as u32)<<12)|(((*rs&0xF)as u32)<<8)|0xA9; out.extend_from_slice(&(raw as u32).to_le_bytes()); pc+=4; }
            Item::Instr(Inst::StHPcirP { pb, rs, off10 }) => { let u10=(((*off10 as i32)&0x3FF)as u32); let hi4=(u10>>6)&0xF; let lo6=u10&0x3F; let raw=(hi4<<28)|(0x12<<22)|(lo6<<16)|(((*pb&0xF)as u32)<<12)|(((*rs&0xF)as u32)<<8)|0xA9; out.extend_from_slice(&(raw as u32).to_le_bytes()); pc+=4; }
            Item::Instr(Inst::StWPcirP { pb, rs, off10 }) => { let u10=(((*off10 as i32)&0x3FF)as u32); let hi4=(u10>>6)&0xF; let lo6=u10&0x3F; let raw=(hi4<<28)|(0x14<<22)|(lo6<<16)|(((*pb&0xF)as u32)<<12)|(((*rs&0xF)as u32)<<8)|0xA9; out.extend_from_slice(&(raw as u32).to_le_bytes()); pc+=4; }
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
            Item::Instr(Inst::CmpRR { a, b, unsigned }) => {
                // RR pseudo encodings: op1=0x0B, op2=0x18 (signed) or 0x19 (unsigned)
                let op2 = if *unsigned { 0x19 } else { 0x18 };
                let raw = (((*b & 0xF) as u32) << 16) | ((op2 as u32) << 20) | (((*a & 0xF) as u32) << 8) | 0x0B;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::CmpRI { a, imm, unsigned }) => {
                // RC pseudo encodings: op1=0x8B, op2=0x18 (signed imm9) or 0x19 (unsigned imm9)
                let op2 = if *unsigned { 0x19 } else { 0x18 };
                let imm9 = (*imm & 0x1FF) as u32;
                let raw = (op2 << 21) | (imm9 << 12) | (((*a & 0xF) as u32) << 8) | 0x8B;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::ShRR { rd, ra, rb, kind }) => {
                let op2 = match *kind { 0 => 0x20, 1 => 0x21, 2 => 0x22, 3 => 0x23, _ => 0x20 };
                let raw = (((*rd & 0xF) as u32) << 28) | (((*rb & 0xF) as u32) << 16) | ((op2 as u32) << 20) | (((*ra & 0xF) as u32) << 8) | 0x0B;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::ShRI { rd, ra, imm, kind }) => {
                let op2 = match *kind { 0 => 0x20, 1 => 0x21, 2 => 0x22, 3 => 0x23, _ => 0x20 };
                let imm9 = (*imm & 0x1FF) as u32;
                let raw = ((op2 as u32) << 21) | (imm9 << 12) | (((*ra & 0xF) as u32) << 8) | (((*rd & 0xF) as u32) << 28) | 0x8B;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::AndnRR { rd, ra, rb }) => {
                let raw = (((*rd & 0xF) as u32) << 28) | (((*rb & 0xF) as u32) << 16) | (0x24 << 20) | (((*ra & 0xF) as u32) << 8) | 0x0B;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::AndnRI { rd, ra, imm }) => {
                let imm9 = (*imm & 0x1FF) as u32;
                let raw = (0x24 << 21) | (imm9 << 12) | (((*ra & 0xF) as u32) << 8) | (((*rd & 0xF) as u32) << 28) | 0x8B;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::NotR { rd, ra }) => {
                let raw = (((*rd & 0xF) as u32) << 28) | (0x25 << 20) | (((*ra & 0xF) as u32) << 8) | 0x0B;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::MinRR { rd, ra, rb, unsigned }) => {
                let op2 = if *unsigned { 0x28 } else { 0x26 };
                let raw = (((*rd & 0xF) as u32) << 28) | (((*rb & 0xF) as u32) << 16) | (op2 << 20) | (((*ra & 0xF) as u32) << 8) | 0x0B;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::MinRI { rd, ra, imm, unsigned }) => {
                let op2 = if *unsigned { 0x28 } else { 0x26 };
                let imm9 = (*imm & 0x1FF) as u32;
                let raw = (op2 << 21) | (imm9 << 12) | (((*ra & 0xF) as u32) << 8) | (((*rd & 0xF) as u32) << 28) | 0x8B;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::MaxRR { rd, ra, rb, unsigned }) => {
                let op2 = if *unsigned { 0x29 } else { 0x27 };
                let raw = (((*rd & 0xF) as u32) << 28) | (((*rb & 0xF) as u32) << 16) | (op2 << 20) | (((*ra & 0xF) as u32) << 8) | 0x0B;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::MaxRI { rd, ra, imm, unsigned }) => {
                let op2 = if *unsigned { 0x29 } else { 0x27 };
                let imm9 = (*imm & 0x1FF) as u32;
                let raw = (op2 << 21) | (imm9 << 12) | (((*ra & 0xF) as u32) << 8) | (((*rd & 0xF) as u32) << 28) | 0x8B;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::MulRR { rd, ra, rb, unsigned }) => {
                let op2 = if *unsigned { 0x2D } else { 0x2C };
                let raw = (((*rd & 0xF) as u32) << 28) | (((*rb & 0xF) as u32) << 16) | (op2 << 20) | (((*ra & 0xF) as u32) << 8) | 0x0B;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::MulRI { rd, ra, imm, unsigned }) => {
                let op2 = if *unsigned { 0x2D } else { 0x2C };
                let imm9 = (*imm & 0x1FF) as u32;
                let raw = (op2 << 21) | (imm9 << 12) | (((*ra & 0xF) as u32) << 8) | (((*rd & 0xF) as u32) << 28) | 0x8B;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::DivRR { rd, ra, rb, unsigned }) => {
                let op2 = if *unsigned { 0x2F } else { 0x2E };
                let raw = (((*rd & 0xF) as u32) << 28) | (((*rb & 0xF) as u32) << 16) | (op2 << 20) | (((*ra & 0xF) as u32) << 8) | 0x0B;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::BFlag { kind, target }) => {
                let tgt = match target { Target::Abs(v) => *v, Target::Label(l) => *labels.get(l).ok_or_else(|| anyhow!("unknown label: {}", l))? };
                let off = (tgt as i64) - (pc as i64) - 4;
                let disp15 = (off >> 1) as i32;
                if disp15 < -(1<<14) || disp15 >= (1<<14) { return Err(anyhow!("branch target out of range")); }
                let d15 = (disp15 as u32) & 0x7FFF;
                // For now, cond mapping 0..3 to beq/bne/bge/blt in decoder 0x4D
                let cond = match *kind { 0 => 0x0, 1 => 0x1, 2 => 0x2, 3 => 0x3, 4 => 0x2, 5 => 0x3, _ => 0x0 };
                let raw = (cond << 30) | (d15 << 15) | 0x4D;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::AddcRR { rd, ra, rb }) => {
                let raw = (((*rd & 0xF) as u32) << 28) | (((*rb & 0xF) as u32) << 16) | (0x05 << 20) | (((*ra & 0xF) as u32) << 8) | 0x0B;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::AddcRI { rd, ra, imm }) => {
                let imm9 = (*imm & 0x1FF) as u32;
                let raw = (0x05 << 21) | (imm9 << 12) | (((*ra & 0xF) as u32) << 8) | (((*rd & 0xF) as u32) << 28) | 0x8B;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::AddxRR { rd, ra, rb }) => {
                let raw = (((*rd & 0xF) as u32) << 28) | (((*rb & 0xF) as u32) << 16) | (0x04 << 20) | (((*ra & 0xF) as u32) << 8) | 0x0B;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::AddxRI { rd, ra, imm }) => {
                let imm9 = (*imm & 0x1FF) as u32;
                let raw = (0x04 << 21) | (imm9 << 12) | (((*ra & 0xF) as u32) << 8) | (((*rd & 0xF) as u32) << 28) | 0x8B;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::JeqImm { ra, imm4, target }) => {
                let tgt = match target { Target::Abs(v) => *v, Target::Label(l) => *labels.get(l).ok_or_else(|| anyhow!("unknown label: {}", l))? };
                let off = (tgt as i64) - (pc as i64) - 4;
                let disp15 = (off >> 1) as i32;
                if disp15 < -(1<<14) || disp15 >= (1<<14) { return Err(anyhow!("jeq const target out of range")); }
                let d15 = (disp15 as u32) & 0x7FFF;
                let raw = (0x0 << 30) | (((*ra & 0xF) as u32) << 8) | (((*imm4 & 0xF) as u32) << 12) | (d15 << 15) | 0xDF;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::JneImm { ra, imm4, target }) => {
                let tgt = match target { Target::Abs(v) => *v, Target::Label(l) => *labels.get(l).ok_or_else(|| anyhow!("unknown label: {}", l))? };
                let off = (tgt as i64) - (pc as i64) - 4;
                let disp15 = (off >> 1) as i32;
                if disp15 < -(1<<14) || disp15 >= (1<<14) { return Err(anyhow!("jne const target out of range")); }
                let d15 = (disp15 as u32) & 0x7FFF;
                let raw = (0x1 << 30) | (((*ra & 0xF) as u32) << 8) | (((*imm4 & 0xF) as u32) << 12) | (d15 << 15) | 0xDF;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            // Logical RC immediates via op1=0x8F, op2=0x08/0x0A/0x0C
            Item::Instr(Inst::AndRI { rd, ra, imm9 }) => {
                let raw = (0x08 << 21) | ((*imm9 & 0x1FF) << 12) | (((*ra & 0xF) as u32) << 8) | (((*rd & 0xF) as u32) << 28) | 0x8F;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::OrRI { rd, ra, imm9 }) => {
                let raw = (0x0A << 21) | ((*imm9 & 0x1FF) << 12) | (((*ra & 0xF) as u32) << 8) | (((*rd & 0xF) as u32) << 28) | 0x8F;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::XorRI { rd, ra, imm9 }) => {
                let raw = (0x0C << 21) | ((*imm9 & 0x1FF) << 12) | (((*ra & 0xF) as u32) << 8) | (((*rd & 0xF) as u32) << 28) | 0x8F;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            // jge/jlt RR encodings via BRR (0x7F for jge, 0x3F for jlt); unsigned uses cond bit 0x01 in [31:30]
            Item::Instr(Inst::JgeRR { a, b, target, unsigned }) => {
                let tgt = match target { Target::Abs(v) => *v, Target::Label(l) => *labels.get(l).ok_or_else(|| anyhow!("unknown label: {}", l))? };
                let off = (tgt as i64) - (pc as i64) - 4;
                let disp15 = (off >> 1) as i32; if disp15 < -(1<<14) || disp15 >= (1<<14) { return Err(anyhow!("jge target out of range")); }
                let d15 = (disp15 as u32) & 0x7FFF; let cond = if *unsigned { 0x01 } else { 0x00 };
                let raw = (cond << 30) | (d15 << 15) | (((*b & 0xF) as u32) << 12) | (((*a & 0xF) as u32) << 8) | 0x7F;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::JltRR { a, b, target, unsigned }) => {
                let tgt = match target { Target::Abs(v) => *v, Target::Label(l) => *labels.get(l).ok_or_else(|| anyhow!("unknown label: {}", l))? };
                let off = (tgt as i64) - (pc as i64) - 4;
                let disp15 = (off >> 1) as i32; if disp15 < -(1<<14) || disp15 >= (1<<14) { return Err(anyhow!("jlt target out of range")); }
                let d15 = (disp15 as u32) & 0x7FFF; let cond = if *unsigned { 0x01 } else { 0x00 };
                let raw = (cond << 30) | (d15 << 15) | (((*b & 0xF) as u32) << 12) | (((*a & 0xF) as u32) << 8) | 0x3F;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            // jge/jlt immediate const4 via BRC (0xFF for jge, 0xBF for jlt)
            Item::Instr(Inst::JgeI { a, imm4, target, unsigned }) => {
                let tgt = match target { Target::Abs(v) => *v, Target::Label(l) => *labels.get(l).ok_or_else(|| anyhow!("unknown label: {}", l))? };
                let off = (tgt as i64) - (pc as i64) - 4; let disp15 = (off >> 1) as i32; if disp15 < -(1<<14) || disp15 >= (1<<14) { return Err(anyhow!("jge imm target out of range")); }
                let d15=(disp15 as u32)&0x7FFF; let cond = if *unsigned { 0x01 } else { 0x00 };
                let raw = (cond << 30) | (d15 << 15) | (((*imm4 & 0xF) as u32) << 12) | (((*a & 0xF) as u32) << 8) | 0xFF;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::JltI { a, imm4, target, unsigned }) => {
                let tgt = match target { Target::Abs(v) => *v, Target::Label(l) => *labels.get(l).ok_or_else(|| anyhow!("unknown label: {}", l))? };
                let off = (tgt as i64) - (pc as i64) - 4; let disp15 = (off >> 1) as i32; if disp15 < -(1<<14) || disp15 >= (1<<14) { return Err(anyhow!("jlt imm target out of range")); }
                let d15=(disp15 as u32)&0x7FFF; let cond = if *unsigned { 0x01 } else { 0x00 };
                let raw = (cond << 30) | (d15 << 15) | (((*imm4 & 0xF) as u32) << 12) | (((*a & 0xF) as u32) << 8) | 0xBF;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            // A-register branches (BRR)
            Item::Instr(Inst::JeqARR { ra, rb, target }) | Item::Instr(Inst::JneARR { ra, rb, target }) => {
                let tgt = match target { Target::Abs(v) => *v, Target::Label(l) => *labels.get(l).ok_or_else(|| anyhow!("unknown label: {}", l))? };
                let off = (tgt as i64) - (pc as i64) - 4;
                let disp15 = (off >> 1) as i32;
                if disp15 < -(1<<14) || disp15 >= (1<<14) { return Err(anyhow!("branch target out of range")); }
                let d15 = (disp15 as u32) & 0x7FFF;
                let cond = if matches!(it, Item::Instr(Inst::JneARR{..})) { 0x1 } else { 0x0 };
                let raw = (cond << 30) | (d15 << 15) | (((*rb & 0xF) as u32) << 12) | (((*ra & 0xF) as u32) << 8) | 0x7D;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::JzAR { ra, target }) | Item::Instr(Inst::JnzAR { ra, target }) => {
                let tgt = match target { Target::Abs(v) => *v, Target::Label(l) => *labels.get(l).ok_or_else(|| anyhow!("unknown label: {}", l))? };
                let off = (tgt as i64) - (pc as i64) - 4;
                let disp15 = (off >> 1) as i32;
                if disp15 < -(1<<14) || disp15 >= (1<<14) { return Err(anyhow!("branch target out of range")); }
                let d15 = (disp15 as u32) & 0x7FFF;
                let cond = if matches!(it, Item::Instr(Inst::JnzAR{..})) { 0x1 } else { 0x0 };
                let raw = (cond << 30) | (d15 << 15) | (((*ra & 0xF) as u32) << 8) | 0xBD;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            // 16-bit SRR logical RR: op1 low byte 0x26 (and), 0xA6 (or), 0xC6 (xor)
            Item::Instr(Inst::AndRR16 { ra, rb }) => {
                let raw16: u16 = (((*rb & 0xF) as u16) << 12) | (((*ra & 0xF) as u16) << 8) | 0x26u16;
                out.extend_from_slice(&raw16.to_le_bytes()); pc += 2;
            }
            Item::Instr(Inst::OrRR16 { ra, rb }) => {
                let raw16: u16 = (((*rb & 0xF) as u16) << 12) | (((*ra & 0xF) as u16) << 8) | 0xA6u16;
                out.extend_from_slice(&raw16.to_le_bytes()); pc += 2;
            }
            Item::Instr(Inst::XorRR16 { ra, rb }) => {
                let raw16: u16 = (((*rb & 0xF) as u16) << 12) | (((*ra & 0xF) as u16) << 8) | 0xC6u16;
                out.extend_from_slice(&raw16.to_le_bytes()); pc += 2;
            }
            // 32-bit RR mov: op1=0x0B, op2=0x1F, rd in [31:28], rb in [19:16]
            Item::Instr(Inst::MovRR { rd, rb }) => {
                let raw = (((*rd & 0xF) as u32) << 28) | (((*rb & 0xF) as u32) << 16) | (0x1F << 20) | 0x0B;
                out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4;
            }
            // Extend macros: mov rd, ra; shl; shr/sar
            Item::Instr(Inst::ZextB { rd, ra }) => {
                if rd != ra { let raw = (((*rd & 0xF) as u32) << 28) | (((*ra & 0xF) as u32) << 16) | (0x1F << 20) | 0x0B; out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4; }
                // shl 24
                let raw_shl = (0x20 << 21) | ((24 & 0x1FF) << 12) | (((*rd & 0xF) as u32) << 8) | (((*rd & 0xF) as u32) << 28) | 0x8B;
                out.extend_from_slice(&(raw_shl as u32).to_le_bytes()); pc += 4;
                // shr 24
                let raw_shr = (0x21 << 21) | ((24 & 0x1FF) << 12) | (((*rd & 0xF) as u32) << 8) | (((*rd & 0xF) as u32) << 28) | 0x8B;
                out.extend_from_slice(&(raw_shr as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::ZextH { rd, ra }) => {
                if rd != ra { let raw = (((*rd & 0xF) as u32) << 28) | (((*ra & 0xF) as u32) << 16) | (0x1F << 20) | 0x0B; out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4; }
                let raw_shl = (0x20 << 21) | ((16 & 0x1FF) << 12) | (((*rd & 0xF) as u32) << 8) | (((*rd & 0xF) as u32) << 28) | 0x8B;
                out.extend_from_slice(&(raw_shl as u32).to_le_bytes()); pc += 4;
                let raw_shr = (0x21 << 21) | ((16 & 0x1FF) << 12) | (((*rd & 0xF) as u32) << 8) | (((*rd & 0xF) as u32) << 28) | 0x8B;
                out.extend_from_slice(&(raw_shr as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::SextB { rd, ra }) => {
                if rd != ra { let raw = (((*rd & 0xF) as u32) << 28) | (((*ra & 0xF) as u32) << 16) | (0x1F << 20) | 0x0B; out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4; }
                let raw_shl = (0x20 << 21) | ((24 & 0x1FF) << 12) | (((*rd & 0xF) as u32) << 8) | (((*rd & 0xF) as u32) << 28) | 0x8B;
                out.extend_from_slice(&(raw_shl as u32).to_le_bytes()); pc += 4;
                let raw_sar = (0x22 << 21) | ((24 & 0x1FF) << 12) | (((*rd & 0xF) as u32) << 8) | (((*rd & 0xF) as u32) << 28) | 0x8B;
                out.extend_from_slice(&(raw_sar as u32).to_le_bytes()); pc += 4;
            }
            Item::Instr(Inst::SextH { rd, ra }) => {
                if rd != ra { let raw = (((*rd & 0xF) as u32) << 28) | (((*ra & 0xF) as u32) << 16) | (0x1F << 20) | 0x0B; out.extend_from_slice(&(raw as u32).to_le_bytes()); pc += 4; }
                let raw_shl = (0x20 << 21) | ((16 & 0x1FF) << 12) | (((*rd & 0xF) as u32) << 8) | (((*rd & 0xF) as u32) << 28) | 0x8B;
                out.extend_from_slice(&(raw_shl as u32).to_le_bytes()); pc += 4;
                let raw_sar = (0x22 << 21) | ((16 & 0x1FF) << 12) | (((*rd & 0xF) as u32) << 8) | (((*rd & 0xF) as u32) << 28) | 0x8B;
                out.extend_from_slice(&(raw_sar as u32).to_le_bytes()); pc += 4;
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
