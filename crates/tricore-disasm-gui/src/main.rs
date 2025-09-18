use anyhow::Result;
use iced::alignment;
use iced::widget::{button, column, container, row, scrollable, text, text_input, toggler, horizontal_rule, vertical_rule, pick_list};
use iced::{executor, theme, Application, Command, Element, Length, Theme, Color};
use iced::widget::canvas::{self, Canvas, Frame, Path as CanvasPath, Stroke, Program, Style as CanvasStyle, Renderer as CanvasRenderer, Text as CanvasText};
use iced::mouse;
use iced::{Point, Size};
use iced::Rectangle;
use std::path::Path;
use std::time::Instant;

use tricore_disasm::{analyze_entries, load_raw_bin, read_u8, read_u32, Image};
use tricore_disasm::analyze::{Edge, EdgeKind};
use tricore_disasm::model::read_u16;
use tricore_rs::disasm::fmt_decoded;
use tricore_rs::decoder::Decoder;
use tricore_rs::isa::tc16::Tc16Decoder;

#[derive(Debug, Default, Clone)]
struct AppState {
    path: String,
    base: String,
    skip: String,
    status: String,
    show_bytes: bool,
    image: Option<Image>,
    visited: Vec<u32>,
    tab: Tab,
    selection: Option<u32>,
    selected_addr: Option<u32>,
    label_edit: String,
    labels: std::collections::HashMap<u32, String>,
    search: String,
    logs: Vec<String>,
    analyze_started: Option<Instant>,
    // Hex editing state: addr -> current edit buffer (2 hex chars)
    hex_edits: std::collections::HashMap<u32, String>,
    // Settings
    show_settings: bool,
    theme: Theme,
    font_size: u16,
    code_color: Option<Color>,
    // Analysis results
    edges: Vec<Edge>,
    // Graph filters
    show_ft: bool,
    show_br: bool,
    show_cbr: bool,
    show_call: bool,
    // Labels persistence
    labels_path: String,
}

#[derive(Debug, Clone)]
enum Msg {
    PathChanged(String),
    BaseChanged(String),
    SkipChanged(String),
    ToggleBytes(bool),
    SwitchTab(Tab),
    SearchChanged(String),
    SelectPc(u32),
    LabelEditChanged(String),
    SaveLabel,
    OpenExample,
    Load,
    LoadedOk(Image),
    LoadedErr(String),
    Analyze,
    AnalyzedOk(Vec<u32>, Vec<Edge>),
    AnalyzedErr(String),
    ToggleSettings,
    ThemePicked(ThemeChoice),
    FontSizePicked(u16),
    CodeColorPicked(ColorChoice),
    SearchGo,
    ToggleEdgeFt(bool),
    ToggleEdgeBr(bool),
    ToggleEdgeCbr(bool),
    ToggleEdgeCall(bool),
    SaveLabels,
    LabelsSaved(Result<(), String>),
    LoadLabels,
    LabelsLoaded(Result<std::collections::HashMap<u32,String>, String>),
    SelectAddr(u32),
    HexEditChanged(u32, String),
    HexEditCommit(u32),
    CopySelection,
    PasteToSearch,
    SaveDisasm,
    DisasmSaved(Result<(), String>),
    SaveImageBin,
    ImageSaved(Result<(), String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab { Code, Disasm, Graph, Hex }

impl Default for Tab { fn default() -> Self { Tab::Code } }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ThemeChoice { Dark, Light }

impl std::fmt::Display for ThemeChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self { ThemeChoice::Dark => write!(f, "Dark"), ThemeChoice::Light => write!(f, "Light") }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ColorChoice { Default, White, Yellow, Cyan, Green }

impl std::fmt::Display for ColorChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColorChoice::Default => write!(f, "Default"),
            ColorChoice::White => write!(f, "White"),
            ColorChoice::Yellow => write!(f, "Yellow"),
            ColorChoice::Cyan => write!(f, "Cyan"),
            ColorChoice::Green => write!(f, "Green"),
        }
    }
}

struct App(AppState);

