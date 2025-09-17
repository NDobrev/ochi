use tricore_rs::isa::tc16::Tc16Decoder;
use tricore_rs::decoder::{Decoder, Op};

#[test]
fn decode_call_disp24_and_ret() {
    let dec = Tc16Decoder::new();
    // CALL disp24 with op1=0x6D; encode disp24=+2 (=> +4 bytes)
    let disp24: u32 = 2;
    let raw32 = ((disp24 & 0xFFFF) << 16) | ((disp24 >> 16) << 8) | 0x6D;
    let d = dec.decode(raw32).expect("call");
    assert!(matches!(d.op, Op::Call));
    assert_eq!(d.width, 4);
    assert_eq!(d.imm, ((disp24 as i32) << 1) as u32);

    // CALLA disp24 (op1=0xED)
    let raw_calla = ((disp24 & 0xFFFF) << 16) | ((disp24 >> 16) << 8) | 0xED;
    let d2 = dec.decode(raw_calla).expect("calla");
    assert!(matches!(d2.op, Op::CallA));
    assert_eq!(d2.width, 4);
    assert!(d2.abs);

    // RET (SYS) op1=0x0D
    let d3 = dec.decode(0x0D).expect("ret");
    assert!(matches!(d3.op, Op::Ret));
    assert_eq!(d3.width, 4);
}

#[test]
fn decode_call_disp8() {
    let dec = Tc16Decoder::new();
    // CALL disp8 (op1=0x5C), disp8=1 => +2 bytes
    let raw16: u16 = ((1u16 as u16) << 8) | 0x5Cu16;
    let d = dec.decode(raw16 as u32).expect("call16");
    assert!(matches!(d.op, Op::Call));
    assert_eq!(d.width, 2);
    assert_eq!(d.imm, 2);
}

