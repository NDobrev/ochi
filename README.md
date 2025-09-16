# ochi — TriCore TC1.6.2 interpreter scaffold

Ochi is a small, test-driven scaffold for a TriCore TC1.6.2 interpreter in Rust. It focuses on clear decoding of instruction encodings and a minimal execution core, with unit tests anchored to the official Instruction Set manual.

- Language: Rust (Edition 2021)
- Scope: Minimal CPU + memory bus, decoder subset, basic executor
- Spec: See `spec/TriCore TC1.6.2 core architecture manual - Instruction Set (Volume 2 of 2) - … .pdf` and the extracted text `spec/tricore_tc162_iset.txt`.

## Quick start

- Build: `cargo build`
- Run tests: `cargo test`
- CLI runner: `cargo run --bin tricore-run -- --help`

The `tricore-run` binary loads a raw binary into linear memory (little‑endian), sets the PC to `--entry` (default 0), and steps up to a fixed cap or until a trap. This is useful to smoke test small hand‑crafted binaries or fuzz inputs.

Example:

```
cargo run --bin tricore-run -- --entry 0 path/to/program.bin
```

## Project layout

- `src/cpu.rs` — CPU core, PSW, traps, fetch+step
- `src/memory.rs` — Bus trait and linear memory backend
- `src/decoder.rs` — Decoded shape + opcode tags
- `src/isa/tc16.rs` — TC1.6.2 decoder (subset) with spec encodings
- `src/exec.rs` — Integer executor with ALU/memory/branch semantics
- `tests/*.rs` — Unit/regression tests mapped to spec behaviors
- `src/bin/tricore-run.rs` — Minimal CLI runner

## Status matrix (implemented vs not)

The table below summarizes what is implemented in the decoder/executor and what is pending. Encodings follow the TC1.6.2 spec (Volume 2), using the op1/op2 bytes noted in the manual tables.

| Area | Implemented | Not Implemented (yet) |
| --- | --- | --- |
| Fetch/step | 16/32‑bit fetch, PC advance by width | Delayed slots, exceptions beyond Unaligned/Invalid/Bus |
| PSW | Bitflags with Z/N/C updates for some ALU ops | Full V/SV/AV/SAV semantics, carry/overflow accuracy per spec |
| Arithmetic | ADD (RR/RC/SRC/SRR), ADDI (RLC), ADDIH (RLC) | SUB/RSUB, ADDC/ADDX, saturation variants beyond tests |
| Logical | AND/OR/XOR: RR (op1=0x0F), RC (op1=0x8F), 16‑bit SRR (0x26/0xA6/0xC6) | NAND/NOR/XNOR, ANDN/ORN, bit test ops |
| Moves (data) | MOV (RLC sign‑ext 0x3B; RR via 0x0B/op2=0x1F; 16‑bit SRC 0x82), MOV.U (0xBB), MOVH (0x7B) | MOV variants for E‑register pairs, extended forms |
| Address ops | MOVH.A (0x91), ADDIH.A (0x11), LEA BO (0x49/op2=0x28), LEA BOL (0xD9) | LEA ABS (0xC5), LEA post/pre‑inc variants (not in spec for LEA) |
| Loads (BO) | LD.B (0x09/op2=0x20), LD.BU (0x21), LD.H (0x22), LD.HU (0x23), LD.W (0x24) | ABS/BOL variants for B/H/W, circular/bit‑reverse forms |
| Stores (BO) | ST.B (0x89/op2=0x20), ST.H (0x22), ST.W (0x24) | ABS/BOL variants for B/H/W, circular/bit‑reverse forms |
| Branch (uncond.) | J disp8 (0x3C), J disp24 (0x1D) | JA/JL/JLA/CALL/RET/RFE families |
| Branch (cond., data regs) | JEQ/JNE BRR (0x5F); JGE/JGE.U BRR (0x7F); JLT/JLT.U BRR (0x3F) | Other conditions (JLE/JGT) and address‑register compares (JEQ.A/JNE.A) |
| Branch (cond., imm4) | JEQ/JNE BRC (0xDF), JGE/JGE.U BRC (0xFF), JLT/JLT.U BRC (0xBF) | Wider immediates, compound forms |
| 16‑bit branch (D15) | JEQ/JNE SBR/SBC forms (0x3E/0xBE, 0x7E/0xFE, 0x1E/0x9E, 0x5E/0xDE) | Other 16‑bit conditional families |
| System | Trap mapping from bus errors; Break trap | Full SYSCALL/exception model, context stack, interrupts |
| CLI | `tricore-run` loads raw bytes, steps with Tc16 decoder + IntExecutor | ELF loader, disassembler, richer stepping/debugging |

Notes:
- All implemented encodings are backed by unit tests under `tests/` for decode + basic semantics.
- For loads/stores, the base register is the address bank `A[b]` and sign/zero extension follows the spec. Halfword/word accesses trap on unaligned addresses.
- Branch offsets in the decoder are stored as final byte offsets (the executor simply adds `imm` to the already‑advanced PC).

## Running the tests

```
cargo test
```

Current tests include:
- ALU: `tests/logic.rs`
- Memory: `tests/memory.rs`
- Branches: `tests/branches.rs`
- Address ops: `tests/addr.rs`
- Smoke: `tests/smoke.rs`

## Contributing / next steps

High‑value next steps:
- LEA ABS (0xC5) and absolute load/store variants
- SUB/RSUB, compare instructions and branch families (JLE/JGT)
- Saturating arithmetic flag behavior exactly per spec (V/SV/AV)
- Disassembly helpers and richer examples

PRs and issue reports are welcome.
