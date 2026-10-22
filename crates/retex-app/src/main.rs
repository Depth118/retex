#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui::{self, Align, Color32, FontFamily, FontId, Rect, RichText, Sense, Stroke, Vec2};
use egui_extras::{Column, TableBuilder};
use retex_core as tm;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Arc;
use std::time::Duration;

const ICON_PNG: &[u8] = include_bytes!("../../../assets/icon-256.png");

const BG: Color32 = Color32::from_rgb(0x0a, 0x0a, 0x0a);
const SURFACE: Color32 = Color32::from_rgb(0x12, 0x12, 0x12);
const HOVER: Color32 = Color32::from_rgb(0x1c, 0x1c, 0x1c);
const PRESSED: Color32 = Color32::from_rgb(0x26, 0x26, 0x26);
const ROW: Color32 = Color32::from_rgb(0x0f, 0x0f, 0x0f);
const BORDER: Color32 = Color32::from_rgb(0x26, 0x26, 0x26);
const BORDER_STRONG: Color32 = Color32::from_rgb(0x3a, 0x3a, 0x3a);
const TEXT: Color32 = Color32::from_rgb(0xf2, 0xf2, 0xf2);
const MUTED: Color32 = Color32::from_rgb(0x8e, 0x8e, 0x8e);
const DIM: Color32 = Color32::from_rgb(0x60, 0x60, 0x60);
const OK: Color32 = Color32::from_rgb(0x8f, 0xb5, 0x9b);
const ERR: Color32 = Color32::from_rgb(0xd4, 0x8c, 0x8c);
const WARN: Color32 = Color32::from_rgb(0xc9, 0xa9, 0x7b);
const CLOSE_HOVER: Color32 = Color32::from_rgb(0xc4, 0x2b, 0x1c);

const TITLE_BAR_HEIGHT: f32 = 38.0;
const WINDOW_BUTTON_WIDTH: f32 = 46.0;
const FOOTER_HEIGHT: f32 = 52.0;
const ACTIVITY_HEIGHT: f32 = 140.0;
const PACK_ROW_HEIGHT: f32 = 44.0;
const PREVIEW_WIDTH: f32 = 140.0;
/// Longest side of the decoded preview texture.
const PREVIEW_MAX_SIDE: u32 = 280;

fn main() -> eframe::Result<()> {
    let initial_pack = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .filter(|path| path.is_file());
    // The app draws its own title bar (see `RetexApp::title_bar`); the icon
    // only identifies the window in the taskbar and Alt+Tab.
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([540.0, 480.0])
            .with_resizable(false)
            .with_maximize_button(false)
            .with_decorations(false)
            .with_icon(std::sync::Arc::new(load_window_icon())),
        ..Default::default()
    };

    eframe::run_native(
        "Retex",
        options,
        Box::new(|cc| {
            let has_semibold = install_fonts(&cc.egui_ctx);
            configure_style(&cc.egui_ctx);
            Ok(Box::new(RetexApp::new(has_semibold, initial_pack.clone())))
        }),
    )
}

// ---------------------------------------------------------------------------
// Theme
// ---------------------------------------------------------------------------

fn install_fonts(ctx: &egui::Context) -> bool {
    let mut fonts = egui::FontDefinitions::default();
    let segoe = std::fs::read(r"C:\Windows\Fonts\segoeui.ttf").ok();
    let semibold = std::fs::read(r"C:\Windows\Fonts\seguisb.ttf").ok();

    if let Some(bytes) = segoe {
        fonts.font_data.insert(
            "segoe".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(bytes)),
        );
        if let Some(list) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            list.insert(0, "segoe".to_owned());
        }
    }

    let mut has_semibold = false;
    if let Some(bytes) = semibold {
        fonts.font_data.insert(
            "segoe_semibold".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(bytes)),
        );
        let mut family = vec!["segoe_semibold".to_owned()];
        if fonts.font_data.contains_key("segoe") {
            family.push("segoe".to_owned());
        }
        fonts
            .families
            .insert(egui::FontFamily::Name("semibold".into()), family);
        has_semibold = true;
    }

    ctx.set_fonts(fonts);
    has_semibold
}

fn configure_style(ctx: &egui::Context) {
    ctx.all_styles_mut(|style| {
        let mut v = egui::Visuals::dark();
        v.panel_fill = BG;
        v.window_fill = BG;
        v.extreme_bg_color = SURFACE;
        v.faint_bg_color = ROW;
        v.override_text_color = Some(TEXT);
        v.hyperlink_color = TEXT;
        v.selection.bg_fill = PRESSED;
        v.selection.stroke = Stroke::new(2.0, MUTED);

        v.widgets.noninteractive.bg_fill = BG;
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
        v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, MUTED);

        v.widgets.inactive.bg_fill = SURFACE;
        v.widgets.inactive.weak_bg_fill = SURFACE;
        v.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
        v.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT);

        v.widgets.hovered.bg_fill = HOVER;
        v.widgets.hovered.weak_bg_fill = HOVER;
        v.widgets.hovered.bg_stroke = Stroke::new(1.0, BORDER_STRONG);
        v.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT);

        v.widgets.active.bg_fill = PRESSED;
        v.widgets.active.weak_bg_fill = PRESSED;
        v.widgets.active.bg_stroke = Stroke::new(1.0, BORDER_STRONG);
        v.widgets.active.fg_stroke = Stroke::new(1.0, TEXT);

        let radius = egui::CornerRadius::same(6);
        v.widgets.inactive.corner_radius = radius;
        v.widgets.hovered.corner_radius = radius;
        v.widgets.active.corner_radius = radius;

        style.visuals = v;
        style.spacing.item_spacing = Vec2::new(8.0, 6.0);
        style.spacing.button_padding = Vec2::new(10.0, 5.0);
        style.spacing.interact_size.y = 26.0;
    });
}

fn load_window_icon() -> egui::IconData {
    let image = image::load_from_memory(ICON_PNG)
        .expect("embedded icon")
        .to_rgba8();
    let (width, height) = image.dimensions();
    egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    }
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    Main,
    Pack,
}

struct LogEntry {
    ok: bool,
    text: String,
}

struct PackItem {
    path: PathBuf,
    enabled: bool,
}

struct EntryRow {
    hash: u32,
    name: String,
    size: u32,
}

struct PackSummary {
    entries: usize,
    texture_bytes: u64,
    file_size: u64,
    comment: String,
}

struct PackView {
    path: PathBuf,
    file_name: String,
    summary: PackSummary,
    entries: Vec<EntryRow>,
    index: Arc<tm::index::PackageIndex>,
}

struct VerifySummary {
    valid: usize,
    invalid: usize,
    mismatched: usize,
    missing: usize,
    first_error: Option<String>,
}

impl VerifySummary {
    fn is_clean(&self) -> bool {
        self.invalid == 0 && self.mismatched == 0 && self.missing == 0
    }

    fn problems(&self) -> usize {
        self.invalid + self.mismatched + self.missing
    }
}

enum VerifyState {
    Running,
    Done(VerifySummary),
    Failed(String),
}

struct Preview {
    entry: usize,
    state: PreviewState,
}

enum PreviewState {
    Loading,
    Ready {
        texture: egui::TextureHandle,
        width: u32,
        height: u32,
    },
    Failed(String),
}

type PreviewResult = Result<(egui::ColorImage, u32, u32), String>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Action {
    Launch,
    Install,
    Shortcut,
}

