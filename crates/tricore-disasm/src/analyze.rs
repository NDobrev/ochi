use std::collections::{HashMap, HashSet, VecDeque};
use serde::Serialize;

use tricore_rs::decoder::Decoder;
use tricore_rs::isa::tc16::Tc16Decoder;

use crate::model::{Image, is_mapped, read_u32};

#[derive(Debug, Clone, Copy)]
pub enum EdgeKind { Fallthrough, Branch, CondBranch, Call }

#[derive(Debug, Clone, Copy)]
pub struct Edge { pub from: u32, pub to: u32, pub kind: EdgeKind }

pub fn analyze_entries(img: &Image, entries: &[u32], max_instr: usize) -> (HashSet<u32>, HashMap<u32, u8>, Vec<Edge>, HashSet<u32>) {
    let dec = Tc16Decoder::new();
    let mut queue: VecDeque<u32> = VecDeque::new();
    let mut visited: HashSet<u32> = HashSet::new();
    let mut widths: HashMap<u32, u8> = HashMap::new();
    let mut edges: Vec<Edge> = Vec::new();
    let mut rets: HashSet<u32> = HashSet::new();
    for &e in entries { if is_mapped(img, e) { queue.push_back(e); } }
    let mut steps = 0usize;
    while let Some(pc) = queue.pop_front() {
        if steps >= max_instr { break; }
        if !visited.insert(pc) { continue; }
        let Some(raw32) = read_u32(img, pc) else { continue; };
        if let Some(d) = dec.decode(raw32) {
            steps += 1;
            widths.insert(pc, d.width);
            let ft = pc.wrapping_add(d.width as u32);
            // Branch classification
            use tricore_rs::decoder::Op::*;
            match d.op {
                J => {
                    let tgt = ft.wrapping_add(d.imm as u32);
                    edges.push(Edge { from: pc, to: tgt, kind: EdgeKind::Branch });
                    if is_mapped(img, tgt) && !visited.contains(&tgt) { queue.push_back(tgt); }
                }
                Jeq | Jne | JeqImm | JneImm | Jge | JgeU | JgeImm | JgeUImm |
                Jlt | JltU | JltImm | JltUImm | JeqA | JneA | Bne | JzA | JnzA => {
                    let tgt = ft.wrapping_add(d.imm as u32);
                    edges.push(Edge { from: pc, to: tgt, kind: EdgeKind::CondBranch });
                    if is_mapped(img, tgt) && !visited.contains(&tgt) { queue.push_back(tgt); }
                    // fallthrough
                    if is_mapped(img, ft) && !visited.contains(&ft) { edges.push(Edge { from: pc, to: ft, kind: EdgeKind::Fallthrough }); queue.push_back(ft); }
                }
                Call => {
                    let tgt = ft.wrapping_add(d.imm as u32);
                    edges.push(Edge { from: pc, to: tgt, kind: EdgeKind::Call });
                    if is_mapped(img, tgt) { queue.push_back(tgt); }
                    if is_mapped(img, ft) { edges.push(Edge { from: pc, to: ft, kind: EdgeKind::Fallthrough }); queue.push_back(ft); }
                }
                CallA => {
                    let tgt = d.imm;
                    edges.push(Edge { from: pc, to: tgt, kind: EdgeKind::Call });
                    if is_mapped(img, ft) { edges.push(Edge { from: pc, to: ft, kind: EdgeKind::Fallthrough }); queue.push_back(ft); }
                    if is_mapped(img, tgt) { queue.push_back(tgt); }
                }
                CallI => {
                    // Unknown target; still add fallthrough
                    if is_mapped(img, ft) { edges.push(Edge { from: pc, to: ft, kind: EdgeKind::Fallthrough }); queue.push_back(ft); }
                }
                Ret => {
                    rets.insert(pc);
                }
                _ => {
                    // Fallthrough by default
                    if is_mapped(img, ft) && !visited.contains(&ft) { edges.push(Edge { from: pc, to: ft, kind: EdgeKind::Fallthrough }); queue.push_back(ft); }
                }
            }
        }
    }
    (visited, widths, edges, rets)
}

#[derive(Debug, Clone, Serialize)]
pub struct Block { pub start: u32, pub end: u32 }

#[derive(Debug, Clone, Serialize)]
pub struct EdgeOut { pub from: u32, pub to: u32, pub kind: String }

#[derive(Debug, Clone, Serialize)]
pub struct FunctionOut { pub entry: u32, pub blocks: Vec<u32> }

#[derive(Debug, Clone, Serialize)]
pub struct Report<Blk=Block> {
    pub entries: Vec<u32>,
    pub blocks: Vec<Blk>,
    pub edges: Vec<EdgeOut>,
    pub functions: Vec<FunctionOut>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Image, Segment};

    #[test]
    fn uncond_jump_edges_and_blocking() {
        // Build a tiny image: J +2 (16-bit), then two 16-bit NOP-like (use MOV D0,#0 and MOV D0,#1)
        // Encode J disp8=1: low byte 0x3C, high byte 0x01 (little-endian)
        let mut bytes = vec![0x3C, 0x01, 0x82, 0x00, 0x82, 0x10];
        let img = Image { segments: vec![Segment { name: "s".into(), base: 0, bytes, perms: "r-x", kind: "raw" }] };
        let seeds = [0u32];
        let (visited, widths, edges) = analyze_entries(&img, &seeds, 100);
        assert!(visited.contains(&0));
        // target should be ft(0)+2 => 0x0004
        let ft = 0u32 + 2;
        let tgt = ft + 2;
        assert!(edges.iter().any(|e| matches!(e.kind, EdgeKind::Branch) && e.from == 0 && e.to == tgt));
        assert!(widths.get(&0).is_some());
    }
}
