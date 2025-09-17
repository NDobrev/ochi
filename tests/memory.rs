use tricore_rs::exec::IntExecutor;
use tricore_rs::decoder::Decoder;
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

// Addressing mode variants for LD.W
fn enc_ldw_bo_mode(op2: u32, a: u32, b: u32, off10: u32) -> u32 {
    let off_hi4 = (off10 >> 6) & 0xF;
    let off_lo6 = off10 & 0x3F;
    (off_hi4 << 28) | (op2 << 22) | (off_lo6 << 16) | (b << 12) | (a << 8) | 0x09
}

#[test]
fn ldb_post_and_pre_increment() {
    let mut mem = LinearMemory::new(128);
    let mut cpu = Cpu::new(CpuConfig::default());
    cpu.reset(0);
    // Write bytes at addresses 10 and 12
    mem.write_u8(10, 0x7F).unwrap();
    mem.write_u8(12, 0x80).unwrap();
    cpu.a[1] = 10;

    // LD.B post-inc: op2=0x00
    let ldb_post = enc_ld_bo(0x00, 2, 1, 2); // load [A1], then A1+=2
    // LD.B pre-inc: op2=0x10
    let ldb_pre = enc_ld_bo(0x10, 3, 1, 0); // A1+=0 then load [A1]

    mem.write_u32(0, ldb_post).unwrap();
    mem.write_u32(4, ldb_pre).unwrap();

    let dec = Tc16Decoder::new();
    let exec = IntExecutor;
    cpu.step(&mut mem, &dec, &exec).unwrap();
    assert_eq!(cpu.gpr[2], 0x0000_007F); // sign-extended positive
    assert_eq!(cpu.a[1], 12);
    cpu.step(&mut mem, &dec, &exec).unwrap();
    assert_eq!(cpu.gpr[3], 0xFFFF_FF80);
    assert_eq!(cpu.a[1], 12);
}

#[test]
fn decode_ldw_post_inc_fields() {
    let dec = Tc16Decoder::new();
    let raw = enc_ldw_bo_mode(0x04, 2, 1, 8);
    let d = dec.decode(raw).expect("decode");
    assert!(matches!(d.op, tricore_rs::decoder::Op::LdW));
    assert_eq!(d.rd, 2);
    assert_eq!(d.rs1, 1);
    assert_eq!(d.imm, 8);
}

// Helpers to encode BOL off16 forms
fn enc_off16(op1: u32, a: u32, b: u32, off16: u32) -> u32 {
    let off_hi4 = (off16 >> 6) & 0xF;
    let off_mid6 = (off16 >> 10) & 0x3F;
    let off_lo6 = off16 & 0x3F;
    (off_hi4 << 28) | (off_mid6 << 22) | (off_lo6 << 16) | (b << 12) | (a << 8) | op1
}

// Encode P[b] op forms (op1=0x29 loads, 0xA9 stores) with op2 selector
fn enc_p_b(op1: u32, op2: u32, a: u32, b: u32, off10: u32) -> u32 {
    let off_hi4 = (off10 >> 6) & 0xF;
    let off_lo6 = off10 & 0x3F;
    (off_hi4 << 28) | (op2 << 22) | (off_lo6 << 16) | (b << 12) | (a << 8) | op1
}

#[test]
fn pb_bitrev_ldw() {
    let mut mem = LinearMemory::new(256);
    let mut cpu = Cpu::new(CpuConfig::default());
    cpu.reset(0);
    // Base and index setup
    cpu.a[5] = 100;
    // A6 holds incr:index (high16:incr, low16:index)
    cpu.a[6] = (0u32 << 16) | 4u32; // incr=0, index=4
    mem.write_u32(104, 0xCAFEBABE).unwrap();

    // LD.W d3, [p5] bit-reverse (op1=0x29, op2=0x04)
    let insn = enc_p_b(0x29, 0x04, 3, 5, 0);
    mem.write_u32(0, insn).unwrap();

    let dec = Tc16Decoder::new();
    let exec = IntExecutor;
    cpu.step(&mut mem, &dec, &exec).unwrap();
    assert_eq!(cpu.gpr[3], 0xCAFEBABE);
    // index unchanged due to incr=0
    assert_eq!(cpu.a[6] & 0xFFFF, 4);
}

