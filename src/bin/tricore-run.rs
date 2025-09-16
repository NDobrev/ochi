use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

use tricore_rs::{exec::IntExecutor, isa::tc16::Tc16Decoder, Cpu, CpuConfig, LinearMemory};

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Run a TriCore binary on the tricore-rs interpreter"
)]
struct Opts {
    #[arg(short, long)]
    entry: Option<u32>,
    #[arg(value_name = "BINFILE")]
    input: String,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let opts = Opts::parse();
    let mut mem = LinearMemory::new(16 * 1024 * 1024);

    let bytes = std::fs::read(&opts.input)?;
    mem.mem[0..bytes.len()].copy_from_slice(&bytes);

    let mut cpu = Cpu::new(CpuConfig::default());
    cpu.reset(opts.entry.unwrap_or(0));

    let dec = Tc16Decoder::new();
    let exec = IntExecutor;

    // Simple run loop with step cap
    for _ in 0..10_000_000u64 {
        if let Err(trap) = cpu.step(&mut mem, &dec, &exec) {
            eprintln!("TRAP: {trap:?}");
            break;
        }
    }

    Ok(())
}
