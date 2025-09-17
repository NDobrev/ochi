use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

use tricore_rs::disasm::fmt_decoded;
use tricore_rs::isa::tc16::Tc16Decoder;
use tricore_rs::decoder::Decoder;

mod model;
mod analyze;
use analyze::{analyze_entries, Block, EdgeKind, EdgeOut, FunctionOut};
use model::{Image, Segment, load_raw_bin, read_u8, read_u32};

#[derive(Parser, Debug)]
#[command(author, version, about = "TriCore disassembler CLI", long_about=None)]
struct Cli {
    /// Load address for the binary in target address space
    #[arg(long, default_value_t = 0u32)]
    base: u32,
    /// Skip N bytes at start of file before loading
    #[arg(long, default_value_t = 0usize)]
    skip: usize,
    /// Input binary path
    #[arg(value_name = "BINFILE")]
    input: String,
    /// Limit bytes loaded (default: to EOF after --skip)
    #[arg(long)]
    len: Option<usize>,
    /// Subcommand
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// List loaded segments (simple single-segment for raw .bin)
    Sections,
    /// Disassemble a range [start, end) in bytes
    Range {
        /// Start address (hex or dec)
        start: String,
        /// End address (hex or dec, exclusive)
        end: String,
        /// Show instruction bytes
        #[arg(long)]
        show_bytes: bool,
        /// Write output to file instead of stdout
        #[arg(long, value_name = "FILE")]
        out: Option<String>,
    },
    /// Analyze code graph from entry points
    Analyze {
        /// Entry addresses (hex or dec). Repeat flag to add multiple entries.
        #[arg(long = "entry", value_name = "ADDR", num_args = 1.., required = false)]
        entries: Vec<String>,
        /// Maximum instructions to decode before stopping
        #[arg(long, default_value_t = 100_000usize)]
        max_instr: usize,
        /// Output format: text or json
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
        /// Emit a linear disassembly listing of analyzed code (text format only)
        #[arg(long)]
        listing: bool,
        /// Show instruction bytes in listing (text format only)
        #[arg(long)]
        show_bytes: bool,
        /// Import labels from JSON (Vec<{ addr, name }>)
        #[arg(long, value_name = "FILE")]
        labels_in: Option<String>,
        /// Export labels to JSON (Vec<{ addr, name }>)
        #[arg(long, value_name = "FILE")]
        labels_out: Option<String>,
        /// Write analysis output to file instead of stdout
        #[arg(long, value_name = "FILE")]
        out: Option<String>,
    },
}

fn parse_u32(s: &str) -> Result<u32> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        Ok(u32::from_str_radix(hex, 16)?)
    } else {
        Ok(s.parse::<u32>()?)
    }
}

fn read_u16(img: &Image, addr: u32) -> Option<u16> { // used by range renderer
    let b0 = read_u8(img, addr)?;
    let b1 = read_u8(img, addr.wrapping_add(1))?;
    Some(u16::from_le_bytes([b0, b1]))
}

fn is_mapped(img: &Image, addr: u32) -> bool {
    img.segments.iter().any(|s| {
        let start = s.base;
        let end = s.base.wrapping_add(s.bytes.len() as u32);
        addr >= start && addr < end
    })
}

// parse_u32 utility stays local to main for CLI parsing

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat { Text, Json }

#[derive(Debug, Clone, serde::Serialize)]
struct BlockOut { start: u32, end: u32, insns: Vec<String> }

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct LabelKV { addr: u32, name: String }

#[derive(Debug, Clone, serde::Serialize)]
struct ReportWithLabels {
    entries: Vec<u32>,
    blocks: Vec<BlockOut>,
    edges: Vec<EdgeOut>,
    functions: Vec<FunctionOut>,
    labels: Vec<LabelKV>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let img = load_raw_bin(Path::new(&cli.input), cli.base, cli.skip, cli.len)?;

