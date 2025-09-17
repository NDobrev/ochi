# TriCore Disassembler GUI (Iced) — Design

## Goals
- Load raw `.bin` images (+ optional map/labels) and analyze.
- Present code blocks, xrefs, labels, and bytes with fast navigation.
- Keep UI responsive on large images (async work, lazy rendering).
- Reuse the existing analysis engine and JSON data model.

## Architecture
- New crate: `crates/tricore-disasm-gui` (workspace member)
  - Depends on `tricore-disasm` (loaders + analysis) and `tricore-rs` (fmt).
- MVC-like with Iced:
  - Model: current `Image`, `ReportWithLabels`, selection, filters, labels, prefs.
  - View: panes for Code, Graph, Segments, Labels, Hex.
  - Controller: Iced `Message`s; heavy work via `Command::perform` (async).

## Data Flow
1. Open File → load to `Image` (reuse `model::load_raw_bin`).
2. Start analysis (async) → `analyze_entries` + block building (as in CLI).
3. On done, store `ReportWithLabels` (blocks with pre-rendered mnemonics, edges, functions, labels).
4. Derive indexes: `addr→block`, `label→addr`, adjacency.
5. UI updates state and renders visible portions only.

## UI Layout
- Top Bar
  - File open, Base/Skip/Entry controls, Analyze, Theme toggle, Search (addr/label/text).
- Sidebar (Left)
  - Segments (name, range, perms)
  - Labels list (filterable, clickable)
  - Functions/Entries list
- Main (Tabs)
  - Code View: virtualized list of blocks: label header + `insns` (addr, bytes, mnemonic)
  - Graph View (Phase 2): block graph (Iced Canvas)
  - Hex View: segment-aware hex with selection
- Bottom Bar
  - Status: file, analysis time, counters; current PC and selection details

## Messages (Iced)
- `FilePicked(PathBuf)`, `LoadStarted`, `LoadFinished(Result<Image>)`
- `AnalyzeRequested`, `AnalyzeFinished(Result<ReportWithLabels>)`
- `NavigateToAddr(u32)`, `NavigateToLabel(String)`, `SelectBlock(u32)`
- `ToggleBytes(bool)`, `FilterUpdated(String)`
- `LabelsImported(Vec<LabelKV>)`, `LabelsExportRequested(PathBuf)`, `LabelsSaved(Result<()>)`
- `ThemeChanged(Theme)`, `FontChanged(i32)`

## Commands (async)
- `load_image(path, base, skip, len)`
- `run_analysis(image, seeds, max_instr, show_bytes)`
- `read_labels`, `write_labels`

## Code View Rendering
- Use `Scrollable` with a virtualized approach (render only visible blocks/lines).
- Show labels (`sub_*`, `loc_*`) above block; highlight selection.
- Click label or instruction → navigate (`NavigateToAddr`).

## Performance
- Use pre-rendered `insns` lines from analysis JSON to avoid UI-thread decoding.
- Paginate/virtualize large lists; compute visible range from scroll offset.
- Debounce search/filter; heavy work off main thread.

## Labels
- Import on open; overlay onto report labels.
- In-place rename in code view; mark dirty and allow export.
- Ensure unique names; auto-suffix on conflicts.

## Shortcuts (suggested)
- `g` then addr (go to address), `l` then name (go to label)
- `n`/`p` (next/prev block), `f` (follow branches)
- `b` (toggle bytes), `/` (search), `s` (save labels), `r` (re-analyze)

## Theming & Fonts
- Iced theme (dark/light), adjustable monospace font size.
- Color accents for labels and xref types.

## Hex View
- Segment-aware addresses; highlight selected; click → navigate to code if analyzed.

## Graph View (Phase 2)
- Use Iced Canvas; simple force-directed layout.
- Nodes are block starts; edges typed: ft/br/cbr/call.
- Click node → scroll to block in Code View.

## Persistence
- Preferences (theme, font, window size) via small JSON or `confy`.
- Recent files, last project, last labels file.

