//! The ERA Studio window.
//!
//! ```text
//! ┌ toolbar: Open · New · Save · Render · Build APK ─────────── status ┐
//! │ project │ tabs                                    │ Preview | Skin │
//! │ tree    │ editor (ERA colouring, line numbers)    │ screen picker  │
//! │         │                                          │ rendered PNG   │
//! ├─────────┴──────────────────────────────────────────┴────────────────┤
//! │ Problems (click to jump) | Output (preview/build log)                │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{
    self, Color32, FontId, Key, KeyboardShortcut, Modifiers, RichText, TextureHandle,
    text::LayoutJob,
};
use era_diagnostics::Severity;

use crate::highlight::{self, Class};
use crate::jobs::{self, Job, JobEvent, JobKind};
use crate::workspace::{Problem, Workspace};

const SAVE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::S);
const RENDER: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::R);
/// Re-check this long after the last keystroke.
const ANALYZE_DELAY: Duration = Duration::from_millis(250);

#[derive(PartialEq, Eq, Clone, Copy)]
enum RightTab {
    Preview,
    Skin,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum BottomTab {
    Problems,
    Output,
}

pub struct Studio {
    ws: Option<Workspace>,
    open_error: Option<String>,
    open_path: String,
    new_name: String,
    /// Open editor tabs, in order; `current` indexes into it.
    tabs: Vec<String>,
    current: Option<usize>,
    /// A cursor jump to apply on the next frame: (file, char index).
    jump: Option<(String, usize)>,
    edited_at: Option<Instant>,
    right: RightTab,
    bottom: BottomTab,
    job: Option<Job>,
    output: Vec<String>,
    status: String,
    render_on_save: bool,
    preview_screen: Option<String>,
    preview_dark: bool,
    textures: BTreeMap<String, (TextureHandle, std::time::SystemTime)>,
}

impl Studio {
    pub fn new(ctx: &egui::Context, path: Option<PathBuf>) -> Self {
        ctx.set_visuals(egui::Visuals::dark());
        let mut s = Studio {
            ws: None,
            open_error: None,
            open_path: String::new(),
            new_name: "my_app".into(),
            tabs: Vec::new(),
            current: None,
            jump: None,
            edited_at: None,
            right: RightTab::Preview,
            bottom: BottomTab::Problems,
            job: None,
            output: Vec::new(),
            status: "Ready".into(),
            render_on_save: true,
            preview_screen: None,
            preview_dark: false,
            textures: BTreeMap::new(),
        };
        if let Some(p) = path {
            s.open_project(&p);
        }
        s
    }

    /// The open project, for tests.
    pub fn workspace(&self) -> Option<&Workspace> {
        self.ws.as_ref()
    }

    /// Edit the open project directly (tests, scripting). Call
    /// [`Studio::mark_edited`] afterwards so problems are re-checked.
    pub fn workspace_mut(&mut self) -> Option<&mut Workspace> {
        self.ws.as_mut()
    }

    /// Re-check on the next frame, as if the user had just typed.
    pub fn mark_edited(&mut self) {
        self.edited_at = Some(Instant::now() - ANALYZE_DELAY);
    }

    /// The file in the active editor tab.
    pub fn active_file(&self) -> Option<String> {
        self.current_file()
    }

    /// The screen shown in the preview pane.
    pub fn preview_screen(&self) -> Option<String> {
        self.preview_screen.clone()
    }

    /// The pending cursor jump (file, char index), for tests.
    pub fn pending_jump(&self) -> Option<&(String, usize)> {
        self.jump.as_ref()
    }

    pub fn open_project(&mut self, path: &Path) {
        match Workspace::open(path) {
            Ok(ws) => {
                self.status = format!("Opened {}", ws.root.display());
                self.open_path = ws.root.display().to_string();
                self.tabs.clear();
                self.current = None;
                self.textures.clear();
                self.preview_screen = ws.app().map(|a| a.start.clone());
                self.ws = Some(ws);
                self.open_error = None;
                self.open_tab("src/app.era");
            }
            Err(e) => self.open_error = Some(e),
        }
    }

