use crate::decoder::{Decoded, Decoder, Op};

/// TriCore TC1.6.2 decoder (initial subset)
/// Implements a small, representative slice of the official encodings
/// based on the TC1.6.2 Instruction Set manual in `spec/`.
pub struct Tc16Decoder;

impl Tc16Decoder {
    pub fn new() -> Self {
        Self
    }
}

impl Decoder for Tc16Decoder {
    fn decode(&self, raw32: u32) -> Option<Decoded> {
        // Helper closures
        #[inline]
        fn sign_ext(v: u32, bits: u32) -> u32 {
            let s = 32 - bits;
            ((v << s) as i32 >> s) as u32
        }
        #[inline]
        fn off18_from_fields(raw32: u32) -> u32 {
            let off9_6 = (raw32 >> 28) & 0xF; // off18[9:6]
            let off13_10 = (raw32 >> 22) & 0xF; // off18[13:10]
            let off5_0 = (raw32 >> 16) & 0x3F; // off18[5:0]
            let off17_14 = (raw32 >> 12) & 0xF; // off18[17:14]
            (off17_14 << 14) | (off13_10 << 10) | (off9_6 << 6) | off5_0
        }
        #[inline]
        fn abs_ea_from_off18(off18: u32) -> u32 {
            let top4 = (off18 >> 14) & 0xF;
            let low14 = off18 & 0x3FFF;
            (top4 << 28) | low14
        }

        // op1 is the low byte of the instruction word; bit 0 distinguishes width
        let op1 = (raw32 & 0xFF) as u8;
        let is_16 = (op1 & 1) == 0;

        if is_16 {
            let raw16 = (raw32 & 0xFFFF) as u16;
            match op1 {
                0x3C => {
                    // J disp8 (SB)
                    let disp8 = ((raw16 >> 8) & 0xFF) as u32;
                    let off = sign_ext(disp8, 8) << 1;
                    return Some(Decoded { op: Op::J, width: 2, rd: 0, rs1: 0, rs2: 0, imm: off, imm2: 0, abs: false, wb: false, pre: false });
                }
                0x42 => {
                    // ADD D[a], D[b] (SRR)
                    let b = ((raw16 >> 12) & 0xF) as u8;
                    let a = ((raw16 >> 8) & 0xF) as u8;
                    return Some(Decoded {
                        op: Op::Add,
                        width: 2,
                        rd: a,
                        rs1: a,
                        rs2: b,
                        imm: 0,
                        imm2: 0,
                        abs: false,
                        wb: false,
                        pre: false,
                    });
                }
                0xC2 => {
                    // ADD D[a], const4 (SRC)
                    let const4 = ((raw16 >> 12) & 0xF) as u32;
                    let a = ((raw16 >> 8) & 0xF) as u8;
                    return Some(Decoded {
                        op: Op::Add,
                        width: 2,
                        rd: a,
                        rs1: a,
                        rs2: 0,
                        imm: sign_ext(const4, 4),
                        imm2: 0,
                        abs: false,
                        wb: false,
                        pre: false,
                    });
                }
                0x82 => {
                    // MOV D[a], const4 (SRC)
                    let const4 = ((raw16 >> 12) & 0xF) as u32;
                    let a = ((raw16 >> 8) & 0xF) as u8;
                    return Some(Decoded {
                        op: Op::MovI,
                        width: 2,
                        rd: a,
                        rs1: 0,
                        rs2: 0,
                        imm: sign_ext(const4, 4),
                        imm2: 0,
                        abs: false,
                        wb: false,
                        pre: false,
                    });
                }
                0x26 => {
                    // AND D[a], D[b] (SRR)
                    let b = ((raw16 >> 12) & 0xF) as u8;
                    let a = ((raw16 >> 8) & 0xF) as u8;
                    return Some(Decoded { op: Op::And, width: 2, rd: a, rs1: a, rs2: b, imm: 0, imm2: 0, abs: false, wb: false, pre: false });
                }
                0xA6 => {
                    // OR D[a], D[b] (SRR)
                    let b = ((raw16 >> 12) & 0xF) as u8;
                    let a = ((raw16 >> 8) & 0xF) as u8;
                    return Some(Decoded { op: Op::Or, width: 2, rd: a, rs1: a, rs2: b, imm: 0, imm2: 0, abs: false, wb: false, pre: false });
                }
                0xC6 => {
                    // XOR D[a], D[b] (SRR)
                    let b = ((raw16 >> 12) & 0xF) as u8;
                    let a = ((raw16 >> 8) & 0xF) as u8;
                    return Some(Decoded { op: Op::Xor, width: 2, rd: a, rs1: a, rs2: b, imm: 0, imm2: 0, abs: false, wb: false, pre: false });
                }
                0x1E | 0x9E => {
                    // JEQ D[15], const4, disp4 (SBC)
                    let const4 = ((raw16 >> 12) & 0xF) as u32;
                    let disp4 = ((raw16 >> 8) & 0xF) as u32;
                    let add16 = if op1 == 0x9E { 16 } else { 0 };
                    let off = ((disp4 + add16) << 1) as u32;
                    return Some(Decoded { op: Op::JeqImm, width: 2, rd: 0, rs1: 15, rs2: 0, imm: off, imm2: sign_ext(const4, 4), abs: false, wb: false, pre: false });
                }
                0x5E | 0xDE => {
                    // JNE D[15], const4, disp4 (SBC)
                    let const4 = ((raw16 >> 12) & 0xF) as u32;
                    let disp4 = ((raw16 >> 8) & 0xF) as u32;
                    let add16 = if op1 == 0xDE { 16 } else { 0 };
                    let off = ((disp4 + add16) << 1) as u32;
                    return Some(Decoded { op: Op::JneImm, width: 2, rd: 0, rs1: 15, rs2: 0, imm: off, imm2: sign_ext(const4, 4), abs: false, wb: false, pre: false });
                }
                0x3E | 0xBE => {
                    // JEQ D[15], D[b], disp4 (SBR)
                    let b = ((raw16 >> 12) & 0xF) as u8;
                    let disp4 = ((raw16 >> 8) & 0xF) as u32;
                    let add16 = if op1 == 0xBE { 16 } else { 0 };
                    let off = ((disp4 + add16) << 1) as u32;
                    return Some(Decoded {
                        op: Op::Jeq,
                        width: 2,
                        rd: 0,
                        rs1: 15,
                        rs2: b,
                        imm: off,
                        imm2: 0,
                        abs: false,
                        wb: false,
                        pre: false,
                    });
                }
                0x7E | 0xFE => {
                    // JNE D[15], D[b], disp4 (SBR)
                    let b = ((raw16 >> 12) & 0xF) as u8;
                    let disp4 = ((raw16 >> 8) & 0xF) as u32;
                    let add16 = if op1 == 0xFE { 16 } else { 0 };
                    let off = ((disp4 + add16) << 1) as u32;
                    return Some(Decoded {
                        op: Op::Jne,
                        width: 2,
                        rd: 0,
                        rs1: 15,
                        rs2: b,
                        imm: off,
                        imm2: 0,
                        abs: false,
                        wb: false,
                        pre: false,
                    });
                }
                _ => return None,
            }
        }

        // 32-bit encodings (op1 bit0 == 1)
        match op1 {
            0x1D => {
                // J disp24 (B)
                let disp_low16 = ((raw32 >> 16) & 0xFFFF) as u32;
                let disp_hi8 = ((raw32 >> 8) & 0xFF) as u32;
                let disp24 = (disp_hi8 << 16) | disp_low16;
                let off = sign_ext(disp24, 24) << 1;
                return Some(Decoded { op: Op::J, width: 4, rd: 0, rs1: 0, rs2: 0, imm: off, imm2: 0, abs: false, wb: false, pre: false });
            }
            0x91 => {
                // MOVH.A A[c], const16 (RLC)
                let c = ((raw32 >> 28) & 0xF) as u8;
                let imm16 = ((raw32 >> 12) & 0xFFFF) as u32;
                return Some(Decoded { op: Op::MovHA, width: 4, rd: c, rs1: 0, rs2: 0, imm: imm16 << 16, imm2: 0, abs: false, wb: false, pre: false });
            }
            0x11 => {
                // ADDIH.A A[c], A[a], const16 (RLC)
                let c = ((raw32 >> 28) & 0xF) as u8;
                let a = ((raw32 >> 8) & 0xF) as u8;
                let imm16 = ((raw32 >> 12) & 0xFFFF) as u32;
                return Some(Decoded { op: Op::Lea, width: 4, rd: c, rs1: a, rs2: 0, imm: imm16 << 16, imm2: 0, abs: false, wb: false, pre: false });
            }
            0x0B => {
                let op2 = ((raw32 >> 20) & 0xFF) as u32;
                match op2 {
                    0x00 => {
                        // ADD D[c], D[a], D[b] (RR)
                        let c = ((raw32 >> 28) & 0xF) as u8;
                        let b = ((raw32 >> 16) & 0xF) as u8;
                        let a = ((raw32 >> 8) & 0xF) as u8;
                Some(Decoded { op: Op::Add, width: 4, rd: c, rs1: a, rs2: b, imm: 0, imm2: 0, abs: false, wb: false, pre: false })
                    }
                    0x08 => {
                        // SUB D[c], D[a], D[b] (RR)
                        let c = ((raw32 >> 28) & 0xF) as u8;
                        let b = ((raw32 >> 16) & 0xF) as u8;
                        let a = ((raw32 >> 8) & 0xF) as u8;
                        Some(Decoded { op: Op::Sub, width: 4, rd: c, rs1: a, rs2: b, imm: 0, imm2: 0, abs: false, wb: false, pre: false })
                    }
                    0x1F => {
                        // MOV D[c], D[b] (RR)
                        let c = ((raw32 >> 28) & 0xF) as u8;
                        let b = ((raw32 >> 16) & 0xF) as u8;
                        Some(Decoded { op: Op::Mov, width: 4, rd: c, rs1: b, rs2: 0, imm: 0, imm2: 0, abs: false, wb: false, pre: false })
                    }
                    _ => None,
                }
            }
            0x49 => {
                // LEA A[a], A[b], off10 (BO) with op2 == 0x28
                if ((raw32 >> 22) & 0x3F) != 0x28 { return None; }
                let off_upper4 = ((raw32 >> 28) & 0xF) as u32;
                let off_lower6 = ((raw32 >> 16) & 0x3F) as u32;
                let off10 = (off_upper4 << 6) | off_lower6;
                let b = ((raw32 >> 12) & 0xF) as u8;
                let a = ((raw32 >> 8) & 0xF) as u8;
                return Some(Decoded { op: Op::Lea, width: 4, rd: a, rs1: b, rs2: 0, imm: sign_ext(off10, 10), imm2: 0, abs: false, wb: false, pre: false });
            }
            0xD9 => {
                // LEA A[a], A[b], off16 (BOL)
                let off_hi4 = ((raw32 >> 28) & 0xF) as u32;      // off16[9:6]
                let off_mid6 = ((raw32 >> 22) & 0x3F) as u32;    // off16[15:10]
                let off_lo6 = ((raw32 >> 16) & 0x3F) as u32;     // off16[5:0]
                let off16 = (off_mid6 << 10) | (off_hi4 << 6) | off_lo6;
                let b = ((raw32 >> 12) & 0xF) as u8;
                let a = ((raw32 >> 8) & 0xF) as u8;
                return Some(Decoded { op: Op::Lea, width: 4, rd: a, rs1: b, rs2: 0, imm: sign_ext(off16, 16), imm2: 0, abs: false, wb: false, pre: false });
            }
            0x0F => {
                // Logical RR: op2 selects AND/OR/XOR (0x08/0x0A/0x0C)
                let op2 = ((raw32 >> 20) & 0xFF) as u32;
                let c = ((raw32 >> 28) & 0xF) as u8;
                let b = ((raw32 >> 16) & 0xF) as u8;
                let a = ((raw32 >> 8) & 0xF) as u8;
                let op = match op2 {
                    0x08 => Op::And,
                    0x0A => Op::Or,
                    0x0C => Op::Xor,
                    _ => return None,
                };
                Some(Decoded { op, width: 4, rd: c, rs1: a, rs2: b, imm: 0, imm2: 0, abs: false, wb: false, pre: false })
            }
            0x8F => {
                // Logical RC with const9: AND/OR/XOR via op2 (0x08/0x0A/0x0C)
                let op2 = ((raw32 >> 21) & 0x7F) as u32;
                let c = ((raw32 >> 28) & 0xF) as u8;
                let a = ((raw32 >> 8) & 0xF) as u8;
                let const9 = ((raw32 >> 12) & 0x1FF) as u32;
                let op = match op2 {
                    0x08 => Op::And,
                    0x0A => Op::Or,
                    0x0C => Op::Xor,
                    _ => return None,
                };
                Some(Decoded { op, width: 4, rd: c, rs1: a, rs2: 0, imm: const9, imm2: 0, abs: false, wb: false, pre: false })
            }
            0x8B => {
                // RC forms by op2 in [27:21]
                let op2 = ((raw32 >> 21) & 0x7F) as u32;
                let c = ((raw32 >> 28) & 0xF) as u8;
                let a = ((raw32 >> 8) & 0xF) as u8;
                let imm9 = ((raw32 >> 12) & 0x1FF) as u32;
                match op2 {
                    0x00 => Some(Decoded { op: Op::Add, width: 4, rd: c, rs1: a, rs2: 0, imm: sign_ext(imm9, 9), imm2: 0, abs: false, wb: false, pre: false }),
                    0x08 => Some(Decoded { op: Op::Sub, width: 4, rd: c, rs1: a, rs2: 0, imm: sign_ext(imm9, 9), imm2: 0, abs: false, wb: false, pre: false }),
                    _ => None,
                }
            }
            0x1B => {
                // ADDI D[c], D[a], const16 (RLC)
                let c = ((raw32 >> 28) & 0xF) as u8;
                let a = ((raw32 >> 8) & 0xF) as u8;
                let imm16 = ((raw32 >> 12) & 0xFFFF) as u32;
                Some(Decoded {
                    op: Op::Add,
                    width: 4,
                    rd: c,
                    rs1: a,
                    rs2: 0,
                    imm: sign_ext(imm16, 16),
                    imm2: 0,
                    abs: false,
                    wb: false,
                    pre: false,
                })
            }
            0x9B => {
                // ADDIH D[c], D[a], const16 (RLC)
                let c = ((raw32 >> 28) & 0xF) as u8;
                let a = ((raw32 >> 8) & 0xF) as u8;
                let imm16 = ((raw32 >> 12) & 0xFFFF) as u32;
                Some(Decoded {
                    op: Op::Add,
                    width: 4,
                    rd: c,
                    rs1: a,
                    rs2: 0,
                    imm: imm16 << 16,
                    imm2: 0,
                    abs: false,
                    wb: false,
                    pre: false,
                })
            }
            0x3B => {
                // MOV D[c], const16 (RLC) sign-extended
                let c = ((raw32 >> 28) & 0xF) as u8;
                let imm16 = ((raw32 >> 12) & 0xFFFF) as u32;
                Some(Decoded {
                    op: Op::MovI,
                    width: 4,
                    rd: c,
                    rs1: 0,
                    rs2: 0,
                    imm: sign_ext(imm16, 16),
                    imm2: 0,
                    abs: false,
                    wb: false,
                    pre: false,
                })
            }
            0xBB => {
                // MOV.U D[c], const16 (RLC) zero-extended
                let c = ((raw32 >> 28) & 0xF) as u8;
                let imm16 = ((raw32 >> 12) & 0xFFFF) as u32;
                Some(Decoded {
                    op: Op::MovI,
                    width: 4,
                    rd: c,
                    rs1: 0,
                    rs2: 0,
                    imm: imm16,
                    imm2: 0,
                    abs: false,
                    wb: false,
                    pre: false,
                })
            }
            0x7B => {
                // MOVH D[c], const16 (RLC) -> imm << 16
                let c = ((raw32 >> 28) & 0xF) as u8;
                let imm16 = ((raw32 >> 12) & 0xFFFF) as u32;
                Some(Decoded {
                    op: Op::MovI,
                    width: 4,
                    rd: c,
                    rs1: 0,
                    rs2: 0,
                    imm: imm16 << 16,
                    imm2: 0,
                    abs: false,
                    wb: false,
                    pre: false,
                })
            }
            0xC5 => {
                // LEA A[a], off18 (ABS)
                let off18 = off18_from_fields(raw32);
                let ea = abs_ea_from_off18(off18);
                let a = ((raw32 >> 8) & 0xF) as u8;
                return Some(Decoded { op: Op::Lea, width: 4, rd: a, rs1: 0, rs2: 0, imm: ea, imm2: 0, abs: true, wb: false, pre: false });
            }
            0x05 => {
                // LD.B/LD.BU/LD.H/LD.HU ABS (selector in off18[9:6])
                let sel = ((raw32 >> 28) & 0xF) as u32;
                let a = ((raw32 >> 8) & 0xF) as u8;
                let off18 = off18_from_fields(raw32);
                let ea = abs_ea_from_off18(off18);
                let op = match sel {
                    0x00 => Op::LdB,
                    0x01 => Op::LdBu,
                    0x02 => Op::LdH,
                    0x03 => Op::LdHu,
                    _ => return None,
                };
                return Some(Decoded { op, width: 4, rd: a, rs1: 0, rs2: 0, imm: ea, imm2: 0, abs: true, wb: false, pre: false });
            }
            0x85 => {
                // LD.W ABS (selector must be 0x00)
                let sel = ((raw32 >> 28) & 0xF) as u32;
                if sel != 0 { return None; }
                let a = ((raw32 >> 8) & 0xF) as u8;
                let off18 = off18_from_fields(raw32);
                let ea = abs_ea_from_off18(off18);
                return Some(Decoded { op: Op::LdW, width: 4, rd: a, rs1: 0, rs2: 0, imm: ea, imm2: 0, abs: true, wb: false, pre: false });
            }
            0x09 => {
                // BO load family: op2 selects the element size and addressing mode
                let op2 = ((raw32 >> 22) & 0x3F) as u32;
                let off_upper4 = ((raw32 >> 28) & 0xF) as u32;
                let off_lower6 = ((raw32 >> 16) & 0x3F) as u32;
                let off10 = (off_upper4 << 6) | off_lower6;
                let b = ((raw32 >> 12) & 0xF) as u8;
                let a = ((raw32 >> 8) & 0xF) as u8;
                let (op, wb, pre) = match op2 {
                    // Post-increment
                    0x00 => (Op::LdB, true, false),
                    0x01 => (Op::LdBu, true, false),
                    0x02 => (Op::LdH, true, false),
                    0x03 => (Op::LdHu, true, false),
                    0x04 => (Op::LdW, true, false),
                    // Pre-increment
                    0x10 => (Op::LdB, true, true),
                    0x11 => (Op::LdBu, true, true),
                    0x12 => (Op::LdH, true, true),
                    0x13 => (Op::LdHu, true, true),
                    0x14 => (Op::LdW, true, true),
                    // Base + short offset (no write-back)
                    0x20 => (Op::LdB, false, false),
                    0x21 => (Op::LdBu, false, false),
                    0x22 => (Op::LdH, false, false),
                    0x23 => (Op::LdHu, false, false),
                    0x24 => (Op::LdW, false, false),
                    _ => return None,
                };
                Some(Decoded { op, width: 4, rd: a, rs1: b, rs2: 0, imm: sign_ext(off10, 10), imm2: 0, abs: false, wb, pre })
            }
            0x89 => {
                // BO store family: op2 selects size and addressing mode
                let op2 = ((raw32 >> 22) & 0x3F) as u32;
                let off_upper4 = ((raw32 >> 28) & 0xF) as u32;
                let off_lower6 = ((raw32 >> 16) & 0x3F) as u32;
                let off10 = (off_upper4 << 6) | off_lower6;
                let b = ((raw32 >> 12) & 0xF) as u8;
                let a = ((raw32 >> 8) & 0xF) as u8;
                let (op, wb, pre) = match op2 {
                    // Post-increment
                    0x00 => (Op::StB, true, false),
                    0x02 => (Op::StH, true, false),
                    0x04 => (Op::StW, true, false),
                    // Pre-increment
                    0x10 => (Op::StB, true, true),
                    0x12 => (Op::StH, true, true),
                    0x14 => (Op::StW, true, true),
                    // Base + short offset (no write-back)
                    0x20 => (Op::StB, false, false),
                    0x22 => (Op::StH, false, false),
                    0x24 => (Op::StW, false, false),
                    _ => return None,
                };
                Some(Decoded { op, width: 4, rd: 0, rs1: b, rs2: a, imm: sign_ext(off10, 10), imm2: 0, abs: false, wb, pre })
            }
            0x25 => {
                // ST.B/ST.H ABS via selector in off18[9:6]
                let sel = ((raw32 >> 28) & 0xF) as u32;
                let a = ((raw32 >> 8) & 0xF) as u8; // source D[a]
                let off18 = off18_from_fields(raw32);
                let ea = abs_ea_from_off18(off18);
                let op = match sel {
                    0x00 => Op::StB,
                    0x02 => Op::StH,
                    _ => return None,
                };
                return Some(Decoded { op, width: 4, rd: 0, rs1: 0, rs2: a, imm: ea, imm2: 0, abs: true, wb: false, pre: false });
            }
            0xA5 => {
                // ST.W ABS (selector must be 0x00)
                let sel = ((raw32 >> 28) & 0xF) as u32;
                if sel != 0 { return None; }
                let a = ((raw32 >> 8) & 0xF) as u8;
                let off18 = off18_from_fields(raw32);
                let ea = abs_ea_from_off18(off18);
                return Some(Decoded { op: Op::StW, width: 4, rd: 0, rs1: 0, rs2: a, imm: ea, imm2: 0, abs: true, wb: false, pre: false });
            }
            0x5F => {
                // JEQ/JNE D[a], D[b], disp15 (BRR)
                let cond = ((raw32 >> 30) & 0x3) as u8; // 00 => JEQ, 01 => JNE
                let a = ((raw32 >> 8) & 0xF) as u8;
                let b = ((raw32 >> 12) & 0xF) as u8;
                let disp15 = ((raw32 >> 15) & 0x7FFF) as u32;
                let off = sign_ext(disp15, 15) << 1;
                let op = match cond { 0 => Op::Jeq, 1 => Op::Jne, _ => return None };
                Some(Decoded { op, width: 4, rd: 0, rs1: a, rs2: b, imm: off, imm2: 0, abs: false, wb: false, pre: false })
            }
            0xDF => {
                // JEQ/JNE D[a], const4, disp15 (BRC)
                let cond = ((raw32 >> 30) & 0x3) as u8; // 00 => JEQ, 01 => JNE
                let a = ((raw32 >> 8) & 0xF) as u8;
                let const4 = ((raw32 >> 12) & 0xF) as u32;
                let disp15 = ((raw32 >> 15) & 0x7FFF) as u32;
                let off = sign_ext(disp15, 15) << 1;
                let op = match cond {
                    0 => Op::JeqImm,
                    1 => Op::JneImm,
                    _ => return None,
                };
                Some(Decoded { op, width: 4, rd: 0, rs1: a, rs2: 0, imm: off, imm2: sign_ext(const4, 4), abs: false, wb: false, pre: false })
            }
            0x7F => {
                // JGE/JGE.U D[a], D[b], disp15 (BRR)
                let unsigned = ((raw32 >> 30) & 0x3) == 0x01;
                let a = ((raw32 >> 8) & 0xF) as u8;
                let b = ((raw32 >> 12) & 0xF) as u8;
                let disp15 = ((raw32 >> 15) & 0x7FFF) as u32;
                let off = sign_ext(disp15, 15) << 1;
                let op = if unsigned { Op::JgeU } else { Op::Jge };
                Some(Decoded { op, width: 4, rd: 0, rs1: a, rs2: b, imm: off, imm2: 0, abs: false, wb: false, pre: false })
            }
            0xFF => {
                // JGE/JGE.U D[a], const4, disp15 (BRC)
                let unsigned = ((raw32 >> 30) & 0x3) == 0x01;
                let a = ((raw32 >> 8) & 0xF) as u8;
                let const4 = ((raw32 >> 12) & 0xF) as u32;
                let disp15 = ((raw32 >> 15) & 0x7FFF) as u32;
                let off = sign_ext(disp15, 15) << 1;
                let op = if unsigned { Op::JgeUImm } else { Op::JgeImm };
                let imm2 = if unsigned { const4 } else { sign_ext(const4, 4) };
                Some(Decoded { op, width: 4, rd: 0, rs1: a, rs2: 0, imm: off, imm2, abs: false, wb: false, pre: false })
            }
            0x3F => {
                // JLT/JLT.U D[a], D[b], disp15 (BRR)
                let unsigned = ((raw32 >> 30) & 0x3) == 0x01;
                let a = ((raw32 >> 8) & 0xF) as u8;
                let b = ((raw32 >> 12) & 0xF) as u8;
                let disp15 = ((raw32 >> 15) & 0x7FFF) as u32;
                let off = sign_ext(disp15, 15) << 1;
                let op = if unsigned { Op::JltU } else { Op::Jlt };
                Some(Decoded { op, width: 4, rd: 0, rs1: a, rs2: b, imm: off, imm2: 0, abs: false, wb: false, pre: false })
            }
            0xBF => {
                // JLT/JLT.U D[a], const4, disp15 (BRC)
                let unsigned = ((raw32 >> 30) & 0x3) == 0x01;
                let a = ((raw32 >> 8) & 0xF) as u8;
                let const4 = ((raw32 >> 12) & 0xF) as u32;
                let disp15 = ((raw32 >> 15) & 0x7FFF) as u32;
                let off = sign_ext(disp15, 15) << 1;
                let op = if unsigned { Op::JltUImm } else { Op::JltImm };
                let imm2 = if unsigned { const4 } else { sign_ext(const4, 4) };
                Some(Decoded { op, width: 4, rd: 0, rs1: a, rs2: 0, imm: off, imm2, abs: false, wb: false, pre: false })
            }
            // Developer convenience
            0xFF => Some(Decoded { op: Op::Syscall, width: 4, rd: 0, rs1: 0, rs2: 0, imm: 0, imm2: 0, abs: false, wb: false, pre: false }),
            _ => None,
        }
    }
}
