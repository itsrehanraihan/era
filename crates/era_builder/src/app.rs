//! The Era Builder window (D-095): a visual builder over ERA source.
//!
//! ```text
//! ┌ Era Builder · Open · Undo Redo · Phone ▾ · 100% · Render real · Build APK · Export ┐
//! │ Insert  (drag)   │                                         │ Properties          │
//! │ Column Row Text  │   canvas: every screen side by side     │ Text  "Hello"       │
//! │ Button Image ... │   click to select · drag to move        │ Layout  width ...   │
//! │ Layers           │   drop from Insert to add               │ Style   background  │
//! │  Home ▸ column   │   Cmd+scroll / pinch to zoom            │ Text    font-size   │
//! ├──────────────────┴─────────────────────────────────────────┴─────────────────────┤
//! │ Code of the selection · problems                                                  │
//! └───────────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! Every action is a source edit (`edit.rs`), saved straight to disk, then
//! the project is re-checked and the canvas re-laid out.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use eframe::egui::{
    self, Color32, CornerRadius, FontId, Key, KeyboardShortcut, Modifiers, Pos2, Rect, RichText,
    Sense, Stroke, StrokeKind, TextureHandle, Vec2, epaint,
};
use era_canvas::{Fill, Frame, Item, Measure, layout_app};
use era_ir::ir::{Content, Element, ElementKind, SourceRef};
use era_studio::jobs::{self, Job, JobEvent, JobKind};
use era_studio::workspace::Workspace;

use crate::edit::{self, Place};

const UNDO: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::Z);
const REDO: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::Z);

/// Device sizes in dp.
pub const DEVICES: &[(&str, f32, f32)] = &[
    ("Phone", 360.0, 780.0),
    ("Small phone", 320.0, 640.0),
    ("Large phone", 412.0, 915.0),
    ("Tablet", 800.0, 1280.0),
];

/// Palette items: (label, snippet kind).
pub const PALETTE: &[(&str, &str)] = &[
    ("Column", "column"),
    ("Row", "row"),
    ("Stack", "stack"),
    ("Card", "card"),
    ("Grid", "grid"),
    ("Text", "text"),
    ("Button", "button"),
    ("Input", "input"),
    ("Box", "box"),
    ("Divider", "divider"),
    ("Spacer", "spacer"),
];

/// What is being dragged.
#[derive(Clone, Debug)]
pub enum Payload {
    Insert(String),
    Move(SourceRef),
}

/// The selected element (by IR id, without `#n` repeat suffixes).
#[derive(Clone, Debug, PartialEq)]
pub struct Selection {
    pub screen: String,
    pub id: String,
    pub kind: ElementKind,
    pub source: Option<SourceRef>,
    /// Inside a component's own code: edits change every use.
    pub from_component: bool,
    /// A component use (`Card { ... }`): edit its props, not its styles.
    pub component: Option<String>,
}

/// `Home/0/2#1` -> `Home/0/2`; repeated copies share one source.
pub fn base_id(id: &str) -> String {
    id.split('/')
        .map(|p| p.split('#').next().unwrap_or(p))
        .collect::<Vec<_>>()
        .join("/")
}

struct EguiMeasure<'a>(&'a egui::Context);

impl Measure for EguiMeasure<'_> {
    fn measure(&self, text: &str, size: f32, _weight: u16, max_width: Option<f32>) -> (f32, f32) {
        let g = self.0.fonts_mut(|f| {
            f.layout(
                text.to_string(),
                FontId::proportional(size),
                Color32::WHITE,
                max_width.unwrap_or(f32::INFINITY),
            )
        });
        (g.size().x.ceil(), g.size().y.ceil())
    }
}

fn c32(c: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])
}

fn hex(c: Color32) -> String {
    let [r, g, b, a] = c.to_srgba_unmultiplied();
    if a == 255 {
        format!("#{r:02X}{g:02X}{b:02X}")
    } else {
        format!("#{r:02X}{g:02X}{b:02X}{a:02X}")
    }
}

fn parse_hex(s: &str) -> Option<Color32> {
    let h = s.trim().strip_prefix('#')?;
    let v = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok();
    match h.len() {
        6 => Some(Color32::from_rgb(v(0)?, v(2)?, v(4)?)),
        8 => Some(Color32::from_rgba_unmultiplied(v(0)?, v(2)?, v(4)?, v(6)?)),
        _ => None,
    }
}

pub struct Builder {
    ws: Option<Workspace>,
    open_path: String,
    open_error: Option<String>,
    status: String,
    job: Option<Job>,
    output: Vec<String>,
    pub device: usize,
    pub zoom: f32,
    pan: Vec2,
    frames: Vec<Frame>,
    pub selected: Option<Selection>,
    hover: Option<String>,
    undo: Vec<(String, String)>,
    redo: Vec<(String, String)>,
    textures: BTreeMap<String, Option<TextureHandle>>,
    /// Properties panel edit buffers, by property name.
    fields: BTreeMap<String, String>,
    fields_for: Option<String>,
    new_prop: (String, String),
    new_screen: String,
}

