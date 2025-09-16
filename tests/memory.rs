use tricore_rs::exec::IntExecutor;
use tricore_rs::isa::tc16::Tc16Decoder;
use tricore_rs::{Cpu, CpuConfig, LinearMemory};
use tricore_rs::Bus;

// Encode LD.W D[a], A[b], off10 (BO): op1=0x09, op2=0x24 at [27:22]
fn enc_ldw_bo(a: u32, b: u32, off10: u32) -> u32 {
    let off_hi4 = (off10 >> 6) & 0xF;
    let off_lo6 = off10 & 0x3F;
    (off_hi4 << 28) | (0x24 << 22) | (off_lo6 << 16) | (b << 12) | (a << 8) | 0x09
}

// Encode ST.W A[b], off10, D[a] (BO): op1=0x89, op2=0x24 at [27:22]
fn enc_stw_bo(a: u32, b: u32, off10: u32) -> u32 {
    let off_hi4 = (off10 >> 6) & 0xF;
    let off_lo6 = off10 & 0x3F;
    (off_hi4 << 28) | (0x24 << 22) | (off_lo6 << 16) | (b << 12) | (a << 8) | 0x89
}

// Encode LD.B/LD.BU/LD.H/LD.HU (BO) by op2 selector
fn enc_ld_bo(op2: u32, a: u32, b: u32, off10: u32) -> u32 {
    let off_hi4 = (off10 >> 6) & 0xF;
    let off_lo6 = off10 & 0x3F;
    (off_hi4 << 28) | (op2 << 22) | (off_lo6 << 16) | (b << 12) | (a << 8) | 0x09
}

// Encode ST.B/ST.H (BO) by op2 selector
fn enc_st_bo(op2: u32, a: u32, b: u32, off10: u32) -> u32 {
    let off_hi4 = (off10 >> 6) & 0xF;
    let off_lo6 = off10 & 0x3F;
    (off_hi4 << 28) | (op2 << 22) | (off_lo6 << 16) | (b << 12) | (a << 8) | 0x89
}

#[test]
fn ldw_uses_address_register() {
    let mut mem = LinearMemory::new(64);
    let mut cpu = Cpu::new(CpuConfig::default());
    cpu.reset(0);

    // Place a word at address 16 (aligned)
    mem.write_u32(16, 0x1122_3344).unwrap();
    // Set A3 = base 12; off10=4 bytes to reach 16
    cpu.a[3] = 12;

    let ldw = enc_ldw_bo(2, 3, 4); // D2 <- [A3 + 4]
    mem.write_u32(0, ldw).unwrap();

    let dec = Tc16Decoder::new();
    let exec = IntExecutor;
    cpu.step(&mut mem, &dec, &exec).unwrap();
    assert_eq!(cpu.gpr[2], 0x1122_3344);
}

#[test]
fn stw_uses_address_register() {
    let mut mem = LinearMemory::new(64);
    let mut cpu = Cpu::new(CpuConfig::default());
    cpu.reset(0);

    cpu.gpr[4] = 0xAABB_CCDD;
    cpu.a[2] = 4;

    let stw = enc_stw_bo(4, 2, 12); // [A2 + 12] = D4
    mem.write_u32(0, stw).unwrap();

    let dec = Tc16Decoder::new();
    let exec = IntExecutor;
    cpu.step(&mut mem, &dec, &exec).unwrap();
    assert_eq!(mem.read_u32(16).unwrap(), 0xAABB_CCDD);
}

#[test]
fn ldb_and_ldbu_sign_and_zero_extend() {
    let mut mem = LinearMemory::new(64);
    let mut cpu = Cpu::new(CpuConfig::default());
    cpu.reset(0);

    // Write 0xFF at address 20
    mem.write_u8(20, 0xFF).unwrap();
    cpu.a[1] = 16;

    // LD.B  D3, [A1 + 4]
    let ldb = enc_ld_bo(0x20, 3, 1, 4);
    mem.write_u32(0, ldb).unwrap();
    // LD.BU D4, [A1 + 4]
    let ldbu = enc_ld_bo(0x21, 4, 1, 4);
    mem.write_u32(4, ldbu).unwrap();

    let dec = Tc16Decoder::new();
    let exec = IntExecutor;
    cpu.step(&mut mem, &dec, &exec).unwrap();
    cpu.step(&mut mem, &dec, &exec).unwrap();
    assert_eq!(cpu.gpr[3], 0xFFFF_FFFF); // sign-extended -1
    assert_eq!(cpu.gpr[4], 0x0000_00FF); // zero-extended
}

#[test]
fn ldh_and_ldhu_with_alignment() {
    let mut mem = LinearMemory::new(64);
    let mut cpu = Cpu::new(CpuConfig::default());
    cpu.reset(0);

    // Write halfword 0xFFEE at aligned address 24
    mem.write_u16(24, 0xFFEE).unwrap();
    cpu.a[2] = 16;

    let ldh = enc_ld_bo(0x22, 5, 2, 8); // [16+8]=24
    let ldhu = enc_ld_bo(0x23, 6, 2, 8);
    mem.write_u32(0, ldh).unwrap();
    mem.write_u32(4, ldhu).unwrap();

    let dec = Tc16Decoder::new();
    let exec = IntExecutor;
    cpu.step(&mut mem, &dec, &exec).unwrap();
    cpu.step(&mut mem, &dec, &exec).unwrap();
    assert_eq!(cpu.gpr[5], 0xFFFF_FFEE); // sign-extended
    assert_eq!(cpu.gpr[6], 0x0000_FFEE); // zero-extended

    // Now try misaligned halfword load (should trap Unaligned)
    cpu.reset(8);
    cpu.a[2] = 15; // base 15 + 8 => 23 misaligned
    mem.write_u32(8, ldh).unwrap();
    let res = cpu.step(&mut mem, &dec, &exec);
    assert!(res.is_err());
}

#[test]
fn stb_and_sth() {
    let mut mem = LinearMemory::new(64);
    let mut cpu = Cpu::new(CpuConfig::default());
    cpu.reset(0);
    cpu.gpr[7] = 0x1234_56F0;
    cpu.gpr[8] = 0xABCD_00EE;
    cpu.a[3] = 8;

    let stb = enc_st_bo(0x20, 7, 3, 5);  // [13] = 0xF0
    let sth = enc_st_bo(0x22, 8, 3, 6);  // [14] = 0x00EE
    mem.write_u32(0, stb).unwrap();
    mem.write_u32(4, sth).unwrap();

    let dec = Tc16Decoder::new();
    let exec = IntExecutor;
    cpu.step(&mut mem, &dec, &exec).unwrap();
    cpu.step(&mut mem, &dec, &exec).unwrap();
    assert_eq!(mem.read_u8(13).unwrap(), 0xF0);
    assert_eq!(mem.read_u16(14).unwrap(), 0x00EE);
}
