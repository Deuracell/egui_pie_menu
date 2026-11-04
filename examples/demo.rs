use eframe::egui;
use egui::{Color32, RichText, Vec2};
use std::time::Duration;
use egui_pie_menu::{PieButton, PieDirection, PieMenu, PieMenuHighlightShape, HintDigits, HintSide, PieMenuResponse, PieMenuSettings, RevealAnimation, ShowBehavior, SmartFloat};

fn main() -> eframe::Result {
    eframe::run_native(
        "egui_pie_menu demo",
        eframe::NativeOptions::default(),
        Box::new(|cc| {
            let font_note = load_system_fonts(&cc.egui_ctx);
            let mut demo = Demo::new();
            demo.font_note = font_note;
            Ok(Box::new(demo))
        }),
    )
}

/// egui's built-in fonts lack Arabic/Hebrew, so try to add system fonts as fallbacks.
fn load_system_fonts(ctx: &egui::Context) -> String {
    const CANDIDATES: &[&str] = &[
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/freefont/FreeSans.ttf",
        "/usr/share/fonts/truetype/noto/NotoSansArabic-Regular.ttf",
        "/usr/share/fonts/truetype/noto/NotoSansHebrew-Regular.ttf",
        "/Library/Fonts/Arial Unicode.ttf",
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        "C:\\Windows\\Fonts\\segoeui.ttf",
        "C:\\Windows\\Fonts\\arial.ttf",
    ];
    let mut fonts = egui::FontDefinitions::default();
    let mut loaded = 0;
    for path in CANDIDATES {
        if let Ok(bytes) = std::fs::read(path) {
            let name = format!("fallback{loaded}");
            fonts.font_data.insert(name.clone(), std::sync::Arc::new(egui::FontData::from_owned(bytes)));
            for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                fonts.families.entry(family).or_default().push(name.clone());
            }
            loaded += 1;
        }
    }
    ctx.set_fonts(fonts);
    if loaded == 0 {
        "No system font with Arabic/Hebrew found: those labels may show as boxes.".to_string()
    } else {
        format!("{loaded} fallback font(s) loaded for non-Latin scripts.")
    }
}

/// Demo languages: button labels for the eight directions (N clockwise to NW).
struct Lang {
    name: &'static str,
    rtl: bool,
    digits: HintDigits,
    labels: [&'static str; 8],
}

const LANGS: &[Lang] = &[
    Lang { name: "English", rtl: false, digits: HintDigits::Western,
        labels: ["Copy", "Paste Special", "Redo", "Save As…", "Delete Permanently", "Cut", "Undo Last Change", "Word wrap"] },
    Lang { name: "Русский", rtl: false, digits: HintDigits::Western,
        labels: ["Копировать", "Специальная вставка", "Повторить", "Сохранить как…", "Удалить навсегда", "Вырезать", "Отменить изменение", "Перенос слов"] },
    Lang { name: "עברית", rtl: true, digits: HintDigits::Western,
        labels: ["העתק", "הדבק מיוחד", "בצע שוב", "שמור בשם…", "מחק לצמיתות", "גזור", "בטל שינוי", "גלישת שורות"] },
    Lang { name: "العربية", rtl: true, digits: HintDigits::ArabicIndic,
        labels: ["نسخ", "لصق خاص", "إعادة", "حفظ باسم…", "حذف نهائي", "قص", "تراجع", "التفاف النص"] },
    // Persian: Persian-only letters (پ چ ژ گ), Farsi yeh/keheh (ی ک) and Persian digits.
    Lang { name: "فارسی", rtl: true, digits: HintDigits::Persian,
        labels: ["کپی", "چسباندن ویژه", "انجام مجدد", "ذخیره با نام…", "حذف دائمی", "برش", "واگرد", "شکستن خط"] },
];

struct Demo {
    /// Index into `LANGS`.
    lang: usize,
    font_note: String,
    menu: PieMenu,
    menu_open: bool,
    last_action: String,
    pinned: bool,
    /// True while the pinned menu is the open one.
    pin_active: bool,
    /// True while the open menu was opened by holding Space (not right-click).
    space_open: bool,

    // Mirrored f32 values for SmartFloat settings (sliders need plain f32)
    bg_radius: f32,
    highlight_radius: f32,
    highlight_angle_deg: f32,

    // Button editor: the source of truth; `buttons` is rebuilt from it every frame.
    button_configs: Vec<ButtonConfig>,
    buttons: Vec<PieButton>,

    // Mirror for show threshold slider (only active in OnMovement mode)
    show_threshold: f32,

    // Side-panel search query
    search: String,

