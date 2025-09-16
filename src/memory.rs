use anyhow::Result;
use serde::{Deserialize, Serialize};

pub trait Bus {
    fn read_u8(&mut self, addr: u32) -> Result<u8>;
    fn read_u16(&mut self, addr: u32) -> Result<u16>;
    fn read_u32(&mut self, addr: u32) -> Result<u32>;
    fn write_u8(&mut self, addr: u32, val: u8) -> Result<()>;
    fn write_u16(&mut self, addr: u32, val: u16) -> Result<()>;
    fn write_u32(&mut self, addr: u32, val: u32) -> Result<()>;
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LinearMemory {
    pub mem: Vec<u8>,
    pub base: u32,
}

impl LinearMemory {
    pub fn new(size: usize) -> Self {
        Self {
            mem: vec![0; size],
            base: 0,
        }
    }
}

impl LinearMemory {
    fn load_le_u16(&self, off: usize) -> u16 {
        u16::from_le_bytes([self.mem[off], self.mem[off + 1]])
    }
    fn load_le_u32(&self, off: usize) -> u32 {
        u32::from_le_bytes([
            self.mem[off],
            self.mem[off + 1],
            self.mem[off + 2],
            self.mem[off + 3],
        ])
    }
    fn store_le_u16(&mut self, off: usize, v: u16) {
        self.mem[off..off + 2].copy_from_slice(&v.to_le_bytes());
    }
    fn store_le_u32(&mut self, off: usize, v: u32) {
        self.mem[off..off + 4].copy_from_slice(&v.to_le_bytes());
    }
}

impl Bus for LinearMemory {
    fn read_u8(&mut self, addr: u32) -> Result<u8> {
        let off = addr.wrapping_sub(self.base) as usize;
        anyhow::ensure!(off < self.mem.len(), "read_u8 OOB at {addr:#x} (base {:#x})", self.base);
        Ok(self.mem[off])
    }
    fn read_u16(&mut self, addr: u32) -> Result<u16> {
        let off = addr.wrapping_sub(self.base) as usize;
        anyhow::ensure!(off + 1 < self.mem.len(), "read_u16 OOB at {addr:#x} (base {:#x})", self.base);
        Ok(self.load_le_u16(off))
    }
    fn read_u32(&mut self, addr: u32) -> Result<u32> {
        let off = addr.wrapping_sub(self.base) as usize;
        anyhow::ensure!(off + 3 < self.mem.len(), "read_u32 OOB at {addr:#x} (base {:#x})", self.base);
        Ok(self.load_le_u32(off))
    }
    fn write_u8(&mut self, addr: u32, val: u8) -> Result<()> {
        let off = addr.wrapping_sub(self.base) as usize;
        anyhow::ensure!(off < self.mem.len(), "write_u8 OOB at {addr:#x} (base {:#x})", self.base);
        self.mem[off] = val;
        Ok(())
    }
    fn write_u16(&mut self, addr: u32, val: u16) -> Result<()> {
        let off = addr.wrapping_sub(self.base) as usize;
        anyhow::ensure!(off + 1 < self.mem.len(), "write_u16 OOB at {addr:#x} (base {:#x})", self.base);
        self.store_le_u16(off, val);
        Ok(())
    }
    fn write_u32(&mut self, addr: u32, val: u32) -> Result<()> {
        let off = addr.wrapping_sub(self.base) as usize;
        anyhow::ensure!(off + 3 < self.mem.len(), "write_u32 OOB at {addr:#x} (base {:#x})", self.base);
        self.store_le_u32(off, val);
        Ok(())
    }
}