impl Application for App {
    type Executor = executor::Default;
    type Message = Msg;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (
            App(AppState {
                base: "0x0".into(),
                tab: Tab::Code,
                theme: theme::Theme::Dark,
                font_size: 16,
                code_color: None,
                edges: Vec::new(),
                show_ft: true,
                show_br: true,
                show_cbr: true,
                show_call: true,
                labels_path: "labels.json".into(),
                ..Default::default()
            }),
            Command::none(),
        )
    }

    fn title(&self) -> String { "TriCore Disassembler GUI".into() }
    fn theme(&self) -> Theme { self.0.theme.clone() }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        eprintln!("[Msg] {:?}", &message);
        match message {
            Msg::PathChanged(s) => { self.0.path = s.clone(); self.push_log(format!("PathChanged: {}", s)); },
            Msg::BaseChanged(s) => { self.0.base = s.clone(); self.push_log(format!("BaseChanged: {}", s)); },
            Msg::SkipChanged(s) => { self.0.skip = s.clone(); self.push_log(format!("SkipChanged: {}", s)); },
            Msg::ToggleBytes(b) => { self.0.show_bytes = b; self.push_log(format!("ToggleBytes: {}", b)); },
            Msg::SwitchTab(t) => self.0.tab = t,
            Msg::SearchChanged(s) => { self.0.search = s.clone(); self.push_log(format!("Search: {}", s)); },
            Msg::SelectPc(pc) => { self.0.selection = Some(pc); self.0.label_edit = self.0.labels.get(&pc).cloned().unwrap_or_default(); self.push_log(format!("SelectPc: {:#010x}", pc)); },
            Msg::LabelEditChanged(s) => { self.0.label_edit = s.clone(); self.push_log(format!("LabelEdit: {}", s)); },
            Msg::SaveLabel => {
                if let Some(pc) = self.0.selection {
                    let name = self.0.label_edit.trim();
                    if !name.is_empty() { self.0.labels.insert(pc, name.to_string()); self.push_log(format!("Saved label '{}' @ {:#010x}", name, pc)); }
                }
            }
            Msg::Load => {
                let path = self.0.path.clone();
                let base = parse_hex(&self.0.base).unwrap_or(0);
                let skip = self.0.skip.trim().parse::<usize>().unwrap_or(0);
                if path.trim().is_empty() {
                    self.0.status = "Please enter a file path (e.g., examples/out/00-basic.bin)".into();
                    self.push_log(self.0.status.clone());
                    return Command::none();
                }
                if !Path::new(&path).exists() {
                    self.0.status = format!("File not found: {}", path);
                    self.push_log(self.0.status.clone());
                    return Command::none();
                }
                self.0.status = format!("Loading {} base={:#x} skip={}…", path, base, skip);
                self.push_log(self.0.status.clone());
                return Command::perform(load_image_async(path, base, skip), |res| match res {
                    Ok(img) => Msg::LoadedOk(img),
                    Err(e) => Msg::LoadedErr(e.to_string()),
                });
            }
            Msg::OpenExample => {
                let ex = "examples/out/00-basic.bin".to_string();
                self.0.path = ex.clone();
                let base = parse_hex(&self.0.base).unwrap_or(0);
                let skip = self.0.skip.trim().parse::<usize>().unwrap_or(0);
                if !Path::new(&ex).exists() {
                    self.0.status = "examples/out/00-basic.bin not found; run scripts/run_examples.sh first".into();
                    self.push_log(self.0.status.clone());
                    return Command::none();
                }
                self.0.status = format!("Loading {} base={:#x} skip={}…", ex, base, skip);
                self.push_log(self.0.status.clone());
                return Command::perform(load_image_async(ex, base, skip), |res| match res {
                    Ok(img) => Msg::LoadedOk(img),
                    Err(e) => Msg::LoadedErr(e.to_string()),
                });
            }
            Msg::LoadedOk(img) => {
                // Store image and auto-run analysis so code shows up immediately
                self.0.image = Some(img.clone());
                let seeds = vec![img.segments.first().map(|s| s.base).unwrap_or(0)];
                self.0.status = format!("Loaded. Analyzing… seeds={:?}", seeds);
                self.0.analyze_started = Some(Instant::now());
                self.push_log(self.0.status.clone());
                return Command::perform(analyze_async(img, seeds), |res| match res {
                    Ok((v, e)) => Msg::AnalyzedOk(v, e),
                    Err(e) => Msg::AnalyzedErr(e.to_string()),
                });
            }
            Msg::LoadedErr(e) => { self.0.status = format!("Load error: {e}"); self.0.image = None; self.0.visited.clear(); self.push_log(self.0.status.clone()); }
            Msg::Analyze => {
                if let Some(img) = &self.0.image {
                    let seeds = vec![img.segments.first().map(|s| s.base).unwrap_or(0)];
                    let img2 = img.clone();
                    self.0.status = format!("Analyzing… seeds={:?}", seeds);
                    self.0.analyze_started = Some(Instant::now());
                    self.push_log(self.0.status.clone());
                    return Command::perform(analyze_async(img2, seeds), |res| match res {
                        Ok((v, e)) => Msg::AnalyzedOk(v, e),
                        Err(e) => Msg::AnalyzedErr(e.to_string()),
                    });
                }
            }
            Msg::AnalyzedOk(mut pcs, edges) => {
                pcs.sort_unstable();
                self.0.visited = pcs;
                self.0.edges = edges;
                let dt = self.0.analyze_started.take().map(|t| t.elapsed()).map(|d| format!(" in {:?}", d)).unwrap_or_default();
                self.0.status = format!("Analysis done{} (visited={}, edges={})", dt, self.0.visited.len(), self.0.edges.len());
                self.push_log(self.0.status.clone());
            }
            Msg::AnalyzedErr(e) => { self.0.status = format!("Analyze error: {e}"); self.0.visited.clear(); self.push_log(self.0.status.clone()); }
            Msg::ToggleSettings => { self.0.show_settings = !self.0.show_settings; }
            Msg::ThemePicked(t) => {
                self.0.theme = match t { ThemeChoice::Dark => Theme::Dark, ThemeChoice::Light => Theme::Light };
            }
            Msg::FontSizePicked(sz) => { self.0.font_size = sz; }
            Msg::CodeColorPicked(choice) => {
                self.0.code_color = match choice {
                    ColorChoice::Default => None,
                    ColorChoice::White => Some(Color::from_rgb(0.95, 0.95, 0.95)),
                    ColorChoice::Yellow => Some(Color::from_rgb(0.95, 0.85, 0.2)),
                    ColorChoice::Cyan => Some(Color::from_rgb(0.2, 0.9, 0.9)),
                    ColorChoice::Green => Some(Color::from_rgb(0.5, 0.95, 0.5)),
                };
            }
            Msg::SearchGo => {
                // Try to navigate to address or label
                if let Some(pc) = parse_nav(&self.0.search, &self.0.labels) {
                    self.0.selection = Some(pc);
                    self.push_log(format!("Navigate to {:#010x}", pc));
                } else {
                    self.0.status = format!("No match for '{}'", self.0.search);
                }
            }
            Msg::ToggleEdgeFt(b) => { self.0.show_ft = b; }
            Msg::ToggleEdgeBr(b) => { self.0.show_br = b; }
            Msg::ToggleEdgeCbr(b) => { self.0.show_cbr = b; }
            Msg::ToggleEdgeCall(b) => { self.0.show_call = b; }
            Msg::SaveLabels => {
                let path = self.0.labels_path.clone();
                let map = self.0.labels.clone();
                return Command::perform(async move {
                    let res = tokio::task::spawn_blocking(move || -> Result<(), String> {
                        let s = serde_json::to_string_pretty(&map).map_err(|e| e.to_string())?;
                        std::fs::write(&path, s).map_err(|e| e.to_string())?;
                        Ok(())
                    }).await.map_err(|e| e.to_string()).and_then(|r| r);
                }, |_| Msg::LabelsSaved(Ok(())));
            }
            Msg::LabelsSaved(r) => {
                match r { Ok(()) => { self.0.status = format!("Labels saved to {}", self.0.labels_path); }, Err(e) => { self.0.status = format!("Save error: {}", e); } }
                self.push_log(self.0.status.clone());
            }
            Msg::LoadLabels => {
                let path = self.0.labels_path.clone();
                return Command::perform(async move {
                    tokio::task::spawn_blocking(move || -> Result<std::collections::HashMap<u32,String>, String> {
                        let data = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
                        let map: std::collections::HashMap<u32,String> = serde_json::from_str(&data).map_err(|e| e.to_string())?;
                        Ok(map)
                    }).await.map_err(|e| e.to_string()).and_then(|r| r)
                }, |r| Msg::LabelsLoaded(r));
            }
            Msg::LabelsLoaded(r) => {
                match r {
                    Ok(m) => { self.0.labels = m; self.0.status = format!("Labels loaded from {}", self.0.labels_path); }
                    Err(e) => { self.0.status = format!("Load error: {}", e); }
                }
                self.push_log(self.0.status.clone());
            }
            Msg::SelectAddr(a) => { self.0.selected_addr = Some(a); }
            Msg::HexEditChanged(addr, s) => {
                // Keep only hex chars, limit to 2
                let filtered: String = s.chars().filter(|c| c.is_ascii_hexdigit()).take(2).collect();
                if filtered.is_empty() { self.0.hex_edits.remove(&addr); } else { self.0.hex_edits.insert(addr, filtered.clone()); }
                self.0.selected_addr = Some(addr);
                // Auto-commit when two hex digits entered
                if filtered.len() == 2 {
                    if let Some(img) = &mut self.0.image {
                        for s in &mut img.segments {
                            let start = s.base; let end = s.base + s.bytes.len() as u32;
                            if addr >= start && addr < end {
                                if let Ok(v) = u8::from_str_radix(&filtered, 16) {
                                    s.bytes[(addr - start) as usize] = v;
                                    self.0.status = format!("Wrote {:#04x} @ {:#010x}", v, addr);
                                    self.push_log(self.0.status.clone());
                                }
                                break;
                            }
                        }
                    }
                    self.0.hex_edits.remove(&addr);
                    if let Some(img2) = self.0.image.clone() {
                        let seeds = vec![img2.segments.first().map(|s| s.base).unwrap_or(0)];
                        self.0.status = "Analyzing after hex edit…".into();
                        self.0.analyze_started = Some(Instant::now());
                        self.push_log(self.0.status.clone());
                        return Command::perform(analyze_async(img2, seeds), |res| match res {
                            Ok((v, e)) => Msg::AnalyzedOk(v, e),
                            Err(e) => Msg::AnalyzedErr(e.to_string()),
                        });
                    }
                }
            }
            Msg::HexEditCommit(addr) => {
                if let Some(img) = &mut self.0.image {
                    // Find segment containing addr
                    for s in &mut img.segments {
                        let start = s.base; let end = s.base + s.bytes.len() as u32;
                        if addr >= start && addr < end {
                            if let Some(buf) = self.0.hex_edits.get(&addr) {
                                if let Ok(v) = u8::from_str_radix(buf, 16) {
                                    s.bytes[(addr - start) as usize] = v;
                                    self.0.status = format!("Wrote {:#04x} @ {:#010x}", v, addr);
                                    self.push_log(self.0.status.clone());
                                }
                            }
                            break;
                        }
                    }
                }
                // Clear the edit buffer after commit
                self.0.hex_edits.remove(&addr);
                // Re-run analysis so Code/Graph reflect new bytes
                if let Some(img2) = self.0.image.clone() {
                    let seeds = vec![img2.segments.first().map(|s| s.base).unwrap_or(0)];
                    self.0.status = "Analyzing after hex edit…".into();
                    self.0.analyze_started = Some(Instant::now());
                    self.push_log(self.0.status.clone());
                    return Command::perform(analyze_async(img2, seeds), |res| match res {
                        Ok((v, e)) => Msg::AnalyzedOk(v, e),
                        Err(e) => Msg::AnalyzedErr(e.to_string()),
                    });
                }
            }
            Msg::CopySelection => {
                // Compose text from current tab selection
                let text = match self.0.tab {
                    Tab::Code => {
                        if let (Some(img), Some(pc)) = (&self.0.image, self.0.selection) {
                            let dec = Tc16Decoder::new();
                            if let Some(raw32) = read_u32(img, pc) { if let Some(d) = dec.decode(raw32) { format!("{pc:#010x}: {}", fmt_decoded(&d)) } else { format!("{pc:#010x}") } } else { format!("{pc:#010x}") }
                        } else { String::new() }
                    }
                    Tab::Hex | Tab::Disasm | Tab::Graph => {
                        if let Some(addr) = self.0.selected_addr { if let Some(img) = &self.0.image { let b = read_u8(img, addr).unwrap_or(0); format!("{addr:#010x}: {:#04x}", b) } else { String::new() } } else { String::new() }
                    }
                };
                if !text.is_empty() { self.0.status = format!("Copied: {}", text); }
                self.push_log(self.0.status.clone());
            }
            Msg::PasteToSearch => {
                // Without OS clipboard, reuse last status tail as a fallback paste stub
                // In a real app, integrate iced clipboard write/read.
                let t = self.0.status.clone();
                if let Some(idx) = t.find(": ") { self.0.search = t[idx+2..].to_string(); }
            }
            Msg::SaveDisasm => {
                if let Some(img) = &self.0.image {
                    let dec = Tc16Decoder::new();
                    let mut lines = Vec::new();
                    for &pc in &self.0.visited {
                        if let Some(raw32) = read_u32(img, pc) { if let Some(d) = dec.decode(raw32) { lines.push(format!("{pc:#010x}: {}", fmt_decoded(&d))); } }
                    }
                    let out = lines.join("\n");
                    return Command::perform(async move {
                        tokio::task::spawn_blocking(move || std::fs::write("disasm.txt", out)).await.map_err(|e| e.to_string()).and_then(|r| r.map_err(|e| e.to_string()))
                    }, |r| match r { Ok(()) => Msg::DisasmSaved(Ok(())), Err(e) => Msg::DisasmSaved(Err(e)) });
                }
            }
            Msg::DisasmSaved(r) => {
                match r { Ok(()) => self.0.status = "Saved disasm.txt".into(), Err(e) => self.0.status = format!("Save failed: {}", e) }
                self.push_log(self.0.status.clone());
            }
            Msg::SaveImageBin => {
                if let Some(img) = &self.0.image {
                    let data: Vec<u8> = if let Some(seg) = img.segments.first() { seg.bytes.clone() } else { Vec::new() };
                    return Command::perform(async move {
                        tokio::task::spawn_blocking(move || std::fs::write("image.bin", data)).await.map_err(|e| e.to_string()).and_then(|r| r.map_err(|e| e.to_string()))
                    }, |r| match r { Ok(()) => Msg::ImageSaved(Ok(())), Err(e) => Msg::ImageSaved(Err(e)) });
                }
            }
            Msg::ImageSaved(r) => { match r { Ok(()) => self.0.status = "Saved image.bin".into(), Err(e) => self.0.status = format!("Save failed: {}", e) } self.push_log(self.0.status.clone()); }
        }
        Command::none()
    }

    fn view(&self) -> Element<Self::Message> {
        let header = row![
            text_input("/path/to/file.bin", &self.0.path).on_input(Msg::PathChanged).width(Length::FillPortion(3)),
            text_input("base", &self.0.base).on_input(Msg::BaseChanged).width(Length::Fixed(100.0)),
            text_input("skip", &self.0.skip).on_input(Msg::SkipChanged).width(Length::Fixed(80.0)),
            button(text("Open")).on_press(Msg::Load),
            button(text("Analyze")).on_press(Msg::Analyze),
            button(text("Open Example")).on_press(Msg::OpenExample),
            toggler(Some("Bytes".into()), self.0.show_bytes, Msg::ToggleBytes).spacing(10),
            vertical_rule(1),
            button(if self.0.tab==Tab::Code { text("[Code]") } else { text("Code") }).on_press(Msg::SwitchTab(Tab::Code)),
            button(if self.0.tab==Tab::Disasm { text("[Disasm]") } else { text("Disasm") }).on_press(Msg::SwitchTab(Tab::Disasm)),
            button(if self.0.tab==Tab::Graph { text("[Graph]") } else { text("Graph") }).on_press(Msg::SwitchTab(Tab::Graph)),
            button(if self.0.tab==Tab::Hex { text("[Hex]") } else { text("Hex") }).on_press(Msg::SwitchTab(Tab::Hex)),
            vertical_rule(1),
            text("Search:"),
            text_input("text | 0xADDR | label", &self.0.search).on_input(Msg::SearchChanged).width(Length::Fixed(240.0)),
            button("Go").on_press(Msg::SearchGo),
            vertical_rule(1),
            button("Copy").on_press(Msg::CopySelection),
            button("Paste").on_press(Msg::PasteToSearch),
            vertical_rule(1),
            button("Save Disasm").on_press(Msg::SaveDisasm),
            button("Save Bin").on_press(Msg::SaveImageBin),
            vertical_rule(1),
            button(text(if self.0.show_settings { "Close Settings" } else { "Settings" })).on_press(Msg::ToggleSettings),
        ].spacing(10).align_items(iced::Alignment::Center);

        // Settings panel (optional)
        let settings_panel: Element<Msg> = if self.0.show_settings {
            let theme_items = vec![ThemeChoice::Dark, ThemeChoice::Light];
            let theme_pick = pick_list(theme_items.clone(),
                Some(if matches!(self.0.theme, Theme::Dark) { ThemeChoice::Dark } else { ThemeChoice::Light }),
                Msg::ThemePicked);

            let font_sizes: Vec<u16> = vec![12, 14, 16, 18, 20, 22, 24];
            let font_pick = pick_list(font_sizes.clone(), Some(self.0.font_size), Msg::FontSizePicked);

            let color_items = vec![ColorChoice::Default, ColorChoice::White, ColorChoice::Yellow, ColorChoice::Cyan, ColorChoice::Green];
            let color_pick = pick_list(color_items, Some(self.0.code_color.map_or(ColorChoice::Default, |c| {
                // Map current color to known choice, fallback Default
                let approx = |a: f32, b: f32| (a - b).abs() < 0.05;
                if approx(c.r, 0.95) && approx(c.g, 0.95) && approx(c.b, 0.95) { ColorChoice::White }
                else if approx(c.r, 0.95) && approx(c.g, 0.85) && approx(c.b, 0.2) { ColorChoice::Yellow }
                else if approx(c.r, 0.2) && approx(c.g, 0.9) && approx(c.b, 0.9) { ColorChoice::Cyan }
                else if approx(c.r, 0.5) && approx(c.g, 0.95) && approx(c.b, 0.5) { ColorChoice::Green }
                else { ColorChoice::Default }
            })), Msg::CodeColorPicked);

            row![
                text("Theme:"), theme_pick,
                text("Font size:"), font_pick,
                text("Code color:"), color_pick,
            ].spacing(10).align_items(iced::Alignment::Center).into()
        } else { container(column![]).into() };

        let status = container(text(&self.0.status)).width(Length::Fill);

        // Sidebar: segments + basic visited list (first 100) + labels
        let mut sidebar = column![text("Segments").size(self.0.font_size).style(theme::Text::Color([0.7,0.8,1.0].into()))].spacing(5);
        if let Some(img) = &self.0.image {
            for s in &img.segments {
                let line = format!("{}  {:#010x}..{:#010x}  {}", s.name, s.base, s.base + s.bytes.len() as u32, s.perms);
                sidebar = sidebar.push(text(line).size(self.0.font_size.saturating_sub(2)));
            }
        } else {
            sidebar = sidebar.push(text("(no image loaded)").size(self.0.font_size.saturating_sub(2)));
        }
        sidebar = sidebar.push(horizontal_rule(10));
        sidebar = sidebar.push(text(format!("Visited PCs: {}", self.0.visited.len())).size(self.0.font_size));
        let mut viscol = column![];
        for &pc in self.0.visited.iter().take(100) {
            viscol = viscol.push(text(format!("{pc:#010x}")).size(self.0.font_size.saturating_sub(2)));
        }
        sidebar = sidebar.push(scrollable(viscol).height(Length::Fixed(160.0)));
        sidebar = sidebar.push(horizontal_rule(10));
        // Labels quick list and save/load
        let mut lblhdr = row![text("Labels").size(self.0.font_size)];
        lblhdr = lblhdr.push(button("Save").on_press(Msg::SaveLabels));
        lblhdr = lblhdr.push(button("Load").on_press(Msg::LoadLabels));
        sidebar = sidebar.push(lblhdr.spacing(6));
        let mut lblcol = column![];
        let mut items: Vec<_> = self.0.labels.iter().map(|(pc,name)| (*pc, name.clone())).collect();
        items.sort_by(|a,b| a.1.cmp(&b.1));
        for (pc, name) in items.into_iter().take(200) {
            let line = format!("{} @ {:#010x}", name, pc);
            lblcol = lblcol.push(button(text(line).size(self.0.font_size.saturating_sub(2))).on_press(Msg::SelectPc(pc)));
        }
        sidebar = sidebar.push(scrollable(lblcol).height(Length::Fill));

        // Code list (simple): decode visited PCs on demand, filter via search
        let mut col: iced::widget::Column<Msg> = column![];
        let dec = Tc16Decoder::new();
        if let Some(img) = &self.0.image {
            if self.0.visited.is_empty() {
                col = col.push(text("No instructions to show yet. Analyzing or no code found.").size(self.0.font_size.saturating_sub(2)));
            }
            let mut pcs = self.0.visited.clone();
            if !self.0.search.trim().is_empty() {
                let q = self.0.search.to_lowercase();
                pcs.retain(|pc| {
                    // address match or mnemonic contains
                    if q.starts_with("0x") { if let Ok(addr) = u32::from_str_radix(q.trim_start_matches("0x"), 16) { return *pc == addr; } }
                    // label match
                    if let Some(name) = self.0.labels.get(pc) { if name.to_lowercase().contains(&q) { return true; } }
                    if let Some(raw32) = read_u32(img, *pc) {
                        if let Some(d) = dec.decode(raw32) { return fmt_decoded(&d).to_lowercase().contains(&q); }
                    }
                    false
                });
            }
            for pc in pcs {
                if let Some(raw32) = read_u32(img, pc) {
                    if let Some(d) = dec.decode(raw32) {
                        let label_prefix = self.0.labels.get(&pc).map(|s| format!("{}: ", s)).unwrap_or_default();
                        let line = if self.0.show_bytes {
                            let mut bytes = Vec::new();
                            for i in 0..(d.width as u32) { bytes.push(read_u8(img, pc + i).unwrap_or(0)); }
                            format!("{label_prefix}{pc:#010x}: {:02x?}  {}", bytes, fmt_decoded(&d))
                        } else {
                            format!("{label_prefix}{pc:#010x}: {}", fmt_decoded(&d))
                        };
                        let mut t = text(line).size(self.0.font_size);
                        if let Some(c) = self.0.code_color { t = t.style(theme::Text::Color(c)); }
                        let btn = button(t).on_press(Msg::SelectPc(pc));
                        col = col.push(btn);
                        if self.0.selection == Some(pc) {
                            let current = self.0.labels.get(&pc).cloned().unwrap_or_default();
                            let edit = row![
                                text("Label:"),
                                text_input(&current, &self.0.label_edit).on_input(Msg::LabelEditChanged).width(Length::Fixed(200.0)),
                                button("Save").on_press(Msg::SaveLabel),
                            ].spacing(5);
                            col = col.push(edit);
                        }
                    }
                }
            }
        }
        let code_view: Element<Msg> = match self.0.tab {
            Tab::Code => scrollable(col).height(Length::Fill).width(Length::Fill).into(),
            Tab::Disasm => {
                // Sequential disassembly of the first segment (preview without analysis)
                let mut lines = column![];
                if let Some(img) = &self.0.image {
                    if let Some(seg) = img.segments.first() {
                        let mut pc = seg.base;
                        let end = seg.base + seg.bytes.len() as u32;
                        let dec = Tc16Decoder::new();
                        let mut count = 0usize;
                        while pc < end && count < 4000 { // cap to 4000 lines
                            let raw32 = if let Some(x) = read_u32(img, pc) {
                                x
                            } else if let Some(h) = read_u16(img, pc) {
                                h as u32
                            } else { break };
                            if let Some(d) = dec.decode(raw32) {
                                let mut bytes = Vec::new();
                                for i in 0..(d.width as u32) { bytes.push(read_u8(img, pc + i).unwrap_or(0)); }
                                let line = if self.0.show_bytes {
                                    format!("{pc:#010x}: {:02x?}  {}", bytes, fmt_decoded(&d))
                                } else {
                                    format!("{pc:#010x}: {}", fmt_decoded(&d))
                                };
                                lines = lines.push(text(line).size(16));
                                pc = pc.saturating_add(d.width as u32);
                            } else {
                                // Unknown encoding: show as .2byte and advance 2
                                let b0 = read_u8(img, pc).unwrap_or(0);
                                let b1 = read_u8(img, pc + 1).unwrap_or(0);
                                let line = format!("{pc:#010x}: .2byte 0x{:02x}{:02x}", b1, b0);
                                lines = lines.push(text(line).size(16));
                                pc = pc.saturating_add(2);
                            }
                            count += 1;
                        }
                    }
                } else {
                    lines = lines.push(text("(no image loaded)").size(14));
                }
                scrollable(lines).height(Length::Fill).width(Length::Fill).into()
            }
            Tab::Graph => {
                // Canvas graph: simple linear layout by address with colored edges
                let toggles = row![
                    toggler(Some("FT".into()), self.0.show_ft, Msg::ToggleEdgeFt).spacing(5),
                    toggler(Some("BR".into()), self.0.show_br, Msg::ToggleEdgeBr).spacing(5),
                    toggler(Some("CBR".into()), self.0.show_cbr, Msg::ToggleEdgeCbr).spacing(5),
                    toggler(Some("CALL".into()), self.0.show_call, Msg::ToggleEdgeCall).spacing(5),
                ].spacing(10);

                // Build node list from visited PCs; layout along X by order
                let mut pcs = self.0.visited.clone();
                pcs.sort_unstable();
                let nodes: Vec<u32> = pcs;
                let graph = GraphCanvas::new(
                    nodes,
                    self.0.edges.clone(),
                    self.0.show_ft,
                    self.0.show_br,
                    self.0.show_cbr,
                    self.0.show_call,
                    self.0.selection,
                    self.0.labels.clone(),
                    self.0.font_size as f32,
                );
                let canvas = Canvas::new(graph).width(Length::Fill).height(Length::Fill);
                column![toggles, canvas].spacing(6).into()
            }
            Tab::Hex => {
                let mut lines = column![];
                if let Some(img) = &self.0.image {
                    if let Some(seg) = img.segments.first() {
                        let mut addr = seg.base;
                        let end = seg.base + seg.bytes.len() as u32;
                        while addr < end.min(seg.base + 1024) { // show up to 1KB
                            // Address column
                            let mut roww = row![text(format!("{addr:#010x}: ")).size(self.0.font_size.saturating_sub(2))].spacing(6);

                            // ASCII panel (clickable per-byte)
                            let mut ascii_row = row![];
                            for i in 0..16 {
                                let a = addr + i;
                                if a >= end { break; }
                                let val = seg.bytes[(a - seg.base) as usize];
                                let ch = if (0x20..=0x7e).contains(&val) { val as char } else { '.' };
                                let is_sel_b = self.0.selected_addr == Some(a);
                                let mut t = text(format!("{}", ch)).size(self.0.font_size.saturating_sub(2));
                                if is_sel_b { t = t.style(theme::Text::Color(Color::from_rgb(1.0, 1.0, 0.4))); }
                                ascii_row = ascii_row.push(button(t).on_press(Msg::SelectAddr(a)).padding(2));
                            }

                            // Bytes as individual editors (text_input per byte)
                            let mut byte_row = row![];
                            for i in 0..16 {
                                let a = addr + i;
                                if a >= end { break; }
                                let val = seg.bytes[(a - seg.base) as usize];
                                let is_sel_b = self.0.selected_addr == Some(a);
                                let displayed = self.0.hex_edits.get(&a).cloned().unwrap_or_else(|| format!("{:02x}", val));
                                let input = text_input("00", &displayed)
                                    .on_input(move |s| Msg::HexEditChanged(a, s))
                                    .on_submit(Msg::HexEditCommit(a))
                                    .width(Length::Fixed(32.0))
                                    .size(self.0.font_size.saturating_sub(2));
                                byte_row = byte_row.push(input);
                            }
                            // Compose: [ADDR] [ASCII] | [HEX]
                            roww = roww.push(ascii_row).push(vertical_rule(1)).push(byte_row);
                            lines = lines.push(roww);
                            addr += 16;
                        }
                    }
                }
                scrollable(lines).height(Length::Fill).width(Length::Fill).into()
            }
        };

        let content = row![
            container(sidebar).width(Length::Fixed(320.0)).padding(10),
            vertical_rule(1),
            container(code_view).padding(10).width(Length::Fill),
        ]
        .height(Length::Fill);

        // Logs
        let mut logcol = column![];
        for line in self.0.logs.iter().rev().take(100).rev() {
            logcol = logcol.push(text(line.clone()).size(12));
        }
        let logs = container(scrollable(logcol)).padding(6);

        // Layout proportions:
        // - Top (header + status): ~20%
        // - Middle (main content): ~70%
        // - Bottom (logs): ~10%
        let top = if self.0.show_settings { column![header, settings_panel, status].spacing(6) } else { column![header, status].spacing(6) };
        let layout = column![
            container(top).height(Length::FillPortion(1)),
            container(content).height(Length::FillPortion(7)),
            container(logs).height(Length::FillPortion(1)),
        ]
        .spacing(10)
        .padding(10);

        container(layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .align_x(alignment::Horizontal::Left)
            .into()
    }
}

