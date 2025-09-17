use anyhow::Error;
use crate::decoder::Decoder;
use crate::exec::Executor;
use crate::memory::Bus;
use bitflags::bitflags;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CpuConfig {
    pub little_endian: bool, // TriCore is typically little-endian
    pub has_fpu: bool,
    pub has_dsp: bool,
}

impl Default for CpuConfig {
    fn default() -> Self {
        Self {
            little_endian: true,
            has_fpu: false,
            has_dsp: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cpu {
    pub pc: u32,        // Program Counter
    pub psw: Psw,       // Program Status Word (subset)
    pub gpr: [u32; 16], // Lower core GPRs (TriCore has multiple register banks; extend as needed)
    pub a: [u32; 16],   // Address regs (A0..A15) — model as needed
    pub cfg: CpuConfig,
}

bitflags! {
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Psw: u32 {
const C = 1 << 0; // Carry
const V = 1 << 1; // Overflow
const Z = 1 << 2; // Zero
const N = 1 << 3; // Negative
const E = 1 << 4; // Endianness (example)
const SV = 1 << 5; // Sticky Overflow
const AV = 1 << 6; // Advanced Overflow
const SAV = 1 << 7; // Sticky Advanced Overflow
// ... extend with real PSW fields
}
}

#[derive(thiserror::Error, Debug)]
pub enum Trap {
    #[error("Invalid instruction at {pc:#010x}")]
    InvalidInstruction { pc: u32 },
    #[error("Unaligned access at {addr:#010x}")]
    Unaligned { addr: u32 },
    #[error("Bus error at {addr:#010x}: {source}")]
    Bus { addr: u32, #[source] source: Error },
    #[error("Breakpoint")]
    Break,
}

impl Cpu {
    pub fn new(cfg: CpuConfig) -> Self {
        Self {
            pc: 0,
            psw: Psw::empty(),
            gpr: [0; 16],
            a: [0; 16],
            cfg,
        }
    }

    pub fn reset(&mut self, reset_pc: u32) {
        self.pc = reset_pc;
    }

    pub fn step<B: Bus, D: Decoder, X: Executor>(
        &mut self,
        bus: &mut B,
        dec: &D,
        exec: &X,
    ) -> Result<(), Trap> {
        let pc = self.pc;
        // TriCore supports 16-bit and 32-bit encodings; fetch 32 then let decoder decide width
        let raw32 = bus
            .read_u32(pc)
            .map_err(|source| Trap::Bus { addr: pc, source })?;
        let d = dec.decode(raw32).ok_or(Trap::InvalidInstruction { pc })?;
        // Advance PC by decoded width (2 or 4)
        self.pc = pc.wrapping_add(d.width as u32);
        exec.exec(self, bus, d)
    }
}
