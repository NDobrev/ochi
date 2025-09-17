pub mod cpu;
pub mod decoder;
pub mod exec;
pub mod disasm;
pub mod instructions;
pub mod memory;

pub mod isa {
    pub mod tc16; // TriCore v1.6 example variant
}

pub use cpu::{Cpu, CpuConfig, Trap};
pub use memory::{Bus, LinearMemory};