enum PackAction {
    Toggle(usize),
    Remove(usize),
    Move { from: usize, to: usize },
    Inspect(usize),
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Persisted {
    game_exe: String,
    packs: Vec<String>,
    disabled_packs: Vec<String>,
    shortcut_name: String,
}

struct RetexApp {
    view: View,
    game_exe: String,
    packs: Vec<PackItem>,
    shortcut_name: String,
    pack: Option<PackView>,
    verify: Option<VerifyState>,
    filter: String,
    preview: Option<Preview>,
    log: Vec<LogEntry>,
    show_log: bool,
    shortcut_dialog: bool,
    /// Running engine action: busy label and its result channel.
    task: Option<(String, Receiver<(bool, String)>)>,
    verify_rx: Option<Receiver<(PathBuf, Result<VerifySummary, String>)>>,
    preview_rx: Option<Receiver<(PathBuf, usize, PreviewResult)>>,
    heading: FontFamily,
    dirty: bool,
}

impl RetexApp {
    fn new(has_semibold: bool, initial_pack: Option<PathBuf>) -> Self {
        let persisted = load_persisted();
        let mut packs: Vec<PackItem> = Vec::new();
        for pack in persisted.packs {
            let path = PathBuf::from(pack);
            if !packs.iter().any(|p| same_path(&p.path, &path)) {
                let enabled = !persisted
                    .disabled_packs
                    .iter()
                    .any(|d| same_path(Path::new(d), &path));
                packs.push(PackItem { path, enabled });
            }
        }

        let heading = if has_semibold {
            FontFamily::Name("semibold".into())
        } else {
            FontFamily::Proportional
        };

        let mut app = Self {
            view: View::Main,
            game_exe: persisted.game_exe,
            packs,
            shortcut_name: persisted.shortcut_name,
            pack: None,
            verify: None,
            filter: String::new(),
            preview: None,
            log: Vec::new(),
            show_log: false,
            shortcut_dialog: false,
            task: None,
            verify_rx: None,
            preview_rx: None,
            heading,
            dirty: false,
        };

        if app.shortcut_name.is_empty() && !app.game_exe.is_empty() {
            app.shortcut_name = default_shortcut_name(Path::new(&app.game_exe));
        }
        if let Some(path) = initial_pack {
            app.open_pack(path);
        }
        app
    }

    fn draw(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll_tasks(&ctx);
        self.handle_drops(&ctx);

        self.title_bar(ui);
        let mut action = self.footer(ui);
        if self.show_log {
            self.activity_panel(ui);
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG).inner_margin(egui::Margin {
                left: 16,
                right: 16,
                top: 14,
                bottom: 10,
            }))
            .show(ui, |ui| match self.view {
                View::Main => self.main_view(ui),
                View::Pack => self.pack_view(ui),
            });

        if let Some(shortcut) = self.shortcut_modal(&ctx) {
            action = Some(shortcut);
        }
        if let Some(action) = action {
            self.run_action(action);
        }
        drop_overlay(&ctx);