impl Builder {
    pub fn new(path: Option<PathBuf>) -> Self {
        let mut b = Builder {
            ws: None,
            open_path: String::new(),
            open_error: None,
            status: "Open an ERA project to start building".into(),
            job: None,
            output: Vec::new(),
            device: 0,
            zoom: 0.8,
            pan: Vec2::new(40.0, 50.0),
            frames: Vec::new(),
            selected: None,
            hover: None,
            undo: Vec::new(),
            redo: Vec::new(),
            textures: BTreeMap::new(),
            fields: BTreeMap::new(),
            fields_for: None,
            new_prop: (String::new(), String::new()),
            new_screen: String::new(),
        };
        if let Some(p) = path {
            b.open_project(&p);
        }
        b
    }

    pub fn workspace(&self) -> Option<&Workspace> {
        self.ws.as_ref()
    }

    pub fn frames(&self) -> &[Frame] {
        &self.frames
    }

    pub fn open_project(&mut self, path: &Path) {
        match Workspace::open(path) {
            Ok(ws) => {
                self.status = format!("Opened {}", ws.root.display());
                self.open_path = ws.root.display().to_string();
                self.ws = Some(ws);
                self.open_error = None;
                self.selected = None;
                self.undo.clear();
                self.redo.clear();
                self.textures.clear();
            }
            Err(e) => self.open_error = Some(e),
        }
    }

    /// Current text of a project file (open buffer or disk).
    pub fn file_text(&mut self, rel: &str) -> Option<String> {
        let ws = self.ws.as_mut()?;
        ws.open_file(rel).ok().map(|b| b.text.clone())
    }

    /// Replace a file's text, save it, re-check the project.
    pub fn apply(&mut self, rel: &str, text: String) {
        let Some(old) = self.file_text(rel) else {
            return;
        };
        if old == text {
            return;
        }
        self.undo.push((rel.to_string(), old));
        self.redo.clear();
        self.write(rel, text);
    }

    fn write(&mut self, rel: &str, text: String) {
        let Some(ws) = &mut self.ws else { return };
        if let Ok(b) = ws.open_file(rel) {
            b.text = text;
        }
        if let Err(e) = ws.save(rel) {
            self.status = format!("Cannot save {rel}: {e}");
        }
        ws.analyze();
        self.fields_for = None;
        let errors = ws.error_count();
        self.status = if errors == 0 {
            format!("Saved {rel}")
        } else {
            format!("Saved {rel}: {errors} problem(s), see below")
        };
    }

    pub fn undo(&mut self) {
        if let Some((rel, text)) = self.undo.pop()
            && let Some(cur) = self.file_text(&rel)
        {
            self.redo.push((rel.clone(), cur));
            self.write(&rel, text);
        }
    }

    pub fn redo(&mut self) {
        if let Some((rel, text)) = self.redo.pop()
            && let Some(cur) = self.file_text(&rel)
        {
            self.undo.push((rel.clone(), cur));
            self.write(&rel, text);
        }
    }

    /// Run a source edit on the file of `src`.
    pub fn edit_at(
        &mut self,
        src: &SourceRef,
        f: impl FnOnce(&str, &str, usize) -> Option<String>,
    ) -> bool {
        let Some(text) = self.file_text(&src.path) else {
            return false;
        };
        match f(&text, &src.path, src.start) {
            Some(new) => {
                self.apply(&src.path, new);
                true
            }
            None => {
                self.status = "That change could not be made here".into();
                false
            }
        }
    }

    /// Insert a palette item into a container.
    pub fn insert_into(&mut self, container: &SourceRef, kind: &str) -> bool {
        let snip = edit::snippet(kind);
        self.edit_at(container, |t, p, s| edit::insert_child(t, p, s, &snip))
    }

    pub fn move_to(&mut self, from: &SourceRef, to: &SourceRef, place: Place) -> bool {
        if from.path != to.path {
            self.status = "Moving between files is not supported yet".into();
            return false;
        }
        let target = to.start;
        self.edit_at(from, |t, p, s| edit::move_node(t, p, s, target, place))
    }

    pub fn delete_selected(&mut self) {
        let Some(sel) = self.selected.clone() else {
            return;
        };
        let Some(src) = sel.source else { return };
        if self.edit_at(&src, edit::delete) {
            self.selected = None;
        }
    }

    pub fn set_prop(&mut self, name: &str, value: &str) {
        let Some(src) = self.selected.as_ref().and_then(|s| s.source.clone()) else {
            return;
        };
        let (name, value) = (name.to_string(), value.trim().to_string());
        if value.is_empty() {
            self.edit_at(&src, |t, p, s| edit::remove_property(t, p, s, &name));
        } else {
            self.edit_at(&src, |t, p, s| edit::set_property(t, p, s, &name, &value));
        }
    }

    pub fn set_label(&mut self, label: &str) {
        let Some(src) = self.selected.as_ref().and_then(|s| s.source.clone()) else {
            return;
        };
        let label = label.to_string();
        self.edit_at(&src, |t, p, s| edit::set_label(t, p, s, &label));
    }

