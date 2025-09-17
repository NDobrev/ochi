use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

use tricore_rs::{exec::IntExecutor, isa::tc16::Tc16Decoder, Cpu, CpuConfig, LinearMemory};
use tricore_rs::Bus;
use tricore_rs::disasm::fmt_decoded;
use tricore_rs::decoder::Decoder;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Run a TriCore binary on the tricore-rs interpreter"
)]
struct Opts {
    #[arg(short, long, help = "Entry PC address (defaults to load address)")]
    entry: Option<u32>,
    #[arg(long, help = "Load address for the binary in target address space", default_value_t = 0u32)]
    load_addr: u32,
    #[arg(long, help = "Skip N bytes at start of file before loading", default_value_t = 0usize)]
    skip: usize,
    #[arg(value_name = "BINFILE")]
    input: String,
    #[arg(long, help = "Disassemble N instructions and exit")] 
    disasm: Option<usize>,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let opts = Opts::parse();
    let bytes = std::fs::read(&opts.input)?;
    anyhow::ensure!(opts.skip <= bytes.len(), "--skip exceeds file size");
    let payload = &bytes[opts.skip..];
    let mut mem = LinearMemory::new(payload.len());
    mem.base = opts.load_addr;
    mem.mem[..payload.len()].copy_from_slice(payload);

    let mut cpu = Cpu::new(CpuConfig::default());
    let entry = opts.entry.unwrap_or(opts.load_addr);
    cpu.reset(entry);

    let dec = Tc16Decoder::new();
    let exec = IntExecutor;

    if let Some(count) = opts.disasm {
        let mut pc = entry;
        for _ in 0..count {
            let raw32 = mem.read_u32(pc)?;
            if let Some(d) = dec.decode(raw32) {
                println!("{pc:#010x}: {}", fmt_decoded(&d));
                pc = pc.wrapping_add(d.width as u32);
            } else {
                println!("{pc:#010x}: .word {raw32:#010x}");
                pc = pc.wrapping_add(4);
            }
        }
        return Ok(());
    } else {
        // Simple run loop with step cap
        for _ in 0..10_000_000u64 {
            if let Err(trap) = cpu.step(&mut mem, &dec, &exec) {
                eprintln!("TRAP: {trap:?}");
                break;
            }
        }
    }

    Ok(())
}