fn parse_hex(s: &str) -> Option<u32> {
    let t = s.trim();
    if let Some(h) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) { u32::from_str_radix(h, 16).ok() } else { t.parse().ok() }
}

fn parse_nav(s: &str, labels: &std::collections::HashMap<u32, String>) -> Option<u32> {
    let t = s.trim();
    if t.is_empty() { return None; }
    if let Some(addr) = parse_hex(t) { return Some(addr); }
    // Try hex without 0x
    if t.chars().all(|c| c.is_ascii_hexdigit()) && t.len() <= 8 {
        if let Ok(addr) = u32::from_str_radix(t, 16) { return Some(addr); }
    }
    // Try label match (exact)
    for (pc, name) in labels.iter() {
        if name == t { return Some(*pc); }
    }
    None
}

async fn load_image_async(path: String, base: u32, skip: usize) -> Result<Image> {
    tokio::task::spawn_blocking(move || load_raw_bin(std::path::Path::new(&path), base, skip, None)).await.unwrap()
}

async fn analyze_async(img: Image, seeds: Vec<u32>) -> Result<(Vec<u32>, Vec<Edge>)> {
    tokio::task::spawn_blocking(move || {
        let (visited, _w, edges, _r) = analyze_entries(&img, &seeds, 100_000);
        Ok::<_, anyhow::Error>((visited.into_iter().collect(), edges))
    }).await.unwrap()
}

