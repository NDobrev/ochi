use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Width {
    W16 = 2,
    W32 = 4,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Op {
    // Placeholder core ops; plug real TriCore ops here
    Add,
    Sub,
    Mov,
    MovI, // move immediate (sign/zero/high are handled in decode)
    MovHA, // MOVH.A (address high move)
    Lea,   // Load effective address into A
    And,
    Or,
    Xor,
    Addc,
    Addx,
    LdW,
    StW,
    // P[b] addressing (bit-reverse and circular)
    LdWPbr,
    LdWPcir,
    StWPbr,
    StWPcir,
    LdBPbr,
    LdBUPbr,
    LdHPbr,
    LdHUPbr,
    LdBPcir,
    LdBUPcir,
    LdHPcir,
    LdHUPcir,
    StBPbr,
    StBPcir,
    StHPbr,
    StHPcir,
    LdB,
    LdBu,
    LdH,
    LdHu,
    StB,
    StH,
    J,
    Jeq,
    Jne,
    JeqA,
    JneA,
    JeqImm,
    JneImm,
    Jge,
    JgeU,
    JgeImm,
    JgeUImm,
    Jlt,
    JltU,
    JltImm,
    JltUImm,
    Bne,
    JzA,
    JnzA,
    Syscall,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Decoded {
    pub op: Op,
    pub width: u8, // 2 or 4
    pub rd: u8,
    pub rs1: u8,
    pub rs2: u8,
    pub imm: u32,
    pub imm2: u32, // optional second immediate (e.g., BRC const)
    // Addressing mode helpers
    pub abs: bool, // when true, `imm` is an absolute EA (no base)
    pub wb: bool,  // write-back to A[rs1]
    pub pre: bool, // true for pre-increment, false for post-increment when wb=true
}

pub trait Decoder {
    fn decode(&self, raw32: u32) -> Option<Decoded>;
}
