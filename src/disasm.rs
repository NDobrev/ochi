use crate::decoder::{Decoded, Op};

pub fn fmt_decoded(d: &Decoded) -> String {
    match d.op {
        Op::Mov => format!("mov d{}, d{}", d.rd, d.rs1),
        Op::MovI => format!("mov d{}, #{:#x}", d.rd, d.imm),
        Op::MovHA => format!("movh.a a{}, #{:#x}", d.rd, d.imm >> 16),
        Op::Lea => {
            if d.abs { format!("lea a{}, [{:#x}]", d.rd, d.imm) }
            else if d.wb && d.pre { format!("lea a{}, [a{}+{:#x}]!", d.rd, d.rs1, d.imm) }
            else { format!("lea a{}, [a{}+{:#x}]", d.rd, d.rs1, d.imm) }
        }
        Op::Add => {
            if d.rs2 != 0 { format!("add d{}, d{}, d{}", d.rd, d.rs1, d.rs2) }
            else { format!("addi d{}, d{}, {:#x}", d.rd, d.rs1, d.imm) }
        }
        Op::Addx => if d.rs2 != 0 { format!("addx d{}, d{}, d{}", d.rd, d.rs1, d.rs2) } else { format!("addx d{}, d{}, {:#x}", d.rd, d.rs1, d.imm) },
        Op::Addc => if d.rs2 != 0 { format!("addc d{}, d{}, d{}", d.rd, d.rs1, d.rs2) } else { format!("addc d{}, d{}, {:#x}", d.rd, d.rs1, d.imm) },
        Op::Sub => {
            if d.rs2 != 0 { format!("sub d{}, d{}, d{}", d.rd, d.rs1, d.rs2) }
            else { format!("rsub d{}, d{}, {:#x}", d.rd, d.rs1, d.imm) }
        }
        Op::And => if d.rs2 != 0 { format!("and d{}, d{}, d{}", d.rd, d.rs1, d.rs2) } else { format!("and d{}, d{}, {:#x}", d.rd, d.rs1, d.imm) },
        Op::Or  => if d.rs2 != 0 { format!("or d{}, d{}, d{}", d.rd, d.rs1, d.rs2) } else { format!("or d{}, d{}, {:#x}", d.rd, d.rs1, d.imm) },
        Op::Xor => if d.rs2 != 0 { format!("xor d{}, d{}, d{}", d.rd, d.rs1, d.rs2) } else { format!("xor d{}, d{}, {:#x}", d.rd, d.rs1, d.imm) },
        Op::Shl => if d.rs2 != 0 { format!("shl d{}, d{}, d{}", d.rd, d.rs1, d.rs2) } else { format!("shl d{}, d{}, {:#x}", d.rd, d.rs1, d.imm) },
        Op::Shr => if d.rs2 != 0 { format!("shr d{}, d{}, d{}", d.rd, d.rs1, d.rs2) } else { format!("shr d{}, d{}, {:#x}", d.rd, d.rs1, d.imm) },
        Op::Sar => if d.rs2 != 0 { format!("sar d{}, d{}, d{}", d.rd, d.rs1, d.rs2) } else { format!("sar d{}, d{}, {:#x}", d.rd, d.rs1, d.imm) },
        Op::Ror => if d.rs2 != 0 { format!("ror d{}, d{}, d{}", d.rd, d.rs1, d.rs2) } else { format!("ror d{}, d{}, {:#x}", d.rd, d.rs1, d.imm) },
        Op::Andn => if d.rs2 != 0 { format!("andn d{}, d{}, d{}", d.rd, d.rs1, d.rs2) } else { format!("andn d{}, d{}, {:#x}", d.rd, d.rs1, d.imm) },
        Op::Not => format!("not d{}, d{}", d.rd, d.rs1),
        Op::Min => if d.rs2 != 0 { format!("min d{}, d{}, d{}", d.rd, d.rs1, d.rs2) } else { format!("min d{}, d{}, {:#x}", d.rd, d.rs1, d.imm) },
        Op::Max => if d.rs2 != 0 { format!("max d{}, d{}, d{}", d.rd, d.rs1, d.rs2) } else { format!("max d{}, d{}, {:#x}", d.rd, d.rs1, d.imm) },
        Op::MinU => if d.rs2 != 0 { format!("min.u d{}, d{}, d{}", d.rd, d.rs1, d.rs2) } else { format!("min.u d{}, d{}, {:#x}", d.rd, d.rs1, d.imm) },
        Op::MaxU => if d.rs2 != 0 { format!("max.u d{}, d{}, d{}", d.rd, d.rs1, d.rs2) } else { format!("max.u d{}, d{}, {:#x}", d.rd, d.rs1, d.imm) },
        Op::Mul => if d.rs2 != 0 { format!("mul d{}, d{}, d{}", d.rd, d.rs1, d.rs2) } else { format!("mul d{}, d{}, {:#x}", d.rd, d.rs1, d.imm) },
        Op::MulU => if d.rs2 != 0 { format!("mul.u d{}, d{}, d{}", d.rd, d.rs1, d.rs2) } else { format!("mul.u d{}, d{}, {:#x}", d.rd, d.rs1, d.imm) },
        Op::Div => format!("div d{}, d{}, d{}", d.rd, d.rs1, d.rs2),
        Op::DivU => format!("div.u d{}, d{}, d{}", d.rd, d.rs1, d.rs2),
        Op::BeqF => format!("beq {:+#x}", d.imm as i32),
        Op::BneF => format!("bne {:+#x}", d.imm as i32),
        Op::BgeF => format!("bge {:+#x}", d.imm as i32),
        Op::BltF => format!("blt {:+#x}", d.imm as i32),
        Op::BgeUF => format!("bge.u {:+#x}", d.imm as i32),
        Op::BltUF => format!("blt.u {:+#x}", d.imm as i32),
        Op::Cmp => if d.rs2 != 0 { format!("cmp d{}, d{}", d.rs1, d.rs2) } else { format!("cmp d{}, {:#x}", d.rs1, d.imm) },
        Op::CmpU => if d.rs2 != 0 { format!("cmp.u d{}, d{}", d.rs1, d.rs2) } else { format!("cmp.u d{}, {:#x}", d.rs1, d.imm) },
        Op::CmpI => format!("cmp d{}, {:#x}", d.rs1, d.imm),
        Op::CmpUI => format!("cmp.u d{}, {:#x}", d.rs1, d.imm),
        Op::LdB => mem("ld.b", d),
        Op::LdBu => mem("ld.bu", d),
        Op::LdH => mem("ld.h", d),
        Op::LdHu => mem("ld.hu", d),
        Op::LdW => mem("ld.w", d),
        Op::LdWPbr => format!("ld.w d{}, [p{}]", d.rd, d.rs1),
        Op::LdWPcir => format!("ld.w d{}, [p{}], {:+#x}", d.rd, d.rs1, d.imm as i32),
        Op::LdBPbr => format!("ld.b d{}, [p{}]", d.rd, d.rs1),
        Op::LdBUPbr => format!("ld.bu d{}, [p{}]", d.rd, d.rs1),
        Op::LdHPbr => format!("ld.h d{}, [p{}]", d.rd, d.rs1),
        Op::LdHUPbr => format!("ld.hu d{}, [p{}]", d.rd, d.rs1),
        Op::LdBPcir => format!("ld.b d{}, [p{}], {:+#x}", d.rd, d.rs1, d.imm as i32),
        Op::LdBUPcir => format!("ld.bu d{}, [p{}], {:+#x}", d.rd, d.rs1, d.imm as i32),
        Op::LdHPcir => format!("ld.h d{}, [p{}], {:+#x}", d.rd, d.rs1, d.imm as i32),
        Op::LdHUPcir => format!("ld.hu d{}, [p{}], {:+#x}", d.rd, d.rs1, d.imm as i32),
        Op::StB => mems("st.b", d),
        Op::StH => mems("st.h", d),
        Op::StW => mems("st.w", d),
        Op::StWPbr => format!("st.w [p{}], d{}", d.rs1, d.rs2),
        Op::StWPcir => format!("st.w [p{}], d{}, {:+#x}", d.rs1, d.rs2, d.imm as i32),
        Op::StBPbr => format!("st.b [p{}], d{}", d.rs1, d.rs2),
        Op::StBPcir => format!("st.b [p{}], d{}, {:+#x}", d.rs1, d.rs2, d.imm as i32),
        Op::StHPbr => format!("st.h [p{}], d{}", d.rs1, d.rs2),
        Op::StHPcir => format!("st.h [p{}], d{}, {:+#x}", d.rs1, d.rs2, d.imm as i32),
        Op::J => format!("j {:+#x}", d.imm as i32),
        Op::Jeq => br("jeq", d, false),
        Op::Jne => br("jne", d, false),
        Op::JeqImm => bri("jeq", d),
        Op::JneImm => bri("jne", d),
        Op::Jge => br("jge", d, false),
        Op::JgeU => br("jge.u", d, false),
        Op::JgeImm => bri("jge", d),
        Op::JgeUImm => bri("jge.u", d),
        Op::Jlt => br("jlt", d, false),
        Op::JltU => br("jlt.u", d, false),
        Op::JltImm => bri("jlt", d),
        Op::JltUImm => bri("jlt.u", d),
        Op::JeqA => br("jeq.a", d, true),
        Op::JneA => br("jne.a", d, true),
        Op::Call => format!("call {:+#x}", d.imm as i32),
        Op::CallA => format!("calla {:#x}", d.imm),
        Op::CallI => format!("calli a{}", d.rs1),
        Op::Ret => "ret".to_string(),
        Op::JzA => format!("jz.a a{}, {:+#x}", d.rs1, d.imm as i32),
        Op::JnzA => format!("jnz.a a{}, {:+#x}", d.rs1, d.imm as i32),
        Op::Bne => br("bne", d, false),
        Op::Syscall => "syscall".to_string(),
    }
}

