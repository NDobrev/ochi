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
            Op::MovI => {
                cpu.gpr[d.rd as usize] = d.imm;
            }
            Op::MovHA => {
                cpu.a[d.rd as usize] = d.imm;
            }
            Op::Lea => {
                if d.abs {
                    cpu.a[d.rd as usize] = d.imm;
                } else {
                    let base = cpu.a[d.rs1 as usize];
                    cpu.a[d.rd as usize] = base.wrapping_add(d.imm);
                }
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
            Op::And => {
                let a = cpu.gpr[d.rs1 as usize];
                let b = if d.rs2 != 0 { cpu.gpr[d.rs2 as usize] } else { d.imm };
                let res = a & b;
                cpu.gpr[d.rd as usize] = res;
                cpu.psw.set(Psw::Z, res == 0);
                cpu.psw.set(Psw::N, (res as i32) < 0);
            }
            Op::Or => {
                let a = cpu.gpr[d.rs1 as usize];
                let b = if d.rs2 != 0 { cpu.gpr[d.rs2 as usize] } else { d.imm };
                let res = a | b;
                cpu.gpr[d.rd as usize] = res;
                cpu.psw.set(Psw::Z, res == 0);
                cpu.psw.set(Psw::N, (res as i32) < 0);
            }
            Op::Xor => {
                let a = cpu.gpr[d.rs1 as usize];
                let b = if d.rs2 != 0 { cpu.gpr[d.rs2 as usize] } else { d.imm };
                let res = a ^ b;
                cpu.gpr[d.rd as usize] = res;
                cpu.psw.set(Psw::Z, res == 0);
                cpu.psw.set(Psw::N, (res as i32) < 0);
            }
            Op::Sub => {
                let a = cpu.gpr[d.rs1 as usize];
                let (res, borrow) = if d.rs2 != 0 {
                    let b = cpu.gpr[d.rs2 as usize];
                    a.overflowing_sub(b)
                } else {
                    // RSUB immediate form: imm - a
                    let (r, b) = (d.imm).overflowing_sub(a);
                    (r, b)
                };
                cpu.gpr[d.rd as usize] = res;
                cpu.psw.set(Psw::Z, res == 0);
                cpu.psw.set(Psw::N, (res as i32) < 0);
                cpu.psw.set(Psw::C, borrow); // check exact meaning vs TriCore
            }
            Op::LdW => {
                let base = cpu.a[d.rs1 as usize];
                let addr = if d.abs {
                    d.imm
                } else if d.wb {
                    if d.pre { base.wrapping_add(d.imm) } else { base }
                } else {
                    base.wrapping_add(d.imm)
                };
                if addr % 4 != 0 {
                    return Err(Trap::Unaligned { addr });
                }
                let val = bus
                    .read_u32(addr)
                    .map_err(|source| Trap::Bus { addr, source })?;
                cpu.gpr[d.rd as usize] = val;
                if !d.abs && d.wb {
                    let new_base = if d.pre { addr } else { addr.wrapping_add(d.imm) };
                    cpu.a[d.rs1 as usize] = new_base;
                }
            }
            Op::StW => {
                let base = cpu.a[d.rs1 as usize];
                let addr = if d.abs {
                    d.imm
                } else if d.wb {
                    if d.pre { base.wrapping_add(d.imm) } else { base }
                } else {
                    base.wrapping_add(d.imm)
                };
                if addr % 4 != 0 {
                    return Err(Trap::Unaligned { addr });
                }
                let val = cpu.gpr[d.rs2 as usize];
                bus
                    .write_u32(addr, val)
                    .map_err(|source| Trap::Bus { addr, source })?;
                if !d.abs && d.wb {
                    let new_base = if d.pre { addr } else { addr.wrapping_add(d.imm) };
                    cpu.a[d.rs1 as usize] = new_base;
                }
            }
            Op::LdB => {
                let base = cpu.a[d.rs1 as usize];
                let addr = if d.abs { d.imm } else if d.wb { if d.pre { base.wrapping_add(d.imm) } else { base } } else { base.wrapping_add(d.imm) };
                let v = bus
                    .read_u8(addr)
                    .map_err(|source| Trap::Bus { addr, source })? as i8 as i32 as u32;
                cpu.gpr[d.rd as usize] = v;
                if !d.abs && d.wb {
                    let new_base = if d.pre { addr } else { addr.wrapping_add(d.imm) };
                    cpu.a[d.rs1 as usize] = new_base;
                }
            }
            Op::LdBu => {
                let base = cpu.a[d.rs1 as usize];
                let addr = if d.abs { d.imm } else if d.wb { if d.pre { base.wrapping_add(d.imm) } else { base } } else { base.wrapping_add(d.imm) };
                let v = bus
                    .read_u8(addr)
                    .map_err(|source| Trap::Bus { addr, source })? as u32;
                cpu.gpr[d.rd as usize] = v;
                if !d.abs && d.wb {
                    let new_base = if d.pre { addr } else { addr.wrapping_add(d.imm) };
                    cpu.a[d.rs1 as usize] = new_base;
                }
            }
            Op::LdH => {
                let base = cpu.a[d.rs1 as usize];
                let addr = if d.abs { d.imm } else if d.wb { if d.pre { base.wrapping_add(d.imm) } else { base } } else { base.wrapping_add(d.imm) };
                if addr % 2 != 0 {
                    return Err(Trap::Unaligned { addr });
                }
                let v = bus
                    .read_u16(addr)
                    .map_err(|source| Trap::Bus { addr, source })? as i16 as i32 as u32;
                cpu.gpr[d.rd as usize] = v;
                if !d.abs && d.wb {
                    let new_base = if d.pre { addr } else { addr.wrapping_add(d.imm) };
                    cpu.a[d.rs1 as usize] = new_base;
                }
            }
            Op::LdHu => {
                let base = cpu.a[d.rs1 as usize];
                let addr = if d.abs { d.imm } else if d.wb { if d.pre { base.wrapping_add(d.imm) } else { base } } else { base.wrapping_add(d.imm) };
                if addr % 2 != 0 {
                    return Err(Trap::Unaligned { addr });
                }
                let v = bus
                    .read_u16(addr)
                    .map_err(|source| Trap::Bus { addr, source })? as u32;
                cpu.gpr[d.rd as usize] = v;
                if !d.abs && d.wb {
                    let new_base = if d.pre { addr } else { addr.wrapping_add(d.imm) };
                    cpu.a[d.rs1 as usize] = new_base;
                }
            }
            Op::StB => {
                let base = cpu.a[d.rs1 as usize];
                let addr = if d.abs { d.imm } else if d.wb && d.pre { base.wrapping_add(d.imm) } else { base };
                let val = (cpu.gpr[d.rs2 as usize] & 0xFF) as u8;
                bus
                    .write_u8(addr, val)
                    .map_err(|source| Trap::Bus { addr, source })?;
                if !d.abs && d.wb {
                    let new_base = if d.pre { addr } else { addr.wrapping_add(d.imm) };
                    cpu.a[d.rs1 as usize] = new_base;
                }
            }
            Op::StH => {
                let base = cpu.a[d.rs1 as usize];
                let addr = if d.abs { d.imm } else if d.wb && d.pre { base.wrapping_add(d.imm) } else { base };
                if addr % 2 != 0 {
                    return Err(Trap::Unaligned { addr });
                }
                let val = (cpu.gpr[d.rs2 as usize] & 0xFFFF) as u16;
                bus
                    .write_u16(addr, val)
                    .map_err(|source| Trap::Bus { addr, source })?;
                if !d.abs && d.wb {
                    let new_base = if d.pre { addr } else { addr.wrapping_add(d.imm) };
                    cpu.a[d.rs1 as usize] = new_base;
                }
            }
            Op::J => {
                // pc was already advanced by fetch; apply pc-relative offset in bytes
                let off = d.imm as i32;
                cpu.pc = cpu.pc.wrapping_add(off as u32);
            }
            Op::Bne => {
                let off = d.imm as i32;
                if cpu.gpr[d.rs1 as usize] != cpu.gpr[d.rs2 as usize] {
                    cpu.pc = cpu.pc.wrapping_add(off as u32);
                }
            }
            Op::Jeq => {
                let off = d.imm as i32;
                if cpu.gpr[d.rs1 as usize] == cpu.gpr[d.rs2 as usize] {
                    cpu.pc = cpu.pc.wrapping_add(off as u32);
                }
            }
            Op::JeqImm => {
                let off = d.imm as i32;
                if cpu.gpr[d.rs1 as usize] == (d.imm2 as u32) {
                    cpu.pc = cpu.pc.wrapping_add(off as u32);
                }
            }
            Op::Jne => {
                let off = d.imm as i32;
                if cpu.gpr[d.rs1 as usize] != cpu.gpr[d.rs2 as usize] {
                    cpu.pc = cpu.pc.wrapping_add(off as u32);
                }
            }
            Op::JneImm => {
                let off = d.imm as i32;
                if cpu.gpr[d.rs1 as usize] != (d.imm2 as u32) {
                    cpu.pc = cpu.pc.wrapping_add(off as u32);
                }
            }
            Op::Jge => {
                let off = d.imm as i32;
                if (cpu.gpr[d.rs1 as usize] as i32) >= (cpu.gpr[d.rs2 as usize] as i32) {
                    cpu.pc = cpu.pc.wrapping_add(off as u32);
                }
            }
            Op::JgeU => {
                let off = d.imm as i32;
                if (cpu.gpr[d.rs1 as usize] as u32) >= (cpu.gpr[d.rs2 as usize] as u32) {
                    cpu.pc = cpu.pc.wrapping_add(off as u32);
                }
            }
            Op::JgeImm => {
                let off = d.imm as i32;
                if (cpu.gpr[d.rs1 as usize] as i32) >= (d.imm2 as i32) {
                    cpu.pc = cpu.pc.wrapping_add(off as u32);
                }
            }
            Op::JgeUImm => {
                let off = d.imm as i32;
                if (cpu.gpr[d.rs1 as usize] as u32) >= (d.imm2 as u32) {
                    cpu.pc = cpu.pc.wrapping_add(off as u32);
                }
            }
            Op::Jlt => {
                let off = d.imm as i32;
                if (cpu.gpr[d.rs1 as usize] as i32) < (cpu.gpr[d.rs2 as usize] as i32) {
                    cpu.pc = cpu.pc.wrapping_add(off as u32);
                }
            }
            Op::JltU => {
                let off = d.imm as i32;
                if (cpu.gpr[d.rs1 as usize] as u32) < (cpu.gpr[d.rs2 as usize] as u32) {
                    cpu.pc = cpu.pc.wrapping_add(off as u32);
                }
            }
            Op::JltImm => {
                let off = d.imm as i32;
                if (cpu.gpr[d.rs1 as usize] as i32) < (d.imm2 as i32) {
                    cpu.pc = cpu.pc.wrapping_add(off as u32);
                }
            }
            Op::JltUImm => {
                let off = d.imm as i32;
                if (cpu.gpr[d.rs1 as usize] as u32) < (d.imm2 as u32) {
                    cpu.pc = cpu.pc.wrapping_add(off as u32);
                }
            }
            Op::Syscall => return Err(Trap::Break),
        }
        Ok(())
    }
}