fn main() -> iced::Result { App::run(iced::Settings::default()) }

impl App {
    fn push_log(&mut self, line: impl Into<String>) {
        let s = line.into();
        eprintln!("[LOG] {}", s);
        self.0.logs.push(s);
        if self.0.logs.len() > 1000 { let n = self.0.logs.len() - 1000; self.0.logs.drain(0..n); }
    }
}

// Simple graph canvas
struct GraphCanvas {
    nodes: Vec<u32>,
    edges: Vec<Edge>,
    show_ft: bool,
    show_br: bool,
    show_cbr: bool,
    show_call: bool,
    selection: Option<u32>,
    labels: std::collections::HashMap<u32, String>,
    font_px: f32,
}

impl GraphCanvas {
    fn new(
        nodes: Vec<u32>,
        edges: Vec<Edge>,
        show_ft: bool,
        show_br: bool,
        show_cbr: bool,
        show_call: bool,
        selection: Option<u32>,
        labels: std::collections::HashMap<u32, String>,
        font_px: f32,
    ) -> Self {
        Self { nodes, edges, show_ft, show_br, show_cbr, show_call, selection, labels, font_px }
    }

    fn node_pos(&self, pc: u32, bounds: Rectangle) -> Point {
        // Fallback position (center) — not used by new layout
        Point::new(bounds.width/2.0, bounds.height/2.0)
    }
}

