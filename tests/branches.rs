use tricore_rs::exec::IntExecutor;
use tricore_rs::isa::tc16::Tc16Decoder;
use tricore_rs::{Cpu, CpuConfig, LinearMemory};
use tricore_rs::Bus;

// Helper to encode J disp24 (op1=0x1D)
fn enc_j_disp24(disp24: u32) -> u32 {
    let low16 = disp24 & 0xFFFF;
    let hi8 = (disp24 >> 16) & 0xFF;
    (low16 << 16) | (hi8 << 8) | 0x1D
}

fn enc_brc(op1: u32, cond: u32, a: u32, c4: u32, disp15: u32) -> u32 {
    ((cond & 0x3) << 30)
        | ((disp15 & 0x7FFF) << 15)
        | ((c4 & 0xF) << 12)
        | ((a & 0xF) << 8)
        | (op1 & 0xFF)
}

fn enc_brr(op1: u32, cond: u32, a: u32, b: u32, disp15: u32) -> u32 {
    ((cond & 0x3) << 30)
        | ((disp15 & 0x7FFF) << 15)
        | ((b & 0xF) << 12)
        | ((a & 0xF) << 8)
        | (op1 & 0xFF)
}

#[test]
fn j_disp8_skips_next_16bit() {
    let mut mem = LinearMemory::new(64);
    let mut cpu = Cpu::new(CpuConfig::default());
    cpu.reset(0);

    // 0x0000: MOV.U D0,#1 (32-bit)
    // 0x0004: J +2 bytes      (16-bit)
    // 0x0006: MOV   D0,#0     (16-bit) [skipped]
    // 0x0008: MOV   D0,#2     (16-bit)
    let movu_d0_1 = (0u32 << 28) | (1u32 << 12) | 0xBB;
    let j_disp8 = ((1u16 as u16) << 8) | 0x3Cu16; // disp8=1, op1=0x3C
    let mov_d0_0 = ((0u16 as u16) << 12) | ((0u16 as u16) << 8) | 0x82u16;
    let mov_d0_2 = ((2u16 as u16) << 12) | ((0u16 as u16) << 8) | 0x82u16;

    mem.write_u32(0, movu_d0_1).unwrap();
    mem.write_u16(4, j_disp8).unwrap();
    mem.write_u16(6, mov_d0_0).unwrap();
    mem.write_u16(8, mov_d0_2).unwrap();

    let dec = Tc16Decoder::new();
    let exec = IntExecutor;
    cpu.step(&mut mem, &dec, &exec).unwrap(); // MOV.U
    cpu.step(&mut mem, &dec, &exec).unwrap(); // J (skip next)
    cpu.step(&mut mem, &dec, &exec).unwrap(); // MOV D0,#2
    assert_eq!(cpu.gpr[0], 2);
}

#[test]
fn j_disp24_skips_next_32bit() {
    let mut mem = LinearMemory::new(64);
    let mut cpu = Cpu::new(CpuConfig::default());
    cpu.reset(0);

    // 0x0000: MOV.U D0,#1 (32-bit)
    // 0x0004: J +4 bytes    (32-bit)
    // 0x0008: MOV.U D0,#0   (32-bit) [skipped]
    // 0x000C: MOV.U D0,#2   (32-bit)
    let movu_d0_1 = (0u32 << 28) | (1u32 << 12) | 0xBB;
    let j_disp24 = enc_j_disp24(2); // disp24=2 -> +4 bytes
    let movu_d0_0 = (0u32 << 28) | (0u32 << 12) | 0xBB;
    let movu_d0_2 = (0u32 << 28) | (2u32 << 12) | 0xBB;

    mem.write_u32(0, movu_d0_1).unwrap();
    mem.write_u32(4, j_disp24).unwrap();
    mem.write_u32(8, movu_d0_0).unwrap();
    mem.write_u32(12, movu_d0_2).unwrap();

    let dec = Tc16Decoder::new();
    let exec = IntExecutor;
    cpu.step(&mut mem, &dec, &exec).unwrap();
    cpu.step(&mut mem, &dec, &exec).unwrap();
    cpu.step(&mut mem, &dec, &exec).unwrap();
    assert_eq!(cpu.gpr[0], 2);
}