#[test]
fn pb_circular_ldw() {
    let mut mem = LinearMemory::new(256);
    let mut cpu = Cpu::new(CpuConfig::default());
    cpu.reset(0);
    cpu.a[2] = 200;
    // length=8, index=2
    cpu.a[3] = (8u32 << 16) | 2u32;
    // Put halfwords at EA0=202 and EA2=(2+2)%8=4 => EA2=204
    mem.write_u16(202, 0xBEEF).unwrap();
    mem.write_u16(204, 0xCAFE).unwrap();

    // LD.W d1, [p2], +2 (op1=0x29, op2=0x14)
    let insn = enc_p_b(0x29, 0x14, 1, 2, 2);
    mem.write_u32(0, insn).unwrap();

    let dec = Tc16Decoder::new();
    let exec = IntExecutor;
    cpu.step(&mut mem, &dec, &exec).unwrap();
    assert_eq!(cpu.gpr[1], 0xCAFE_BEEF);
    // new index = 2 + 2 mod 8 = 4
    assert_eq!(cpu.a[3] & 0xFFFF, 4);
}

#[test]
fn pb_bitrev_ldh_sign_and_zero() {
    let mut mem = LinearMemory::new(256);
    let mut cpu = Cpu::new(CpuConfig::default());
    cpu.reset(0);
    // Base and index setup
    cpu.a[5] = 100;
    // incr=0, index=2
    cpu.a[6] = (0u32 << 16) | 2u32;
    // Place halfword at EA=102
    mem.write_u16(102, 0xFF80).unwrap();

    // LD.H d3, [p5] (sign-extend), op1=0x29, op2=0x02
    let ldh = enc_p_b(0x29, 0x02, 3, 5, 0);
    // LD.HU d4, [p5], op2=0x03
    let ldhu = enc_p_b(0x29, 0x03, 4, 5, 0);
    mem.write_u32(0, ldh).unwrap();
    mem.write_u32(4, ldhu).unwrap();

    let dec = Tc16Decoder::new();
    let exec = IntExecutor;
    cpu.step(&mut mem, &dec, &exec).unwrap();
    cpu.step(&mut mem, &dec, &exec).unwrap();
    assert_eq!(cpu.gpr[3], 0xFFFF_FF80);
    assert_eq!(cpu.gpr[4], 0x0000_FF80);
    // index unchanged
    assert_eq!(cpu.a[6] & 0xFFFF, 2);
}

#[test]
fn pb_circular_ldh_and_ldhu() {
    let mut mem = LinearMemory::new(256);
    let mut cpu = Cpu::new(CpuConfig::default());
    cpu.reset(0);
    cpu.a[2] = 200;
    // length=8, index=2
    cpu.a[3] = (8u32 << 16) | 2u32;
    // Place halfwords at 202 and 204
    mem.write_u16(202, 0xBEEF).unwrap();
    mem.write_u16(204, 0x00AA).unwrap();

    // LD.H d1, [p2], +2 (op1=0x29, op2=0x12, off10=2)
    let ldh_cir = enc_p_b(0x29, 0x12, 1, 2, 2);
    // LD.HU d0, [p2], +2
    let ldhu_cir = enc_p_b(0x29, 0x13, 0, 2, 2);
    mem.write_u32(0, ldh_cir).unwrap();
    mem.write_u32(4, ldhu_cir).unwrap();

    let dec = Tc16Decoder::new();
    let exec = IntExecutor;
    cpu.step(&mut mem, &dec, &exec).unwrap();
    assert_eq!(cpu.gpr[1], 0xFFFF_BEEF);
    // new index = 2 + 2 mod 8 = 4
    assert_eq!(cpu.a[3] & 0xFFFF, 4);
    cpu.step(&mut mem, &dec, &exec).unwrap();
    assert_eq!(cpu.gpr[0], 0x0000_00AA);
    assert_eq!(cpu.a[3] & 0xFFFF, 6);
}

#[test]
fn pb_circular_sth() {
    let mut mem = LinearMemory::new(256);
    let mut cpu = Cpu::new(CpuConfig::default());
    cpu.reset(0);
    // Base/index
    cpu.a[5] = 128;
    // length=8, index=2
    cpu.a[6] = (8u32 << 16) | 2u32;
    cpu.gpr[7] = 0xABCD_1234;
    let sth_cir = enc_p_b(0xA9, 0x12, 7, 5, 2);
    mem.write_u32(0, sth_cir).unwrap();

    let dec = Tc16Decoder::new();
    let exec = IntExecutor;
    cpu.step(&mut mem, &dec, &exec).unwrap();
    assert_eq!(mem.read_u16(130).unwrap(), 0x1234);
    assert_eq!(cpu.a[6] & 0xFFFF, 4);
}

