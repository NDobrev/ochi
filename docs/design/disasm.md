# TriCore Disassembler — Design

## 1) Design Goals
- Load raw ECU binary images and map them into a target address space.
- Disassemble TriCore TC1.6.2 (mixed 16/32-bit encodings) reliably and reproducibly.
- Identify and present logical blocks (code/data/unknown) with basic cross-references.
- Offer a clean CLI now and a reusable analysis core for a future GUI.
- Be safe by default (read-only), fast on large images, and configurable.

## 2) Supported Features (Initial → Near-term)
- Loaders
  - RawBinLoader: read `.bin` with `--base`, `--skip`, `--len`.
  - MapLoader (optional): YAML/TOML that defines multiple segments (name, base, perms, kind), useful for ECU profiles (PFLASH/DFLASH/RAM).
- Memory Model
  - Immutable `Image` with named `Segment`s (range, perms: R/W/X, kind: Flash/Ram/Other).
  - Read-only `MemoryView` for address→byte/word access with hole checks.
- Disassembly
  - Range disassembly: decode and print `{addr, bytes, mnemonic, operands}` using existing `Tc16Decoder`.
  - Width-aware decoding (16/32-bit), byte rendering, optional comments.
- Analysis (seeded)
  - Worklist-based decode from explicit entries (`--entry`), following fallthrough and near branches.
  - Build basic blocks, collect xrefs, form simple function regions.
  - Guardrails: segment perms, decode validity, limits (`--max-instr`, `--max-bytes`).
- Output
  - Text renderer: objdump-like listing with optional bytes/labels/xrefs.
  - JSON renderer: segments, blocks, functions, and xrefs for GUI ingestion.
- CLI UX
  - `sections` (list segments), `range <start> <end>`, `function <addr>`, `analyze` (graph + summary).
  - Common flags: `--base`, `--skip`, `--len`, `--map`, `--entry`, `--format text|json`, `--out`, `--show-bytes`.

## 3) Extensibility
- Architecture
  - Core analysis library + thin CLI binary.
  - Modules: `loaders`, `memory`, `decoder` (use existing), `analysis`, `render`.
- Abstractions
  - `trait MemoryView { read_u8/u16/u32(..) -> Option<T> }` backed by segments.
  - `trait Loader { fn load(&Path) -> Image }` for pluggable formats (ELF/HEX/S19 later).
  - `trait Renderer { fn text/json(..) }` for multiple outputs and future GUI adapters.
- Profiles & Heuristics
  - ECU profiles in YAML (commonly used address ranges and perms).
  - Toggleable heuristics (dense-decode windowing, literal pool & string detection).
- Integration Paths
  - “Serve” mode (IPC/JSON over stdio or TCP) for GUI to query the analysis live.
  - Stable JSON schema and library API returning a `Report` (segments, blocks, functions, xrefs, symbols).

## 4) Task Plan
- Phase 1 — Scaffold & Range Disasm
  - Project scaffolding
    - Add `tricore-disasm` crate to the workspace; depend on `tricore-rs`.
    - Wire up Clap-based CLI entry; `anyhow` for errors.
  - CLI surface (v1)
    - Global flags: `--base`, `--skip`, `--len?` (optional), input path.
    - Subcommands:
      - `sections` — list loaded segments (single raw segment for now).
      - `range <start> <end>` — disassemble byte range (half-open) with `--show-bytes`.
    - Address parsing helper: hex (`0x...`) and decimal.
  - Loader & memory
    - RawBinLoader: read file, apply `--skip`, limit by `--len` if provided.
    - Map payload into a single segment at `--base`.
    - Minimal `Image/Segment` types and a read-only `MemoryView` (can wrap existing `LinearMemory`).
  - Disassembly loop
    - Use `Tc16Decoder` to decode 16/32-bit instructions.
    - On success: advance `pc` by decoded width; render with `fmt_decoded`.
    - On failure: print `.word <raw32>` and advance by 4.
    - Bound checks: stop on OOB; print `<oob>` sentinel line.
    - `--show-bytes`: render 2 or 4 bytes alongside text.
  - Output formatting
    - Consistent columns: address, bytes (optional), mnemonic+operands.
    - Configurable address width (fixed 32-bit for now).
  - Errors & exit codes
    - User-friendly messages for file errors, skip/len bounds, bad addresses.
  - Tests
    - Unit: `parse_u32` (hex/dec), loader mapping (`base`, `skip`), OOB behavior.
    - Integration: snapshot for `range` on a small buffer (both with/without `--show-bytes`).
    - Smoke: `sections` output sanity.
  - Docs
    - Usage examples for `sections` and `range` in README or `docs/`.
- Phase 2 — Seeded Analysis & Xrefs
  - Worklist decoder (fallthrough + direct branches), basic blocks, xrefs.
  - Function boundary hints (call targets) and label generation.
  - JSON renderer for blocks/functions/xrefs; CLI `analyze`.
  - Tests: graph correctness on small programs, width mixing.
- Phase 3 — Segments & Config
  - MapLoader (YAML/TOML), segment perms/kinds; support multiple entries.
  - AnalysisConfig: limits, heuristics toggles, label styles; import/export symbols.
  - Tests: multi-segment images, non-executable regions respected.
- Phase 4 — Data Identification
  - Literal pool recognition (PC-relative loads), string and array sniffing in RO data.
  - Code→Data xrefs and block classification (Code/Data/Unknown/Gap).
  - Enrich text comments with target labels and pool annotations.
- Phase 5 — Performance & Robustness
  - Fast address→segment lookup (sorted ranges + binary search or interval tree).
  - Guard against decode storms, add time/step limits; fuzz basic inputs.
  - Snapshot tests for formatter stability.
- Phase 6 — GUI Readiness
  - Serve mode for live queries; stable JSON schema.
  - Document API and examples; package profiles; basic integration guide.

---

This plan delivers a useful CLI quickly while building a solid core the GUI can reuse without rework.