#[test]
fn jeq_sbc_imm_taken() {
    let mut mem = LinearMemory::new(64);
    let mut cpu = Cpu::new(CpuConfig::default());
    cpu.reset(0);

    // Set D15 = 3
    cpu.gpr[15] = 3;
    // 0x0000: JEQ D15,#3,+1 (skip next 16-bit)
    // 0x0002: MOV D0,#0  (16-bit) [skipped]
    // 0x0004: MOV D0,#7  (16-bit)
    let jeq_sbc = ((3u16 as u16) << 12) | ((1u16 as u16) << 8) | 0x1Eu16;
    let mov_d0_0 = ((0u16 as u16) << 12) | ((0u16 as u16) << 8) | 0x82u16;
    let mov_d0_7 = ((7u16 as u16) << 12) | ((0u16 as u16) << 8) | 0x82u16;

    mem.write_u16(0, jeq_sbc).unwrap();
    mem.write_u16(2, mov_d0_0).unwrap();
    mem.write_u16(4, mov_d0_7).unwrap();

    let dec = Tc16Decoder::new();
    let exec = IntExecutor;
    cpu.step(&mut mem, &dec, &exec).unwrap(); // JEQ -> skip
    cpu.step(&mut mem, &dec, &exec).unwrap(); // MOV D0,#7
    assert_eq!(cpu.gpr[0], 7);
}

#[test]
fn jge_brc_signed_taken() {
    let mut mem = LinearMemory::new(128);
    let mut cpu = Cpu::new(CpuConfig::default());
    cpu.reset(0);

    // D1 = 5
    let movu_d1_5 = (1u32 << 28) | (5u32 << 12) | 0xBB;
    // JGE D1, #3, +1 (signed) -> cond=00, op1=0xFF
    let jge = enc_brc(0xFF, 0, 1, 3, 1);
    // MOV D0,#0 (16-bit) [skipped]
    let mov_d0_0 = ((0u16 as u16) << 12) | ((0u16 as u16) << 8) | 0x82u16;
    // MOV D0,#2 (16-bit) [executed]
    let mov_d0_2 = ((2u16 as u16) << 12) | ((0u16 as u16) << 8) | 0x82u16;

    mem.write_u32(0, movu_d1_5).unwrap();
    mem.write_u32(4, jge).unwrap();
    mem.write_u16(8, mov_d0_0).unwrap();
    mem.write_u16(10, mov_d0_2).unwrap();

    let dec = Tc16Decoder::new();
    let exec = IntExecutor;
    cpu.step(&mut mem, &dec, &exec).unwrap();
    cpu.step(&mut mem, &dec, &exec).unwrap();
    cpu.step(&mut mem, &dec, &exec).unwrap();
    assert_eq!(cpu.gpr[0], 2);
}

#[test]
fn jltu_brc_unsigned_taken() {
    let mut mem = LinearMemory::new(256);
    let mut cpu = Cpu::new(CpuConfig::default());
    cpu.reset(0);

    // D1 = 1
    let movu_d1_1 = (1u32 << 28) | (1u32 << 12) | 0xBB;
    // JLT.U D1, #2, +1: op1=0xBF, cond=01
    let jltu = enc_brc(0xBF, 1, 1, 2, 1);
    // MOV D0,#0 (16-bit) [skipped]
    let mov_d0_0 = ((0u16 as u16) << 12) | ((0u16 as u16) << 8) | 0x82u16;
    // MOV D0,#2 (16-bit)
    let mov_d0_2 = ((2u16 as u16) << 12) | ((0u16 as u16) << 8) | 0x82u16;

    mem.write_u32(0, movu_d1_1).unwrap();
    mem.write_u32(4, jltu).unwrap();
    mem.write_u16(8, mov_d0_0).unwrap();
    mem.write_u16(10, mov_d0_2).unwrap();

    let dec = Tc16Decoder::new();
    let exec = IntExecutor;
    for _ in 0..3 { cpu.step(&mut mem, &dec, &exec).unwrap(); }
    assert_eq!(cpu.gpr[0], 2);
}
