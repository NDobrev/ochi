use tricore_rs::exec::IntExecutor;
use tricore_rs::isa::tc16::Tc16Decoder;
use tricore_rs::{Cpu, CpuConfig, LinearMemory};
use tricore_rs::Bus;

fn enc_abs_common(op1: u32, sel: u32, a: u32, off18: u32) -> u32 {
    let off9_6 = (off18 >> 6) & 0xF;
    let off13_10 = (off18 >> 10) & 0xF;
    let off5_0 = off18 & 0x3F;
    let off17_14 = (off18 >> 14) & 0xF;
    // Layout: [31:28]=off9_6, [27:26]=sel_high? embedded via sel nibble in off9_6 for these forms
    (off9_6 << 28) | (off13_10 << 22) | (off5_0 << 16) | (off17_14 << 12) | (a << 8) | op1
}

fn abs_ea(off18: u32) -> u32 {
    let top4 = (off18 >> 14) & 0xF;
    let low14 = off18 & 0x3FFF;
    (top4 << 28) | low14
}

#[test]
fn lea_abs_sets_address() {
    let mut mem = LinearMemory::new(256);
    let mut cpu = Cpu::new(CpuConfig::default());
    cpu.reset(0);
    // off18: top4=0, low14=40 => EA=40
    let off18 = 40u32; 
    let insn = enc_abs_common(0xC5, 0, 3, off18); // LEA ABS A3, off18
    mem.write_u32(0, insn).unwrap();
    let dec = Tc16Decoder::new();
    let exec = IntExecutor;
    cpu.step(&mut mem, &dec, &exec).unwrap();
    assert_eq!(cpu.a[3], abs_ea(off18));
}

#[test]
fn ldw_abs_and_stb_abs() {
    let mut mem = LinearMemory::new(256);
    let mut cpu = Cpu::new(CpuConfig::default());
    cpu.reset(0);

    // Prepare memory
    let off18 = 32u32; // EA=32, ensures off18[9:6]=0
    mem.write_u32(abs_ea(off18), 0xA1B2_C3D4).unwrap();
    // LD.W D5, [ABS off18]
    let ldw = enc_abs_common(0x85, 0, 5, off18);
    // ST.B [ABS off18+4], D5
    let off18_b = 36u32; // next byte (off18[9:6]=0)
    let stb = enc_abs_common(0x25, 0, 5, off18_b);

    mem.write_u32(0, ldw).unwrap();
    mem.write_u32(4, stb).unwrap();

    let dec = Tc16Decoder::new();
    let exec = IntExecutor;
    cpu.step(&mut mem, &dec, &exec).unwrap();
    assert_eq!(cpu.gpr[5], 0xA1B2_C3D4);
    cpu.step(&mut mem, &dec, &exec).unwrap();
    assert_eq!(mem.read_u8(abs_ea(off18_b)).unwrap(), 0xD4);
}

// (LD.H/LD.HU ABS encodings exist in the decoder; exercised indirectly via BO path in other tests.)