        // Undecorated windows have no system frame; outline the edge so the
        // window does not bleed into a dark desktop.
        ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::unique("window_border"),
        ))
        .rect_stroke(
            ctx.viewport_rect(),
            0.0,
            Stroke::new(1.0, BORDER_STRONG),
            egui::StrokeKind::Inside,
        );

        if self.dirty {
            self.save_state();
            self.dirty = false;
        }
    }

    // -- chrome -------------------------------------------------------------

    /// Custom title bar in place of the system one. Empty space drags the
    /// window.
    fn title_bar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("title_bar")
            .resizable(false)
            .show_separator_line(false)
            .exact_size(TITLE_BAR_HEIGHT)
            .frame(egui::Frame::new().fill(BG).inner_margin(egui::Margin {
                left: 16,
                ..Default::default()
            }))
            .show(ui, |ui| {
                let bar = ui.max_rect();
                // Interact with the background first so the widgets added
                // on top of it take precedence.
                let drag = ui.interact(bar, ui.scope_id().with("drag"), Sense::drag());
                if drag.drag_started_by(egui::PointerButton::Primary) {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }

                ui.horizontal_centered(|ui| {
                    ui.label(
                        RichText::new("Retex")
                            .size(12.5)
                            .family(self.heading.clone())
                            .color(TEXT),
                    );
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        if window_button(ui, WindowButton::Close).clicked() {
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if window_button(ui, WindowButton::Minimize).clicked() {
                            ui.ctx()
                                .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }
                    });
                });

                ui.painter()
                    .hline(bar.x_range(), bar.bottom() - 0.5, Stroke::new(1.0, BORDER));
            });
    }

    /// Always-visible action bar: latest status on the left, Launch and the
    /// overflow menu on the right.
    fn footer(&mut self, ui: &mut egui::Ui) -> Option<Action> {
        let mut action = None;
        let busy = self.task.is_some();
        let has_exe = Path::new(self.game_exe.trim()).is_file();
        let has_packs = self.packs.iter().any(|p| p.enabled);
        let mut inspect_file = false;

        egui::Panel::bottom("footer")
            .resizable(false)
            .show_separator_line(false)
            .exact_size(FOOTER_HEIGHT)
            .frame(egui::Frame::new().fill(BG).inner_margin(egui::Margin {
                left: 10,
                right: 12,
                ..Default::default()
            }))
            .show(ui, |ui| {
                let bar = ui.max_rect();
                let full = ui.ctx().viewport_rect().x_range();
                ui.painter()
                    .hline(full, bar.top() + 0.5, Stroke::new(1.0, BORDER));

                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    let more = more_button(ui);
                    egui::Popup::menu(&more)
                        .align(egui::RectAlign::TOP_END)
                        .gap(6.0)
                        .width(190.0)
                        .show(|ui| {
                            let tools = !busy && has_exe && has_packs;
                            if menu_item(ui, "Install ASI hook", tools)
                                .on_hover_text("Copy retex.asi and retex.ini into the game folder")
                                .on_disabled_hover_text("Needs a game and an enabled pack")
                                .clicked()
                            {
                                action = Some(Action::Install);
                            }
                            if menu_item(ui, "Create desktop shortcut…", tools)
                                .on_disabled_hover_text("Needs a game and an enabled pack")
                                .clicked()
                            {
                                self.shortcut_dialog = true;
                            }
                            ui.separator();
                            if menu_item(ui, "Inspect a pack file…", true).clicked() {
                                inspect_file = true;
                            }
                        });

                    ui.add_space(6.0);
                    if primary_button(ui, "Launch game", !busy && has_exe)
                        .on_disabled_hover_text(if busy { "Busy" } else { "Choose a game first" })
                        .clicked()
                    {
                        action = Some(Action::Launch);
                    }
                    ui.add_space(10.0);

                    ui.with_layout(egui::Layout::left_to_right(Align::Center), |ui| {
                        self.status(ui);
                    });
                });
            });

        if inspect_file {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("TexMod pack", &["tpf"])
                .pick_file()
            {
                self.open_pack(path);
            }
        }
        action
    }

    /// Footer status: a spinner while busy, otherwise the latest activity
    /// entry. Clicking it toggles the activity log.
    fn status(&mut self, ui: &mut egui::Ui) {
        if let Some((label, _)) = &self.task {
            ui.add_space(6.0);
            ui.add(egui::Spinner::new().size(12.0).color(MUTED));
            ui.label(RichText::new(label).size(11.0).color(MUTED));
            return;
        }
        let Some(entry) = self.log.first() else {
            ui.add_space(6.0);
            ui.label(RichText::new("Ready").size(11.0).color(DIM));
            return;
        };

        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 30.0), Sense::click());
        let painter = ui.painter();
        if response.hovered() || self.show_log {
            painter.rect_filled(rect, 6.0, HOVER);
        }
        let first_line = entry.text.lines().next().unwrap_or_default();
        paint_line(
            painter,
            egui::pos2(rect.left() + 10.0, rect.center().y),
            first_line,
            11.0,
            if entry.ok { MUTED } else { ERR },
            rect.width() - 16.0,
        );
        if response
            .on_hover_text(if self.show_log {
                "Hide activity"
            } else {
                "Show activity"
            })
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            self.show_log = !self.show_log;
        }
    }

    fn activity_panel(&mut self, ui: &mut egui::Ui) {
        let mut clear = false;
        egui::Panel::bottom("activity")
            .resizable(false)
            .show_separator_line(false)
            .exact_size(ACTIVITY_HEIGHT)
            .frame(
                egui::Frame::new()
                    .fill(SURFACE)
                    .inner_margin(egui::Margin::symmetric(16, 8)),
            )
            .show(ui, |ui| {
                let full = ui.ctx().viewport_rect().x_range();
                ui.painter()
                    .hline(full, ui.max_rect().top() - 8.0, Stroke::new(1.0, BORDER));
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Activity")
                            .size(11.5)
                            .family(self.heading.clone())
                            .color(MUTED),
                    );
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if text_button(ui, "Hide", true).clicked() {
                            self.show_log = false;
                        }
                        if text_button(ui, "Clear", !self.log.is_empty()).clicked() {
                            clear = true;
                        }
                    });
                });
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for entry in &self.log {
                            ui.label(RichText::new(&entry.text).size(10.5).color(if entry.ok {
                                MUTED
                            } else {
                                ERR
                            }));
                        }
                    });
            });
        if clear {
            self.log.clear();
            self.show_log = false;
        }
    }

    // -- main view ----------------------------------------------------------

    fn main_view(&mut self, ui: &mut egui::Ui) {
        section_label(ui, "Game", &self.heading);
        ui.add_space(6.0);
        self.game_card(ui);
        ui.add_space(18.0);
        self.packs_header(ui);
        ui.add_space(6.0);
        self.pack_list(ui);
    }

    fn game_card(&mut self, ui: &mut egui::Ui) {
        let mut browse = false;
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 52.0), Sense::click());
        let painter = ui.painter();

        if self.game_exe.trim().is_empty() {
            let hovered = response.hovered();
            drop_zone(
                painter,
                rect,
                "Drop a game .exe here, or click to browse",
                hovered,
            );
            browse = response
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked();
        } else {
            let exe = PathBuf::from(self.game_exe.trim());
            let found = exe.is_file();
            card(painter, rect);
            let name = exe
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let dir = exe
                .parent()
                .map(|d| d.display().to_string())
                .unwrap_or_default();
            let text_width = rect.width() - 14.0 - 90.0;
            paint_line(
                painter,
                egui::pos2(rect.left() + 14.0, rect.top() + 17.0),
                &name,
                13.0,
                TEXT,
                text_width,
            );
            let (detail, color) = if found {
                (dir, DIM)
            } else {
                (format!("Not found: {dir}"), ERR)
            };
            paint_line(
                painter,
                egui::pos2(rect.left() + 14.0, rect.top() + 35.0),
                &detail,
                10.5,
                color,
                text_width,
            );

            let button_rect = Rect::from_min_max(
                egui::pos2(rect.right() - 90.0, rect.top()),
                egui::pos2(rect.right() - 10.0, rect.bottom()),
            );
            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(button_rect)
                    .layout(egui::Layout::right_to_left(Align::Center)),
                |ui| {
                    if ghost_button(ui, "Change", true).clicked() {
                        browse = true;
                    }
                },
            );
        }

        if browse {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Executable", &["exe"])
                .pick_file()
            {
                self.set_game(path);
            }
        }
    }

    fn packs_header(&mut self, ui: &mut egui::Ui) {
        let busy = self.task.is_some();
        let mut add = false;
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 28.0), Sense::hover());
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(rect)
                .layout(egui::Layout::left_to_right(Align::Center)),
            |ui| {
                section_label(ui, "Packs", &self.heading).on_hover_text(
                    "Packs apply top to bottom. If two packs replace the same \
                     texture, the higher one wins. Drag to reorder.",
                );
                if !self.packs.is_empty() {
                    let enabled = self.packs.iter().filter(|p| p.enabled).count();
                    let total: u64 = self
                        .packs
                        .iter()
                        .filter(|p| p.enabled)
                        .filter_map(|p| std::fs::metadata(&p.path).ok())
                        .map(|m| m.len())
                        .sum();
                    ui.label(
                        RichText::new(format!(
                            "{enabled} of {} enabled · {}",
                            self.packs.len(),
                            human_bytes(total)
                        ))
                        .size(10.5)
                        .color(DIM),
                    );
                }
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    if ghost_button(ui, "Add", !busy).clicked() {
                        add = true;
                    }
                });
            },
        );

        if add {
            self.pick_packs();
        }
    }

    fn pick_packs(&mut self) {
        if let Some(files) = rfd::FileDialog::new()
            .add_filter("TexMod pack", &["tpf"])
            .pick_files()
        {
            for file in files {
                self.add_pack(file);
            }
        }
    }

    fn pack_list(&mut self, ui: &mut egui::Ui) {
        if self.packs.is_empty() {
            let (rect, response) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 96.0), Sense::click());
            drop_zone(
                ui.painter(),
                rect,
                "Drop .tpf packs here, or click to add",
                response.hovered(),
            );
            if response
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                self.pick_packs();
            }
            return;
        }

        let busy = self.task.is_some();
        let dragging = egui::DragAndDrop::has_payload_of_type::<usize>(ui.ctx());
        let mut action = None;
        let count = self.packs.len();
        // +2 for the card border, so a list that fits never shows a scrollbar.
        let list_height = (PACK_ROW_HEIGHT * count as f32 + 2.0).min(ui.available_height());
        let (outer, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), list_height), Sense::hover());
        card(ui.painter(), outer);

        ui.scope_builder(egui::UiBuilder::new().max_rect(outer.shrink(1.0)), |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let width = ui.available_width();
                    for (index, pack) in self.packs.iter().enumerate() {
                        let (rect, row) = ui
                            .allocate_exact_size(Vec2::new(width, PACK_ROW_HEIGHT), Sense::click());
                        let hovered = ui.rect_contains_pointer(rect) && !dragging;
                        let painter = ui.painter();
                        if index > 0 {
                            painter.hline(
                                rect.left() + 12.0..=rect.right() - 12.0,
                                rect.top(),
                                Stroke::new(1.0, BORDER),
                            );
                        }
                        if hovered {
                            painter.rect_filled(rect.shrink(3.0), 6.0, HOVER);
                        }

                        // Drag handle: reorders via egui's drag-and-drop payload.
                        let handle_rect = Rect::from_center_size(
                            egui::pos2(rect.left() + 14.0, rect.center().y),
                            Vec2::new(14.0, 24.0),
                        );
                        let handle = ui
                            .interact(
                                handle_rect,
                                ui.scope_id().with(("pack_handle", index)),
                                Sense::drag(),
                            )
                            .on_hover_cursor(egui::CursorIcon::Grab);
                        handle.dnd_set_drag_payload(index);
                        paint_grip(
                            ui.painter(),
                            handle_rect.center(),
                            if handle.hovered() || handle.dragged() {
                                TEXT
                            } else {
                                DIM
                            },
                        );
                        if let Some(from) = row.dnd_hover_payload::<usize>() {
                            let y = if *from < index {
                                rect.bottom()
                            } else {
                                rect.top()
                            };
                            if *from != index {
                                ui.painter().hline(
                                    rect.left() + 8.0..=rect.right() - 8.0,
                                    y,
                                    Stroke::new(2.0, TEXT),
                                );
                            }
                        }
                        if let Some(from) = row.dnd_release_payload::<usize>() {
                            action = Some(PackAction::Move {
                                from: *from,
                                to: index,
                            });
                        }

                        let check_rect = Rect::from_center_size(
                            egui::pos2(rect.left() + 36.0, rect.center().y),
                            Vec2::splat(16.0),
                        );
                        let check = ui
                            .interact(
                                check_rect.expand(4.0),
                                ui.scope_id().with(("pack_enabled", index)),
                                Sense::click(),
                            )
                            .on_hover_text(if pack.enabled {
                                "Enabled: click to skip this pack"
                            } else {
                                "Disabled: click to apply this pack"
                            });
                        paint_checkbox(ui.painter(), check_rect, pack.enabled, check.hovered());
                        if check.clicked() && !busy {
                            action = Some(PackAction::Toggle(index));
                        }

                        let exists = pack.path.is_file();
                        let name = pack
                            .path
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_default();
                        let parent = pack
                            .path
                            .parent()
                            .map(|p| p.display().to_string())
                            .unwrap_or_default();
                        let text_left = rect.left() + 54.0;
                        let text_width = rect.right() - 80.0 - text_left;
                        let name_color = match (exists, pack.enabled) {
                            (false, _) => ERR,
                            (true, true) => TEXT,
                            (true, false) => DIM,
                        };
                        paint_line(
                            ui.painter(),
                            egui::pos2(text_left, rect.top() + 14.0),
                            &name,
                            12.5,
                            name_color,
                            text_width,
                        );
                        paint_line(
                            ui.painter(),
                            egui::pos2(text_left, rect.top() + 30.0),
                            &if exists {
                                parent
                            } else {
                                format!("Missing: {parent}")
                            },
                            10.0,
                            if exists { DIM } else { ERR },
                            text_width,
                        );

                        // Right side: Remove on hover, chevron always (the row
                        // opens the pack details).
                        paint_chevron(
                            ui.painter(),
                            egui::pos2(rect.right() - 16.0, rect.center().y),
                            if hovered { TEXT } else { DIM },
                        );
                        if hovered {
                            let remove_rect = Rect::from_min_max(
                                egui::pos2(rect.right() - 80.0, rect.top()),
                                egui::pos2(rect.right() - 26.0, rect.bottom()),
                            );
                            ui.scope_builder(
                                egui::UiBuilder::new()
                                    .max_rect(remove_rect)
                                    .layout(egui::Layout::right_to_left(Align::Center)),
                                |ui| {
                                    if text_button(ui, "Remove", !busy).clicked() {
                                        action = Some(PackAction::Remove(index));
                                    }
                                },
                            );
                        }

                        if row
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                            && action.is_none()
                        {
                            action = Some(PackAction::Inspect(index));
                        }
                    }
                });
        });

        match action {
            Some(PackAction::Toggle(index)) => {
                self.packs[index].enabled = !self.packs[index].enabled;
                self.dirty = true;
            }
            Some(PackAction::Remove(index)) => {
                let removed = self.packs.remove(index);
                self.log(true, format!("Removed {}", file_label(&removed.path)));
                self.dirty = true;
            }
            Some(PackAction::Move { from, to }) if from != to && from < self.packs.len() => {
                let item = self.packs.remove(from);
                self.packs.insert(to, item);
                self.dirty = true;
            }
            Some(PackAction::Inspect(index)) => {
                let path = self.packs[index].path.clone();
                if path.is_file() {
                    self.open_pack(path);
                } else {
                    self.log(false, format!("Pack not found: {}", path.display()));
                }
            }
            _ => {}
        }
    }

    // -- pack details -------------------------------------------------------

    fn pack_view(&mut self, ui: &mut egui::Ui) {
        let Some(pack) = &self.pack else {
            self.view = View::Main;
            return;
        };
        let mut back = false;
        let mut clicked = None;

        // Header: back, name, verification badge.
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 28.0), Sense::hover());
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(rect)
                .layout(egui::Layout::left_to_right(Align::Center)),
            |ui| {
                if text_button(ui, "‹ Back", true).clicked() {
                    back = true;
                }
                ui.add_space(4.0);
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    verify_badge(ui, self.verify.as_ref());
                    ui.add_space(8.0);
                    ui.with_layout(egui::Layout::left_to_right(Align::Center), |ui| {
                        ui.add(
                            egui::Label::new(
                                RichText::new(&pack.file_name)
                                    .size(14.0)
                                    .family(self.heading.clone())
                                    .color(TEXT),
                            )
                            .truncate(),
                        );
                    });
                });
            },
        );

        ui.label(
            RichText::new(format!(
                "{} textures · {} of texture data · {} on disk",
                pack.summary.entries,
                human_bytes(pack.summary.texture_bytes),
                human_bytes(pack.summary.file_size)
            ))
            .size(10.5)
            .color(DIM),
        );
        if !pack.summary.comment.is_empty() {
            ui.add(
                egui::Label::new(
                    RichText::new(pack.summary.comment.lines().next().unwrap_or_default())
                        .size(10.5)
                        .color(MUTED),
                )
                .truncate(),
            )
            .on_hover_text(&pack.summary.comment);
        }
        ui.add_space(8.0);

        ui.add_sized(
            [ui.available_width(), 28.0],
            egui::TextEdit::singleline(&mut self.filter).hint_text("Filter by hash or name"),
        );
        ui.add_space(8.0);

        // "0x1A2B" matches hashes starting with those digits; anything else
        // matches anywhere in the hash or name.
        let needle = self.filter.trim().to_ascii_lowercase();
        let hash_prefix = needle.strip_prefix("0x");
        let filtered: Vec<usize> = pack
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                let hash = format!("{:08x}", entry.hash);
                match hash_prefix {
                    Some(prefix) => hash.starts_with(prefix),
                    None => {
                        needle.is_empty()
                            || hash.contains(&needle)
                            || entry.name.to_ascii_lowercase().contains(&needle)
                    }
                }
            })
            .map(|(index, _)| index)
            .collect();
        let selected = self.preview.as_ref().map(|p| p.entry);

        let body_height = ui.available_height();
        let table_width = ui.available_width() - PREVIEW_WIDTH - 12.0;
        ui.horizontal_top(|ui| {
            // Top-down inside the row, or the table body lands beside its header.
            let column = egui::Layout::top_down(Align::Min);
            ui.allocate_ui_with_layout(Vec2::new(table_width, body_height), column, |ui| {
                if filtered.is_empty() {
                    ui.label(RichText::new("No matching textures.").size(11.0).color(DIM));
                    return;
                }
                TableBuilder::new(ui)
                    .sense(Sense::click())
                    .vscroll(true)
                    .max_scroll_height(body_height - 22.0)
                    .auto_shrink([false, false])
                    .cell_layout(egui::Layout::left_to_right(Align::Center))
                    .column(Column::exact(78.0))
                    .column(Column::exact(58.0))
                    .column(Column::remainder().clip(true))
                    .header(20.0, |mut header| {
                        for title in ["Hash", "Size", "Name"] {
                            header.col(|ui| {
                                ui.label(RichText::new(title).size(9.5).color(DIM));
                            });
                        }
                    })
                    .body(|body| {
                        body.rows(21.0, filtered.len(), |mut row| {
                            let index = filtered[row.index()];
                            let entry = &pack.entries[index];
                            row.set_selected(selected == Some(index));
                            row.col(|ui| {
                                ui.label(
                                    RichText::new(format!("{:08X}", entry.hash))
                                        .monospace()
                                        .size(10.5)
                                        .color(MUTED),
                                );
                            });
                            row.col(|ui| {
                                ui.label(
                                    RichText::new(human_bytes(entry.size as u64))
                                        .size(10.5)
                                        .color(DIM),
                                );
                            });
                            row.col(|ui| {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(&entry.name).size(10.5).color(TEXT),
                                    )
                                    .truncate()
                                    .selectable(false),
                                );
                            });
                            if row.response().clicked() {
                                clicked = Some(index);
                            }
                        });
                    });
            });
            ui.add_space(12.0);
            ui.allocate_ui_with_layout(Vec2::new(PREVIEW_WIDTH, body_height), column, |ui| {
                preview_panel(ui, pack, self.preview.as_ref());
            });
        });

        if back {
            self.view = View::Main;
        }
        if let Some(index) = clicked {
            self.request_preview(ui.ctx(), index);
        }
    }

    fn request_preview(&mut self, ctx: &egui::Context, entry: usize) {
        let Some(pack) = &self.pack else {
            return;
        };
        if self.preview.as_ref().is_some_and(|p| p.entry == entry) {
            return;
        }
        let index = pack.index.clone();
        let path = pack.path.clone();
        let (tx, rx) = channel();
        self.preview = Some(Preview {
            entry,
            state: PreviewState::Loading,
        });
        self.preview_rx = Some(rx);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send((path, entry, decode_preview(&index, entry)));
            ctx.request_repaint();
        });
    }

    fn shortcut_modal(&mut self, ctx: &egui::Context) -> Option<Action> {
        if !self.shortcut_dialog {
            return None;
        }
        let mut create = false;
        let mut cancel = false;
        let default_name = default_shortcut_name(Path::new(self.game_exe.trim()));

        let modal = egui::Modal::new(egui::Id::unique("shortcut_modal"))
            .backdrop_color(Color32::from_black_alpha(160))
            .frame(
                egui::Frame::new()
                    .fill(SURFACE)
                    .stroke(Stroke::new(1.0, BORDER_STRONG))
                    .corner_radius(10.0)
                    .inner_margin(18),
            )
            .show(ctx, |ui| {
                ui.set_width(300.0);
                ui.label(
                    RichText::new("Create desktop shortcut")
                        .size(14.0)
                        .family(self.heading.clone())
                        .color(TEXT),
                );
                ui.label(
                    RichText::new("Starts the game with your enabled packs.")
                        .size(10.5)
                        .color(DIM),
                );
                ui.add_space(12.0);
                ui.label(RichText::new("Name").size(10.5).color(MUTED));
                let name = ui.add_sized(
                    [300.0, 28.0],
                    egui::TextEdit::singleline(&mut self.shortcut_name).hint_text(&default_name),
                );
                if name.changed() {
                    self.dirty = true;
                }
                if name.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    create = true;
                }
                ui.add_space(14.0);
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    if primary_button(ui, "Create", true).clicked() {
                        create = true;
                    }
                    ui.add_space(6.0);
                    if ghost_button(ui, "Cancel", true).clicked() {
                        cancel = true;
                    }
                });
            });

        if modal.should_close() || cancel {
            self.shortcut_dialog = false;
        }
        if create {
            self.shortcut_dialog = false;
            if self.shortcut_name.trim().is_empty() {
                self.shortcut_name = default_name;
            }
            return Some(Action::Shortcut);
        }
        None
    }

    // -- actions ------------------------------------------------------------

    fn enabled_packs(&self) -> Vec<PathBuf> {
        self.packs
            .iter()
            .filter(|p| p.enabled)
            .map(|p| p.path.clone())
            .collect()
    }

    fn run_action(&mut self, action: Action) {
        let exe = PathBuf::from(self.game_exe.trim());
        if !exe.is_file() {
            self.log(false, "Game executable not found.");
            return;
        }
        let packs = self.enabled_packs();
        if action != Action::Launch && packs.is_empty() {
            self.log(false, "Enable at least one pack first.");
            return;
        }
        let shortcut_name = self.shortcut_name.trim().to_string();

        match action {
            Action::Launch => self.spawn("Launching…", move || {
                write_config(&exe, &packs)?;
                run_engine(vec!["launch".into(), exe.as_os_str().into()])
            }),
            Action::Install => self.spawn("Installing ASI hook…", move || {
                write_config(&exe, &packs)?;
                let game_dir = exe
                    .parent()
                    .ok_or_else(|| "game path has no directory".to_string())?;
                let mut args: Vec<OsString> = vec!["install".into(), game_dir.as_os_str().into()];
                for pack in &packs {
                    args.push("--pack".into());
                    args.push(pack.as_os_str().into());
                }
                run_engine(args)
            }),
            Action::Shortcut => self.spawn("Creating shortcut…", move || {
                write_config(&exe, &packs)?;
                let mut args: Vec<OsString> =
                    vec!["shortcut".into(), exe.as_os_str().into(), "--inject".into()];
                if !shortcut_name.is_empty() {
                    args.push("--name".into());
                    args.push(shortcut_name.into());
                }
                run_engine(args)
            }),
        }
    }

    fn start_verify(&mut self, path: PathBuf) {
        let (tx, rx) = channel();
        self.verify = Some(VerifyState::Running);
        self.verify_rx = Some(rx);
        std::thread::spawn(move || {
            let result = verify_pack(&path);
            let _ = tx.send((path, result));
        });
    }

    fn spawn<F>(&mut self, label: &str, task: F)
    where
        F: FnOnce() -> Result<String, String> + Send + 'static,
    {
        let (tx, rx) = channel();
        self.task = Some((label.to_string(), rx));
        std::thread::spawn(move || {
            let _ = tx.send(match task() {
                Ok(text) => (true, text),
                Err(text) => (false, text),
            });
        });
    }

    fn poll_tasks(&mut self, ctx: &egui::Context) {
        let mut pending = false;

        if let Some((_, rx)) = &self.task {
            match rx.try_recv() {
                Ok((ok, text)) => {
                    self.task = None;
                    self.log(ok, text);
                }
                Err(TryRecvError::Empty) => pending = true,
                Err(TryRecvError::Disconnected) => {
                    self.task = None;
                    self.log(false, "Background task ended unexpectedly.");
                }
            }
        }

        if let Some(rx) = &self.verify_rx {
            match rx.try_recv() {
                Ok((path, result)) => {
                    self.verify_rx = None;
                    if self.pack.as_ref().is_some_and(|p| p.path == path) {
                        self.verify = Some(match result {
                            Ok(summary) => {
                                if !summary.is_clean() {
                                    self.log(false, summary_text(&summary));
                                }
                                VerifyState::Done(summary)
                            }
                            Err(error) => VerifyState::Failed(error),
                        });
                    }
                }
                Err(TryRecvError::Empty) => pending = true,
                Err(TryRecvError::Disconnected) => {
                    self.verify_rx = None;
                    self.verify = Some(VerifyState::Failed("Verification stopped".into()));
                }
            }
        }

        if let Some(rx) = &self.preview_rx {
            match rx.try_recv() {
                Ok((path, entry, result)) => {
                    self.preview_rx = None;
                    let current = self.pack.as_ref().is_some_and(|p| p.path == path);
                    if let Some(preview) = self.preview.as_mut().filter(|p| p.entry == entry) {
                        if current {
                            preview.state = match result {
                                Ok((image, width, height)) => PreviewState::Ready {
                                    texture: ctx.load_texture(
                                        "retex-preview",
                                        image,
                                        egui::TextureOptions::LINEAR,
                                    ),
                                    width,
                                    height,
                                },
                                Err(error) => PreviewState::Failed(error),
                            };
                        }
                    }
                }
                Err(TryRecvError::Empty) => pending = true,
                Err(TryRecvError::Disconnected) => self.preview_rx = None,
            }
        }

        if pending {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }

    // -- state --------------------------------------------------------------

    fn set_game(&mut self, path: PathBuf) {
        let path = std::path::absolute(&path).unwrap_or(path);
        self.game_exe = path.display().to_string();
        if self.shortcut_name.trim().is_empty() {
            self.shortcut_name = default_shortcut_name(&path);
        }
        self.dirty = true;
        self.log(true, format!("Game set to {}", file_label(&path)));
    }

    fn add_pack(&mut self, path: PathBuf) {
        let path = std::path::absolute(&path).unwrap_or(path);
        if self.packs.iter().any(|p| same_path(&p.path, &path)) {
            return;
        }
        self.log(true, format!("Added {}", file_label(&path)));
        self.packs.push(PackItem {
            path,
            enabled: true,
        });
        self.dirty = true;
    }

    fn open_pack(&mut self, path: PathBuf) {
        match load_pack(&path) {
            Ok(view) => {
                self.pack = Some(view);
                self.view = View::Pack;
                self.filter.clear();
                self.preview = None;
                self.preview_rx = None;
                self.start_verify(path);
            }
            Err(error) => self.log(false, error),
        }
    }

    fn handle_drops(&mut self, ctx: &egui::Context) {
        let dropped: Vec<PathBuf> = ctx.input(|input| {
            input
                .raw
                .dropped_files
                .iter()
                .map(|file| file.path().to_path_buf())
                .collect()
        });
        for path in dropped {
            match extension(&path).as_deref() {
                Some("tpf") => {
                    self.add_pack(path);
                    self.view = View::Main;
                }
                Some("exe") => {
                    self.set_game(path);
                    self.view = View::Main;
                }
                _ => self.log(false, format!("Unsupported file: {}", file_label(&path))),
            }
        }
    }

    fn log(&mut self, ok: bool, text: impl Into<String>) {
        self.log.insert(
            0,
            LogEntry {
                ok,
                text: text.into(),
            },
        );
        self.log.truncate(40);
    }

    fn save_state(&self) {
        let Some(path) = config_path() else {
            return;
        };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let data = Persisted {
            game_exe: self.game_exe.clone(),
            packs: self
                .packs
                .iter()
                .map(|pack| pack.path.display().to_string())
                .collect(),
            disabled_packs: self
                .packs
                .iter()
                .filter(|pack| !pack.enabled)
                .map(|pack| pack.path.display().to_string())
                .collect(),
            shortcut_name: self.shortcut_name.clone(),
        };
        if let Ok(text) = serde_json::to_string_pretty(&data) {
            let _ = std::fs::write(path, text);
        }
    }
}

