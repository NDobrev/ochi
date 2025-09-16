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
    LdW,
    StW,
    J,
    Bne,
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
}

pub trait Decoder {
    fn decode(&self, raw32: u32) -> Option<Decoded>;
}