    // Mirror for the slide-out start shape slider (kept while "Own start shape" is off)
    slide_start_shape: f32,
    // Mirror for the slide-out fade duration slider (kept while "Fade while sliding" is off)
    slide_fade_ms: u64,
    // Colour picker value for number hints (kept while "Custom colour" is off)
    hint_color: Color32,
    /// Config index of each menu button (see `build_buttons`).
    button_map: Vec<usize>,
    // Number hints on/off, and whether they go inside the buttons or are painted beside
    show_hints: bool,
    hints_inside: bool,
}

impl Demo {
    /// Applies the selected language: labels, mirroring, digit style, and mnemonics.
    /// English restores the hand-picked keys; other languages use generated ones.
    fn apply_language(&mut self) {
        let l = &LANGS[self.lang];
        let defaults = default_button_configs();
        for (i, cfg) in self.button_configs.iter_mut().enumerate() {
            cfg.label = l.labels[i].to_string();
            if self.lang == 0 {
                cfg.mnemonic = defaults[i].mnemonic;
                cfg.key = defaults[i].key.clone();
            } else {
                cfg.mnemonic = MnemonicMode::Auto;
                cfg.key.clear();
            }
        }
        self.menu.settings.mirror_horizontal = l.rtl;
        self.menu.settings.number_hints.digits = l.digits;
    }

    fn new() -> Self {
        let button_configs = default_button_configs();
        let (buttons, button_map) = build_buttons(&button_configs);

        let mut menu = PieMenu::new();
        menu.settings.input.auto_mnemonics = true; // so "Auto" buttons get a letter
        let bg_radius = menu.settings.center_indicator.background_radius.get();
        let highlight_radius = menu.settings.center_indicator.highlight_radius.get();
        let highlight_angle_deg = menu.settings.center_indicator.highlight_angle.to_degrees();

        Self {
            lang: 0,
            font_note: String::new(),
            menu,
            menu_open: false,
            last_action: String::new(),
            pinned: false,
            pin_active: false,
            space_open: false,
            bg_radius,
            highlight_radius,
            highlight_angle_deg,
            button_configs,
            buttons,
            show_threshold: DEFAULT_SHOW_THRESHOLD,
            search: String::new(),
            slide_start_shape: DEFAULT_SLIDE_START_SHAPE,
            slide_fade_ms: 200,
            hint_color: Color32::GRAY,
            button_map,
            show_hints: false,
            hints_inside: true,
        }
    }
}

/// Demo-side defaults for values the library settings don't hold directly.
const DEFAULT_SHOW_THRESHOLD: f32 = 10.0;
const DEFAULT_SLIDE_START_SHAPE: f32 = -1.0; // start the slide as a diamond

// ── Button editor model ─────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum ButtonKind {
    Button,
    Checkbox,
}

/// Editor-side mnemonic choice; `Key` takes its letter from `ButtonConfig::key`.
#[derive(Clone, Copy, PartialEq)]
enum MnemonicMode {
    /// Follow the menu's "Auto mnemonics" setting (`Mnemonic::Auto`).
    Auto,
    /// Explicit letter (`Mnemonic::Key`).
    Key,
    /// Never a mnemonic (`Mnemonic::None`).
    None,
}

#[derive(Clone)]
struct ButtonConfig {
    label: String,
    /// Optional icon (e.g. an emoji) shown before the label.
    icon: String,
    tooltip: String,
    kind: ButtonKind,
    mnemonic: MnemonicMode,
    /// The explicit mnemonic letter when `mnemonic` is `Key`.
    key: String,
    color: Color32,
    /// Checkbox state when `kind` is `Checkbox`.
    checked: bool,
    /// Disabled buttons cannot be selected (`PieButton::with_enabled`).
    enabled: bool,
    /// Absent buttons are left out of the menu entirely (their direction is empty).
    present: bool,
}

const DIR_NAMES: [&str; 8] = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];

fn direction(i: usize) -> PieDirection {
    [
        PieDirection::North, PieDirection::NorthEast, PieDirection::East, PieDirection::SouthEast,
        PieDirection::South, PieDirection::SouthWest, PieDirection::West, PieDirection::NorthWest,
    ][i].clone()
}

/// The demo's standard buttons (one per direction, N clockwise to NW).
fn default_button_configs() -> Vec<ButtonConfig> {
    let b = |label: &str, icon: &str, tooltip: &str, key: Option<char>, (r, g, bl): (u8, u8, u8)| ButtonConfig {
        label: label.into(),
        icon: icon.into(),
        tooltip: tooltip.into(),
        kind: ButtonKind::Button,
        mnemonic: if key.is_some() { MnemonicMode::Key } else { MnemonicMode::Auto },
        key: key.map(String::from).unwrap_or_default(),
        color: Color32::from_rgb(r, g, bl),
        checked: false,
        enabled: true,
        present: true,
    };
    vec![
        b("Copy",               "",   "Ctrl+C",       Some('c'), ( 70, 130, 180)),
        b("Paste Special",      "",   "Ctrl+Shift+V", Some('p'), ( 70, 130, 180)),
        b("Redo",               "",   "Ctrl+Y",       Some('r'), (100, 160, 100)),
        b("Save As…",           "",   "Ctrl+Shift+S", Some('s'), (180, 140,  60)),
        b("Delete Permanently", "🗑", "Del",          Some('d'), (180,  70,  70)),
        b("Cut",                "",   "Ctrl+X",       Some('t'), (180, 100,  60)),
        b("Undo Last Change",   "",   "Ctrl+Z",       Some('u'), (100, 160, 100)),
        ButtonConfig { kind: ButtonKind::Checkbox, ..b("Word wrap", "", "", None, (130, 90, 180)) },
    ]
}

