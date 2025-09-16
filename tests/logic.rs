use tricore_rs::exec::IntExecutor;
use tricore_rs::isa::tc16::Tc16Decoder;
use tricore_rs::{Cpu, CpuConfig, LinearMemory};
use tricore_rs::Bus;

// Helpers for MOV.U, logical RR/RC
fn enc_movu(c: u32, imm16: u32) -> u32 { (c << 28) | (imm16 << 12) | 0xBB }
fn enc_logic_rr(op2: u32, c: u32, a: u32, b: u32) -> u32 { (c<<28) | (op2<<20) | (b<<16) | (a<<8) | 0x0F }
fn enc_logic_rc(op2: u32, c: u32, a: u32, imm9: u32) -> u32 { (c<<28) | (op2<<21) | ((imm9 & 0x1FF)<<12) | (a<<8) | 0x8F }

#[test]
fn and_or_xor_rr_and_rc() {
    let mut mem = LinearMemory::new(128);
    let mut cpu = Cpu::new(CpuConfig::default());
    cpu.reset(0);

    // d1 = 0xF0F0, d2 = 0x0FF0
    mem.write_u32(0, enc_movu(1, 0xF0F0)).unwrap();
    mem.write_u32(4, enc_movu(2, 0x0FF0)).unwrap();
    // d3 = d1 & d2 => 0x00F0
    mem.write_u32(8, enc_logic_rr(0x08, 3, 1, 2)).unwrap();
    // d4 = d3 | 0x0F => 0x00FF
    mem.write_u32(12, enc_logic_rc(0x0A, 4, 3, 0x0F)).unwrap();
    // d5 = d4 ^ d1 => 0xF00F
    mem.write_u32(16, enc_logic_rr(0x0C, 5, 4, 1)).unwrap();
    // sub tests: d6 = d1 - d2 (RR op2=0x08, op1=0x0B)
    mem.write_u32(20, (6u32<<28) | (0x08u32<<20) | (2u32<<16) | (1u32<<8) | 0x0B).unwrap();
    // rsub rc: d7 = 0x0010 - d1 (op1=0x8B, op2=0x08, imm9=0x10)
    let rsub = (7u32<<28) | (0x08u32<<21) | ((0x10u32&0x1FF)<<12) | (1u32<<8) | 0x8B;
    mem.write_u32(24, rsub).unwrap();

    let dec = Tc16Decoder::new();
    let exec = IntExecutor;
    for _ in 0..7 { cpu.step(&mut mem, &dec, &exec).unwrap(); }
    assert_eq!(cpu.gpr[3], 0x0000_00F0);
    assert_eq!(cpu.gpr[4], 0x0000_00FF);
    assert_eq!(cpu.gpr[5], 0x0000_F00F);
    assert_eq!(cpu.gpr[6], 0x0000_E100);
    assert_eq!(cpu.gpr[7], 0xFFFF_0F20);
}