struct GraphState { offset: (f32,f32), scale: f32, dragging: Option<Point> }

impl Default for GraphState { fn default() -> Self { Self { offset: (40.0, 40.0), scale: 1.0, dragging: None } } }

impl Program<Msg> for GraphCanvas {
    type State = GraphState;

    fn draw(
        &self,
        state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<<iced::Renderer as CanvasRenderer>::Geometry> {
        let mut frame = Frame::new(renderer, Size::new(bounds.width, bounds.height));
        let (ox, oy) = state.offset;
        let sc = state.scale;

        use std::collections::HashMap;

        // Compute layered layout from edges: level per node (BFS from roots), then X per level
        let margin = 40.0;
        let node_r = 6.0_f32;
        let avail_w = (bounds.width - 2.0 * margin).max(1.0);
        let avail_h = (bounds.height - 2.0 * margin).max(1.0);

        // Build filtered adjacency and indegree for layout
        let mut adj: HashMap<u32, Vec<u32>> = HashMap::new();
        let mut indeg: HashMap<u32, usize> = HashMap::new();
        for &pc in &self.nodes { indeg.entry(pc).or_insert(0); adj.entry(pc).or_insert_with(Vec::new); }
        for e in &self.edges {
            let show = match e.kind { EdgeKind::Fallthrough => self.show_ft, EdgeKind::Branch => self.show_br, EdgeKind::CondBranch => self.show_cbr, EdgeKind::Call => self.show_call };
            if !show { continue; }
            adj.entry(e.from).or_default().push(e.to);
            *indeg.entry(e.to).or_insert(0) += 1;
            indeg.entry(e.from).or_insert(0);
        }
        // Roots: zero indegree; if none, pick first node
        let mut roots: Vec<u32> = self.nodes.iter().copied().filter(|pc| indeg.get(pc).copied().unwrap_or(0) == 0).collect();
        if roots.is_empty() { if let Some(&first) = self.nodes.first() { roots.push(first); } }
        // BFS levels
        let mut level: HashMap<u32, usize> = HashMap::new();
        use std::collections::VecDeque;
        let mut q = VecDeque::new();
        for r in roots { level.insert(r, 0); q.push_back(r); }
        while let Some(u) = q.pop_front() {
            let lu = level[&u];
            if let Some(vs) = adj.get(&u) {
                for &v in vs {
                    let lv = level.get(&v).copied();
                    if lv.map_or(true, |x| lu + 1 < x) { level.insert(v, lu + 1); q.push_back(v); }
                }
            }
        }
        // Assign any missing nodes: order by address, append after max level
        let max_level = level.values().copied().max().unwrap_or(0);
        let mut next_level = max_level + 1;
        for &pc in &self.nodes {
            if !level.contains_key(&pc) { level.insert(pc, next_level); next_level += 1; }
        }
        let levels_total = level.values().copied().max().unwrap_or(0) + 1;
        let min_v_spacing = (self.font_px.max(12.0) + 2.0 * node_r + 16.0).max(48.0);
        let mut y_step = if levels_total > 1 { avail_h / (levels_total as f32 - 1.0) } else { 0.0 };
        if y_step < min_v_spacing && levels_total > 1 { y_step = min_v_spacing; }
        // Group nodes per level ordered by address
        let mut by_level: HashMap<usize, Vec<u32>> = HashMap::new();
        for &pc in &self.nodes { by_level.entry(level[&pc]).or_default().push(pc); }
        for vs in by_level.values_mut() { vs.sort_unstable(); }
        // Compute positions
        let mut pos: HashMap<u32, Point> = HashMap::new();
        for (lev, vs) in by_level.iter() {
            let k = vs.len();
            let y = margin + (*lev as f32) * y_step;
            let x_step = if k > 1 { avail_w / (k as f32 - 1.0) } else { 0.0 };
            for (i, pc) in vs.iter().enumerate() {
                let x = margin + (i as f32) * x_step;
                pos.insert(*pc, Point::new(x, y));
            }
        }

        // Draw edges with arrowheads
        for e in &self.edges {
            let show = match e.kind {
                EdgeKind::Fallthrough => self.show_ft,
                EdgeKind::Branch => self.show_br,
                EdgeKind::CondBranch => self.show_cbr,
                EdgeKind::Call => self.show_call,
            };
            if !show { continue; }
            let p0w = pos.get(&e.from).copied().unwrap_or(Point::new(bounds.width/2.0, bounds.height/2.0));
            let p1w = pos.get(&e.to).copied().unwrap_or(Point::new(bounds.width/2.0, bounds.height/2.0));
            let p0 = Point::new(p0w.x * sc + ox, p0w.y * sc + oy);
            let p1 = Point::new(p1w.x * sc + ox, p1w.y * sc + oy);
            let color = match e.kind {
                EdgeKind::Fallthrough => Color::from_rgb(0.6,0.6,0.6),
                EdgeKind::Branch => Color::from_rgb(0.9,0.7,0.2),
                EdgeKind::CondBranch => Color::from_rgb(0.2,0.7,0.9),
                EdgeKind::Call => Color::from_rgb(0.4,0.95,0.4),
            };
            let stroke = Stroke { width: 2.0, style: CanvasStyle::Solid(color), ..Default::default() };
            let path = CanvasPath::line(p0, p1);
            frame.stroke(&path, stroke);
            // Arrowhead at p1
            let dx = p1.x - p0.x;
            let dy = p1.y - p0.y;
            let len = (dx*dx + dy*dy).sqrt();
            if len > 0.0001 {
                let ux = dx / len;
                let uy = dy / len;
            let arrow_len = 10.0_f32 * sc.max(0.5);
            let wing = 5.0_f32 * sc.max(0.5);
                let backx = p1.x - ux * arrow_len;
                let backy = p1.y - uy * arrow_len;
                // perpendicular (left) is (-uy, ux)
                let left = Point::new(backx + (-uy) * wing, backy + ux * wing);
                let right = Point::new(backx - (-uy) * wing, backy - ux * wing);
                let ah_stroke = Stroke { width: 2.0, style: CanvasStyle::Solid(color), ..Default::default() };
                frame.stroke(&CanvasPath::line(p1, left), ah_stroke.clone());
                frame.stroke(&CanvasPath::line(p1, right), ah_stroke);
            }
        }

        // Draw nodes + captions
        for &pc in &self.nodes {
            let pw = pos.get(&pc).copied().unwrap_or(Point::new(bounds.width/2.0, bounds.height/2.0));
            let p = Point::new(pw.x * sc + ox, pw.y * sc + oy);
            let circle = CanvasPath::circle(p, 6.0);
            let stroke = Stroke {
                width: if Some(pc) == self.selection { 3.0 } else { 1.5 },
                style: CanvasStyle::Solid(if Some(pc) == self.selection { Color::from_rgb(1.0, 1.0, 1.0) } else { Color::from_rgb(0.8, 0.8, 0.8) }),
                ..Default::default()
            };
            frame.stroke(&circle, stroke);

            // Caption: label if present, else short address
            let caption = self.labels.get(&pc).cloned().unwrap_or_else(|| format!("{pc:#06x}"));
            let color = if Some(pc) == self.selection { Color::from_rgb(1.0, 1.0, 1.0) } else { Color::from_rgb(0.85, 0.85, 0.85) };
            let mut text = CanvasText {
                content: caption,
                position: Point::new(p.x, p.y + (6.0 + 4.0)),
                color,
                size: (self.font_px.max(10.0) - 2.0) * sc.clamp(0.6, 1.5),
                ..Default::default()
            };
            text.horizontal_alignment = iced::alignment::Horizontal::Center;
            text.vertical_alignment = iced::alignment::Vertical::Top;
            frame.fill_text(text);
        }
        vec![frame.into_geometry()]
    }

    fn update(&self, state: &mut Self::State, event: canvas::Event, bounds: Rectangle, cursor: mouse::Cursor) -> (canvas::event::Status, Option<Msg>) {
        use canvas::event::Status;
        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(pos) = cursor.position_in(bounds) {
                    // Hit-test nodes (apply inverse transform)
                    let (ox, oy) = state.offset;
                    let sc = state.scale;
                    let inv = |p: Point| Point::new((p.x - ox)/sc, (p.y - oy)/sc);
                    state.dragging = Some(pos);
                    let posw = inv(pos);
                    let mut best: Option<(f32, u32)> = None;
                    for &pc in &self.nodes {
                        let p = self.node_pos(pc, bounds);
                        let dx = p.x - pos.x;
                        let dy = p.y - pos.y;
                        let d2 = dx*dx + dy*dy;
                        if d2 <= 10.0*10.0 {
                            if best.map_or(true, |(bd, _)| d2 < bd) { best = Some((d2, pc)); }
                        }
                    }
                    if let Some((_, pc)) = best { return (Status::Captured, Some(Msg::SelectPc(pc))); }
                }
                (Status::Ignored, None)
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => { state.dragging = None; (Status::Captured, None) }
            canvas::Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if let Some(prev) = state.dragging.take() {
                    let dx = position.x - prev.x;
                    let dy = position.y - prev.y;
                    state.offset.0 += dx;
                    state.offset.1 += dy;
                    state.dragging = Some(position);
                    return (Status::Captured, None);
                }
                (Status::Ignored, None)
            }
            canvas::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let z = match delta { mouse::ScrollDelta::Lines { y, .. } => y, mouse::ScrollDelta::Pixels { y, .. } => y / 40.0 };
                let factor = if z > 0.0 { 1.1 } else { 0.9 };
                state.scale = (state.scale * factor).clamp(0.5, 4.0);
                (Status::Captured, None)
            }
            _ => (canvas::event::Status::Ignored, None),
        }
    }
}