/// Builds the menu's buttons from the editor. The menu resolves the mnemonics itself.
/// The menu's buttons from the present configs, plus each button's config index
/// (button indices shift when a direction is left out).
fn build_buttons(configs: &[ButtonConfig]) -> (Vec<PieButton>, Vec<usize>) {
    let map: Vec<usize> = (0..configs.len()).filter(|&i| configs[i].present).collect();
    let buttons = map.iter().map(|&i| (i, &configs[i]))
        .map(|(i, c)| {
            let b = PieButton::new(direction(i)).with_label(c.label.clone()).with_enabled(c.enabled);
            match (c.mnemonic, c.key.chars().next()) {
                (MnemonicMode::Auto, _)       => b,
                (MnemonicMode::Key, Some(k))  => b.with_mnemonic(k),
                // Key mode with no letter typed yet behaves like None.
                (MnemonicMode::Key, None) | (MnemonicMode::None, _) => b.without_mnemonic(),
            }
        })
        .collect();
    (buttons, map)
}

const ALL_REVEALS: &[RevealAnimation] = &[
    RevealAnimation::None,
    RevealAnimation::Fade,
    RevealAnimation::SlideOut,
];

fn reveal_label(r: RevealAnimation) -> &'static str {
    match r {
        RevealAnimation::None     => "None (instant)",
        RevealAnimation::Fade     => "Fade",
        RevealAnimation::SlideOut => "Slide out",
    }
}

/// Draws a settings section, unless a search query is active and matches
/// neither the section title nor its keywords (both case-insensitive).
fn section(
    ui: &mut egui::Ui,
    query: &str,
    shown: &mut usize,
    title: &str,
    keywords: &str,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    if !query.is_empty()
        && !title.to_lowercase().contains(query)
        && !keywords.to_lowercase().contains(query)
    {
        return;
    }
    *shown += 1;
    ui.heading(title);
    ui.separator();
    add_contents(ui);
    ui.add_space(8.0);
}

/// Small "⟲" button that resets a value; disabled while the value is already the default.
fn reset_button(ui: &mut egui::Ui, is_default: bool) -> bool {
    ui.add_enabled(!is_default, egui::Button::new("⟲").small())
        .on_hover_text("Reset to default")
        .clicked()
}

/// A slider with a reset button beside it. Returns true if the value changed.
fn slider_reset<N: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    value: &mut N,
    default: N,
    range: std::ops::RangeInclusive<N>,
    step: f64,
) -> bool {
    ui.horizontal(|ui| {
        let mut changed = ui.add(egui::Slider::new(value, range).step_by(step)).changed();
        if reset_button(ui, *value == default) {
            *value = default;
            changed = true;
        }
        changed
    })
    .inner
}

/// A button with an optional number hint before or after its label (`true` = after),
/// so the button grows to fit the digit.
fn hint_button(label: impl Into<egui::WidgetText>, hint: Option<(RichText, bool)>, gap: f32) -> egui::Button<'static> {
    let label: egui::WidgetText = label.into();
    match hint {
        Some((h, true))  => egui::Button::new((label, h)).gap(gap),
        Some((h, false)) => egui::Button::new((h, label)).gap(gap),
        None             => egui::Button::new(label),
    }
}

const ALL_HINT_SIDES: &[HintSide] = &[HintSide::Trailing, HintSide::Outside, HintSide::Left, HintSide::Right];

fn hint_side_label(s: HintSide) -> &'static str {
    match s {
        HintSide::Trailing => "After label (reading order)",
        HintSide::Outside => "Outside (away from centre)",
        HintSide::Left    => "Left",
        HintSide::Right   => "Right",
    }
}

const ALL_SHAPES: &[PieMenuHighlightShape] = &[
    PieMenuHighlightShape::None,
    PieMenuHighlightShape::Arc,
    PieMenuHighlightShape::Slice,
    PieMenuHighlightShape::Circle,
    PieMenuHighlightShape::ArcSlice,
    PieMenuHighlightShape::ArcCircle,
    PieMenuHighlightShape::SliceCircle,
    PieMenuHighlightShape::ArcSliceCircle,
];

fn shape_label(s: PieMenuHighlightShape) -> &'static str {
    match s {
        PieMenuHighlightShape::None           => "None",
        PieMenuHighlightShape::Arc            => "Arc",
        PieMenuHighlightShape::Slice          => "Slice",
        PieMenuHighlightShape::Circle         => "Circle",
        PieMenuHighlightShape::ArcSlice       => "Arc + Slice",
        PieMenuHighlightShape::ArcCircle      => "Arc + Circle",
        PieMenuHighlightShape::SliceCircle    => "Slice + Circle",
        PieMenuHighlightShape::ArcSliceCircle => "Arc + Slice + Circle",
    }
}