    match cli.cmd {
        Command::Sections => {
            println!("{:<10} {:<#12} {:<#12} {:<6} {:<6}", "name", "start", "end", "perms", "kind");
            for s in &img.segments {
                let start = s.base;
                let end = s.base + (s.bytes.len() as u32);
                println!(
                    "{:<10} {start:#010x} {end:#010x} {:<6} {:<6}",
                    s.name, s.perms, s.kind
                );
            }
        }
        Command::Range { start, end, show_bytes, out } => {
            let start = parse_u32(&start)?;
            let end = parse_u32(&end)?;
            anyhow::ensure!(end >= start, "end must be >= start");

            let dec = Tc16Decoder::new();
            let mut pc = start;
            let mut buf = String::new();
            while pc < end {
                let Some(raw32) = read_u32(&img, pc) else { println!("{pc:#010x}: <oob>"); break; };
                if let Some(d) = dec.decode(raw32) {
                    if show_bytes {
                        let w = d.width as u32;
                        let mut bytes = Vec::new();
                        for i in 0..w { bytes.push(read_u8(&img, pc + i).unwrap_or(0)); }
                        use std::fmt::Write as _;
                        let _ = write!(buf, "{pc:#010x}: ");
                        for b in bytes { let _ = write!(buf, "{:02x} ", b); }
                        let _ = writeln!(buf, "  {}", fmt_decoded(&d));
                    } else {
                        use std::fmt::Write as _;
                        let _ = writeln!(buf, "{pc:#010x}: {}", fmt_decoded(&d));
                    }
                    pc = pc.wrapping_add(d.width as u32);
                } else {
                    use std::fmt::Write as _;
                    let _ = writeln!(buf, "{pc:#010x}: .word {raw32:#010x}");
                    pc = pc.wrapping_add(4);
                }
            }
            if let Some(path) = out { std::fs::write(path, buf)?; } else { print!("{}", buf); }
        }
        Command::Analyze { entries, max_instr, format, listing, show_bytes, labels_in, labels_out, out } => {
            // default seed: start of first segment
            let mut seeds: Vec<u32> = if entries.is_empty() {
                img.segments.get(0).map(|s| s.base).into_iter().collect()
            } else {
                let mut v = Vec::new();
                for e in entries { v.push(parse_u32(&e)?); }
                v
            };
            seeds.sort_unstable();
            seeds.dedup();
            let (visited, widths, edges, rets) = analyze_entries(&img, &seeds, max_instr);

            // Compute block starts: entries + all edge destinations
            let mut block_starts: HashSet<u32> = seeds.iter().copied().collect();
            for e in &edges { block_starts.insert(e.to); }

            // Build blocks by linear sweep from each start until next start/unknown
            let mut starts: Vec<u32> = block_starts.into_iter().collect();
            starts.sort_unstable();
            let mut blocks: Vec<Block> = Vec::new();
            let mut addr_to_block: HashMap<u32, u32> = HashMap::new(); // pc -> block start
            for &start in &starts {
                if !visited.contains(&start) { continue; }
                // Avoid duplicating blocks if we've already assigned this start
                if addr_to_block.contains_key(&start) { continue; }
                let mut cur = start;
                loop {
                    let Some(&w) = widths.get(&cur) else { break };
                    let next = cur.wrapping_add(w as u32);
                    // Is current instruction an unconditional branch? If so, close after it.
                    let is_uncond = edges.iter().any(|e| e.from == cur && matches!(e.kind, EdgeKind::Branch));
                    let is_ret = rets.contains(&cur);
                    // If next is a new block start or we hit an uncond branch or unknown/visited gap, end block at next
                    let should_end = is_uncond || is_ret
                        || !visited.contains(&next)
                        || starts.binary_search(&next).is_ok();
                    if should_end {
                        let end = next;
                        blocks.push(Block { start, end });
                        // Map all PCs from start to end into this block
                        let mut pc = start;
                        while pc < end {
                            addr_to_block.insert(pc, start);
                            if let Some(&ww) = widths.get(&pc) { pc = pc.wrapping_add(ww as u32); } else { break; }
                        }
                        break;
                    } else {
                        cur = next;
                    }
                }
            }

            // Normalize edges to block-level
            let mut edges_out: Vec<EdgeOut> = Vec::new();
            for e in &edges {
                let from_block = *addr_to_block.get(&e.from).unwrap_or(&e.from);
                let to_block = starts.iter().copied().find(|&s| s == e.to).unwrap_or(e.to);
                let kind = match e.kind { EdgeKind::Fallthrough => "ft", EdgeKind::Branch => "br", EdgeKind::CondBranch => "cbr", EdgeKind::Call => "call" }.to_string();
                edges_out.push(EdgeOut { from: from_block, to: to_block, kind });
            }

            // Functions: treat each seed as a root and collect reachable block starts
            let mut functions: Vec<FunctionOut> = Vec::new();
            // Build adjacency from block-level edges
            let mut adj: HashMap<u32, Vec<u32>> = HashMap::new();
            for e in &edges_out { adj.entry(e.from).or_default().push(e.to); }
            for &entry in &seeds {
                // Map entry to block start
                let entry_block = starts.iter().copied().find(|&s| s == entry).unwrap_or(entry);
                let mut seen: HashSet<u32> = HashSet::new();
                let mut q = VecDeque::new();
                q.push_back(entry_block);
                while let Some(b) = q.pop_front() {
                    if !seen.insert(b) { continue; }
                    if let Some(nexts) = adj.get(&b) {
                        for &n in nexts { q.push_back(n); }
                    }
                }
                let mut blks: Vec<u32> = seen.into_iter().collect();
                blks.sort_unstable();
                functions.push(FunctionOut { entry: entry_block, blocks: blks });
            }

            // Prepare labels (imported or autogenerated)
            let mut labels: HashMap<u32, String> = HashMap::new();
            if let Some(path) = &labels_in {
                if let Ok(txt) = std::fs::read_to_string(path) {
                    if let Ok(v) = serde_json::from_str::<Vec<LabelKV>>(&txt) {
                        for kv in v { labels.insert(kv.addr, kv.name); }
                    }
                }
            }
            for &e in &seeds { labels.entry(e).or_insert_with(|| format!("sub_{e:08x}")); }
            for b in &blocks { labels.entry(b.start).or_insert_with(|| format!("loc_{:08x}", b.start)); }

            match format {
                OutputFormat::Json => {
                    let report_blocks = enrich_blocks_with_mnemonics(&img, &widths, &blocks, show_bytes);
                    // Optionally export labels
                    if let Some(outp) = &labels_out {
                        let mut arr: Vec<LabelKV> = Vec::new();
                        for (addr, name) in &labels { arr.push(LabelKV { addr: *addr, name: name.clone() }); }
                        let _ = std::fs::write(outp, serde_json::to_string_pretty(&arr).unwrap_or_default());
                    }
                    let mut lbl_vec: Vec<LabelKV> = labels.iter().map(|(k,v)| LabelKV { addr: *k, name: v.clone() }).collect();
                    lbl_vec.sort_by_key(|kv| kv.addr);
                    let report = ReportWithLabels { entries: seeds.clone(), blocks: report_blocks, edges: edges_out, functions, labels: lbl_vec };
                    let json = serde_json::to_string_pretty(&report)?;
                    if let Some(path) = out { std::fs::write(path, json)?; } else { println!("{}", json); }
                }
                OutputFormat::Text => {
                    println!("Analysis summary:");
                    println!("  entries   : {:?}", seeds.iter().map(|a| format!("{a:#010x}")).collect::<Vec<_>>());
                    println!("  insts     : {}", visited.len());
                    println!("  blocks    : {}", blocks.len());
                    println!("  edges     : {}", edges.len());
                    println!("  functions : {}", functions.len());
                    println!("Edges:");
                    for e in &edges_out {
                        println!("  {:#010x} -> {:#010x} ({})", e.from, e.to, e.kind);
                    }
                    if listing {
                        // Order visited addresses ascending
                        let mut pcs: Vec<u32> = visited.iter().copied().collect();
                        pcs.sort_unstable();
                        let dec = Tc16Decoder::new();
                        println!("\nListing (analyzed PCs):");
                        for pc in pcs {
                            if let Some(lbl) = labels.get(&pc) {
                                println!("{pc:#010x} <{lbl}>:");
                            }
                            if let Some(raw32) = read_u32(&img, pc) {
                                if let Some(d) = dec.decode(raw32) {
                                    if show_bytes {
                                        let w = d.width as u32;
                                        let mut bytes = Vec::new();
                                        for i in 0..w { bytes.push(read_u8(&img, pc + i).unwrap_or(0)); }
                                        print!("  {pc:#010x}: ");
                                        for b in bytes { print!("{:02x} ", b); }
                                        println!("  {}", fmt_decoded(&d));
                                    } else {
                                        println!("  {pc:#010x}: {}", fmt_decoded(&d));
                                    }
                                } else {
                                    println!("  {pc:#010x}: .word {raw32:#010x}");
                                }
                            }
                        }
                    }
                    // Optionally export labels
                    if let Some(outp) = &labels_out {
                        let mut arr: Vec<LabelKV> = Vec::new();
                        for (addr, name) in &labels { arr.push(LabelKV { addr: *addr, name: name.clone() }); }
                        let _ = std::fs::write(outp, serde_json::to_string_pretty(&arr).unwrap_or_default());
                    }
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_u32_hex_and_dec() {
        assert_eq!(parse_u32("0x10").unwrap(), 0x10);
        assert_eq!(parse_u32("16").unwrap(), 16);
        assert!(parse_u32("zz").is_err());
    }

    #[test]
    fn loader_maps_skip_and_len() {
        let cwd = std::env::current_dir().unwrap();
        let path = cwd.join("_test_bin.bin");
        std::fs::write(&path, [0u8, 1, 2, 3, 4, 5]).unwrap();
        let img = load_raw_bin(&path, 0x1000_0000, 2, Some(3)).unwrap();
        assert_eq!(img.segments.len(), 1);
        let s = &img.segments[0];
        assert_eq!(s.base, 0x1000_0000);
        assert_eq!(s.bytes, vec![2, 3, 4]);
        assert_eq!(read_u32(&img, 0x1000_0000).unwrap(), 0x00040302);
        assert!(read_u32(&img, 0x1000_0002 + 2).is_none());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn range_disasm_decodes_simple() {
        // Craft a 32-bit MOV.U D0,#2 instruction: c=0, imm16=2, op1=0xBB
        let raw32 = (0u32 << 28) | (2u32 << 12) | 0xBBu32;
        let bytes = raw32.to_le_bytes();
        let seg = Segment { name: "s".into(), base: 0, bytes: bytes.to_vec(), perms: "r-x", kind: "raw" };
        let img = Image { segments: vec![seg] };
        let dec = Tc16Decoder::new();
        let pc = 0u32;
        let raw32_rd = read_u32(&img, pc).unwrap();
        assert_eq!(raw32_rd, raw32);
        let d = dec.decode(raw32_rd).unwrap();
        let text = fmt_decoded(&d);
        assert!(text.contains("mov d0, #0x2"));
    }
}

fn enrich_blocks_with_mnemonics(img: &Image, widths: &HashMap<u32,u8>, blocks: &Vec<Block>, show_bytes: bool) -> Vec<BlockOut> {
    let dec = Tc16Decoder::new();
    let mut out = Vec::with_capacity(blocks.len());
    for b in blocks {
        let mut lines = Vec::new();
        let mut pc = b.start;
        while pc < b.end {
            if let Some(raw32) = read_u32(img, pc) {
                if let Some(d) = dec.decode(raw32) {
                    if show_bytes {
                        let mut bs = Vec::new();
                        for i in 0..(d.width as u32) { bs.push(read_u8(img, pc + i).unwrap_or(0)); }
                        let mut s = format!("{pc:#010x}: ");
                        for bb in bs { s.push_str(&format!("{:02x} ", bb)); }
                        s.push_str("  ");
                        s.push_str(&fmt_decoded(&d));
                        lines.push(s);
                    } else {
                        lines.push(format!("{pc:#010x}: {}", fmt_decoded(&d)));
                    }
                    pc = pc.wrapping_add(d.width as u32);
                    continue;
                }
            }
            break;
        }
        out.push(BlockOut { start: b.start, end: b.end, insns: lines });
    }
    out
}
