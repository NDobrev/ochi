use crate::cpu::{Cpu, Psw, Trap};
use crate::decoder::{Decoded, Op};
use crate::memory::Bus;

pub trait Executor {
    fn exec<B: Bus>(&self, cpu: &mut Cpu, bus: &mut B, d: Decoded) -> Result<(), Trap>;
}

pub struct IntExecutor;
impl Executor for IntExecutor {
    fn exec<B: Bus>(&self, cpu: &mut Cpu, bus: &mut B, d: Decoded) -> Result<(), Trap> {
        match d.op {
            Op::Mov => {
                cpu.gpr[d.rd as usize] = cpu.gpr[d.rs1 as usize];
            }
            Op::Add => {
                let a = cpu.gpr[d.rs1 as usize];
                let b = if d.rs2 != 0 {
                    cpu.gpr[d.rs2 as usize]
                } else {
                    d.imm
                };
                let (res, carry) = a.overflowing_add(b);
                cpu.gpr[d.rd as usize] = res;
                cpu.psw.set(Psw::Z, res == 0);
                cpu.psw.set(Psw::N, (res as i32) < 0);
                cpu.psw.set(Psw::C, carry);
                // TODO: proper signed overflow for TriCore semantics
            }
            Op::Sub => {
                let a = cpu.gpr[d.rs1 as usize];
                let b = cpu.gpr[d.rs2 as usize];
                let (res, borrow) = a.overflowing_sub(b);
                cpu.gpr[d.rd as usize] = res;
                cpu.psw.set(Psw::Z, res == 0);
                cpu.psw.set(Psw::N, (res as i32) < 0);
                cpu.psw.set(Psw::C, borrow); // check exact meaning vs TriCore
            }
            Op::LdW => {
                let base = cpu.gpr[d.rs1 as usize];
                let addr = base.wrapping_add(d.imm);
                if addr % 4 != 0 {
                    return Err(Trap::Unaligned { addr });
                }
                let val = bus
                    .read_u32(addr)
                    .map_err(|source| Trap::Bus { addr, source })?;
                cpu.gpr[d.rd as usize] = val;
            }
            Op::StW => {
                let base = cpu.gpr[d.rs1 as usize];
                let addr = base.wrapping_add(d.imm);
                if addr % 4 != 0 {
                    return Err(Trap::Unaligned { addr });
                }
                let val = cpu.gpr[d.rs2 as usize];
                bus
                    .write_u32(addr, val)
                    .map_err(|source| Trap::Bus { addr, source })?;
            }
            Op::J => {
                // pc was already advanced by fetch; apply pc-relative offset (sign-extend as needed)
                let off = ((d.imm as i32) << 8) >> 8; // example sign-extend 24->32
                cpu.pc = cpu.pc.wrapping_add(off as u32);
            }
            Op::Bne => {
                let off = ((d.imm as i32) << 20) >> 20; // example sign-extend 12->32
                if cpu.gpr[d.rs1 as usize] != cpu.gpr[d.rs2 as usize] {
                    cpu.pc = cpu.pc.wrapping_add(off as u32);
                }
            }
            Op::Syscall => return Err(Trap::Break),
        }
        Ok(())
    }
}