    /// Open a file in a tab (or focus it).
    pub fn open_tab(&mut self, rel: &str) {
        let Some(ws) = &mut self.ws else { return };
        if !Workspace::is_text(rel) {
            self.status = format!("{rel} is not a text file");
            return;
        }
        if let Err(e) = ws.open_file(rel) {
            self.status = format!("Cannot open {rel}: {e}");
            return;
        }
        let idx = match self.tabs.iter().position(|t| t == rel) {
            Some(i) => i,
            None => {
                self.tabs.push(rel.to_string());
                self.tabs.len() - 1
            }
        };
        self.current = Some(idx);
        // The preview follows the screen you are editing (`Screens/Home.era` -> Home).
        if let Some(stem) = rel.strip_suffix(".era").and_then(|r| r.rsplit('/').next())
            && let Some(app) = self.ws.as_ref().and_then(|w| w.app())
            && app.screens.iter().any(|s| s.name == stem)
        {
            self.preview_screen = Some(stem.to_string());
        }
    }

    fn current_file(&self) -> Option<String> {
        self.current.and_then(|i| self.tabs.get(i).cloned())
    }

    fn save_all(&mut self) {
        let Some(ws) = &mut self.ws else { return };
        match ws.save_all() {
            Ok(n) => {
                self.status = if n == 0 {
                    "Nothing to save".into()
                } else {
                    format!("Saved {n} file(s)")
                };
                ws.reload();
                if n > 0 && self.render_on_save && ws.error_count() == 0 {
                    self.start_job(JobKind::Preview);
                }
            }
            Err(e) => self.status = format!("Save failed: {e}"),
        }
    }

    fn start_job(&mut self, kind: JobKind) {
        if self.job.is_some() {
            return;
        }
        let Some(ws) = &mut self.ws else { return };
        if ws.dirty_count() > 0 && ws.save_all().is_err() {
            self.status = "Save failed".into();
            return;
        }
        self.output.clear();
        self.output
            .push(format!("$ era {}", jobs::args(kind, &ws.root).join(" ")));
        self.job = Some(jobs::start(kind, &ws.root));
        self.status = format!("{}...", kind.label());
    }

    fn poll_job(&mut self, ctx: &egui::Context) {
        let Some(job) = &self.job else { return };
        let mut finished = None;
        while let Ok(ev) = job.events.try_recv() {
            match ev {
                JobEvent::Line(l) => self.output.push(l),
                JobEvent::Finished { success } => finished = Some(success),
            }
        }
        match finished {
            Some(success) => {
                let kind = job.kind;
                let secs = job.started.elapsed().as_secs_f32();
                self.job = None;
                self.status = format!(
                    "{} {} in {secs:.1}s",
                    kind.label(),
                    if success { "finished" } else { "failed" }
                );
                if !success {
                    self.bottom = BottomTab::Output;
                }
                if kind == JobKind::Preview {
                    self.textures.clear();
                }
            }
            None => ctx.request_repaint_after(Duration::from_millis(100)),
        }
    }

    /// The preview texture for a screen, loaded from its PNG (cached until
    /// the file changes).
    fn texture(&mut self, ctx: &egui::Context, name: &str) -> Option<TextureHandle> {
        let ws = self.ws.as_ref()?;
        let path = ws.preview_dir().join(format!("{name}.png"));
        let modified = std::fs::metadata(&path).and_then(|m| m.modified()).ok()?;
        if let Some((tex, at)) = self.textures.get(name)
            && *at == modified
        {
            return Some(tex.clone());
        }
        let img = image::open(&path).ok()?.to_rgba8();
        let size = [img.width() as usize, img.height() as usize];
        let color = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
        let tex = ctx.load_texture(
            format!("preview-{name}"),
            color,
            egui::TextureOptions::LINEAR,
        );
        self.textures
            .insert(name.to_string(), (tex.clone(), modified));
        Some(tex)
    }
}

impl eframe::App for Studio {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = &root.ctx().clone();
        if ctx.input_mut(|i| i.consume_shortcut(&SAVE)) {
            self.save_all();
        }
        if ctx.input_mut(|i| i.consume_shortcut(&RENDER)) {
            self.start_job(JobKind::Preview);
        }
        self.poll_job(ctx);
        if let Some(t) = self.edited_at {
            if t.elapsed() >= ANALYZE_DELAY {
                if let Some(ws) = &mut self.ws {
                    ws.analyze();
                }
                self.edited_at = None;
            } else {
                ctx.request_repaint_after(ANALYZE_DELAY);
            }
        }

