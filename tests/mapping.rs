use tricore_rs::{Cpu, CpuConfig, LinearMemory};
use tricore_rs::isa::tc16::Tc16Decoder;
use tricore_rs::exec::IntExecutor;
use tricore_rs::Bus;

#[test]
fn high_mapped_memory_executes() {
    // Create memory segment mapped at a high address
    let load = 0x8000_0000u32;
    let mut mem = LinearMemory::new(16);
    mem.base = load;

    // Encode: MOV.U D0, #2 (op1=0xBB, c=0, imm16=2)
    let movu_d0_2 = (0u32 << 28) | (2u32 << 12) | 0xBB;
    mem.write_u32(load, movu_d0_2).unwrap();

    let mut cpu = Cpu::new(CpuConfig::default());
    cpu.reset(load);
    let dec = Tc16Decoder::new();
    let exec = IntExecutor;
    cpu.step(&mut mem, &dec, &exec).unwrap();
    assert_eq!(cpu.gpr[0], 2);
}

