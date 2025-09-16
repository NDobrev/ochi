use crate::decoder::Op;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddrMode {
    Reg,
    RegImm,
    PcRel,
}

#[derive(Debug, Clone, Copy)]
pub struct InstrDesc {
    pub op: Op,
    pub mnemonic: &'static str,
    pub mode: AddrMode,
}

pub const TABLE: &[InstrDesc] = &[
    InstrDesc {
        op: Op::Mov,
        mnemonic: "mov",
        mode: AddrMode::Reg,
    },
    InstrDesc {
        op: Op::Add,
        mnemonic: "add",
        mode: AddrMode::Reg,
    },
    InstrDesc {
        op: Op::Sub,
        mnemonic: "sub",
        mode: AddrMode::Reg,
    },
    InstrDesc {
        op: Op::LdW,
        mnemonic: "ld.w",
        mode: AddrMode::RegImm,
    },
    InstrDesc {
        op: Op::StW,
        mnemonic: "st.w",
        mode: AddrMode::RegImm,
    },
    InstrDesc {
        op: Op::J,
        mnemonic: "j",
        mode: AddrMode::PcRel,
    },
    InstrDesc {
        op: Op::Bne,
        mnemonic: "bne",
        mode: AddrMode::RegImm,
    },
    InstrDesc {
        op: Op::Syscall,
        mnemonic: "syscall",
        mode: AddrMode::Reg,
    },
];