impl eframe::App for Demo {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let ctx = &ctx;
        // Sampled before the sidebar runs: if a sidebar widget has focus or a popup
        // (e.g. a colour picker) is open, Esc belongs to it and must not also reach the
        // menu, which would dismiss/unpin it. A second Esc then goes to the menu.
        let esc_for_sidebar = ctx.memory(|m| m.focused().is_some()) || egui::Popup::is_any_open(ctx);
        let d = PieMenuSettings::default(); // defaults for the reset buttons
        (self.buttons, self.button_map) = build_buttons(&self.button_configs);
        // Keys in effect, per config (absent buttons have none), for the editor.
        let mut mnemonics: Vec<Option<char>> = vec![None; self.button_configs.len()];
        for (bi, m) in self.menu.mnemonics(&self.buttons).into_iter().enumerate() {
            mnemonics[self.button_map[bi]] = m;
        }
        // Problems (shared key / direction) are shown in red below; the menu stays closed
        // until they are fixed, because debug builds assert on them when it opens.
        let problems = self.menu.check_buttons(&self.buttons);
        let mut bad = [false; 8];
        for p in &problems {
            for i in p.buttons() { bad[self.button_map[i]] = true; }
        }
        egui::Panel::left("settings").min_size(220.0).show(ui, |ui| {
            // ── Search ───────────────────────────────────────────────
            ui.add_space(4.0);
            // One fixed-height row laid out right-to-left: the clear button takes its
            // space first, then the text field fills exactly what is left, so the row
            // never overflows (overflow made the resizable panel grow every frame).
            // The button is always present (just disabled when empty) and the text
            // field has a fixed id, so typing doesn't shift widget ids and drop focus.
            let row = egui::vec2(ui.available_width(), ui.spacing().interact_size.y);
            ui.allocate_ui_with_layout(row, egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let has_text = !self.search.is_empty();
                if ui
                    .add_enabled(has_text, egui::Button::new("✖").small())
                    .on_hover_text("Clear search")
                    .clicked()
                {
                    self.search.clear();
                }
                ui.add(
                    egui::TextEdit::singleline(&mut self.search)
                        .id(egui::Id::new("settings_search"))
                        .hint_text("🔍 Search settings…")
                        .desired_width(f32::INFINITY),
                );
            });
            ui.separator();

            let query = self.search.trim().to_lowercase();
            let mut shown = 0;

            egui::ScrollArea::vertical().show(ui, |ui| {
                section(ui, &query, &mut shown, "Layout", "shape factor diamond circle square radius", |ui| {
                    ui.label("Shape factor  (−1 diamond · 0 circle · +1 square)");
                    slider_reset(ui, &mut self.menu.settings.shape_factor, d.shape_factor, -1.0..=1.0, 0.01);

                    ui.label("Radius");
                    slider_reset(ui, &mut self.menu.settings.layout_radius, d.layout_radius, 40.0..=200.0, 1.0);
                });

                section(ui, &query, &mut shown, "Center background", "radius fill color colour stroke width", |ui| {
                    ui.label("Radius");
                    let def = d.center_indicator.background_radius.get();
                    if slider_reset(ui, &mut self.bg_radius, def, 0.0..=60.0, 0.5) {
                        self.menu.settings.center_indicator.background_radius = SmartFloat::new(self.bg_radius);
                    }

                    ui.label("Fill color");
                    ui.color_edit_button_srgba(&mut self.menu.settings.center_indicator.background_fill_color);

                    ui.label("Stroke width");
                    slider_reset(ui, &mut self.menu.settings.center_indicator.background_stroke.width,
                        d.center_indicator.background_stroke.width, 0.0..=10.0, 0.5);

                    ui.label("Stroke color");
                    ui.color_edit_button_srgba(&mut self.menu.settings.center_indicator.background_stroke.color);
                });

                section(ui, &query, &mut shown, "Highlight",
                    "shape arc slice circle radius angle fill color colour stroke width snap button", |ui| {
                    ui.label("Shape");
                    let current = self.menu.settings.center_indicator.highlight_shape;
                    egui::ComboBox::from_id_salt("shape")
                        .selected_text(shape_label(current))
                        .show_ui(ui, |ui| {
                            for &s in ALL_SHAPES {
                                ui.selectable_value(
                                    &mut self.menu.settings.center_indicator.highlight_shape,
                                    s, shape_label(s),
                                );
                            }
                        });

                    ui.label("Radius");
                    let def = d.center_indicator.highlight_radius.get();
                    if slider_reset(ui, &mut self.highlight_radius, def, 1.0..=60.0, 0.5) {
                        self.menu.settings.center_indicator.highlight_radius = SmartFloat::new(self.highlight_radius);
                    }

                    ui.label("Arc angle (°)");
                    let def = d.center_indicator.highlight_angle.to_degrees();
                    if slider_reset(ui, &mut self.highlight_angle_deg, def, 5.0..=360.0, 1.0) {
                        self.menu.settings.center_indicator.highlight_angle = self.highlight_angle_deg.to_radians();
                    }

                    ui.label("Circle radius");
                    slider_reset(ui, &mut self.menu.settings.center_indicator.highlight_circle_radius,
                        d.center_indicator.highlight_circle_radius, 1.0..=40.0, 0.5);

                    ui.label("Fill color");
                    ui.color_edit_button_srgba(&mut self.menu.settings.center_indicator.highlight_fill_color);

                    ui.label("Stroke width");
                    slider_reset(ui, &mut self.menu.settings.center_indicator.highlight_stroke.width,
                        d.center_indicator.highlight_stroke.width, 0.0..=10.0, 0.5);

                    ui.label("Stroke color");
                    ui.color_edit_button_srgba(&mut self.menu.settings.center_indicator.highlight_stroke.color);

                    ui.checkbox(&mut self.menu.settings.animations.center_highlight_snapping, "Snap to button");
                });

                section(ui, &query, &mut shown, "Reveal animation",
                    "animation fade none instant slide slideout duration start radius shape factor diamond square opacity", |ui| {
                    let current = self.menu.settings.animations.reveal;
                    egui::ComboBox::from_id_salt("reveal")
                        .selected_text(reveal_label(current))
                        .show_ui(ui, |ui| {
                            for &r in ALL_REVEALS {
                                ui.selectable_value(&mut self.menu.settings.animations.reveal, r, reveal_label(r));
                            }
                        });

                    let reveal = self.menu.settings.animations.reveal;
                    if matches!(reveal, RevealAnimation::Fade | RevealAnimation::SlideOut) {
                        ui.label("Duration (ms)");
                        let mut ms = self.menu.settings.animations.reveal_duration.as_millis() as u64;
                        let def = d.animations.reveal_duration.as_millis() as u64;
                        if slider_reset(ui, &mut ms, def, 0..=1000, 1.0) {
                            self.menu.settings.animations.reveal_duration = Duration::from_millis(ms);
                        }
                    }

                    if reveal == RevealAnimation::SlideOut {
                        // Fade first, right under the main duration.
                        let mut fade = self.menu.settings.animations.slide_fade.is_some();
                        if ui.checkbox(&mut fade, "Fade while sliding").changed() {
                            self.menu.settings.animations.slide_fade =
                                fade.then(|| Duration::from_millis(self.slide_fade_ms));
                        }
                        ui.add_enabled_ui(fade, |ui| {
                            ui.label("Fade duration (ms)");
                            let def = d.animations.slide_fade.map_or(200, |f| f.as_millis() as u64);
                            if slider_reset(ui, &mut self.slide_fade_ms, def, 0..=1000, 1.0) {
                                self.menu.settings.animations.slide_fade = Some(Duration::from_millis(self.slide_fade_ms));
                            }
                        });

                        ui.label("Start radius  (0 = from the centre)");
                        slider_reset(ui, &mut self.menu.settings.animations.slide_start_radius,
                            d.animations.slide_start_radius, 0.0..=300.0, 1.0);

                        let mut own_shape = self.menu.settings.animations.slide_start_shape_factor.is_some();
                        if ui.checkbox(&mut own_shape, "Own start shape").changed() {
                            self.menu.settings.animations.slide_start_shape_factor =
                                own_shape.then_some(self.slide_start_shape);
                        }
                        ui.add_enabled_ui(own_shape, |ui| {
                            ui.label("Start shape  (−1 diamond · 0 circle · +1 square)");
                            if slider_reset(ui, &mut self.slide_start_shape, DEFAULT_SLIDE_START_SHAPE, -1.0..=1.0, 0.01) {
                                self.menu.settings.animations.slide_start_shape_factor = Some(self.slide_start_shape);
                            }
                        });
                    }
                });

                section(ui, &query, &mut shown, "Center label",
                    "title text color colour background stroke width", |ui| {
                    ui.checkbox(&mut self.menu.settings.label.display, "Show title");
                    ui.label("Text color");
                    ui.color_edit_button_srgba(&mut self.menu.settings.label.text_color);
                    ui.label("Background color");
                    ui.color_edit_button_srgba(&mut self.menu.settings.label.background_color);
                    ui.label("Stroke width");
                    slider_reset(ui, &mut self.menu.settings.label.background_stroke.width,
                        d.label.background_stroke.width, 0.0..=4.0, 0.5);
                    ui.label("Stroke color");
                    ui.color_edit_button_srgba(&mut self.menu.settings.label.background_stroke.color);
                });

                section(ui, &query, &mut shown, "Show behaviour",
                    "behavior instant movement threshold quick tap double tap", |ui| {
                    let is_instant = matches!(self.menu.settings.show_behavior, ShowBehavior::Instant);
                    egui::ComboBox::from_id_salt("show_behavior")
                        .selected_text(if is_instant { "Instant" } else { "On movement" })
                        .show_ui(ui, |ui| {
                            if ui.selectable_label(is_instant, "Instant").clicked() {
                                self.menu.settings.show_behavior = ShowBehavior::Instant;
                            }
                            if ui.selectable_label(!is_instant, "On movement").clicked() {
                                self.menu.settings.show_behavior =
                                    ShowBehavior::OnMovement { threshold: self.show_threshold };
                            }
                        });

                    if let ShowBehavior::OnMovement { ref mut threshold } = self.menu.settings.show_behavior {
                        ui.label("Threshold (px)");
                        if slider_reset(ui, threshold, DEFAULT_SHOW_THRESHOLD, 0.0..=50.0, 0.5) {
                            self.show_threshold = *threshold;
                        }
                    }

                    if is_instant {
                        ui.label("QuickTap / DoubleTap not available.");
                    }
                });

                section(ui, &query, &mut shown, "Number hints",
                    "number hints digits keys inside beside painted side trailing after outside left right spacing gap color colour", |ui| {
                    ui.checkbox(&mut self.show_hints, "Show number hints");
                    ui.add_enabled_ui(self.show_hints, |ui| {
                        ui.horizontal(|ui| {
                            ui.radio_value(&mut self.hints_inside, true, "Inside button");
                            ui.radio_value(&mut self.hints_inside, false, "Beside (painted)");
                        });

                        ui.label("Side");
                        let current = self.menu.settings.number_hints.side;
                        egui::ComboBox::from_id_salt("hint_side")
                            .selected_text(hint_side_label(current))
                            .show_ui(ui, |ui| {
                                for &s in ALL_HINT_SIDES {
                                    ui.selectable_value(&mut self.menu.settings.number_hints.side, s, hint_side_label(s));
                                }
                            });

                        ui.label("Spacing (px)");
                        slider_reset(ui, &mut self.menu.settings.number_hints.spacing,
                            d.number_hints.spacing, 0.0..=40.0, 0.5);

                        let mut custom = self.menu.settings.number_hints.color.is_some();
                        if ui.checkbox(&mut custom, "Custom colour")
                            .on_hover_text("Off: egui's weak text colour (dimmed). Either way the digit brightens on hover.")
                            .changed()
                        {
                            self.menu.settings.number_hints.color = custom.then_some(self.hint_color);
                        }
                        ui.add_enabled_ui(custom, |ui| {
                            if ui.color_edit_button_srgba(&mut self.hint_color).changed() {
                                self.menu.settings.number_hints.color = Some(self.hint_color);
                            }
                        });
                    });
                });

                section(ui, &query, &mut shown, "Buttons",
                    "buttons label icon emoji tooltip kind checkbox mnemonic mnemonics key auto none colour color", |ui| {
                    ui.checkbox(&mut self.menu.settings.input.auto_mnemonics, "Auto mnemonics")
                        .on_hover_text("Generate mnemonics from labels for buttons set to Auto");
                    if ui.button("Reset buttons").clicked() {
                        self.button_configs = default_button_configs();
                    }
                    for (i, cfg) in self.button_configs.iter_mut().enumerate() {
                        let assigned = mnemonics[i];
                        let title = format!("{} · {}", DIR_NAMES[i], cfg.label);
                        let title: egui::WidgetText = if bad[i] {
                            RichText::new(format!("⚠ {title}")).color(Color32::RED).into()
                        } else {
                            title.into()
                        };
                        // Checkbox left of the header: include the button, or leave its
                        // direction empty. Clicking it doesn't expand/collapse the header.
                        egui::collapsing_header::CollapsingState::load_with_default_open(
                            ui.ctx(), ui.make_persistent_id(("button_cfg", i)), false)
                            .show_header(ui, |ui| {
                                ui.checkbox(&mut cfg.present, "")
                                    .on_hover_text("Include this button in the menu (untick to leave the direction empty)");
                                ui.add_enabled(cfg.present, egui::Label::new(title));
                            })
                            .body(|ui| ui.add_enabled_ui(cfg.present, |ui| {
                                egui::Grid::new(("button_grid", i)).num_columns(2).show(ui, |ui| {
                                    ui.label("Label");
                                    ui.add(egui::TextEdit::singleline(&mut cfg.label).desired_width(130.0));
                                    ui.end_row();

                                    ui.label("Icon");
                                    ui.add(egui::TextEdit::singleline(&mut cfg.icon).desired_width(40.0))
                                        .on_hover_text("Shown before the label, e.g. an emoji");
                                    ui.end_row();

                                    ui.label("Tooltip");
                                    ui.add(egui::TextEdit::singleline(&mut cfg.tooltip).desired_width(130.0));
                                    ui.end_row();

                                    ui.label("Enabled");
                                    ui.checkbox(&mut cfg.enabled, "")
                                        .on_hover_text("Disabled buttons are skipped by mouse, number keys and mnemonics");
                                    ui.end_row();

                                    ui.label("Kind");
                                    ui.horizontal(|ui| {
                                        ui.radio_value(&mut cfg.kind, ButtonKind::Button, "Button");
                                        ui.radio_value(&mut cfg.kind, ButtonKind::Checkbox, "Checkbox");
                                    });
                                    ui.end_row();

                                    ui.label("Mnemonic");
                                    ui.horizontal(|ui| {
                                        egui::ComboBox::from_id_salt(("mnemonic_mode", i))
                                            .width(60.0)
                                            .selected_text(match cfg.mnemonic {
                                                MnemonicMode::Auto => "Auto",
                                                MnemonicMode::Key  => "Key",
                                                MnemonicMode::None => "None",
                                            })
                                            .show_ui(ui, |ui| {
                                                ui.selectable_value(&mut cfg.mnemonic, MnemonicMode::Auto, "Auto")
                                                    .on_hover_text("Follow the menu's Auto mnemonics setting");
                                                ui.selectable_value(&mut cfg.mnemonic, MnemonicMode::Key, "Key");
                                                ui.selectable_value(&mut cfg.mnemonic, MnemonicMode::None, "None");
                                            });
                                        if cfg.mnemonic == MnemonicMode::Key {
                                            let resp = ui.add(egui::TextEdit::singleline(&mut cfg.key).char_limit(1).desired_width(16.0));
                                            if bad[i] {
                                                ui.painter().rect_stroke(
                                                    resp.rect.expand(2.0), 3.0,
                                                    egui::Stroke::new(2.0, Color32::RED),
                                                    egui::StrokeKind::Outside,
                                                );
                                                resp.on_hover_text("⚠ This key is also used by another button");
                                            }
                                        }
                                        let got = assigned.map_or("none".to_string(), |c| c.to_string());
                                        ui.label(RichText::new(format!("→ {got}")).weak())
                                            .on_hover_text("The mnemonic this button actually has");
                                    });
                                    ui.end_row();

                                    ui.label("Colour");
                                    ui.color_edit_button_srgba(&mut cfg.color);
                                    ui.end_row();
                                });
                            }));
                    }
                });

                section(ui, &query, &mut shown, "Language / RTL", "language rtl right to left mirror hebrew arabic persian farsi russian unicode digits", |ui| {
                    let mut changed = false;
                    egui::ComboBox::from_id_salt("lang")
                        .selected_text(LANGS[self.lang].name)
                        .show_ui(ui, |ui| {
                            for (i, l) in LANGS.iter().enumerate() {
                                changed |= ui.selectable_value(&mut self.lang, i, l.name).changed();
                            }
                        });
                    if changed { self.apply_language(); }
                    ui.checkbox(&mut self.menu.settings.mirror_horizontal, "Mirror layout (RTL)");
                    ui.label(RichText::new("egui has no bidi: keep each label single-script.").small());
                    if !self.font_note.is_empty() { ui.label(RichText::new(&self.font_note).small()); }
                });

                if shown == 0 {
                    ui.label(RichText::new("No matching settings.").italics().color(Color32::GRAY));
                }

                ui.separator();
                ui.label(RichText::new(&self.last_action).size(13.0));
            });
        });

        // The menu needs at least one button and no configuration problems.
        let menu_ok = problems.is_empty() && !self.buttons.is_empty();
        if !menu_ok {
            self.menu_open = false;
            self.pin_active = false;
        }
        if !problems.is_empty() {
            egui::Window::new(RichText::new("⚠ Button configuration problem").color(Color32::RED))
                .anchor(egui::Align2::CENTER_TOP, [0.0, 40.0])
                .collapsible(false)
                .resizable(false)
                .frame(egui::Frame::window(&ctx.global_style()).stroke(egui::Stroke::new(2.0, Color32::RED)))
                .show(ctx, |ui| {
                    for p in &problems {
                        ui.colored_label(Color32::LIGHT_RED, format!("• {p}"));
                    }
                    ui.label("The menu is disabled until this is fixed (see the Buttons section).");
                });
            self.menu_open = false;
        }

        egui::CentralPanel::default().show(ui, |ui| {
            // The panel's own area (not its clip rect, which can be larger), so the
            // pinned menu, the result text and the pointer checks share one centre.
            let viewport = ui.max_rect();
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.pinned, "Pin pie-menu");
                ui.label("·  Right-click or hold Space to open  ·  Number keys 1–9 to pick  ·  Esc / 5 to dismiss (Esc also unpins)");
            });
            // Result of the last action: near the top so the menu never covers it, and
            // centred on the same x as the pinned menu.
            let text_top = ui.cursor().min.y + 20.0;
            let text_rect = ui.painter().text(
                egui::pos2(viewport.center().x, text_top),
                egui::Align2::CENTER_TOP,
                &self.last_action,
                egui::FontId::proportional(18.0),
                Color32::LIGHT_GRAY,
            );
            // Pinned menu: the pointer counts only below this top area, so the pin
            // checkbox never hovers or clicks a pie button.
            let menu_area = egui::Rect::from_min_max(
                egui::pos2(viewport.min.x, text_rect.max.y.max(text_top + 18.0)), viewport.max);

            // Unticking the pin closes the pinned menu.
            if !self.pinned && self.pin_active {
                self.pin_active = false;
                self.menu_open = false;
            }
            // Pinned: open once at the centre, visible straight away.
            if self.pinned && menu_ok {
                if !self.pin_active {
                    self.pin_active = true;
                    self.space_open = false;
                    self.menu.open(viewport.center());
                    self.menu.reveal();
                    self.menu_open = true;
                }
                self.menu.position = viewport.center();
            }

            let pointer = ctx.input(|i| i.pointer.latest_pos());
            let pointer_in_viewport = pointer.is_some_and(|p| viewport.contains(p));
            let pointer_in_menu_area = pointer.is_some_and(|p| menu_area.contains(p));
            if !self.pinned && !self.menu_open && menu_ok {
                if let Some(pos) = ctx.input(|i| {
                    i.pointer.secondary_pressed().then(|| i.pointer.latest_pos()).flatten()
                }) {
                    self.menu.open(pos);
                    self.menu_open = true;
                    self.space_open = false;
                } else if pointer_in_viewport
                    && ctx.memory(|m| m.focused().is_none())
                    && ctx.input(|i| i.events.iter().any(|e| matches!(e,
                        egui::Event::Key { key: egui::Key::Space, pressed: true, repeat: false, .. })))
                {
                    // Demo-only shortcut: hold Space like the right button. Only while the
                    // pointer is over the viewport and no sidebar widget has focus.
                    self.menu.open(pointer.unwrap());
                    self.menu_open = true;
                    self.space_open = true;
                }
            }

            if self.menu_open && menu_ok {
                // Pinned: the pointer only counts inside the viewport, below the top bar.
                let mouse_pos = if self.pinned { pointer.filter(|_| pointer_in_menu_area) } else { pointer };
                let key_down = if self.pinned {
                    true
                } else if self.space_open {
                    ctx.input(|i| i.key_down(egui::Key::Space))
                } else {
                    ctx.input(|i| i.pointer.secondary_down())
                };
                // Per-frame input tweaks; the user's settings are restored after `show`.
                // Pinned: don't block the rest of the UI; clicks select only inside the
                // menu area (outside, a click would otherwise dismiss); the pointer never
                // dismisses. Esc is held back while the sidebar owns it.
                let saved_input = self.menu.settings.input.clone();
                self.menu.settings.block_background = !self.pinned;
                let input = &mut self.menu.settings.input;
                if self.pinned {
                    input.select_on_primary_click &= pointer_in_menu_area;
                    input.dismiss_inputs.retain(|d| matches!(d, egui_pie_menu::PieMenuDismissInput::Key(_)));
                }
                if esc_for_sidebar {
                    input.dismiss_inputs.retain(|d| *d != egui_pie_menu::PieMenuDismissInput::Key(egui::Key::Escape));
                }

                // "Beside" mode lets the menu paint the hints itself; "inside" mode draws
                // them from `b.number_hint` in the closure below.
                self.menu.settings.number_hints.show = self.show_hints && !self.hints_inside;
                let hint_gap = self.menu.settings.number_hints.spacing;

                match self.menu.show(ctx, &self.buttons, mouse_pos, key_down, Some("Edit"),
                    |ui, b| {
                        let (idx, hovered) = (b.idx, b.hovered);
                        let cfg = &self.button_configs[self.button_map[idx]];
                        let (bg, fg) = if !b.enabled {
                            (cfg.color.gamma_multiply(0.25), Color32::DARK_GRAY)
                        } else if hovered {
                            (cfg.color, Color32::WHITE)
                        } else {
                            (cfg.color.gamma_multiply(0.6), Color32::LIGHT_GRAY)
                        };
                        // Hint digit in the library's colour (the normal or custom colour,
                        // with more contrast while hovered).
                        let inside = self.show_hints && self.hints_inside;
                        let hint = b.number_hint.as_ref().filter(|_| inside).map(|h| {
                            (RichText::new(&h.text).color(h.color), h.on_right)
                        });

                        // Icon + label, with the mnemonic underlined.
                        let mut label = b.label_text().color(fg);
                        if !cfg.icon.is_empty() {
                            label = label.prefix(format!("{}  ", cfg.icon));
                        }
                        let text: egui::WidgetText = label.into();

                        let resp = match cfg.kind {
                            // Display only: the toggle happens on Selected.
                            ButtonKind::Checkbox => egui::Frame::new()
                                .fill(bg)
                                .corner_radius(6.0)
                                .inner_margin(Vec2::new(10.0, 5.0))
                                .show(ui, |ui| {
                                    ui.spacing_mut().item_spacing.x = hint_gap;
                                    ui.horizontal(|ui| {
                                        if let Some((h, false)) = &hint { ui.label(h.clone()); }
                                        let mut display = cfg.checked;
                                        ui.checkbox(&mut display, text);
                                        if let Some((h, true)) = &hint { ui.label(h.clone()); }
                                    });
                                })
                                .response,
                            ButtonKind::Button => {
                                ui.spacing_mut().button_padding = Vec2::new(10.0, 5.0);
                                ui.add(hint_button(text, hint, hint_gap).fill(bg).corner_radius(6.0))
                            }
                        };
                        if !cfg.tooltip.is_empty() {
                            resp.on_hover_text(&cfg.tooltip);
                        }
                    })
                {
                    PieMenuResponse::Selected(idx) => {
                        let cfg = &mut self.button_configs[self.button_map[idx]];
                        self.last_action = if cfg.kind == ButtonKind::Checkbox {
                            cfg.checked = !cfg.checked;
                            format!("Selected: {} ({})", cfg.label, if cfg.checked { "on" } else { "off" })
                        } else {
                            format!("Selected: {}", cfg.label)
                        };
                        if !self.pinned { self.menu_open = false; }
                    }
                    PieMenuResponse::Dismissed => {
                        // Esc (or 5) unpins; pointer dismissals are disabled while pinned.
                        self.pinned = false;
                        self.pin_active = false;
                        self.last_action = "Dismissed.".to_string();
                        self.menu_open = false;
                    }
                    PieMenuResponse::QuickTap => {
                        self.last_action = "Quick tap — default action.".to_string();
                        if !self.pinned { self.menu_open = false; }
                    }
                    PieMenuResponse::DoubleTap => {
                        self.last_action = "Double tap — alternate action.".to_string();
                        if !self.pinned { self.menu_open = false; }
                    }
                    PieMenuResponse::None => {}
                }
                self.menu.settings.input = saved_input;
            }
        });
    }
}