impl eframe::App for RetexApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.draw(ui, frame);
    }
}

// ---------------------------------------------------------------------------
// Widgets
// ---------------------------------------------------------------------------

fn section_label(ui: &mut egui::Ui, title: &str, heading: &FontFamily) -> egui::Response {
    ui.label(
        RichText::new(title)
            .size(12.5)
            .family(heading.clone())
            .color(TEXT),
    )
}

/// One line of text, vertically centred on `pos.y`, cut with an ellipsis at
/// `max_width`.
fn paint_line(
    painter: &egui::Painter,
    pos: egui::Pos2,
    text: &str,
    size: f32,
    color: Color32,
    max_width: f32,
) {
    let mut job = egui::text::LayoutJob::simple_singleline(
        text.to_owned(),
        FontId::proportional(size),
        color,
    );
    job.wrap = egui::text::TextWrapping::truncate_at_width(max_width.max(0.0));
    let galley = painter.layout_job(job);
    let top = pos.y - galley.size().y / 2.0;
    painter.galley(egui::pos2(pos.x, top), galley, color);
}

fn card(painter: &egui::Painter, rect: Rect) {
    painter.rect(
        rect,
        8.0,
        SURFACE,
        Stroke::new(1.0, BORDER),
        egui::StrokeKind::Inside,
    );
}