fn mem(mn: &str, d: &Decoded) -> String {
    if d.abs { format!("{} d{}, [{:#x}]", mn, d.rd, d.imm) }
    else if d.wb && d.pre { format!("{} d{}, [a{}+{:#x}]!", mn, d.rd, d.rs1, d.imm) }
    else if d.wb { format!("{} d{}, [a{}], {:#x}", mn, d.rd, d.rs1, d.imm) }
    else { format!("{} d{}, [a{}+{:#x}]", mn, d.rd, d.rs1, d.imm) }
}

fn mems(mn: &str, d: &Decoded) -> String {
    if d.abs { format!("{} [{:#x}], d{}", mn, d.imm, d.rs2) }
    else if d.wb && d.pre { format!("{} [a{}+{:#x}]!, d{}", mn, d.rs1, d.imm, d.rs2) }
    else if d.wb { format!("{} [a{}], d{}", mn, d.rs1, d.rs2) }
    else { format!("{} [a{}+{:#x}], d{}", mn, d.rs1, d.imm, d.rs2) }
}

fn br(mn: &str, d: &Decoded, addr: bool) -> String {
    if addr { format!("{} a{}, a{}, {:+#x}", mn, d.rs1, d.rs2, d.imm as i32) }
    else { format!("{} d{}, d{}, {:+#x}", mn, d.rs1, d.rs2, d.imm as i32) }
}

fn bri(mn: &str, d: &Decoded) -> String {
    format!("{} d{}, {:#x}, {:+#x}", mn, d.rs1, d.imm2, d.imm as i32)
}
