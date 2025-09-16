use tricore_rs::exec::IntExecutor;
use tricore_rs::isa::tc16::Tc16Decoder;
use tricore_rs::{Cpu, CpuConfig, LinearMemory};
use tricore_rs::Bus;

fn enc_movh_a(c: u32, imm16: u32) -> u32 { (c << 28) | (imm16 << 12) | 0x91 }

fn enc_lea_bo(a: u32, b: u32, off10: u32) -> u32 {
    let off_hi4 = (off10 >> 6) & 0xF;
    let off_lo6 = off10 & 0x3F;
    (off_hi4 << 28) | (0x28 << 22) | (off_lo6 << 16) | (b << 12) | (a << 8) | 0x49
}

fn enc_lea_bol(a: u32, b: u32, off16: u32) -> u32 {
    let off_hi4 = (off16 >> 6) & 0xF;    // bits 9:6
    let off_mid6 = (off16 >> 10) & 0x3F; // bits 15:10
    let off_lo6 = off16 & 0x3F;          // bits 5:0
    (off_hi4 << 28) | (off_mid6 << 22) | (off_lo6 << 16) | (b << 12) | (a << 8) | 0xD9
}

#[test]
fn movh_a_then_lea_updates_address() {
    let mut mem = LinearMemory::new(64);
    let mut cpu = Cpu::new(CpuConfig::default());
    cpu.reset(0);

    // A1 = 0x2001_0000; then add 12 => 0x2001_000C
    let movh_a = enc_movh_a(1, 0x2001);
    let lea = enc_lea_bo(1, 1, 12);

    mem.write_u32(0, movh_a).unwrap();
    mem.write_u32(4, lea).unwrap();

    let dec = Tc16Decoder::new();
    let exec = IntExecutor;
    cpu.step(&mut mem, &dec, &exec).unwrap();
    cpu.step(&mut mem, &dec, &exec).unwrap();
    assert_eq!(cpu.a[1], 0x2001_0000 + 12);
}

#[test]
fn addih_a_and_lea_bol_build_address() {
    let mut mem = LinearMemory::new(64);
    let mut cpu = Cpu::new(CpuConfig::default());
    cpu.reset(0);

    // A2 = 0x1000_0000
    let movh_a = enc_movh_a(2, 0x1000);
    // ADDIH.A A2, A2, 0x0001 => A2 += 0x0001_0000 -> 0x1001_0000
    let addih_a = (2u32 << 28) | (1u32 << 12) | (2u32 << 8) | 0x11u32;
    // LEA BOL A2, A2, 0x2345 => A2 += 0x2345 => 0x1001_2345
    let lea_bol = enc_lea_bol(2, 2, 0x2345);

    mem.write_u32(0, movh_a).unwrap();
    mem.write_u32(4, addih_a).unwrap();
    mem.write_u32(8, lea_bol).unwrap();

    let dec = Tc16Decoder::new();
    let exec = IntExecutor;
    cpu.step(&mut mem, &dec, &exec).unwrap();
    cpu.step(&mut mem, &dec, &exec).unwrap();
    cpu.step(&mut mem, &dec, &exec).unwrap();
    assert_eq!(cpu.a[2], 0x1001_2345);
}
