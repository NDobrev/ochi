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
                // overflow and advanced overflow per spec-ish behavior
                let a_s = a as i32;
                let b_s = b as i32;
                let r_s = res as i32;
                let overflow = ((a_s ^ r_s) & (b_s ^ r_s)) < 0;
                cpu.psw.set(Psw::V, overflow);
                if overflow { cpu.psw.insert(Psw::SV); }
                let av = ((res >> 31) & 1) ^ ((res >> 30) & 1) == 1;
                cpu.psw.set(Psw::AV, av);
                if av { cpu.psw.insert(Psw::SAV); }
            }
            Op::Addx => {
                let a = cpu.gpr[d.rs1 as usize];
                let b = if d.rs2 != 0 { cpu.gpr[d.rs2 as usize] } else { d.imm };
                let (res, carry) = a.overflowing_add(b);
                cpu.gpr[d.rd as usize] = res;
                cpu.psw.set(Psw::Z, res == 0);
                cpu.psw.set(Psw::N, (res as i32) < 0);
                cpu.psw.set(Psw::C, carry); // carry-out only
                let a_s = a as i32; let b_s = b as i32; let r_s = res as i32;
                let overflow = ((a_s ^ r_s) & (b_s ^ r_s)) < 0;
                cpu.psw.set(Psw::V, overflow);
                if overflow { cpu.psw.insert(Psw::SV); }
                let av = ((res >> 31) & 1) ^ ((res >> 30) & 1) == 1;
                cpu.psw.set(Psw::AV, av);
                if av { cpu.psw.insert(Psw::SAV); }
            }
            Op::Addc => {
                let a = cpu.gpr[d.rs1 as usize];
                let b = if d.rs2 != 0 { cpu.gpr[d.rs2 as usize] } else { d.imm };
                let carry_in = if cpu.psw.contains(Psw::C) { 1u32 } else { 0 };
                let (tmp, c1) = a.overflowing_add(b);
                let (res, c2) = tmp.overflowing_add(carry_in);
                let carry = c1 || c2;
                cpu.gpr[d.rd as usize] = res;
                cpu.psw.set(Psw::Z, res == 0);
                cpu.psw.set(Psw::N, (res as i32) < 0);
                cpu.psw.set(Psw::C, carry);
                let sum64 = (a as i64) + (b as i64) + (carry_in as i64);
                let overflow = sum64 > i32::MAX as i64 || sum64 < i32::MIN as i64;
                cpu.psw.set(Psw::V, overflow);
                if overflow { cpu.psw.insert(Psw::SV); }
                let av = ((res >> 31) & 1) ^ ((res >> 30) & 1) == 1;
                cpu.psw.set(Psw::AV, av);
                if av { cpu.psw.insert(Psw::SAV); }
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
                let a_s = a as i32;
                let b_s = if d.rs2 != 0 { cpu.gpr[d.rs2 as usize] as i32 } else { d.imm as i32 };
                let r_s = res as i32;
                let overflow = ((a_s ^ b_s) & (a_s ^ r_s)) < 0;
                cpu.psw.set(Psw::V, overflow);
                if overflow { cpu.psw.insert(Psw::SV); }
                let av = ((res >> 31) & 1) ^ ((res >> 30) & 1) == 1;
                cpu.psw.set(Psw::AV, av);
                if av { cpu.psw.insert(Psw::SAV); }
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
            Op::LdWPbr => {
                // Bit-reverse addressing: index/incr in A[b+1]
                let b = d.rs1 as usize;
                let ab = cpu.a[b];
                let ab1 = cpu.a[(b + 1) & 0xF];
                let index = (ab1 & 0xFFFF) as u32;
                let incr = (ab1 >> 16) as u32;
                let ea = ab.wrapping_add(index);
                if ea % 4 != 0 { return Err(Trap::Unaligned { addr: ea }); }
                let val = bus.read_u32(ea).map_err(|source| Trap::Bus { addr: ea, source })?;
                cpu.gpr[d.rd as usize] = val;
                let rev = |x: u32| -> u32 { (x as u16).reverse_bits() as u32 };
                let new_index = rev(rev(index).wrapping_add(rev(incr))) & 0xFFFF;
                cpu.a[(b + 1) & 0xF] = ((incr & 0xFFFF) << 16) | new_index;
            }
            Op::LdBPbr => {
                let b = d.rs1 as usize;
                let ab = cpu.a[b];
                let ab1 = cpu.a[(b + 1) & 0xF];
                let index = (ab1 & 0xFFFF) as u32;
                let incr = (ab1 >> 16) as u32;
                let ea = ab.wrapping_add(index);
                let val = bus.read_u8(ea).map_err(|source| Trap::Bus { addr: ea, source })? as i8 as i32 as u32;
                cpu.gpr[d.rd as usize] = val;
                let rev = |x: u32| -> u32 { (x as u16).reverse_bits() as u32 };
                let new_index = rev(rev(index).wrapping_add(rev(incr))) & 0xFFFF;
                cpu.a[(b + 1) & 0xF] = ((incr & 0xFFFF) << 16) | new_index;
            }
            Op::LdBUPbr => {
                let b = d.rs1 as usize;
                let ab = cpu.a[b];
                let ab1 = cpu.a[(b + 1) & 0xF];
                let index = (ab1 & 0xFFFF) as u32;
                let incr = (ab1 >> 16) as u32;
                let ea = ab.wrapping_add(index);
                let val = bus.read_u8(ea).map_err(|source| Trap::Bus { addr: ea, source })? as u32;
                cpu.gpr[d.rd as usize] = val;
                let rev = |x: u32| -> u32 { (x as u16).reverse_bits() as u32 };
                let new_index = rev(rev(index).wrapping_add(rev(incr))) & 0xFFFF;
                cpu.a[(b + 1) & 0xF] = ((incr & 0xFFFF) << 16) | new_index;
            }
            Op::LdHPbr => {
                let b = d.rs1 as usize;
                let ab = cpu.a[b];
                let ab1 = cpu.a[(b + 1) & 0xF];
                let index = (ab1 & 0xFFFF) as u32;
                let incr = (ab1 >> 16) as u32;
                let ea = ab.wrapping_add(index);
                if ea % 2 != 0 { return Err(Trap::Unaligned { addr: ea }); }
                let val = bus.read_u16(ea).map_err(|source| Trap::Bus { addr: ea, source })? as i16 as i32 as u32;
                cpu.gpr[d.rd as usize] = val;
                let rev = |x: u32| -> u32 { (x as u16).reverse_bits() as u32 };
                let new_index = rev(rev(index).wrapping_add(rev(incr))) & 0xFFFF;
                cpu.a[(b + 1) & 0xF] = ((incr & 0xFFFF) << 16) | new_index;
            }
            Op::LdHUPbr => {
                let b = d.rs1 as usize;
                let ab = cpu.a[b];
                let ab1 = cpu.a[(b + 1) & 0xF];
                let index = (ab1 & 0xFFFF) as u32;
                let incr = (ab1 >> 16) as u32;
                let ea = ab.wrapping_add(index);
                if ea % 2 != 0 { return Err(Trap::Unaligned { addr: ea }); }
                let val = bus.read_u16(ea).map_err(|source| Trap::Bus { addr: ea, source })? as u32;
                cpu.gpr[d.rd as usize] = val;
                let rev = |x: u32| -> u32 { (x as u16).reverse_bits() as u32 };
                let new_index = rev(rev(index).wrapping_add(rev(incr))) & 0xFFFF;
                cpu.a[(b + 1) & 0xF] = ((incr & 0xFFFF) << 16) | new_index;
            }
            Op::LdBPcir => {
                let b = d.rs1 as usize;
                let ab = cpu.a[b];
                let ab1 = cpu.a[(b + 1) & 0xF];
                let index = (ab1 & 0xFFFF) as u32;
                let length = (ab1 >> 16) as u32;
                let ea = ab.wrapping_add(index);
                let val = bus.read_u8(ea).map_err(|source| Trap::Bus { addr: ea, source })? as i8 as i32 as u32;
                cpu.gpr[d.rd as usize] = val;
                let mut new_index = (index as i32).wrapping_add(d.imm as i32);
                if length != 0 {
                    if new_index < 0 { new_index += length as i32; }
                    else { new_index = new_index.rem_euclid(length as i32); }
                }
                cpu.a[(b + 1) & 0xF] = (length << 16) | ((new_index as u32) & 0xFFFF);
            }
            Op::LdBUPcir => {
                let b = d.rs1 as usize;
                let ab = cpu.a[b];
                let ab1 = cpu.a[(b + 1) & 0xF];
                let index = (ab1 & 0xFFFF) as u32;
                let length = (ab1 >> 16) as u32;
                let ea = ab.wrapping_add(index);
                let val = bus.read_u8(ea).map_err(|source| Trap::Bus { addr: ea, source })? as u32;
                cpu.gpr[d.rd as usize] = val;
                let mut new_index = (index as i32).wrapping_add(d.imm as i32);
                if length != 0 {
                    if new_index < 0 { new_index += length as i32; }
                    else { new_index = new_index.rem_euclid(length as i32); }
                }
                cpu.a[(b + 1) & 0xF] = (length << 16) | ((new_index as u32) & 0xFFFF);
            }
            Op::LdHPcir => {
                let b = d.rs1 as usize;
                let ab = cpu.a[b];
                let ab1 = cpu.a[(b + 1) & 0xF];
                let index = (ab1 & 0xFFFF) as u32;
                let length = (ab1 >> 16) as u32;
                let ea = ab.wrapping_add(index);
                if ea % 2 != 0 { return Err(Trap::Unaligned { addr: ea }); }
                let val = bus.read_u16(ea).map_err(|source| Trap::Bus { addr: ea, source })? as i16 as i32 as u32;
                cpu.gpr[d.rd as usize] = val;
                let mut new_index = (index as i32).wrapping_add(d.imm as i32);
                if length != 0 {
                    if new_index < 0 { new_index += length as i32; }
                    else { new_index = new_index.rem_euclid(length as i32); }
                }
                cpu.a[(b + 1) & 0xF] = (length << 16) | ((new_index as u32) & 0xFFFF);
            }
            Op::LdHUPcir => {
                let b = d.rs1 as usize;
                let ab = cpu.a[b];
                let ab1 = cpu.a[(b + 1) & 0xF];
                let index = (ab1 & 0xFFFF) as u32;
                let length = (ab1 >> 16) as u32;
                let ea = ab.wrapping_add(index);
                if ea % 2 != 0 { return Err(Trap::Unaligned { addr: ea }); }
                let val = bus.read_u16(ea).map_err(|source| Trap::Bus { addr: ea, source })? as u32;
                cpu.gpr[d.rd as usize] = val;
                let mut new_index = (index as i32).wrapping_add(d.imm as i32);
                if length != 0 {
                    if new_index < 0 { new_index += length as i32; }
                    else { new_index = new_index.rem_euclid(length as i32); }
                }
                cpu.a[(b + 1) & 0xF] = (length << 16) | ((new_index as u32) & 0xFFFF);
            }
            Op::LdWPcir => {
                // Circular addressing: index/length in A[b+1], off10 in imm
                let b = d.rs1 as usize;
                let ab = cpu.a[b];
                let ab1 = cpu.a[(b + 1) & 0xF];
                let index = (ab1 & 0xFFFF) as u32;
                let length = (ab1 >> 16) as u32;
                let ea0 = ab.wrapping_add(index);
                let ea2 = ab.wrapping_add(if length != 0 { (index + 2) % length } else { index + 2 });
                if ea0 % 2 != 0 || ea2 % 2 != 0 { return Err(Trap::Unaligned { addr: if ea0 % 2 != 0 { ea0 } else { ea2 } }); }
                let lo = bus.read_u16(ea0).map_err(|source| Trap::Bus { addr: ea0, source })? as u32;
                let hi = bus.read_u16(ea2).map_err(|source| Trap::Bus { addr: ea2, source })? as u32;
                cpu.gpr[d.rd as usize] = (hi << 16) | lo;
                // update index
                let mut new_index = (index as i32).wrapping_add(d.imm as i32);
                if length != 0 {
                    if new_index < 0 { new_index += length as i32; }
                    else { new_index = new_index.rem_euclid(length as i32); }
                }
                cpu.a[(b + 1) & 0xF] = (length << 16) | ((new_index as u32) & 0xFFFF);
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
            Op::StWPbr => {
                let b = d.rs1 as usize;
                let ab = cpu.a[b];
                let ab1 = cpu.a[(b + 1) & 0xF];
                let index = (ab1 & 0xFFFF) as u32;
                let incr = (ab1 >> 16) as u32;
                let ea = ab.wrapping_add(index);
                if ea % 4 != 0 { return Err(Trap::Unaligned { addr: ea }); }
                let val = cpu.gpr[d.rs2 as usize];
                bus.write_u32(ea, val).map_err(|source| Trap::Bus { addr: ea, source })?;
                let rev = |x: u32| -> u32 { (x as u16).reverse_bits() as u32 };
                let new_index = rev(rev(index).wrapping_add(rev(incr))) & 0xFFFF;
                cpu.a[(b + 1) & 0xF] = ((incr & 0xFFFF) << 16) | new_index;
            }
            Op::StBPbr => {
                let b = d.rs1 as usize;
                let ab = cpu.a[b];
                let ab1 = cpu.a[(b + 1) & 0xF];
                let index = (ab1 & 0xFFFF) as u32;
                let incr = (ab1 >> 16) as u32;
                let ea = ab.wrapping_add(index);
                let val = (cpu.gpr[d.rs2 as usize] & 0xFF) as u8;
                bus.write_u8(ea, val).map_err(|source| Trap::Bus { addr: ea, source })?;
                let rev = |x: u32| -> u32 { (x as u16).reverse_bits() as u32 };
                let new_index = rev(rev(index).wrapping_add(rev(incr))) & 0xFFFF;
                cpu.a[(b + 1) & 0xF] = ((incr & 0xFFFF) << 16) | new_index;
            }
            Op::StHPbr => {
                let b = d.rs1 as usize;
                let ab = cpu.a[b];
                let ab1 = cpu.a[(b + 1) & 0xF];
                let index = (ab1 & 0xFFFF) as u32;
                let incr = (ab1 >> 16) as u32;
                let ea = ab.wrapping_add(index);
                if ea % 2 != 0 { return Err(Trap::Unaligned { addr: ea }); }
                let val = (cpu.gpr[d.rs2 as usize] & 0xFFFF) as u16;
                bus.write_u16(ea, val).map_err(|source| Trap::Bus { addr: ea, source })?;
                let rev = |x: u32| -> u32 { (x as u16).reverse_bits() as u32 };
                let new_index = rev(rev(index).wrapping_add(rev(incr))) & 0xFFFF;
                cpu.a[(b + 1) & 0xF] = ((incr & 0xFFFF) << 16) | new_index;
            }
            Op::StBPcir => {
                let b = d.rs1 as usize;
                let ab = cpu.a[b];
                let ab1 = cpu.a[(b + 1) & 0xF];
                let index = (ab1 & 0xFFFF) as u32;
                let length = (ab1 >> 16) as u32;
                let ea = ab.wrapping_add(index);
                let val = (cpu.gpr[d.rs2 as usize] & 0xFF) as u8;
                bus.write_u8(ea, val).map_err(|source| Trap::Bus { addr: ea, source })?;
                let mut new_index = (index as i32).wrapping_add(d.imm as i32);
                if length != 0 {
                    if new_index < 0 { new_index += length as i32; }
                    else { new_index = new_index.rem_euclid(length as i32); }
                }
                cpu.a[(b + 1) & 0xF] = (length << 16) | ((new_index as u32) & 0xFFFF);
            }
            Op::StHPcir => {
                let b = d.rs1 as usize;
                let ab = cpu.a[b];
                let ab1 = cpu.a[(b + 1) & 0xF];
                let index = (ab1 & 0xFFFF) as u32;
                let length = (ab1 >> 16) as u32;
                let ea = ab.wrapping_add(index);
                if ea % 2 != 0 { return Err(Trap::Unaligned { addr: ea }); }
                let val = (cpu.gpr[d.rs2 as usize] & 0xFFFF) as u16;
                bus.write_u16(ea, val).map_err(|source| Trap::Bus { addr: ea, source })?;
                let mut new_index = (index as i32).wrapping_add(d.imm as i32);
                if length != 0 {
                    if new_index < 0 { new_index += length as i32; }
                    else { new_index = new_index.rem_euclid(length as i32); }
                }
                cpu.a[(b + 1) & 0xF] = (length << 16) | ((new_index as u32) & 0xFFFF);
            }
            Op::StWPcir => {
                let b = d.rs1 as usize;
                let ab = cpu.a[b];
                let ab1 = cpu.a[(b + 1) & 0xF];
                let index = (ab1 & 0xFFFF) as u32;
                let length = (ab1 >> 16) as u32;
                let ea = ab.wrapping_add(index);
                if ea % 4 != 0 { return Err(Trap::Unaligned { addr: ea }); }
                let val = cpu.gpr[d.rs2 as usize];
                bus.write_u32(ea, val).map_err(|source| Trap::Bus { addr: ea, source })?;
                let mut new_index = (index as i32).wrapping_add(d.imm as i32);
                if length != 0 {
                    if new_index < 0 { new_index += length as i32; }
                    else { new_index = new_index.rem_euclid(length as i32); }
                }
                cpu.a[(b + 1) & 0xF] = (length << 16) | ((new_index as u32) & 0xFFFF);
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
                let addr = if d.abs {
                    d.imm
                } else if d.wb {
                    if d.pre { base.wrapping_add(d.imm) } else { base }
                } else {
                    base.wrapping_add(d.imm)
                };
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
                let addr = if d.abs {
                    d.imm
                } else if d.wb {
                    if d.pre { base.wrapping_add(d.imm) } else { base }
                } else {
                    base.wrapping_add(d.imm)
                };
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
            Op::JeqA => {
                let off = d.imm as i32;
                if cpu.a[d.rs1 as usize] == cpu.a[d.rs2 as usize] {
                    cpu.pc = cpu.pc.wrapping_add(off as u32);
                }
            }
            Op::JneA => {
                let off = d.imm as i32;
                if cpu.a[d.rs1 as usize] != cpu.a[d.rs2 as usize] {
                    cpu.pc = cpu.pc.wrapping_add(off as u32);
                }
            }
            Op::JzA => {
                let off = d.imm as i32;
                if cpu.a[d.rs1 as usize] == 0 {
                    cpu.pc = cpu.pc.wrapping_add(off as u32);
                }
            }
            Op::JnzA => {
                let off = d.imm as i32;
                if cpu.a[d.rs1 as usize] != 0 {
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
            Op::Call | Op::CallA | Op::CallI => {
                // Not implemented in this scaffold; treat as no-op for now
            }
            Op::Ret => {
                // Not implemented; treat as no-op
            }
            Op::Syscall => return Err(Trap::Break),
        }
        Ok(())
    }
}