    pub fn select_id(&mut self, screen: &str, id: &str) {
        let Some(app) = self.ws.as_ref().and_then(|w| w.app()) else {
            return;
        };
        fn find<'a>(e: &'a Element, id: &str) -> Option<&'a Element> {
            if e.id == id {
                return Some(e);
            }
            e.children.iter().find_map(|c| find(c, id))
        }
        let base = base_id(id);
        if let Some(s) = app.screens.iter().find(|s| s.name == screen)
            && let Some(e) = find(&s.root, &base)
        {
            let instance = e
                .origin
                .last()
                .is_some_and(|c| e.id.ends_with(&format!(":{c}")));
            self.selected = Some(Selection {
                screen: screen.to_string(),
                id: base,
                kind: e.kind,
                source: e.source.clone(),
                from_component: !e.origin.is_empty() && !instance,
                component: instance.then(|| e.origin.last().unwrap().clone()),
            });
        }
    }

    fn start_job(&mut self, kind: JobKind) {
        if self.job.is_some() {
            return;
        }
        let Some(ws) = &mut self.ws else { return };
        let _ = ws.save_all();
        self.output.clear();
        self.output
            .push(format!("$ era {}", jobs::args(kind, &ws.root).join(" ")));
        self.job = Some(jobs::start(kind, &ws.root));
        self.status = format!("{}...", kind.label());
    }

    fn poll_job(&mut self, ctx: &egui::Context) {
        let Some(job) = &self.job else { return };
        let mut done = None;
        while let Ok(ev) = job.events.try_recv() {
            match ev {
                JobEvent::Line(l) => self.output.push(l),
                JobEvent::Finished { success } => done = Some(success),
            }
        }
        match done {
            Some(ok) => {
                self.status = format!(
                    "{} {}",
                    job.kind.label(),
                    if ok {
                        "finished"
                    } else {
                        "failed (see Output)"
                    }
                );
                self.job = None;
            }
            None => ctx.request_repaint_after(Duration::from_millis(100)),
        }
    }

    fn texture(&mut self, ctx: &egui::Context, rel: &str) -> Option<TextureHandle> {
        if let Some(t) = self.textures.get(rel) {
            return t.clone();
        }
        let path = self.ws.as_ref()?.root.join("Assets").join(rel);
        let tex = image::open(&path).ok().map(|img| {
            let img = img.to_rgba8();
            let size = [img.width() as usize, img.height() as usize];
            ctx.load_texture(
                format!("asset-{rel}"),
                egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()),
                egui::TextureOptions::LINEAR,
            )
        });
        self.textures.insert(rel.to_string(), tex.clone());
        tex
    }
}

impl eframe::App for Builder {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = root.ctx().clone();
        self.poll_job(&ctx);
        let typing = ctx.egui_wants_keyboard_input();
        if !typing {
            if ctx.input_mut(|i| i.consume_shortcut(&REDO)) {
                self.redo();
            } else if ctx.input_mut(|i| i.consume_shortcut(&UNDO)) {
                self.undo();
            }
            if ctx.input(|i| i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace)) {
                self.delete_selected();
            }
        }
        self.toolbar(root);
        if self.ws.is_none() {
            self.welcome(root);
            return;
        }
        // Lay out every screen for the canvas.
        let (_, w, h) = DEVICES[self.device];
        self.frames = match self.ws.as_ref().and_then(|ws| ws.app()) {
            Some(app) => {
                // The start screen first, then the rest in project order.
                let mut f = layout_app(app, w, h, &EguiMeasure(&ctx));
                f.sort_by_key(|f| f.screen != app.start);
                f
            }
            None => self.frames.clone(),
        };
        self.bottom(root);
        self.left(root);
        self.right(root);
        self.canvas(root);
    }
}

// ----- panels ---------------------------------------------------------------

