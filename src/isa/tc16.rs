use crate::decoder::{Decoded, Decoder, Op};

/// Example TC1.6-ish decoder skeleton with a tiny, fake subset.
/// Replace patterns with the actual bit layouts.
pub struct Tc16Decoder;

impl Tc16Decoder {
    pub fn new() -> Self {
        Self
    }
}

impl Decoder for Tc16Decoder {
    fn decode(&self, raw32: u32) -> Option<Decoded> {
        // Inspect top bits to decide 16 vs 32 width (placeholder rule):
        let top5 = (raw32 >> 27) & 0x1F;
        if top5 == 0b00010 {
            // pretend 16-bit compact encoding in low half
            let raw16 = (raw32 & 0xFFFF) as u16;
            // fake pattern: 0b00010_rrr_iii_iiiii => ADD r, imm5
            let op = Op::Add;
            let rd = ((raw16 >> 8) & 0x7) as u8;
            let imm = (raw16 & 0x1F) as u32;
            return Some(Decoded {
                op,
                width: 2,
                rd,
                rs1: rd,
                rs2: 0,
                imm,
            });
        }
        // 32-bit formats (placeholder dissect)
        let opfield = (raw32 >> 24) & 0xFF;
        match opfield {
            0x10 => {
                // MOV rd, rs1
                let rd = ((raw32 >> 20) & 0xF) as u8;
                let rs1 = ((raw32 >> 16) & 0xF) as u8;
                Some(Decoded {
                    op: Op::Mov,
                    width: 4,
                    rd,
                    rs1,
                    rs2: 0,
                    imm: 0,
                })
            }
            0x20 => {
                // ADD rd, rs1, rs2
                let rd = ((raw32 >> 20) & 0xF) as u8;
                let rs1 = ((raw32 >> 16) & 0xF) as u8;
                let rs2 = ((raw32 >> 12) & 0xF) as u8;
                Some(Decoded {
                    op: Op::Add,
                    width: 4,
                    rd,
                    rs1,
                    rs2,
                    imm: 0,
                })
            }
            0x21 => {
                // SUB rd, rs1, rs2
                let rd = ((raw32 >> 20) & 0xF) as u8;
                let rs1 = ((raw32 >> 16) & 0xF) as u8;
                let rs2 = ((raw32 >> 12) & 0xF) as u8;
                Some(Decoded {
                    op: Op::Sub,
                    width: 4,
                    rd,
                    rs1,
                    rs2,
                    imm: 0,
                })
            }
            0x30 => {
                // LD.W rd, [rs1 + imm12]
                let rd = ((raw32 >> 20) & 0xF) as u8;
                let rs1 = ((raw32 >> 16) & 0xF) as u8;
                let imm = raw32 & 0x0FFF;
                Some(Decoded {
                    op: Op::LdW,
                    width: 4,
                    rd,
                    rs1,
                    rs2: 0,
                    imm,
                })
            }
            0x31 => {
                // ST.W [rs1 + imm12], rs2
                let rs1 = ((raw32 >> 20) & 0xF) as u8;
                let rs2 = ((raw32 >> 16) & 0xF) as u8;
                let imm = raw32 & 0x0FFF;
                Some(Decoded {
                    op: Op::StW,
                    width: 4,
                    rd: 0,
                    rs1,
                    rs2,
                    imm,
                })
            }
            0x40 => {
                // J (pc-relative)
                let imm = raw32 & 0x00FF_FFFF;
                Some(Decoded {
                    op: Op::J,
                    width: 4,
                    rd: 0,
                    rs1: 0,
                    rs2: 0,
                    imm,
                })
            }
            0x41 => {
                // BNE rs1, rs2, imm
                let rs1 = ((raw32 >> 20) & 0xF) as u8;
                let rs2 = ((raw32 >> 16) & 0xF) as u8;
                let imm = raw32 & 0x0FFF;
                Some(Decoded {
                    op: Op::Bne,
                    width: 4,
                    rd: 0,
                    rs1,
                    rs2,
                    imm,
                })
            }
            0xFF => Some(Decoded {
                op: Op::Syscall,
                width: 4,
                rd: 0,
                rs1: 0,
                rs2: 0,
                imm: 0,
            }),
            _ => None,
        }
    }
}