## Crate Setup
- `crates/tricore-disasm-gui/Cargo.toml`
  - `iced = { version = "0.10", features = ["tokio", "canvas", "svg"] }`
  - `tokio`, `serde`, `serde_json`
  - `tricore-disasm = { path = "../tricore-disasm" }`
  - `tricore-rs = { path = "../.." }`

## Shared API (from tricore-disasm)
- Public functions to load image, run analysis, and enrich blocks.
- `LabelKV`, `ReportWithLabels` structs reused by GUI.

## Roadmap
- Phase 1 (MVP)
  - File open, image load
  - Run analyze (async), show code blocks with bytes
  - Navigate by address/label; labels pane; import/export labels
- Phase 2
  - Graph view (Canvas), Hex view; selection sync
  - Label editing in place; search (addr/label/text)
- Phase 3
  - Map loader UI, multi-segment support; config UI (seeds/limits)
- Phase 4
  - Performance polish (virtualization), snapshot tests of rendering

---

This GUI builds on the existing loader/analysis and the enriched JSON report so the UI stays responsive and simple. Next step: scaffold the `tricore-disasm-gui` crate and render the first window with a basic code listing.

## Phase-by-Phase Detailed Plan (Views, Buttons, Interactions)

### Phase 1 — MVP (Open → Analyze → Browse Code)
**Views**
- Top Bar
  - Button: Open…
  - Text inputs: Base (hex), Skip (dec), Seeds (multi-address input)
  - Button: Analyze (async)
  - Toggles: Show Bytes, Theme (Light/Dark)
  - Status text: Idle | Loading | Analyzing… | Error
- Sidebar (left)
  - Segments: list (name, start, end, perms) → click to navigate
  - Labels: filter box + list (name @ addr) → click to navigate
  - Entries/Functions: tabbed lists → click to navigate
- Main (Code Tab)
  - Scrollable/virtualized blocks; header with label, lines with addr/bytes/mnemonic
- Bottom Bar
  - PC, selection, counts (blocks/edges), quick Go-to address field + Go

**Interactions**
- Open… → pick file → load_image (async) → status updates
- Analyze → run_analysis (async) with seeds → on done, render code
- Labels pane filter (debounced); click label/entry/function navigates and highlights
- Toggle Show Bytes / Theme → re-render theme/lines (no reanalysis)

**State/Async**
- State: file_path, image, seeds, base/skip, report, labels, selection, progress, prefs
- Commands: load_image, run_analysis

**Acceptance**
- Can open .bin, analyze, and browse code with labels and bytes

### Phase 2 — Graph & Hex, Label Editing, Search
**Views**
- Graph Tab (Canvas): nodes=blocks, edges typed (ft/br/cbr/call); zoom+/−, fit, edge-type toggles
- Hex Tab: segment dropdown, hex view with address column; selection highlight
- Code/Labels: in-place label editing (click label → input → Enter saves)
- Search: top field (auto/addr/label/text), collapsible results pane

**Interactions**
- Graph: click node → navigate code; hover → tooltip; pan/zoom
- Hex: click byte → navigate code
- Labels: rename (validate unique/non-empty), Save labels…, Import labels…
- Search: debounced; clicking result navigates

**State/Async**
- graph_layout cached; search_query/results
- Commands: compute_graph_layout, run_search

**Acceptance**
- Working graph/hex; label editing; search navigates properly

### Phase 3 — Config & Maps, Multi-Segment, Preferences
**Views**
- Settings dialog: analysis limits, seeds editor, show-bytes pre-render; map loader; ECU profiles
- Segments pane: multi-segment controls

**Interactions**
- Apply settings → re-analyze; Cancel reverts
- Import map merges/replaces; profiles fill base/perms
- Preferences persist (theme, font, recent files)

**Acceptance**
- Multi-segment analysis and persistent settings work

### Phase 4 — Performance & QA
**Perf**
- Virtualized code list; lazy block rendering; cached search indices

**Testing**
- Snapshot tests of rendered code lines
- Property tests for label edits and navigation
- Fuzz large inputs (guarded)

**Polish**
- Shortcuts: g (go to), l (label), n/p (next/prev block), f (follow), b (bytes), / (search), s (save), r (re-analyze)
- Context menu: copy address/line