/// Dashed outline with a centred hint, for empty states that accept drops.
fn drop_zone(painter: &egui::Painter, rect: Rect, hint: &str, hovered: bool) {
    if hovered {
        painter.rect_filled(rect, 8.0, SURFACE);
    }
    dashed_rect(
        painter,
        rect.shrink(0.5),
        if hovered { MUTED } else { BORDER_STRONG },
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        hint,
        FontId::proportional(11.5),
        if hovered { TEXT } else { MUTED },
    );
}

fn dashed_rect(painter: &egui::Painter, rect: Rect, color: Color32) {
    let points = [
        rect.left_top(),
        rect.right_top(),
        rect.right_bottom(),
        rect.left_bottom(),
        rect.left_top(),
    ];
    painter.extend(egui::Shape::dashed_line(
        &points,
        Stroke::new(1.0, color),
        5.0,
        4.0,
    ));
}

/// Full-window hint while files are dragged over the app.
fn drop_overlay(ctx: &egui::Context) {
    let hovered: Vec<Option<PathBuf>> = ctx.input(|i| {
        i.raw
            .hovered_files
            .iter()
            .map(|file| file.path.clone())
            .collect()
    });
    if hovered.is_empty() {
        return;
    }
    let packs = hovered
        .iter()
        .filter(|p| p.as_deref().and_then(extension).as_deref() == Some("tpf"))
        .count();
    let exes = hovered
        .iter()
        .filter(|p| p.as_deref().and_then(extension).as_deref() == Some("exe"))
        .count();
    let unknown_paths = hovered.iter().all(|p| p.is_none());

    let (text, color) = match (packs, exes) {
        (0, 0) if unknown_paths => ("Drop to add".to_string(), TEXT),
        (0, 0) => (
            "Only .tpf packs and .exe files are supported".to_string(),
            ERR,
        ),
        (1, 0) => ("Drop to add pack".to_string(), TEXT),
        (n, 0) => (format!("Drop to add {n} packs"), TEXT),
        (0, _) => ("Drop to set the game".to_string(), TEXT),
        _ => ("Drop to set the game and add packs".to_string(), TEXT),
    };

    let mut rect = ctx.viewport_rect();
    rect.min.y += TITLE_BAR_HEIGHT;
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::unique("drop_overlay"),
    ));
    painter.rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(10, 10, 10, 235));
    dashed_rect(&painter, rect.shrink(14.0), MUTED);
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        FontId::proportional(14.0),
        color,
    );
}

