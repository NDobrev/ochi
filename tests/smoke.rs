use tricore_rs::exec::IntExecutor;
use tricore_rs::isa::tc16::Tc16Decoder;
use tricore_rs::{Cpu, CpuConfig, LinearMemory};
use tricore_rs::Bus; // bring trait into scope for write_u{16,32}

fn enc_brc(op1: u32, cond: u32, a: u32, c4: u32, disp15: u32) -> u32 {
    ((cond & 0x3) << 30) | ((disp15 & 0x7FFF) << 15) | ((c4 & 0xF) << 12) | ((a & 0xF) << 8) | (op1 & 0xFF)
}

#[test]
fn addi_const16_spec() {
    let mut mem = LinearMemory::new(1024);
    let mut cpu = Cpu::new(CpuConfig::default());
    cpu.reset(0);

    // Encode: ADDI D[c], D[a], const16 (RLC), op1 = 0x1B
    // Fields: c:31..28, const16:27..12, a:11..8, op1:7..0
    let c = 0u32; // D0
    let a = 0u32; // D0
    let const16 = 5u32;
    let raw32 = (c << 28) | (const16 << 12) | (a << 8) | 0x1B;
    mem.write_u32(0, raw32).unwrap();

    let dec = Tc16Decoder::new();
    let exec = IntExecutor;
    cpu.step(&mut mem, &dec, &exec).unwrap();
    assert_eq!(cpu.gpr[0], 5);
}

#[test]
fn jeq_sbc_branch_taken_smoke() {
    let mut mem = LinearMemory::new(1024);
    let mut cpu = Cpu::new(CpuConfig::default());
    cpu.reset(0);

    // Program:
    //   0x0000: JEQ D15,#1,+1     (16-bit)
    //   0x0002: MOV D0,#0         (16-bit) [skipped]
    //   0x0004: MOV D0,#2         (16-bit)
    cpu.gpr[15] = 1;
    let jeq = ((1u16 as u16) << 12) | ((1u16 as u16) << 8) | 0x1Eu16;
    let mov_d0_0_16 = ((0u16 as u16) << 12) | ((0u16 as u16) << 8) | 0x82u16;
    let mov_d0_2_16 = ((2u16 as u16) << 12) | ((0u16 as u16) << 8) | 0x82u16;

    mem.write_u16(0, jeq).unwrap();
    mem.write_u16(2, mov_d0_0_16).unwrap();
    mem.write_u16(4, mov_d0_2_16).unwrap();

    let dec = Tc16Decoder::new();
    let exec = IntExecutor;

    cpu.step(&mut mem, &dec, &exec).unwrap(); // JEQ (16-bit)
    cpu.step(&mut mem, &dec, &exec).unwrap(); // MOV D0,#2
    assert_eq!(cpu.gpr[0], 2);
}