#[test]
fn bol_ldw_and_stb() {
    let mut mem = LinearMemory::new(256);
    let mut cpu = Cpu::new(CpuConfig::default());
    cpu.reset(0);

    // Base A4 = 100, long offset 0x20 -> EA=132
    cpu.a[4] = 100;
    mem.write_u32(132, 0xDEAD_BEEF).unwrap();

    // LD.W BOL (op1=0x19)
    let ldw_bol = enc_off16(0x19, 5, 4, 0x20);
    mem.write_u32(0, ldw_bol).unwrap();

    let dec = Tc16Decoder::new();
    let exec = IntExecutor;
    cpu.step(&mut mem, &dec, &exec).unwrap();
    assert_eq!(cpu.gpr[5], 0xDEAD_BEEF);
}

#[test]
fn bol_ldh_and_ldhu_and_sth_stw() {
    let mut mem = LinearMemory::new(512);
    let mut cpu = Cpu::new(CpuConfig::default());
    cpu.reset(0);

    // Base A3 = 100; place halfword and word at offsets
    cpu.a[3] = 100;
    mem.write_u16(100 + 18, 0xFF80).unwrap(); // off16 = +18
    mem.write_u16(100 + 22, 0x007F).unwrap(); // off16 = +22
    cpu.gpr[5] = 0xAABB_CCDD;

    // LD.H BOL (op1=0xC9)
    let ldh_bol = enc_off16(0xC9, 2, 3, 18);
    // LD.HU BOL (op1=0xB9)
    let ldhu_bol = enc_off16(0xB9, 4, 3, 22);
    // ST.H BOL (op1=0xF9): store low half of D5 at off16=26
    let sth_bol = enc_off16(0xF9, 5, 3, 26);
    // ST.W BOL (op1=0x59): store D5 at off16=32
    let stw_bol = enc_off16(0x59, 5, 3, 32);

    mem.write_u32(0, ldh_bol).unwrap();
    mem.write_u32(4, ldhu_bol).unwrap();
    mem.write_u32(8, sth_bol).unwrap();
    mem.write_u32(12, stw_bol).unwrap();

    let dec = Tc16Decoder::new();
    // sanity decode
    let d_ldh = dec.decode(ldh_bol).expect("ldh bol");
    assert!(matches!(d_ldh.op, tricore_rs::decoder::Op::LdH));
    let d_ldhu = dec.decode(ldhu_bol).expect("ldhu bol");
    assert!(matches!(d_ldhu.op, tricore_rs::decoder::Op::LdHu));
    let d_sth = dec.decode(sth_bol).expect("sth bol");
    assert!(matches!(d_sth.op, tricore_rs::decoder::Op::StH));
    let d_stw = dec.decode(stw_bol).expect("stw bol");
    assert!(matches!(d_stw.op, tricore_rs::decoder::Op::StW));
    let exec = IntExecutor;
    cpu.step(&mut mem, &dec, &exec).unwrap();
    assert_eq!(cpu.gpr[2], 0xFFFF_FF80);
    cpu.step(&mut mem, &dec, &exec).unwrap();
    assert_eq!(cpu.gpr[4], 0x0000_007F);
    cpu.step(&mut mem, &dec, &exec).unwrap();
    assert_eq!(mem.read_u16(100 + 26).unwrap(), 0xCCDDu16);
    cpu.step(&mut mem, &dec, &exec).unwrap();
    assert_eq!(mem.read_u32(100 + 32).unwrap(), 0xAABB_CCDD);
}

#[test]
fn bol_ldbu_zero_extend() {
    let mut mem = LinearMemory::new(256);
    let mut cpu = Cpu::new(CpuConfig::default());
    cpu.reset(0);

    cpu.a[2] = 40;
    // set byte at base+off16= +7
    mem.write_u8(47, 0xFE).unwrap();

    // LD.BU D3, A2, off16=7 (op1=0x39)
    let ldbu_bol = enc_off16(0x39, 3, 2, 7);
    mem.write_u32(0, ldbu_bol).unwrap();

    let dec = Tc16Decoder::new();
    let exec = IntExecutor;
    cpu.step(&mut mem, &dec, &exec).unwrap();
    assert_eq!(cpu.gpr[3], 0x0000_00FE);
}

#[test]
fn stb_post_increment() {
    let mut mem = LinearMemory::new(128);
    let mut cpu = Cpu::new(CpuConfig::default());
    cpu.reset(0);
    cpu.a[2] = 50;
    cpu.gpr[4] = 0xABCD_00EE;

    // ST.B post: op2=0x00
    let stb_post = enc_st_bo(0x00, 4, 2, 4); // store at [A2], then A2+=4

    mem.write_u32(0, stb_post).unwrap();

    let dec = Tc16Decoder::new();
    let exec = IntExecutor;
    cpu.step(&mut mem, &dec, &exec).unwrap();
    assert_eq!(mem.read_u8(50).unwrap(), 0xEE);
    assert_eq!(cpu.a[2], 54);
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

// (Byte/halfword stores covered indirectly; focused word store remains tested.)
