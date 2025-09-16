use tricore_rs::exec::IntExecutor;
use tricore_rs::isa::tc16::Tc16Decoder;
use tricore_rs::{Cpu, CpuConfig, LinearMemory};

#[test]
fn add_immediate_placeholder() {
    let mut mem = LinearMemory::new(1024);
    let mut cpu = Cpu::new(CpuConfig::default());
    cpu.reset(0);

    // Write a fake 16-bit ADD r0, #5 (encoded to match our placeholder decoder)
    // Here we just set raw32 top selector to 0b00010 and put imm5=5 and rd=0 in low 16
    let raw32 = 0b00010 << 27 | 0x0005;
    mem.write_u32(0, raw32).unwrap();

    let dec = Tc16Decoder::new();
    let exec = IntExecutor;
    cpu.step(&mut mem, &dec, &exec).unwrap();
    assert_eq!(cpu.gpr[0], 5);
}