        self.toolbar(root);
        if self.ws.is_none() {
            self.welcome(root);
            return;
        }
        self.bottom_panel(root);
        self.project_tree(root);
        self.right_panel(root);
        self.editor(root);
    }
}

// ----- panels ---------------------------------------------------------------

impl Studio {
    fn toolbar(&mut self, root: &mut egui::Ui) {
        egui::Panel::top("toolbar").show(root, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("ERA Studio").strong());
                ui.separator();
                if ui
                    .button("Open…")
                    .on_hover_text("Open an ERA project folder")
                    .clicked()
                    && let Some(dir) = rfd::FileDialog::new()
                        .set_title("Open ERA project")
                        .pick_folder()
                {
                    self.open_project(&dir);
                }
                let has_ws = self.ws.is_some();
                let dirty = self.ws.as_ref().map_or(0, |w| w.dirty_count());
                if ui
                    .add_enabled(
                        has_ws && dirty > 0,
                        egui::Button::new(format!("Save ({dirty})")),
                    )
                    .on_hover_text("Save all files (Cmd+S)")
                    .clicked()
                {
                    self.save_all();
                }
                let busy = self.job.is_some();
                let errors = self.ws.as_ref().map_or(0, |w| w.error_count());
                if ui
                    .add_enabled(has_ws && !busy && errors == 0, egui::Button::new("Render"))
                    .on_hover_text("Render every screen with the real Compose code (Cmd+R)")
                    .clicked()
                {
                    self.start_job(JobKind::Preview);
                }
                if ui
                    .add_enabled(
                        has_ws && !busy && errors == 0,
                        egui::Button::new("Build APK"),
                    )
                    .on_hover_text("era build android (needs ANDROID_HOME)")
                    .clicked()
                {
                    self.start_job(JobKind::BuildAndroid);
                }
                ui.checkbox(&mut self.render_on_save, "Render on save");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.job.is_some() {
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
                ui.add_space(120.0);
                ui.heading(RichText::new("ERA Studio").size(36.0));
                ui.label("Design native apps in ERA: edit, check and preview in one place.");
                ui.add_space(24.0);
                if ui
                    .button(RichText::new("Open project…").size(18.0))
                    .clicked()
                    && let Some(dir) = rfd::FileDialog::new()
                        .set_title("Open ERA project")
                        .pick_folder()
                {
                    self.open_project(&dir);
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let label = ui.label("or a path:");
                    ui.text_edit_singleline(&mut self.open_path)
                        .labelled_by(label.id);
                    if ui.button("Open").clicked() {
                        let p = PathBuf::from(self.open_path.trim());
                        self.open_project(&p);
                    }
                });
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    let label = ui.label("New project named");
                    ui.text_edit_singleline(&mut self.new_name)
                        .labelled_by(label.id);
                    if ui.button("Create in…").clicked()
                        && let Some(parent) = rfd::FileDialog::new()
                            .set_title("Choose a folder for the new project")
                            .pick_folder()
                    {
                        let dir = parent.join(self.new_name.trim());
                        match era_project::scaffold::create_project(&dir) {
                            Ok(c) => self.open_project(&c.root),
                            Err(d) => self.open_error = Some(d.message),
                        }
                    }
                });
                if let Some(e) = &self.open_error {
                    ui.add_space(12.0);
                    ui.colored_label(Color32::from_rgb(244, 71, 71), e);
                }
            });
        });
    }

    fn project_tree(&mut self, root: &mut egui::Ui) {
        let mut open = None;
        egui::Panel::left("tree")
            .resizable(true)
            .default_size(230.0)
            .show(root, |ui| {
                let Some(ws) = &self.ws else { return };
                ui.add_space(4.0);
                ui.label(RichText::new(ws.name()).strong());
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let current = self.current_file();
                    for group in ws.tree() {
                        egui::CollapsingHeader::new(group.name)
                            .default_open(true)
                            .show(ui, |ui| {
                                for rel in &group.files {
                                    let name = rel.rsplit('/').next().unwrap_or(rel);
                                    let dirty = ws.buffers.get(rel).is_some_and(|b| b.is_dirty());
                                    let errors = ws
                                        .problems()
                                        .iter()
                                        .filter(|p| {
                                            p.severity == Severity::Error
                                                && p.path.as_deref() == Some(rel)
                                        })
                                        .count();
                                    let mut text = RichText::new(format!(
                                        "{}{name}",
                                        if dirty { "* " } else { "" }
                                    ));
                                    if errors > 0 {
                                        text = text.color(Color32::from_rgb(244, 71, 71));
                                    } else if !Workspace::is_text(rel) {
                                        text = text.weak();
                                    }
                                    let r = ui.selectable_label(
                                        current.as_deref() == Some(rel.as_str()),
                                        text,
                                    );
                                    if r.clicked() {
                                        open = Some(rel.clone());
                                    }
                                    r.on_hover_text(rel);
                                }
                            });
                    }
                });
            });
        if let Some(rel) = open {
            self.open_tab(&rel);
        }
    }

    fn bottom_panel(&mut self, root: &mut egui::Ui) {
        let mut jump: Option<Problem> = None;
        egui::Panel::bottom("bottom")
            .resizable(true)
            .default_size(170.0)
            .show(root, |ui| {
                let Some(ws) = &self.ws else { return };
                let problems = ws.problems();
                let errors = ws.error_count();
                let warnings = problems.len() - errors;
                ui.horizontal(|ui| {
                    let label = format!("Problems ({errors} errors, {warnings} warnings)");
                    ui.selectable_value(&mut self.bottom, BottomTab::Problems, label);
                    ui.selectable_value(&mut self.bottom, BottomTab::Output, "Output");
                });
                ui.separator();
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .stick_to_bottom(self.bottom == BottomTab::Output)
                    .show(ui, |ui| match self.bottom {
                        BottomTab::Problems if problems.is_empty() => {
                            ui.label(
                                RichText::new("No problems. The project checks cleanly.")
                                    .color(Color32::from_rgb(106, 190, 120)),
                            );
                        }
                        BottomTab::Problems => {
                            for p in problems {
                                let (icon, color) = match p.severity {
                                    Severity::Error => ("error", Color32::from_rgb(244, 71, 71)),
                                    Severity::Warning => {
                                        ("warning", Color32::from_rgb(230, 180, 60))
                                    }
                                };
                                let place = match (&p.path, p.line) {
                                    (Some(path), Some((l, c))) => format!("{path}:{l}:{c}"),
                                    (Some(path), None) => path.clone(),
                                    _ => String::new(),
                                };
                                let r = ui.horizontal(|ui| {
                                    ui.label(RichText::new(icon).color(color).strong());
                                    ui.label(RichText::new(p.code).monospace().weak());
                                    let r = ui.link(&p.message);
                                    ui.label(RichText::new(place).weak());
                                    if let Some(h) = &p.help {
                                        ui.label(RichText::new(format!("- {h}")).italics().weak());
                                    }
                                    r
                                });
                                if r.inner.clicked() {
                                    jump = Some(p.clone());
                                }
                            }
                        }
                        BottomTab::Output => {
                            for line in &self.output {
                                ui.label(RichText::new(line).monospace());
                            }
                        }
                    });
            });
        if let Some(p) = jump
            && let Some(path) = p.path
        {
            self.open_tab(&path);
            if let Some(ci) = p.char_index {
                self.jump = Some((path, ci));
            }
        }
    }

    fn right_panel(&mut self, root: &mut egui::Ui) {
        egui::Panel::right("right")
            .resizable(true)
            .default_size(420.0)
            .show(root, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.right, RightTab::Preview, "Preview");
                    ui.selectable_value(&mut self.right, RightTab::Skin, "Skin");
                });
                ui.separator();
                match self.right {
                    RightTab::Preview => {
                        let ctx = ui.ctx().clone();
                        self.preview(ui, &ctx)
                    }
                    RightTab::Skin => self.skin(ui),
                }
            });
    }

    fn preview(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let Some(ws) = &self.ws else { return };
        let Some(app) = ws.app() else {
            ui.label("Fix the problems below to preview.");
            return;
        };
        let screens: Vec<String> = app.screens.iter().map(|s| s.name.clone()).collect();
        let has_dark = app.skin.dark_colors.is_some();
        if self
            .preview_screen
            .as_ref()
            .is_none_or(|s| !screens.contains(s))
        {
            self.preview_screen = Some(app.start.clone());
        }
        ui.horizontal(|ui| {
            egui::ComboBox::from_label("Screen")
                .selected_text(self.preview_screen.clone().unwrap_or_default())
                .show_ui(ui, |ui| {
                    for s in &screens {
                        ui.selectable_value(&mut self.preview_screen, Some(s.clone()), s);
                    }
                });
            if has_dark {
                ui.checkbox(&mut self.preview_dark, "Dark");
            }
        });
        let name = format!(
            "{}{}",
            self.preview_screen.clone().unwrap_or_default(),
            if self.preview_dark && has_dark {
                "-dark"
            } else {
                ""
            }
        );
        match self.texture(ctx, &name) {
            Some(tex) => {
                let avail = ui.available_size();
                let [w, h] = tex.size().map(|v| v as f32);
                let scale = (avail.x / w).min((avail.y - 8.0) / h).min(1.0);
                ui.add_space(4.0);
                ui.vertical_centered(|ui| {
                    ui.add(
                        egui::Image::new(&tex)
                            .fit_to_exact_size(egui::vec2(w * scale, h * scale))
                            .corner_radius(18.0),
                    );
                });
            }
            None => {
                ui.add_space(20.0);
                ui.label("No render yet.");
                ui.label(RichText::new("Press Render (Cmd+R). The first render downloads Kotlin and Compose and takes a minute or two.").weak());
            }
        }
    }

    fn skin(&mut self, ui: &mut egui::Ui) {
        let Some(app) = self.ws.as_ref().and_then(|w| w.app()) else {
            ui.label("Fix the problems below to see the skin.");
            return;
        };
        let skin = &app.skin;
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.label(
                RichText::new(format!(
                    "Skin {}{}",
                    skin.name,
                    if skin.builtin { " (built-in)" } else { "" }
                ))
                .strong(),
            );
            ui.add_space(6.0);
            ui.label(RichText::new("Colors").weak());
            egui::Grid::new("colors")
                .num_columns(3)
                .spacing([10.0, 6.0])
                .show(ui, |ui| {
                    for (name, c) in &skin.colors {
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(28.0, 18.0), egui::Sense::hover());
                        ui.painter().rect_filled(
                            rect,
                            4.0,
                            Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a),
                        );
                        ui.label(RichText::new(name).monospace());
                        ui.label(
                            RichText::new(format!("#{:02X}{:02X}{:02X}", c.r, c.g, c.b))
                                .weak()
                                .monospace(),
                        );
                        ui.end_row();
                    }
                });
            for (title, table) in [
                ("Spacing", &skin.spacing),
                ("Radius", &skin.radius),
                ("Elevation", &skin.elevation),
            ] {
                ui.add_space(8.0);
                ui.label(RichText::new(title).weak());
                ui.horizontal_wrapped(|ui| {
                    for (k, v) in table {
                        ui.label(RichText::new(format!("{k} {v}")).monospace());
                    }
                });
            }
            ui.add_space(8.0);
            ui.label(RichText::new(format!("Text styles, font {}", skin.typography.font)).weak());
            for (name, st) in &skin.typography.styles {
                ui.label(
                    RichText::new(format!("{name}: {} sp, weight {}", st.size, st.weight))
                        .size((st.size as f32).min(36.0)),
                );
            }
        });
    }

    fn editor(&mut self, root: &mut egui::Ui) {
        egui::CentralPanel::default().show(root, |ui| {
            // Tabs.
            let mut close = None;
            ui.horizontal_wrapped(|ui| {
                for (i, rel) in self.tabs.iter().enumerate() {
                    let dirty = self
                        .ws
                        .as_ref()
                        .and_then(|w| w.buffers.get(rel))
                        .is_some_and(|b| b.is_dirty());
                    let name = rel.rsplit('/').next().unwrap_or(rel);
                    let label = format!("{}{name}", if dirty { "* " } else { "" });
                    if ui
                        .selectable_label(self.current == Some(i), label)
                        .on_hover_text(rel)
                        .clicked()
                    {
                        self.current = Some(i);
                    }
                    if ui.small_button("x").on_hover_text("Close").clicked() {
                        close = Some(i);
                    }
                    ui.add_space(6.0);
                }
            });
            if let Some(i) = close {
                let rel = self.tabs.remove(i);
                let dirty = self
                    .ws
                    .as_ref()
                    .and_then(|w| w.buffers.get(&rel))
                    .is_some_and(|b| b.is_dirty());
                if !dirty && let Some(ws) = &mut self.ws {
                    ws.close_file(&rel);
                }
                self.current = if self.tabs.is_empty() {
                    None
                } else {
                    Some(i.min(self.tabs.len() - 1))
                };
            }
            ui.separator();

            let Some(rel) = self.current_file() else {
                ui.label("Open a file from the project tree.");
                return;
            };
            let dark = ui.visuals().dark_mode;
            let jump = self.jump.take_if(|(f, _)| *f == rel);
            let Some(ws) = &mut self.ws else { return };
            let Some(buffer) = ws.buffers.get_mut(&rel) else {
                return;
            };
            let font = FontId::monospace(14.0);
            // Code does not wrap: long lines scroll sideways, so lines stay aligned
            // with their numbers.
            let mut layouter =
                |ui: &egui::Ui, text: &dyn egui::TextBuffer, _wrap: f32| -> Arc<egui::Galley> {
                    let mut job = layout_job(text.as_str(), font.clone(), dark);
                    job.wrap.max_width = f32::INFINITY;
                    ui.fonts_mut(|f| f.layout_job(job))
                };
            let mut changed = false;
            egui::ScrollArea::both()
                .id_salt(&rel)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        let lines = buffer.text.lines().count().max(1)
                            + usize::from(buffer.text.ends_with('\n'));
                        let numbers: String = (1..=lines).map(|n| format!("{n}\n")).collect();
                        ui.label(RichText::new(numbers).font(FontId::monospace(14.0)).weak());
                        let id = egui::Id::unique(("editor", &rel));
                        let output = egui::TextEdit::multiline(&mut buffer.text)
                            .id(id)
                            .code_editor()
                            .desired_width(f32::INFINITY)
                            .clip_text(false)
                            .desired_rows(30)
                            .lock_focus(true)
                            .layouter(&mut layouter)
                            .show(ui);
                        let response = output.response.clone();
                        response.widget_info(|| {
                            egui::WidgetInfo::labeled(
                                egui::Role::MultilineTextInput,
                                true,
                                format!("Editor: {rel}"),
                            )
                        });
                        changed = response.changed();
                        if let Some((_, ci)) = jump {
                            let mut state = output.state.clone();
                            let cursor = egui::text::CCursor::new(ci);
                            state
                                .cursor
                                .set_char_range(Some(egui::text::CCursorRange::one(cursor)));
                            state.store(ui.ctx(), id);
                            output.response.request_focus();
                            let pos = output.galley.pos_from_cursor(cursor);
                            ui.scroll_to_rect(
                                pos.translate(output.galley_pos.to_vec2()),
                                Some(egui::Align::Center),
                            );
                        }
                    });
                });
            if changed {
                self.edited_at = Some(Instant::now());
            }
        });
    }
}

/// Colour ERA source for the editor.
pub fn layout_job(text: &str, font: FontId, dark: bool) -> LayoutJob {
    let mut job = LayoutJob::default();
    for (range, class) in highlight::classify(text) {
        let [r, g, b] = highlight::color(class, dark);
        let mut format = egui::TextFormat::simple(font.clone(), Color32::from_rgb(r, g, b));
        match class {
            Class::Color(cr, cg, cb, _) => format.background = Color32::from_rgb(cr, cg, cb),
            Class::Error => {
                format.underline = egui::Stroke::new(1.5, Color32::from_rgb(244, 71, 71))
            }
            Class::Comment => format.italics = true,
            _ => {}
        }
        job.append(&text[range], 0.0, format);
    }
    job
}