fn paint_grip(painter: &egui::Painter, center: egui::Pos2, color: Color32) {
    for dx in [-2.5, 2.5] {
        for dy in [-5.0, 0.0, 5.0] {
            painter.circle_filled(center + Vec2::new(dx, dy), 1.2, color);
        }
    }
}

fn paint_checkbox(painter: &egui::Painter, rect: Rect, checked: bool, hovered: bool) {
    if checked {
        painter.rect_filled(rect, 4.0, TEXT);
        let a = rect.left_top() + Vec2::new(3.5, 8.0);
        let b = rect.left_top() + Vec2::new(6.5, 11.0);
        let c = rect.left_top() + Vec2::new(12.5, 5.0);
        let stroke = Stroke::new(1.8, BG);
        painter.line_segment([a, b], stroke);
        painter.line_segment([b, c], stroke);
    } else {
        painter.rect_stroke(
            rect,
            4.0,
            Stroke::new(1.0, if hovered { MUTED } else { BORDER_STRONG }),
            egui::StrokeKind::Inside,
        );
    }
}

fn paint_chevron(painter: &egui::Painter, center: egui::Pos2, color: Color32) {
    let stroke = Stroke::new(1.3, color);
    painter.line_segment(
        [center + Vec2::new(-2.0, -4.0), center + Vec2::new(2.0, 0.0)],
        stroke,
    );
    painter.line_segment(
        [center + Vec2::new(2.0, 0.0), center + Vec2::new(-2.0, 4.0)],
        stroke,
    );
}