impl Builder {
    fn toolbar(&mut self, root: &mut egui::Ui) {
        egui::Panel::top("toolbar").show(root, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Era Builder").strong().size(16.0));
                ui.separator();
                if ui.button("Open…").on_hover_text("Open an ERA project folder").clicked()
                    && let Some(dir) = rfd::FileDialog::new().set_title("Open ERA project").pick_folder()
                {
                    self.open_project(&dir);
                }
                let has = self.ws.is_some();
                if ui
                    .add_enabled(has && !self.undo.is_empty(), egui::Button::new("Undo"))
                    .on_hover_text("Cmd+Z")
                    .clicked()
                {
                    self.undo();
                }
                if ui
                    .add_enabled(has && !self.redo.is_empty(), egui::Button::new("Redo"))
                    .on_hover_text("Cmd+Shift+Z")
                    .clicked()
                {
                    self.redo();
                }
                ui.separator();
                egui::ComboBox::from_label("Device")
                    .selected_text(DEVICES[self.device].0)
                    .show_ui(ui, |ui| {
                        for (i, (name, w, h)) in DEVICES.iter().enumerate() {
                            ui.selectable_value(&mut self.device, i, format!("{name} ({w}x{h})"));
                        }
                    });
                ui.add(
                    egui::Slider::new(&mut self.zoom, 0.25..=2.0)
                        .text("Zoom")
                        .custom_formatter(|v, _| format!("{:.0}%", v * 100.0)),
                );
                ui.separator();
                let busy = self.job.is_some();
                let ok = has && !busy && self.ws.as_ref().is_some_and(|w| w.error_count() == 0);
                if ui
                    .add_enabled(ok, egui::Button::new("Render real"))
                    .on_hover_text("Render every screen with the generated Compose code (exact output)")
                    .clicked()
                {
                    self.start_job(JobKind::Preview);
                }
                if ui
                    .add_enabled(ok, egui::Button::new("Build APK"))
                    .on_hover_text("era build android (needs ANDROID_HOME)")
                    .clicked()
                {
                    self.start_job(JobKind::BuildAndroid);
                }
                if ui
                    .add_enabled(ok, egui::Button::new("Export Kotlin"))
                    .on_hover_text("era export android: an Android Studio project with Kotlin + Jetpack Compose")
                    .clicked()
                {
                    self.start_job(JobKind::ExportAndroid);
                }
                ui.add_enabled(false, egui::Button::new("Export Swift"))
                    .on_disabled_hover_text("SwiftUI export is planned (D-082)");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if busy {
                        ui.spinner();
                    }
                    ui.label(&self.status);
                    if let Some(ws) = &self.ws {
                        ui.separator();
                        ui.label(RichText::new(ws.name()).strong());
                    }
                });
            });
        });
    }

    fn welcome(&mut self, root: &mut egui::Ui) {
        egui::CentralPanel::default().show(root, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(140.0);
                ui.heading(RichText::new("Era Builder").size(40.0));
                ui.label("Design native apps visually. Export Kotlin today, Swift next.");
                ui.add_space(24.0);
                if ui
                    .button(RichText::new("Open project…").size(18.0))
                    .clicked()
                    && let Some(dir) = rfd::FileDialog::new().pick_folder()
                {
                    self.open_project(&dir);
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add_space(ui.available_width() / 2.0 - 220.0);
                    ui.add(
                        egui::TextEdit::singleline(&mut self.open_path)
                            .hint_text("/path/to/project")
                            .desired_width(340.0),
                    );
                    if ui.button("Open").clicked() {
                        let p = PathBuf::from(self.open_path.clone());
                        self.open_project(&p);
                    }
                });
                if let Some(e) = &self.open_error {
                    ui.colored_label(Color32::from_rgb(220, 80, 80), e);
                }
            });
        });
    }

    fn left(&mut self, root: &mut egui::Ui) {
        egui::Panel::left("left")
            .resizable(true)
            .default_size(250.0)
            .show(root, |ui| {
                ui.label(RichText::new("Insert").strong());
                ui.label(
                    RichText::new("Drag onto the canvas or a layer")
                        .small()
                        .weak(),
                );
                egui::Grid::new("palette")
                    .num_columns(3)
                    .spacing([6.0, 6.0])
                    .show(ui, |ui| {
                        for (i, (label, kind)) in PALETTE.iter().enumerate() {
                            let id = ui.make_persistent_id(("palette", *kind));
                            ui.dnd_drag_source(id, Payload::Insert(kind.to_string()), |ui| {
                                egui::Frame::group(ui.style())
                                    .inner_margin(egui::Margin::symmetric(8, 4))
                                    .show(ui, |ui| {
                                        ui.label(*label);
                                    });
                            });
                            if i % 3 == 2 {
                                ui.end_row();
                            }
                        }
                    });
                ui.add_space(8.0);
                ui.separator();
                ui.label(RichText::new("Screens & layers").strong());
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.new_screen)
                            .hint_text("NewScreen")
                            .desired_width(120.0),
                    );
                    if ui.button("+ Screen").clicked() {
                        self.add_screen();
                    }
                });
                egui::ScrollArea::vertical().show(ui, |ui| self.layers(ui));
            });
    }

    fn add_screen(&mut self) {
        let name = self.new_screen.trim().to_string();
        let valid = name.chars().next().is_some_and(|c| c.is_ascii_uppercase())
            && name.chars().all(|c| c.is_ascii_alphanumeric());
        if !valid {
            self.status = "Screen names are PascalCase, like Profile".into();
            return;
        }
        let Some(ws) = &mut self.ws else { return };
        let rel = format!("src/Screens/{name}.era");
        let path = ws.root.join(&rel);
        if path.exists() {
            self.status = format!("{rel} already exists");
            return;
        }
        let _ = std::fs::create_dir_all(path.parent().unwrap());
        let text = format!(
            "screen {name} {{\n    title: \"{name}\"\n\n    column {{\n        width: 100%\n        padding: 24px\n        gap: 12px\n\n        text \"{name}\" {{ style: heading }}\n    }}\n}}\n"
        );
        match std::fs::write(&path, text) {
            Ok(()) => {
                ws.reload();
                ws.analyze();
                self.status = format!("Added {rel}");
                self.new_screen.clear();
            }
            Err(e) => self.status = format!("Cannot write {rel}: {e}"),
        }
    }

    fn layers(&mut self, ui: &mut egui::Ui) {
        let Some(app) = self.ws.as_ref().and_then(|w| w.app()).cloned() else {
            ui.label("Fix the problems below to see layers.");
            return;
        };
        let mut select = None;
        let mut drop: Option<(Payload, SourceRef, Place)> = None;
        let mut screens: Vec<_> = app.screens.iter().collect();
        screens.sort_by_key(|s| s.name != app.start);
        for s in screens {
            egui::CollapsingHeader::new(RichText::new(&s.name).strong())
                .id_salt(("screen", &s.name))
                .default_open(true)
                .show(ui, |ui| {
                    self.layer(ui, &s.name, &s.root, &mut select, &mut drop);
                });
        }
        if let Some((screen, id)) = select {
            self.select_id(&screen, &id);
        }
        if let Some((payload, target, place)) = drop {
            match payload {
                Payload::Insert(kind) if place == Place::Inside => {
                    self.insert_into(&target, &kind);
                }
                Payload::Insert(kind) => {
                    let snip = edit::snippet(&kind);
                    let after = place == Place::After;
                    self.edit_at(&target, |t, p, s| {
                        edit::insert_next_to(t, p, s, &snip, after)
                    });
                }
                Payload::Move(from) => {
                    self.move_to(&from, &target, place);
                }
            }
        }
    }

    fn layer(
        &self,
        ui: &mut egui::Ui,
        screen: &str,
        e: &Element,
        select: &mut Option<(String, String)>,
        drop: &mut Option<(Payload, SourceRef, Place)>,
    ) {
        let name = match (&e.kind, &e.content) {
            (_, Some(Content::Repeat { var, list })) => format!("for {var} in {list}"),
            (_, _)
                if !e.origin.is_empty()
                    && e.id.ends_with(&format!(":{}", e.origin.last().unwrap())) =>
            {
                format!("{} (component)", e.origin.last().unwrap())
            }
            (_, Some(Content::Text(t))) => format!(
                "{} \"{}\"",
                e.kind.name(),
                t.chars().take(18).collect::<String>()
            ),
            (k, _) => k.name().to_string(),
        };
        let selected = self.selected.as_ref().is_some_and(|s| s.id == e.id);
        let editable = e.source.is_some();
        let container = matches!(
            e.kind,
            ElementKind::Column
                | ElementKind::Row
                | ElementKind::Stack
                | ElementKind::Grid
                | ElementKind::Card
                | ElementKind::Scroll
        );
        let mut row = |ui: &mut egui::Ui| {
            let r = ui.add(
                egui::Button::selectable(selected, name.clone()).sense(Sense::click_and_drag()),
            );
            if r.clicked() {
                *select = Some((screen.to_string(), e.id.clone()));
            }
            if editable && let Some(src) = &e.source {
                r.dnd_set_drag_payload(Payload::Move(src.clone()));
                if let Some(p) = r.dnd_hover_payload::<Payload>() {
                    let _ = p;
                    ui.painter().rect_stroke(
                        r.rect,
                        2.0,
                        Stroke::new(1.5, Color32::from_rgb(60, 140, 255)),
                        StrokeKind::Outside,
                    );
                }
                if let Some(p) = r.dnd_release_payload::<Payload>() {
                    let y = ui
                        .input(|i| i.pointer.interact_pos())
                        .map_or(r.rect.center().y, |p| p.y);
                    let place =
                        if container && (y - r.rect.center().y).abs() < r.rect.height() * 0.25 {
                            Place::Inside
                        } else if y < r.rect.center().y {
                            Place::Before
                        } else {
                            Place::After
                        };
                    *drop = Some(((*p).clone(), src.clone(), place));
                }
            }
        };
        let children: Vec<&Element> = e.children.iter().collect();
        if children.is_empty() {
            row(ui);
        } else {
            let id = ui.make_persistent_id(("layer", &e.id));
            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
                .show_header(ui, row)
                .body(|ui| {
                    for c in children {
                        self.layer(ui, screen, c, select, drop);
                    }
                });
        }
    }

    fn right(&mut self, root: &mut egui::Ui) {
        egui::Panel::right("properties")
            .resizable(true)
            .default_size(300.0)
            .show(root, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.properties(ui));
            });
    }

    /// Fields shown in the properties panel: (section, property, hint).
    const FIELDS: &'static [(&'static str, &'static str, &'static str)] = &[
        ("Layout", "width", "100%, 120px, fill, fit"),
        ("Layout", "height", "48px, fill"),
        ("Layout", "padding", "16px or 8px 16px"),
        ("Layout", "margin", "0 auto"),
        ("Layout", "gap", "12px"),
        ("Layout", "align-items", "start center end stretch"),
        (
            "Layout",
            "justify-content",
            "start center end space-between",
        ),
        ("Layout", "flex", "1"),
        (
            "Style",
            "background",
            "#FFFFFF, primary, linear-gradient(…)",
        ),
        ("Style", "border-radius", "12px or 50%"),
        ("Style", "border", "1px solid #E5E5E5"),
        ("Style", "box-shadow", "0 8px 24px rgba(0,0,0,.15)"),
        ("Style", "opacity", "0.5"),
        ("Text", "color", "#171717 or a skin colour"),
        ("Text", "font-size", "16px"),
        ("Text", "font-weight", "400, 600, bold"),
        ("Text", "text-align", "start center end"),
        ("Interaction", "on-press", "go Screen, back, Store.action"),
        ("Interaction", "visible", "true, Store.flag"),
    ];

    fn properties(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Properties").strong());
        let Some(sel) = self.selected.clone() else {
            ui.label(RichText::new("Select an element on the canvas or in Layers.").weak());
            return;
        };
        let Some(src) = sel.source.clone() else {
            ui.label("This element is generated and has no source to edit.");
            return;
        };
        let text = self.file_text(&src.path).unwrap_or_default();
        let key = format!("{}:{}", src.path, src.start);
        if self.fields_for.as_deref() != Some(key.as_str()) {
            self.fields.clear();
            for (_, name, _) in Self::FIELDS {
                let v = edit::property(&text, &src.path, src.start, name).unwrap_or_default();
                self.fields.insert(name.to_string(), v);
            }
            let label = match edit::argument(&text, &src.path, src.start) {
                Some(era_syntax::Value {
                    kind: era_syntax::ValueKind::String(s),
                    ..
                }) => s,
                _ => String::new(),
            };
            self.fields.insert("__label".into(), label);
            self.fields_for = Some(key);
        }
        ui.horizontal(|ui| {
            let title = sel
                .component
                .clone()
                .unwrap_or_else(|| sel.kind.name().to_string());
            ui.label(RichText::new(title).monospace().strong());
            let line = text[..src.start.min(text.len())].matches('\n').count() + 1;
            ui.label(RichText::new(format!("{}:{line}", src.path)).small().weak());
        });
        if sel.from_component {
            ui.label(
                RichText::new("Part of a component: changes apply to every use.")
                    .small()
                    .color(Color32::from_rgb(200, 150, 40)),
            );
        }
        if ui
            .button("Delete element")
            .on_hover_text("Delete / Backspace")
            .clicked()
        {
            self.delete_selected();
            return;
        }
        ui.separator();
        let mut commit: Vec<(String, String)> = Vec::new();
        if matches!(sel.kind, ElementKind::Text | ElementKind::Button) {
            let l = ui.label(RichText::new("Content").strong());
            let v = self.fields.get_mut("__label").unwrap();
            let r = ui
                .add(egui::TextEdit::singleline(v).desired_width(f32::INFINITY))
                .labelled_by(l.id);
            if r.lost_focus() && r.changed()
                || r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter))
            {
                commit.push(("__label".into(), v.clone()));
            }
            ui.separator();
        }
        let mut section = "";
        let fields: &[(&str, &str, &str)] = if sel.component.is_some() {
            &[]
        } else {
            Self::FIELDS
        };
        if sel.component.is_some() {
            ui.label(
                RichText::new(
                    "A component: set its props below. Its look is edited in the component itself.",
                )
                .small()
                .weak(),
            );
        }
        for (sec, name, hint) in fields {
            let text_only = *sec == "Text"
                && !matches!(
                    sel.kind,
                    ElementKind::Text | ElementKind::Button | ElementKind::Input
                );
            if text_only {
                continue;
            }
            if *sec != section {
                section = sec;
                ui.add_space(4.0);
                ui.label(RichText::new(*sec).strong());
            }
            ui.horizontal(|ui| {
                let l = ui.add_sized(
                    [110.0, 18.0],
                    egui::Label::new(RichText::new(*name).monospace().small()),
                );
                let v = self.fields.get_mut(*name).unwrap();
                if matches!(*name, "background" | "color")
                    && let Some(mut c) = parse_hex(v).or(Some(Color32::TRANSPARENT))
                    && egui::color_picker::color_edit_button_srgba(
                        ui,
                        &mut c,
                        egui::color_picker::Alpha::OnlyBlend,
                    )
                    .changed()
                {
                    *v = hex(c);
                    commit.push((name.to_string(), v.clone()));
                }
                let r = ui
                    .add(
                        egui::TextEdit::singleline(v)
                            .hint_text(*hint)
                            .desired_width(f32::INFINITY),
                    )
                    .labelled_by(l.id);
                if r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter))
                    || (r.lost_focus() && r.changed())
                {
                    commit.push((name.to_string(), v.clone()));
                }
            });
        }
        ui.add_space(6.0);
        ui.separator();
        ui.label(RichText::new("All properties in code").strong());
        for (n, v) in edit::properties(&text, &src.path, src.start) {
            let slot = format!("__code:{n}");
            let buf = self.fields.entry(slot.clone()).or_insert(v);
            ui.horizontal(|ui| {
                let l = ui.add_sized(
                    [110.0, 18.0],
                    egui::Label::new(RichText::new(&n).monospace().small()),
                );
                let r = ui
                    .add(egui::TextEdit::singleline(buf).desired_width(f32::INFINITY))
                    .labelled_by(l.id);
                if r.lost_focus() && (r.changed() || ui.input(|i| i.key_pressed(Key::Enter))) {
                    commit.push((n.clone(), buf.clone()));
                }
            });
        }
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.new_prop.0)
                    .hint_text("property")
                    .desired_width(100.0),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.new_prop.1)
                    .hint_text("value")
                    .desired_width(100.0),
            );
            if ui.button("Set").clicked() && !self.new_prop.0.trim().is_empty() {
                commit.push((self.new_prop.0.trim().to_string(), self.new_prop.1.clone()));
                self.new_prop = (String::new(), String::new());
            }
        });
        for (name, value) in commit {
            if name == "__label" {
                self.set_label(&value);
            } else {
                self.set_prop(&name, &value);
            }
        }
    }

    fn bottom(&mut self, root: &mut egui::Ui) {
        egui::Panel::bottom("bottom")
            .resizable(true)
            .default_size(150.0)
            .show(root, |ui| {
                egui::ScrollArea::vertical()
                    .stick_to_bottom(self.job.is_some())
                    .show(ui, |ui| {
                        let problems: Vec<_> = self
                            .ws
                            .as_ref()
                            .map(|w| w.problems().to_vec())
                            .unwrap_or_default();
                        if problems.is_empty() {
                            ui.label(RichText::new("No problems").weak());
                        }
                        for p in problems {
                            let at = match (&p.path, p.line) {
                                (Some(f), Some((l, c))) => format!("{f}:{l}:{c} "),
                                (Some(f), None) => format!("{f} "),
                                _ => String::new(),
                            };
                            ui.label(
                                RichText::new(format!("{at}[{}] {}", p.code, p.message))
                                    .monospace()
                                    .small(),
                            );
                        }
                        if let Some(sel) = &self.selected
                            && let Some(src) = &sel.source
                            && let Some(ws) = &self.ws
                            && let Some(b) = ws.buffers.get(&src.path)
                            && src.end <= b.text.len()
                        {
                            ui.separator();
                            ui.label(RichText::new(format!("Code: {}", src.path)).strong());
                            ui.label(
                                RichText::new(&b.text[src.start..src.end])
                                    .monospace()
                                    .small(),
                            );
                        }
                        for l in &self.output {
                            ui.label(RichText::new(l).monospace().small());
                        }
                    });
            });
    }

    fn canvas(&mut self, root: &mut egui::Ui) {
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(Color32::from_rgb(38, 40, 44)))
            .show(root, |ui| {
                let (resp, painter) =
                    ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
                let ctx = ui.ctx().clone();
                // Zoom (pinch / Cmd+scroll) around the pointer; scroll pans.
                if resp.hovered() {
                    let (zoom, scroll) = ctx.input(|i| (i.zoom_delta(), i.smooth_scroll_delta));
                    if zoom != 1.0 {
                        let old = self.zoom;
                        self.zoom = (self.zoom * zoom).clamp(0.25, 2.0);
                        if let Some(p) = ctx.input(|i| i.pointer.hover_pos()) {
                            let anchor = p - resp.rect.min - self.pan;
                            self.pan -= anchor * (self.zoom / old - 1.0);
                        }
                    } else {
                        self.pan += scroll;
                    }
                }
                let z = self.zoom;
                let gap = 80.0;
                let (min, pan) = (resp.rect.min, self.pan);
                let origin = move |i: usize, f: &Frame| {
                    min + pan + Vec2::new(i as f32 * (f.width + gap) * z, 0.0)
                };
                let to_screen = |o: Pos2, r: &era_canvas::Rect| {
                    Rect::from_min_size(o + Vec2::new(r.x, r.y) * z, Vec2::new(r.w, r.h) * z)
                };
                let pointer = ctx.input(|i| i.pointer.interact_pos().or(i.pointer.hover_pos()));
                // Which item is under the pointer?
                let mut under: Option<(usize, Item)> = None;
                let mut drop_target: Option<Item> = None;
                if let Some(p) = pointer {
                    for (i, f) in self.frames.iter().enumerate() {
                        let o = origin(i, f);
                        let local = (p - o) / z;
                        if let Some(it) = f.hit(local.x, local.y) {
                            under = Some((i, it.clone()));
                        }
                        if let Some(c) = f.container_at(local.x, local.y) {
                            drop_target = Some(c.clone());
                        }
                    }
                }
                let frames = self.frames.clone();
                for (i, f) in frames.iter().enumerate() {
                    let o = origin(i, f);
                    let screen_rect = Rect::from_min_size(o, Vec2::new(f.width, f.height) * z);
                    painter.text(
                        o - Vec2::new(0.0, 8.0),
                        egui::Align2::LEFT_BOTTOM,
                        &f.screen,
                        FontId::proportional(13.0),
                        Color32::from_gray(200),
                    );
                    painter.rect_filled(screen_rect, CornerRadius::same(6), c32(f.background));
                    let clipped = painter.with_clip_rect(screen_rect);
                    for it in &f.items {
                        self.paint_item(&ctx, &clipped, to_screen(o, &it.rect), it, z);
                    }
                    // Hover and selection outlines.
                    for it in &f.items {
                        let r = to_screen(o, &it.rect);
                        let base = base_id(&it.id);
                        if self
                            .selected
                            .as_ref()
                            .is_some_and(|s| s.id == base && s.screen == f.screen)
                        {
                            let exact = self.hover.as_deref() != Some(&it.id);
                            painter.rect_stroke(
                                r,
                                0.0,
                                Stroke::new(
                                    if exact { 2.0 } else { 1.0 },
                                    Color32::from_rgb(60, 140, 255),
                                ),
                                StrokeKind::Outside,
                            );
                        } else if self.hover.as_deref() == Some(it.id.as_str()) {
                            painter.rect_stroke(
                                r,
                                0.0,
                                Stroke::new(1.0, Color32::from_rgb(120, 180, 255)),
                                StrokeKind::Outside,
                            );
                        }
                    }
                }
                self.hover = under.as_ref().map(|(_, it)| it.id.clone());

                // Drag a selected element to move it.
                if resp.drag_started()
                    && let Some((_, it)) = &under
                    && let Some(src) = &it.source
                    && self
                        .selected
                        .as_ref()
                        .is_some_and(|s| s.id == base_id(&it.id))
                {
                    egui::DragAndDrop::set_payload(&ctx, Payload::Move(src.clone()));
                }
                if resp.drag_stopped() || resp.dragged() {
                    if egui::DragAndDrop::has_any_payload(&ctx) {
                        // handled by the drop below
                    } else {
                        self.pan += resp.drag_delta();
                    }
                }
                // Drop target highlight.
                if egui::DragAndDrop::has_payload_of_type::<Payload>(&ctx)
                    && let Some(t) = &drop_target
                    && let Some((i, _)) = &under
                {
                    let r = to_screen(origin(*i, &frames[*i]), &t.rect);
                    painter.rect_stroke(
                        r,
                        2.0,
                        Stroke::new(2.0, Color32::from_rgb(70, 200, 120)),
                        StrokeKind::Inside,
                    );
                }
                if let Some(p) = resp.dnd_release_payload::<Payload>()
                    && let Some(target) = drop_target.and_then(|t| t.source)
                {
                    match &*p {
                        Payload::Insert(kind) => {
                            self.insert_into(&target, kind);
                        }
                        Payload::Move(from) if from != &target => {
                            self.move_to(from, &target, Place::Inside);
                        }
                        Payload::Move(_) => {}
                    }
                }
                if resp.clicked() {
                    match &under {
                        Some((i, it)) => {
                            let screen = frames[*i].screen.clone();
                            self.select_id(&screen, &it.id);
                        }
                        None => self.selected = None,
                    }
                }
                if self.frames.is_empty() {
                    painter.text(
                        resp.rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "Fix the problems below to see the canvas",
                        FontId::proportional(16.0),
                        Color32::from_gray(200),
                    );
                }
            });
    }

    fn paint_item(
        &mut self,
        ctx: &egui::Context,
        painter: &egui::Painter,
        r: Rect,
        it: &Item,
        z: f32,
    ) {
        if it.opacity <= 0.01 {
            return;
        }
        let fade = |c: Color32| c.gamma_multiply(it.opacity);
        let cr = |v: f32| {
            (v * z)
                .clamp(0.0, 255.0)
                .min(r.width().min(r.height()) / 2.0) as u8
        };
        let radius = CornerRadius {
            nw: cr(it.radius[0]),
            ne: cr(it.radius[1]),
            se: cr(it.radius[2]),
            sw: cr(it.radius[3]),
        };
        if let Some(s) = &it.shadow {
            let shadow = epaint::Shadow {
                offset: [(s.x * z) as i8, (s.y * z) as i8],
                blur: (s.blur * z).min(255.0) as u8,
                spread: (s.spread * z).clamp(0.0, 255.0) as u8,
                color: fade(c32(s.color)),
            };
            painter.add(shadow.as_shape(r, radius));
        }
        match &it.fill {
            Fill::Solid(c) => {
                painter.rect_filled(r, radius, fade(c32(*c)));
            }
            Fill::Linear(angle, stops) if !stops.is_empty() => {
                // Corner colours from the gradient line (an editor approximation).
                let (s, c) = angle.to_radians().sin_cos();
                let dir = Vec2::new(s, -c);
                let half = (r.width() * s.abs() + r.height() * c.abs()) / 2.0;
                let color_at = |p: Pos2| {
                    let t = (((p - r.center()).dot(dir) / half.max(0.01)) + 1.0) / 2.0;
                    sample(stops, t)
                };
                let mut mesh = egui::Mesh::default();
                for p in [
                    r.left_top(),
                    r.right_top(),
                    r.right_bottom(),
                    r.left_bottom(),
                ] {
                    mesh.colored_vertex(p, fade(c32(color_at(p))));
                }
                mesh.add_triangle(0, 1, 2);
                mesh.add_triangle(0, 2, 3);
                painter.add(egui::Shape::mesh(mesh));
            }
            Fill::Radial(stops) | Fill::Linear(_, stops) if !stops.is_empty() => {
                painter.rect_filled(r, radius, fade(c32(sample(stops, 0.5))));
            }
            _ => {}
        }
        if let Some(path) = &it.image {
            match self.texture(ctx, path) {
                Some(t) => {
                    painter.image(
                        t.id(),
                        r,
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        fade(Color32::WHITE),
                    );
                }
                None => {
                    painter.rect_filled(r, radius, Color32::from_gray(200));
                }
            }
        }
        if let Some((_, tint)) = &it.icon {
            painter.rect_filled(
                r.shrink(r.width() * 0.15),
                CornerRadius::same(3),
                fade(c32(*tint)).gamma_multiply(0.6),
            );
        }
        if let Some((w, c)) = it.border {
            painter.rect_stroke(
                r,
                radius,
                Stroke::new(w * z, fade(c32(c))),
                StrokeKind::Inside,
            );
        }
        if let Some(t) = &it.text {
            let pad = 0.0;
            let g = painter.layout(
                t.text.clone(),
                FontId::proportional((t.size * z).max(1.0)),
                fade(c32(t.color)),
                (r.width() - pad).max(1.0),
            );
            let x = match (it.kind, t.align) {
                (ElementKind::Button, _) | (_, 1) => r.center().x - g.size().x / 2.0,
                (_, 2) => r.right() - g.size().x,
                _ => r.left(),
            };
            let y = match it.kind {
                ElementKind::Button | ElementKind::Input => r.center().y - g.size().y / 2.0,
                _ => r.top(),
            };
            painter.galley(Pos2::new(x, y), g, Color32::WHITE);
        }
    }
}

/// Colour at `t` (0-1) along gradient stops.
fn sample(stops: &[(f32, [u8; 4])], t: f32) -> [u8; 4] {
    let t = t.clamp(0.0, 1.0);
    let mut prev = stops[0];
    for s in stops {
        if t <= s.0 {
            let span = (s.0 - prev.0).max(1e-6);
            let k = ((t - prev.0) / span).clamp(0.0, 1.0);
            let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * k).round() as u8;
            return [
                mix(prev.1[0], s.1[0]),
                mix(prev.1[1], s.1[1]),
                mix(prev.1[2], s.1[2]),
                mix(prev.1[3], s.1[3]),
            ];
        }
        prev = *s;
    }
    prev.1
}
