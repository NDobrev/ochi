use tricore_rs::isa::tc16::Tc16Decoder;
use tricore_rs::disasm::fmt_decoded;
use tricore_rs::decoder::Decoder;

#[test]
fn disasm_add_mov() {
    let dec = Tc16Decoder::new();
    // ADDI d3, d1, #0x10
    let addi = (3u32<<28) | (0x10u32<<12) | (1u32<<8) | 0x1B;
    let d = dec.decode(addi).unwrap();
    let s = fmt_decoded(&d);
    assert!(s.starts_with("addi d3, d1, 0x10"));

    // MOV.U d2, #0x1234
    let movu = (2u32<<28) | (0x1234u32<<12) | 0xBB;
    let d2 = dec.decode(movu).unwrap();
    let s2 = fmt_decoded(&d2);
    assert!(s2.starts_with("mov d2, #0x1234"));
}