/// Square "more actions" button drawn as three dots.
fn more_button(ui: &mut egui::Ui) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(30.0), Sense::click());
    let fill = if response.hovered() { HOVER } else { SURFACE };
    ui.painter().rect(
        rect,
        6.0,
        fill,
        Stroke::new(
            1.0,
            if response.hovered() {
                BORDER_STRONG
            } else {
                BORDER
            },
        ),
        egui::StrokeKind::Inside,
    );
    for dx in [-5.0, 0.0, 5.0] {
        ui.painter()
            .circle_filled(rect.center() + Vec2::new(dx, 0.0), 1.5, TEXT);
    }
    response
        .on_hover_text("More actions")
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn menu_item(ui: &mut egui::Ui, label: &str, enabled: bool) -> egui::Response {
    let color = if enabled { TEXT } else { DIM };
    ui.add_enabled(
        enabled,
        egui::Button::new(RichText::new(label).size(11.5).color(color))
            .frame(false)
            .min_size(Vec2::new(0.0, 26.0)),
    )
}

fn verify_badge(ui: &mut egui::Ui, state: Option<&VerifyState>) {
    let (text, color, detail) = match state {
        None => return,
        Some(VerifyState::Running) => {
            ui.label(RichText::new("Verifying…").size(10.5).color(MUTED));
            ui.add(egui::Spinner::new().size(12.0).color(MUTED));
            return;
        }
        Some(VerifyState::Done(summary)) if summary.is_clean() => (
            format!("{} valid", summary.valid),
            OK,
            summary_text(summary),
        ),
        Some(VerifyState::Done(summary)) => {
            let problems = summary.problems();
            // Broken textures are errors; definition mismatches are warnings.
            let color = if summary.invalid > 0 { ERR } else { WARN };
            (
                format!("{problems} problem{}", if problems == 1 { "" } else { "s" }),
                color,
                summary_text(summary),
            )
        }
        Some(VerifyState::Failed(error)) => ("Verify failed".into(), ERR, error.clone()),
    };

    let galley = ui
        .painter()
        .layout_no_wrap(text, FontId::proportional(10.5), color);
    let size = Vec2::new(galley.size().x + 20.0, 22.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().rect(
        rect,
        11.0,
        SURFACE,
        Stroke::new(1.0, BORDER),
        egui::StrokeKind::Inside,
    );
    ui.painter().galley(
        egui::pos2(rect.left() + 10.0, rect.center().y - galley.size().y / 2.0),
        galley,
        color,
    );
    response.on_hover_text(detail);
}

fn preview_panel(ui: &mut egui::Ui, pack: &PackView, preview: Option<&Preview>) {
    let (frame, _) = ui.allocate_exact_size(Vec2::splat(PREVIEW_WIDTH), Sense::hover());
    let painter = ui.painter();

    // Checkerboard so transparent texels are visible.
    painter.rect_filled(frame, 6.0, Color32::from_rgb(0x16, 0x16, 0x16));
    let cell = 10.0;
    let cells = (PREVIEW_WIDTH / cell) as usize;
    for y in 0..cells {
        for x in (y % 2..cells).step_by(2) {
            let min = frame.min + Vec2::new(x as f32 * cell, y as f32 * cell);
            painter.rect_filled(
                Rect::from_min_size(min, Vec2::splat(cell)),
                0.0,
                Color32::from_rgb(0x1e, 0x1e, 0x1e),
            );
        }
    }
    painter.rect_stroke(
        frame,
        6.0,
        Stroke::new(1.0, BORDER),
        egui::StrokeKind::Inside,
    );

    let Some(preview) = preview else {
        painter.text(
            frame.center(),
            egui::Align2::CENTER_CENTER,
            "Select a texture",
            FontId::proportional(10.5),
            DIM,
        );
        return;
    };

    let entry = &pack.entries[preview.entry];
    match &preview.state {
        PreviewState::Loading => {
            ui.put(
                Rect::from_center_size(frame.center(), Vec2::splat(18.0)),
                egui::Spinner::new(),
            );
        }
        PreviewState::Failed(error) => {
            painter.text(
                frame.center(),
                egui::Align2::CENTER_CENTER,
                "No preview",
                FontId::proportional(10.5),
                ERR,
            );
            ui.interact(frame, ui.scope_id().with("preview_error"), Sense::hover())
                .on_hover_text(error);
        }
        PreviewState::Ready {
            texture,
            width,
            height,
        } => {
            let inner = frame.shrink(6.0);
            let scale = (inner.width() / *width as f32).min(inner.height() / *height as f32);
            let size = Vec2::new(*width as f32, *height as f32) * scale.min(8.0);
            let rect = Rect::from_center_size(inner.center(), size);
            painter.image(
                texture.id(),
                rect,
                Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
    }

    ui.add_space(8.0);
    if let PreviewState::Ready { width, height, .. } = &preview.state {
        ui.label(
            RichText::new(format!("{width} × {height}"))
                .size(12.0)
                .color(TEXT),
        );
    }
    ui.label(
        RichText::new(format!(
            "{} · {}",
            extension(Path::new(&entry.name))
                .unwrap_or_default()
                .to_ascii_uppercase(),
            human_bytes(entry.size as u64)
        ))
        .size(10.5)
        .color(DIM),
    );
    ui.label(
        RichText::new(format!("0x{:08X}", entry.hash))
            .monospace()
            .size(10.5)
            .color(MUTED),
    );
}

#[derive(Clone, Copy)]
enum WindowButton {
    Minimize,
    Close,
}

/// Title-bar window control, drawn like the Windows 11 caption buttons.
fn window_button(ui: &mut egui::Ui, kind: WindowButton) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(WINDOW_BUTTON_WIDTH, TITLE_BAR_HEIGHT),
        Sense::click(),
    );
    if ui.is_rect_visible(rect) {
        let hovered = response.hovered();
        let (fill, glyph) = match (kind, hovered) {
            (WindowButton::Close, true) => (CLOSE_HOVER, Color32::WHITE),
            (_, true) => (HOVER, TEXT),
            _ => (Color32::TRANSPARENT, MUTED),
        };
        let painter = ui.painter();
        painter.rect_filled(rect, 0.0, fill);
        let center = rect.center();
        let stroke = Stroke::new(1.0, glyph);
        let half = 5.0;
        match kind {
            WindowButton::Minimize => {
                painter.hline(center.x - half..=center.x + half, center.y, stroke);
            }
            WindowButton::Close => {
                let d = Vec2::splat(half);
                painter.line_segment([center - d, center + d], stroke);
                painter.line_segment(
                    [
                        center + Vec2::new(-half, half),
                        center + Vec2::new(half, -half),
                    ],
                    stroke,
                );
            }
        }
    }
    response
}

fn primary_button(ui: &mut egui::Ui, label: &str, enabled: bool) -> egui::Response {
    let (fill, fg) = if enabled {
        (TEXT, BG)
    } else {
        (Color32::from_rgb(0x1a, 0x1a, 0x1a), DIM)
    };
    let button = egui::Button::new(RichText::new(label).size(12.5).color(fg))
        .fill(fill)
        .stroke(Stroke::NONE)
        .corner_radius(6.0)
        .min_size(Vec2::new(0.0, 30.0));
    ui.add_enabled(enabled, button)
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Bordered secondary button. Disabled ones drop the fill and border so they
/// read as unavailable rather than as a subtle variant.
fn ghost_button(ui: &mut egui::Ui, label: &str, enabled: bool) -> egui::Response {
    let button = if enabled {
        egui::Button::new(RichText::new(label).size(11.5).color(TEXT))
    } else {
        egui::Button::new(RichText::new(label).size(11.5).color(DIM))
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::NONE)
    };
    ui.add_enabled(
        enabled,
        button.corner_radius(6.0).min_size(Vec2::new(0.0, 28.0)),
    )
    .on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn text_button(ui: &mut egui::Ui, label: &str, enabled: bool) -> egui::Response {
    let color = if enabled { MUTED } else { DIM };
    let button = egui::Button::new(RichText::new(label).size(10.5).color(color)).frame(false);
    ui.add_enabled(enabled, button)
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}

// ---------------------------------------------------------------------------
// Engine / package helpers
// ---------------------------------------------------------------------------

fn engine_path() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    let dir = exe
        .parent()
        .ok_or_else(|| "cannot determine the app directory".to_string())?;
    let cwd = std::env::current_dir().unwrap_or_default();
    let candidates = [
        dir.join("retex.exe"),
        dir.join("../release/retex.exe"),
        dir.join("../debug/retex.exe"),
        cwd.join("target/debug/retex.exe"),
        cwd.join("target/release/retex.exe"),
    ];
    for candidate in candidates {
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err(
        "retex.exe was not found next to the app. Build it with `cargo build -p retex-cli`."
            .to_string(),
    )
}

fn run_engine(args: Vec<OsString>) -> Result<String, String> {
    let engine = engine_path()?;
    let mut command = std::process::Command::new(&engine);
    command.args(&args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let output = command
        .output()
        .map_err(|error| format!("could not run retex.exe: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let text = match (stdout.is_empty(), stderr.is_empty()) {
        (false, false) => format!("{stdout}\n{stderr}"),
        (false, true) => stdout,
        (true, false) => stderr,
        (true, true) => String::new(),
    };
    if output.status.success() {
        Ok(if text.is_empty() {
            "Done.".to_string()
        } else {
            text
        })
    } else if text.is_empty() {
        Err(format!("retex failed with {}", output.status))
    } else {
        Err(text)
    }
}

fn write_config(exe: &Path, packs: &[PathBuf]) -> Result<(), String> {
    let dir = exe
        .parent()
        .ok_or_else(|| "game path has no parent directory".to_string())?;
    let mut config = String::from("# Retex\r\n");
    for pack in packs {
        if !pack.is_file() {
            return Err(format!("Pack not found: {}", pack.display()));
        }
        config.push_str(&format!("pack={}\r\n", pack.display()));
    }
    let path = dir.join("retex.ini");
    std::fs::write(&path, config)
        .map_err(|error| format!("could not write {}: {error}", path.display()))
}

fn load_pack(path: &Path) -> Result<PackView, String> {
    let index = tm::index::PackageIndex::open(path)
        .map_err(|error| format!("Could not open pack: {error}"))?;
    let mut texture_bytes = 0u64;
    let mut entries = Vec::with_capacity(index.entries.len());
    for entry in &index.entries {
        texture_bytes += entry.uncompressed_size as u64;
        entries.push(EntryRow {
            hash: entry.hash,
            name: entry.name.clone(),
            size: entry.uncompressed_size,
        });
    }
    let file_size = std::fs::metadata(path).map(|meta| meta.len()).unwrap_or(0);
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("pack.tpf")
        .to_string();
    Ok(PackView {
        path: path.to_path_buf(),
        file_name,
        summary: PackSummary {
            entries: entries.len(),
            texture_bytes,
            file_size,
            comment: String::from_utf8_lossy(&index.comment).trim().to_string(),
        },
        entries,
        index: Arc::new(index),
    })
}

/// Read and decode one entry into a preview-sized image.
fn decode_preview(index: &tm::index::PackageIndex, entry: usize) -> PreviewResult {
    let meta = index
        .entries
        .get(entry)
        .ok_or_else(|| "entry out of range".to_string())?;
    let data = index.read_entry(meta).map_err(|error| error.to_string())?;
    let image =
        image::load_from_memory(&data).map_err(|error| format!("Cannot decode: {error}"))?;
    let (width, height) = (image.width(), image.height());
    let mut thumb = image
        .thumbnail(PREVIEW_MAX_SIDE, PREVIEW_MAX_SIDE)
        .to_rgba8();
    // Many legacy BMPs carry an unused, all-zero alpha channel; show those
    // opaque instead of fully transparent.
    if thumb.pixels().all(|pixel| pixel[3] == 0) {
        for pixel in thumb.pixels_mut() {
            pixel[3] = 255;
        }
    }
    let size = [thumb.width() as usize, thumb.height() as usize];
    Ok((
        egui::ColorImage::from_rgba_unmultiplied(size, thumb.as_raw()),
        width,
        height,
    ))
}

fn verify_pack(path: &Path) -> Result<VerifySummary, String> {
    let index = tm::index::PackageIndex::open(path)
        .map_err(|error| format!("Could not open pack: {error}"))?;

    let mut def_map: HashMap<String, u32> = HashMap::new();
    if let Some(meta) = index
        .entries
        .iter()
        .find(|entry| entry.name.eq_ignore_ascii_case("texmod.def"))
    {
        let data = index
            .read_entry(meta)
            .map_err(|error| format!("Could not read texmod.def: {error}"))?;
        let text = String::from_utf8_lossy(&data);
        for (hash, name) in tm::tpf::parse_definition(&text)
            .map_err(|error| format!("Could not parse texmod.def: {error}"))?
        {
            def_map.insert(name, hash);
        }
    }

    let mut valid = 0usize;
    let mut invalid = 0usize;
    let mut mismatched = 0usize;
    let mut first_error = None;

    for entry in &index.entries {
        if entry.name.to_ascii_lowercase().ends_with(".def") {
            continue;
        }
        match index.read_entry(entry) {
            Ok(data) => {
                if is_texture(&entry.name, &data) {
                    valid += 1;
                } else {
                    invalid += 1;
                    if first_error.is_none() {
                        first_error = Some(format!("Invalid texture data: {}", entry.name));
                    }
                }
                if let Some(&expected) = def_map.get(&entry.name) {
                    if expected != entry.hash {
                        mismatched += 1;
                    }
                }
            }
            Err(error) => {
                invalid += 1;
                if first_error.is_none() {
                    first_error = Some(format!("{}: {error}", entry.name));
                }
            }
        }
    }

    let missing = def_map
        .keys()
        .filter(|name| !index.entries.iter().any(|entry| &entry.name == *name))
        .count();

    Ok(VerifySummary {
        valid,
        invalid,
        mismatched,
        missing,
        first_error,
    })
}

fn summary_text(summary: &VerifySummary) -> String {
    let mut text = format!(
        "Verified: {} textures valid, {} invalid, {} def mismatches, {} missing",
        summary.valid, summary.invalid, summary.mismatched, summary.missing
    );
    if let Some(error) = &summary.first_error {
        text.push_str(&format!("\n{error}"));
    }
    text
}

fn is_texture(name: &str, data: &[u8]) -> bool {
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".bmp") {
        is_valid_bmp(data)
    } else if lower.ends_with(".dds") {
        data.len() >= 128 && &data[..4] == b"DDS "
    } else {
        true
    }
}

fn is_valid_bmp(data: &[u8]) -> bool {
    if data.len() < 54 || &data[..2] != b"BM" {
        return false;
    }
    let offset = u32::from_le_bytes([data[10], data[11], data[12], data[13]]) as usize;
    let width = i32::from_le_bytes([data[18], data[19], data[20], data[21]]);
    let height = i32::from_le_bytes([data[22], data[23], data[24], data[25]]);
    let bpp = u16::from_le_bytes([data[28], data[29]]) as usize;
    if bpp == 0 {
        return false;
    }
    let needed =
        offset + width.unsigned_abs() as usize * height.unsigned_abs() as usize * (bpp / 8);
    needed <= data.len()
}

fn load_persisted() -> Persisted {
    let Some(path) = config_path() else {
        return Persisted::default();
    };
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn config_path() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|base| PathBuf::from(base).join("Retex").join("gui.json"))
}

fn extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
}

fn file_label(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

fn default_shortcut_name(path: &Path) -> String {
    format!(
        "{} (Retex)",
        path.file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("Game")
    )
}

fn same_path(a: &Path, b: &Path) -> bool {
    a.to_string_lossy().to_ascii_lowercase() == b.to_string_lossy().to_ascii_lowercase()
}

fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}
