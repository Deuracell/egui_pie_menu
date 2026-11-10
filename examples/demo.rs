use eframe::egui;
use egui::{Color32, RichText, Vec2};
use std::time::Duration;
use egui_pie_menu::{HintDigits, HintSide, PieButton, PieDirection, PieMenu, PieMenuResponse, PieMenuSettings, RevealAnimation, ShowBehavior, SubPieCenter};

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
    last_action: String,
    /// The "Pin pie-menu" checkbox.
    pinned: bool,
    /// True while the open menu was opened by holding Space (not right-click).
    space_open: bool,

    // Mirrored f32 values for SmartFloat settings (drag values need plain f32)
    bg_radius: f32,
    highlight_radius: f32,
    highlight_angle_deg: f32,

    // Button editor: the source of truth; `buttons` is rebuilt from it every frame.
    button_configs: Vec<ButtonConfig>,
    /// The root level, sub-pies included (see `build_buttons`).
    buttons: Vec<PieButton>,

    // Mirror for show threshold drag value (only active in OnMovement mode)
    show_threshold: f32,

    // Side-panel search query
    search: String,

    // Mirror for the slide-out start shape drag value (kept while "Own start shape" is off)
    slide_start_shape: f32,
    // Mirror for the slide-out fade duration drag value (kept while "Fade while sliding" is off)
    slide_fade_ms: u64,
    // Colour picker value for number hints (kept while "Custom colour" is off)
    hint_color: Color32,
    /// Height of every pie button in the demo (both kinds), in px.
    button_height: f32,
    // Drag value for "Intercardinal height" (kept while it is off)
    intercardinal_height: f32,
    // Colour picker value for "Force contrast text" (kept while it is off)
    label_color: Color32,
    // Number hints (drawn inside the buttons) on/off
    show_hints: bool,
    // Drag value for the sub-pie hover delay (kept while "Navigate on hover" is off)
    sub_hover_ms: u64,
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
        let buttons = build_buttons(&button_configs);

        let mut menu = PieMenu::new().with_title("Edit");
        menu.settings.input.auto_mnemonics = true; // so "Auto" buttons get a letter
        let bg_radius = menu.settings.center_indicator.background_radius.unwrap_or(15.0);
        let highlight_radius = menu.settings.center_indicator.highlight_radius.unwrap_or(15.0);
        let highlight_angle_deg = menu.settings.center_indicator.highlight_angle.to_degrees();

        Self {
            lang: 0,
            font_note: String::new(),
            menu,
            last_action: String::new(),
            pinned: false,
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
            hint_color: Color32::WHITE,
            button_height: DEFAULT_BUTTON_HEIGHT,
            intercardinal_height: 0.5,
            label_color: Color32::WHITE,
            show_hints: false,
            sub_hover_ms: DEFAULT_SUB_HOVER_MS,
        }
    }
}

/// Demo-side defaults for values the library settings don't hold directly.
const DEFAULT_SHOW_THRESHOLD: f32 = 10.0;
/// The demo's pie buttons are drawn by the demo, so their height is a demo setting.
const DEFAULT_BUTTON_HEIGHT: f32 = 24.0;
const DEFAULT_SLIDE_START_SHAPE: f32 = -1.0; // start the slide as a diamond
const DEFAULT_SUB_HOVER_MS: u64 = 400;

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
    /// Selecting the button opens `children` as a sub-pie instead of returning it.
    submenu: bool,
    /// The sub-pie's buttons, one per direction like the root list (N clockwise to NW).
    /// Kept while `submenu` is off; filled in the first time it is turned on.
    children: Vec<ButtonConfig>,
    /// A back button (`PieButton::back`): inside a sub-pie it returns to the parent pie.
    back: bool,
}

impl ButtonConfig {
    /// A plain button with a generated mnemonic.
    fn new(label: impl Into<String>, color: Color32) -> Self {
        Self {
            label: label.into(),
            icon: String::new(),
            tooltip: String::new(),
            kind: ButtonKind::Button,
            mnemonic: MnemonicMode::Auto,
            key: String::new(),
            color,
            checked: false,
            enabled: true,
            present: true,
            submenu: false,
            children: Vec::new(),
            back: false,
        }
    }

    /// Eight sub-pie buttons in this button's colour for a parent sitting in direction
    /// `dir` (index into `DIR_NAMES`): the four cardinal ones present, labelled from
    /// `labels` (N, E, S, W), plus a back button opposite the parent's direction, so it
    /// points back the way the sub-pie was entered.
    fn sub_pie(&self, dir: usize, labels: [&str; 4]) -> Vec<ButtonConfig> {
        let back_dir = (dir + 4) % 8;
        (0..8)
            .map(|i| {
                if i == back_dir {
                    return ButtonConfig { back: true, ..ButtonConfig::new("Back", self.color) };
                }
                let label = if i % 2 == 0 { labels[i / 2].to_string() } else { format!("{} {}", self.label, DIR_NAMES[i]) };
                ButtonConfig { present: i % 2 == 0, ..ButtonConfig::new(label, self.color) }
            })
            .collect()
    }

    /// Generic sub-pie buttons for a button (in direction `dir`) that had none yet.
    fn default_sub_pie(&self, dir: usize) -> Vec<ButtonConfig> {
        let l = |d: &str| format!("{} {d}", self.label);
        self.sub_pie(dir, [&l("N"), &l("E"), &l("S"), &l("W")])
    }
}

const DIR_NAMES: [&str; 8] = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];

/// The demo's standard buttons (one per direction, N clockwise to NW).
fn default_button_configs() -> Vec<ButtonConfig> {
    let b = |label: &str, icon: &str, tooltip: &str, key: Option<char>, (r, g, bl): (u8, u8, u8)| ButtonConfig {
        icon: icon.into(),
        tooltip: tooltip.into(),
        mnemonic: if key.is_some() { MnemonicMode::Key } else { MnemonicMode::Auto },
        key: key.map(String::from).unwrap_or_default(),
        ..ButtonConfig::new(label, Color32::from_rgb(r, g, bl))
    };
    // "Paste Special" (NE) opens a sub-pie, with a back button in the opposite direction (SW).
    let mut paste = b("Paste Special", "", "Ctrl+Shift+V", Some('p'), (70, 130, 180));
    paste.submenu = true;
    paste.children = paste.sub_pie(1, ["As Text", "Values Only", "Formatting", "As Link"]);
    vec![
        b("Copy",               "",   "Ctrl+C",       Some('c'), ( 70, 130, 180)),
        paste,
        b("Redo",               "",   "Ctrl+Y",       Some('r'), (100, 160, 100)),
        b("Save As…",           "",   "Ctrl+Shift+S", Some('s'), (180, 140,  60)),
        b("Delete Permanently", "🗑", "Del",          Some('d'), (180,  70,  70)),
        b("Cut",                "",   "Ctrl+X",       Some('t'), (180, 100,  60)),
        b("Undo Last Change",   "",   "Ctrl+Z",       Some('u'), (100, 160, 100)),
        ButtonConfig { kind: ButtonKind::Checkbox, ..b("Word wrap", "", "", None, (130, 90, 180)) },
    ]
}

/// Builds one level's buttons from the present configs, sub-pies included. Button indices
/// skip absent configs (see [`nth_present`]). The menu resolves the mnemonics itself.
fn build_buttons(configs: &[ButtonConfig]) -> Vec<PieButton> {
    configs.iter().enumerate().filter(|(_, c)| c.present)
        .map(|(i, c)| {
            let mut b = PieButton::new(PieDirection::ALL[i]).with_label(c.label.clone()).with_enabled(c.enabled);
            b.back = c.back;
            if c.submenu {
                b = b.with_children(build_buttons(&c.children));
            }
            match (c.mnemonic, c.key.chars().next()) {
                (MnemonicMode::Auto, _)       => b,
                (MnemonicMode::Key, Some(k))  => b.with_mnemonic(k),
                // Key mode with no letter typed yet behaves like None.
                (MnemonicMode::Key, None) | (MnemonicMode::None, _) => b.without_mnemonic(),
            }
        })
        .collect()
}

/// Config index of the `n`th button of a level (absent configs have no button).
fn nth_present(configs: &[ButtonConfig], n: usize) -> Option<usize> {
    configs.iter().enumerate().filter(|(_, c)| c.present).nth(n).map(|(i, _)| i)
}

/// The configs along `route` (button indices from the root, as in `PieMenu::path` and
/// `PieMenu::selected_path`), or `None` if the route no longer matches the editor.
fn route_configs<'a>(mut configs: &'a [ButtonConfig], route: &[usize]) -> Option<Vec<&'a ButtonConfig>> {
    let mut out = Vec::new();
    for (step, &n) in route.iter().enumerate() {
        let cfg = &configs[nth_present(configs, n)?];
        out.push(cfg);
        if step + 1 < route.len() {
            if !cfg.submenu {
                return None;
            }
            configs = &cfg.children;
        }
    }
    Some(out)
}

/// The configs of the sub-pie level reached by `path` (the root level if it is empty or stale).
fn level_configs<'a>(root: &'a [ButtonConfig], path: &[usize]) -> &'a [ButtonConfig] {
    match route_configs(root, path).as_deref() {
        Some([.., parent]) if parent.submenu && parent.children.iter().any(|c| c.present) => &parent.children,
        _ => root,
    }
}

/// The config of the button at the end of `route`, for changing it.
fn config_at_mut<'a>(mut configs: &'a mut [ButtonConfig], route: &[usize]) -> Option<&'a mut ButtonConfig> {
    let (&last, parents) = route.split_last()?;
    for &n in parents {
        let i = nth_present(configs, n)?;
        configs = &mut configs[i].children;
    }
    let i = nth_present(configs, last)?;
    Some(&mut configs[i])
}

/// Problems (shared key / direction) of every level, each prefixed with its sub-pie's name.
fn collect_problems(menu: &PieMenu, configs: &[ButtonConfig], level: &str, out: &mut Vec<String>) {
    for p in menu.check_buttons(&build_buttons(configs)) {
        out.push(if level.is_empty() { p.to_string() } else { format!("{level}: {p}") });
    }
    for c in configs.iter().filter(|c| c.present && c.submenu) {
        let name = if level.is_empty() { c.label.clone() } else { format!("{level} › {}", c.label) };
        collect_problems(menu, &c.children, &name, out);
    }
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
    // Collapsible; Layout and Buttons start open. A search opens every matching section.
    let default_open = matches!(title, "Layout" | "Buttons");
    egui::CollapsingHeader::new(RichText::new(title).strong().size(15.0))
        .id_salt(("section", title))
        .default_open(default_open)
        .open((!query.is_empty()).then_some(true))
        .show(ui, add_contents);
}

/// Small "⟲" button that resets a value; disabled while the value is already the default.
fn reset_button(ui: &mut egui::Ui, is_default: bool) -> bool {
    ui.add_enabled(!is_default, egui::Button::new("⟲").small())
        .on_hover_text("Reset to default")
        .clicked()
}

/// A drag value with a reset button beside it, decimals and drag speed from `step`.
/// Returns true if the value changed.
fn drag_reset<N: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    value: &mut N,
    default: N,
    range: std::ops::RangeInclusive<N>,
    step: f64,
) -> bool {
    // Decimals from the step (1 → 0, 0.01 → 2, …); drag speed: one step per pixel, but
    // at least 1/300 of the range so wide ranges (e.g. 0–1000 ms) stay quick to drag.
    let decimals = (-step.log10()).ceil().max(0.0) as usize;
    let span = range.end().to_f64() - range.start().to_f64();
    let speed = step.max(span / 300.0);
    drag_reset_with(ui, value, default, range, |d| d.speed(speed).fixed_decimals(decimals))
}

/// A drag value in pixels: whole pixels, shown without decimals, "px" suffix.
fn px_drag(ui: &mut egui::Ui, value: &mut f32, default: f32, range: std::ops::RangeInclusive<f32>) -> bool {
    let changed = drag_reset_with(ui, value, default, range, |d| d.speed(1.0).fixed_decimals(0).suffix(" px"));
    if changed {
        *value = value.round();
    }
    changed
}

/// A [`egui::DragValue`] (drag to change, click to type) plus a reset button, with the
/// drag value configured by `configure`. Returns true if the value changed.
fn drag_reset_with<N: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    value: &mut N,
    default: N,
    range: std::ops::RangeInclusive<N>,
    configure: impl for<'s> FnOnce(egui::DragValue<'s>) -> egui::DragValue<'s>,
) -> bool {
    // Full row: the reset button against the right edge, the drag value filling the
    // rest of the sidebar width (item spacing between), so all fields line up.
    let height = ui.spacing().interact_size.y;
    ui.allocate_ui_with_layout(
        Vec2::new(ui.available_width(), height),
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            let mut changed = false;
            if reset_button(ui, *value == default) {
                *value = default;
                changed = true;
            }
            // Blender-style fill: the field's background, then a bar showing where the
            // value sits in its range, then the drag value on top with a transparent
            // background so the bar shows through under the number. The bar is egui's
            // selection colour at full strength while enabled and a neutral grey while
            // disabled (still showing the value's position), so the two states differ in
            // hue, not only in brightness.
            let rect = ui.available_rect_before_wrap();
            let rect = egui::Rect::from_center_size(rect.center(), Vec2::new(rect.width(), height));
            let (lo, hi) = (range.start().to_f64(), range.end().to_f64());
            let fraction = if hi > lo { ((value.to_f64() - lo) / (hi - lo)).clamp(0.0, 1.0) as f32 } else { 0.0 };
            let visuals = ui.visuals().clone();
            let state = if ui.rect_contains_pointer(rect) { &visuals.widgets.hovered } else { &visuals.widgets.inactive };
            let radius = state.corner_radius;
            ui.painter().rect_filled(rect, radius, state.weak_bg_fill);
            if fraction > 0.0 {
                let mut bar = rect;
                bar.set_width(rect.width() * fraction);
                let fill = if ui.is_enabled() { visuals.selection.bg_fill } else { visuals.weak_text_color() };
                ui.painter().rect_filled(bar, radius, fill);
            }
            let drag = configure(egui::DragValue::new(&mut *value).range(range));
            changed |= ui
                .scope(|ui| {
                    let w = &mut ui.visuals_mut().widgets;
                    for s in [&mut w.inactive, &mut w.hovered, &mut w.active] {
                        s.bg_fill = Color32::TRANSPARENT;
                        s.weak_bg_fill = Color32::TRANSPARENT;
                    }
                    ui.add_sized(rect.size(), drag)
                })
                .inner
                .changed();
            changed
        },
    )
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

/// The button editor for one level: a collapsible entry per direction. Buttons with a
/// sub-pie nest the same editor for their children.
fn button_list_editor(ui: &mut egui::Ui, menu: &PieMenu, configs: &mut [ButtonConfig]) {
    // This level's buttons as the menu will see them: the mnemonic in effect per config
    // (absent buttons have none) and the configs with a problem, shown in red.
    let buttons = build_buttons(configs);
    let config_of: Vec<usize> = (0..configs.len()).filter(|&i| configs[i].present).collect();
    let mut mnemonics: Vec<Option<char>> = vec![None; configs.len()];
    for (bi, m) in menu.mnemonics(&buttons).into_iter().enumerate() {
        mnemonics[config_of[bi]] = m;
    }
    let mut bad = vec![false; configs.len()];
    for p in menu.check_buttons(&buttons) {
        for i in p.buttons() {
            bad[config_of[i]] = true;
        }
    }
    for (i, cfg) in configs.iter_mut().enumerate() {
        let assigned = mnemonics[i];
        let title = format!("{} · {}{}", DIR_NAMES[i], cfg.label, if cfg.submenu { "  ▶" } else if cfg.back { "  ◀" } else { "" });
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

                    ui.label("Back button");
                    ui.checkbox(&mut cfg.back, "")
                        .on_hover_text("Inside a sub-pie, selecting this button returns to the parent pie instead of selecting anything");
                    ui.end_row();

                    ui.label("Sub-pie");
                    if ui.checkbox(&mut cfg.submenu, "")
                        .on_hover_text("Selecting this button opens its own pie of buttons (Backspace or a click on the centre goes back)")
                        .changed()
                        && cfg.submenu
                        && cfg.children.is_empty()
                    {
                        cfg.children = cfg.default_sub_pie(i);
                    }
                    ui.end_row();
                });
                if cfg.submenu {
                    // The same editor again, for this button's sub-pie (to any depth).
                    egui::CollapsingHeader::new("Sub-pie buttons")
                        .id_salt(("sub_pie", i))
                        .show(ui, |ui| button_list_editor(ui, menu, &mut cfg.children));
                }
            }));
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
        self.buttons = build_buttons(&self.button_configs);
        // Problems (shared key / direction) in any level are shown in red in the editor;
        // the menu stays closed until they are fixed, because debug builds assert on them
        // when the level opens.
        let mut problems = Vec::new();
        collect_problems(&self.menu, &self.button_configs, "", &mut problems);
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

            // Solid (non-floating) scrollbar, so it takes its own space instead of covering
            // the reset buttons at the right edge of each row.
            ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
            egui::ScrollArea::vertical().show(ui, |ui| {
                section(ui, &query, &mut shown, "Layout", "shape factor diamond circle square radius diagonal intercardinal height", |ui| {
                    ui.label("Radius");
                    px_drag(ui, &mut self.menu.settings.layout_radius, d.layout_radius, 40.0..=200.0);

                    ui.label("Shape factor  (−1 diamond · 0 circle · +1 square)");
                    drag_reset(ui, &mut self.menu.settings.shape_factor, d.shape_factor, -1.0..=1.0, 0.01);

                    let mut own_height = self.menu.settings.intercardinal_height.is_some();
                    if ui.checkbox(&mut own_height, "Intercardinal height")
                        .on_hover_text("Place NE/SE/SW/NW at a fraction of the N/S height (0.5 = halfway between N/S and E/W); they slide along the layout shape. Off: at 45°.")
                        .changed()
                    {
                        self.menu.settings.intercardinal_height = own_height.then_some(self.intercardinal_height);
                    }
                    ui.add_enabled_ui(own_height, |ui| {
                        if drag_reset_with(ui, &mut self.intercardinal_height, 0.5, 0.0..=1.0, |d| d.speed(0.001).fixed_decimals(3)) {
                            self.menu.settings.intercardinal_height = Some(self.intercardinal_height);
                        }
                    });
                });

                section(ui, &query, &mut shown, "Buttons",
                    "buttons button height size text colour color force contrast blender", |ui| {
                    ui.label("Button height")
                        .on_hover_text("Height of every pie button. Content taller than this (e.g. a large icon) still makes a button taller.");
                    px_drag(ui, &mut self.button_height, DEFAULT_BUTTON_HEIGHT, 20.0..=60.0);

                    ui.horizontal(|ui| {
                        let mut forced = self.menu.settings.text_color.is_some();
                        if ui.checkbox(&mut forced, "Force contrast text")
                            .on_hover_text("On: this colour whether hovered or not (Blender-style, white by default). Off: egui's widget colours (dimmer normally, brighter on hover). Disabled buttons are greyed out either way.")
                            .changed()
                        {
                            self.menu.settings.text_color = forced.then_some(self.label_color);
                        }
                        ui.add_enabled_ui(forced, |ui| {
                            if ui.color_edit_button_srgba(&mut self.label_color).changed() {
                                self.menu.settings.text_color = Some(self.label_color);
                            }
                        });
                    });
                });

                section(ui, &query, &mut shown, "Number hints",
                    "number hints digits keys side trailing after outside left right spacing gap color colour force contrast", |ui| {
                    ui.checkbox(&mut self.show_hints, "Show number hints");
                    ui.add_enabled_ui(self.show_hints, |ui| {
                        ui.label("Side");
                        let current = self.menu.settings.number_hints.side;
                        egui::ComboBox::from_id_salt("hint_side")
                            .selected_text(hint_side_label(current))
                            .show_ui(ui, |ui| {
                                for &s in ALL_HINT_SIDES {
                                    ui.selectable_value(&mut self.menu.settings.number_hints.side, s, hint_side_label(s));
                                }
                            });

                        ui.label("Spacing");
                        px_drag(ui, &mut self.menu.settings.number_hints.spacing,
                            d.number_hints.spacing, 0.0..=40.0);

                        let mut custom = self.menu.settings.number_hints.color.is_some();
                        if ui.checkbox(&mut custom, "Force contrast digits")
                            .on_hover_text("On: this colour whether hovered or not (Blender-style). Off: egui's widget colours (dimmer normally, brighter on hover). Independent of the label colour.")
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

                section(ui, &query, &mut shown, "Button list",
                    "buttons list label icon emoji tooltip kind checkbox mnemonic mnemonics key auto none colour color enabled present remove reset sub-pie subpie submenu children nested back return", |ui| {
                    ui.checkbox(&mut self.menu.settings.input.auto_mnemonics, "Auto mnemonics")
                        .on_hover_text("Generate mnemonics from labels for buttons set to Auto");
                    if ui.button("Reset buttons").clicked() {
                        self.button_configs = default_button_configs();
                    }
                    button_list_editor(ui, &self.menu, &mut self.button_configs);
                });

                section(ui, &query, &mut shown, "Sub-pies",
                    "sub-pie subpie submenu centre center pointer parent button hover delay timer automatic open back", |ui| {
                    ui.label("Centre of a sub-pie")
                        .on_hover_text("Where a sub-pie opens: at the pointer, in place of its parent, or at the button that opened it (the back button then points at the parent).");
                    let label = |c: SubPieCenter| match c {
                        SubPieCenter::Pointer => "At the pointer",
                        SubPieCenter::Parent => "Same as the parent",
                        SubPieCenter::Button => "At the opening button",
                    };
                    egui::ComboBox::from_id_salt("sub_pie_center")
                        .selected_text(label(self.menu.settings.sub_pie_center))
                        .show_ui(ui, |ui| {
                            for c in [SubPieCenter::Pointer, SubPieCenter::Parent, SubPieCenter::Button] {
                                ui.selectable_value(&mut self.menu.settings.sub_pie_center, c, label(c));
                            }
                        });

                    let mut auto = self.menu.settings.sub_pie_hover_delay.is_some();
                    if ui.checkbox(&mut auto, "Navigate on hover")
                        .on_hover_text("Resting the pointer on a sub-pie button opens it after this delay, without a click or release; resting on a back button returns to the parent.")
                        .changed()
                    {
                        self.menu.settings.sub_pie_hover_delay = auto.then(|| Duration::from_millis(self.sub_hover_ms));
                    }
                    ui.add_enabled_ui(auto, |ui| {
                        ui.label("Hover delay (ms)");
                        if drag_reset(ui, &mut self.sub_hover_ms, DEFAULT_SUB_HOVER_MS, 0..=2000, 1.0) {
                            self.menu.settings.sub_pie_hover_delay = Some(Duration::from_millis(self.sub_hover_ms));
                        }
                    });
                    ui.label(RichText::new("Give a button a sub-pie or make it a back button in the Button list.").small());
                });

                section(ui, &query, &mut shown, "Center background", "radius fill color colour stroke width", |ui| {
                    ui.label("Radius");
                    let def = d.center_indicator.background_radius.unwrap_or(15.0);
                    if px_drag(ui, &mut self.bg_radius, def, 0.0..=60.0) {
                        self.menu.settings.center_indicator.background_radius = Some(self.bg_radius);
                    }

                    ui.label("Fill color");
                    ui.color_edit_button_srgba(&mut self.menu.settings.center_indicator.background_fill_color);

                    ui.label("Stroke width");
                    px_drag(ui, &mut self.menu.settings.center_indicator.background_stroke.width,
                        d.center_indicator.background_stroke.width, 0.0..=10.0);

                    ui.label("Stroke color");
                    ui.color_edit_button_srgba(&mut self.menu.settings.center_indicator.background_stroke.color);
                });

                section(ui, &query, &mut shown, "Highlight",
                    "shape arc slice circle radius angle fill color colour stroke width snap button", |ui| {
                    ui.label("Shape");
                    ui.horizontal(|ui| {
                        let shape = &mut self.menu.settings.center_indicator.highlight_shape;
                        ui.checkbox(&mut shape.arc, "Arc");
                        ui.checkbox(&mut shape.slice, "Slice");
                        ui.checkbox(&mut shape.circle, "Circle");
                    });

                    ui.label("Radius");
                    let def = d.center_indicator.highlight_radius.unwrap_or(15.0);
                    if px_drag(ui, &mut self.highlight_radius, def, 1.0..=60.0) {
                        self.menu.settings.center_indicator.highlight_radius = Some(self.highlight_radius);
                    }

                    ui.label("Arc angle (°)");
                    let def = d.center_indicator.highlight_angle.to_degrees();
                    if drag_reset(ui, &mut self.highlight_angle_deg, def, 5.0..=360.0, 1.0) {
                        self.menu.settings.center_indicator.highlight_angle = self.highlight_angle_deg.to_radians();
                    }

                    ui.label("Circle radius");
                    px_drag(ui, &mut self.menu.settings.center_indicator.highlight_circle_radius,
                        d.center_indicator.highlight_circle_radius, 1.0..=40.0);

                    ui.label("Fill color");
                    ui.color_edit_button_srgba(&mut self.menu.settings.center_indicator.highlight_fill_color);

                    ui.label("Stroke width");
                    px_drag(ui, &mut self.menu.settings.center_indicator.highlight_stroke.width,
                        d.center_indicator.highlight_stroke.width, 0.0..=10.0);

                    ui.label("Stroke color");
                    ui.color_edit_button_srgba(&mut self.menu.settings.center_indicator.highlight_stroke.color);

                    ui.checkbox(&mut self.menu.settings.center_indicator.highlight_snapping, "Snap to button");
                });

                section(ui, &query, &mut shown, "Center label",
                    "title text color colour background stroke width", |ui| {
                    ui.checkbox(&mut self.menu.settings.title.show, "Show title");
                    ui.label("Text color");
                    ui.color_edit_button_srgba(&mut self.menu.settings.title.text_color);
                    ui.label("Background color");
                    ui.color_edit_button_srgba(&mut self.menu.settings.title.background_color);
                    ui.label("Stroke width");
                    px_drag(ui, &mut self.menu.settings.title.background_stroke.width,
                        d.title.background_stroke.width, 0.0..=4.0);
                    ui.label("Stroke color");
                    ui.color_edit_button_srgba(&mut self.menu.settings.title.background_stroke.color);
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
                        ui.label("Threshold");
                        if px_drag(ui, threshold, DEFAULT_SHOW_THRESHOLD, 0.0..=50.0) {
                            self.show_threshold = *threshold;
                        }
                    }

                    if is_instant {
                        ui.label("QuickTap / DoubleTap not available.");
                    }
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
                        if drag_reset(ui, &mut ms, def, 0..=1000, 1.0) {
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
                            if drag_reset(ui, &mut self.slide_fade_ms, def, 0..=1000, 1.0) {
                                self.menu.settings.animations.slide_fade = Some(Duration::from_millis(self.slide_fade_ms));
                            }
                        });

                        ui.label("Start radius  (0 = from the centre)");
                        px_drag(ui, &mut self.menu.settings.animations.slide_start_radius,
                            d.animations.slide_start_radius, 0.0..=300.0);

                        let mut own_shape = self.menu.settings.animations.slide_start_shape_factor.is_some();
                        if ui.checkbox(&mut own_shape, "Own start shape").changed() {
                            self.menu.settings.animations.slide_start_shape_factor =
                                own_shape.then_some(self.slide_start_shape);
                        }
                        ui.add_enabled_ui(own_shape, |ui| {
                            ui.label("Start shape  (−1 diamond · 0 circle · +1 square)");
                            if drag_reset(ui, &mut self.slide_start_shape, DEFAULT_SLIDE_START_SHAPE, -1.0..=1.0, 0.01) {
                                self.menu.settings.animations.slide_start_shape_factor = Some(self.slide_start_shape);
                            }
                        });
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
                    ui.label(RichText::new("No matching settings.").italics().color(ui.visuals().weak_text_color()));
                }

                ui.separator();
                ui.label(RichText::new(&self.last_action).size(13.0));
            });
        });

        // The menu needs at least one button and no configuration problems.
        let menu_ok = problems.is_empty() && !self.buttons.is_empty();
        if !menu_ok {
            self.menu.close();
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
                    ui.label("The menu is disabled until this is fixed (see the Button list section).");
                });
        }

        egui::CentralPanel::default().show(ui, |ui| {
            // The panel's own area (not its clip rect, which can be larger), so the
            // pinned menu, the result text and the pointer checks share one centre.
            let viewport = ui.max_rect();
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.pinned, "Pin pie-menu");
                ui.separator();
                // egui's own theme switch (System / Dark / Light), so the default text colours,
                // which come from the theme, can be checked in both.
                egui::widgets::global_theme_preference_buttons(ui);
                ui.separator();
                ui.label("Right-click or hold Space to open  ·  Number keys 1–9 to pick  ·  Esc / 5 to dismiss  ·  ▶ opens a sub-pie (◀, Backspace or centre click: back)  ·  Pinned: only Esc unpins");
            });
            // Result of the last action: near the top so the menu never covers it, and
            // centred on the same x as the pinned menu.
            let text_top = ui.cursor().min.y + 20.0;
            let text_rect = ui.painter().text(
                egui::pos2(viewport.center().x, text_top),
                egui::Align2::CENTER_TOP,
                &self.last_action,
                egui::FontId::proportional(18.0),
                ui.visuals().text_color(),
            );

            // Pinned: the menu sits at the centre and stays open. Its pointer area is the
            // viewport below the top bar, so the pin checkbox and the sidebar never hover
            // or click a pie button.
            if self.pinned && menu_ok {
                if !self.menu.is_pinned() {
                    self.menu.pin(viewport.center());
                }
                // The root level follows the viewport; an open sub-pie is placed by the menu
                // (see "Centre of a sub-pie").
                if self.menu.path().is_empty() {
                    self.menu.position = viewport.center();
                }
                self.menu.pointer_area = Some(egui::Rect::from_min_max(
                    egui::pos2(viewport.min.x, text_rect.max.y.max(text_top + 18.0)),
                    viewport.max,
                ));
            } else {
                // Unticking the pin closes the pinned menu.
                if self.menu.is_pinned() {
                    self.menu.close();
                }
                self.menu.pointer_area = None;
            }

            let pointer = ctx.input(|i| i.pointer.latest_pos());
            if !self.pinned && !self.menu.is_open() && menu_ok {
                if let Some(pos) = pointer.filter(|_| ctx.input(|i| i.pointer.secondary_pressed())) {
                    self.menu.open(pos);
                    self.space_open = false;
                } else if let Some(pos) = pointer.filter(|p| viewport.contains(*p))
                    && ctx.memory(|m| m.focused().is_none())
                    && ctx.input(|i| i.events.iter().any(|e| matches!(e,
                        egui::Event::Key { key: egui::Key::Space, pressed: true, repeat: false, .. })))
                {
                    // Demo-only shortcut: hold Space like the right button. Only while the
                    // pointer is over the viewport and no sidebar widget has focus.
                    self.menu.open(pos);
                    self.space_open = true;
                }
            }

            // Esc belongs to the sidebar (see `esc_for_sidebar`): keep it from the menu.
            if esc_for_sidebar && self.menu.is_open() {
                ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
            }

            // The key or button that opened the menu (a pinned menu ignores it).
            let key_down = if self.space_open {
                ctx.input(|i| i.key_down(egui::Key::Space))
            } else {
                ctx.input(|i| i.pointer.secondary_down())
            };
            let hint_gap = self.menu.settings.number_hints.spacing;
            let button_h = self.button_height;

            // The level being shown this frame: the root, or the open sub-pie, whose
            // parent's label becomes the title.
            let path = self.menu.path().to_vec();
            let level = level_configs(&self.button_configs, &path);
            let parent = route_configs(&self.button_configs, &path).and_then(|r| r.last().copied());
            self.menu.title = Some(parent.map_or("Edit", |c| c.label.as_str()).to_string());

            // Does nothing while the menu is closed; closes itself on any response but
            // `None` (a pinned menu stays open after a selection).
            let response = self.menu.show(ctx, &self.buttons, key_down, |ui, b| {
                let (idx, hovered) = (b.idx, b.hovered);
                let cfg = &level[nth_present(level, idx).expect("a button per present config")];
                let bg = if !b.enabled {
                    cfg.color.gamma_multiply(0.25)
                } else if hovered {
                    cfg.color
                } else {
                    cfg.color.gamma_multiply(0.6)
                };
                // Label and digit colours come from the library: egui's widget colours
                // for the hover state, or the forced colour; disabled is greyed out.
                let hint = b.number_hint.as_ref().filter(|_| self.show_hints).map(|h| {
                    (RichText::new(&h.text).color(h.color), h.on_right)
                });

                // Icon + label, with the mnemonic underlined.
                let mut label = b.label_text();
                // Without an icon of their own, sub-pie and back buttons get an arrow.
                let icon = match (cfg.icon.as_str(), cfg.submenu, cfg.back) {
                    ("", true, _) => "▶",
                    ("", _, true) => "◀",
                    (icon, ..) => icon,
                };
                if !icon.is_empty() {
                    label = label.prefix(format!("{icon}  "));
                }
                let text: egui::WidgetText = label.into();

                let resp = match cfg.kind {
                    // Display only: the toggle happens on Selected.
                    // Both kinds get the same fixed height ("Button height"): no vertical
                    // padding, content centred in a row of exactly that height.
                    ButtonKind::Checkbox => egui::Frame::new()
                        .fill(bg)
                        .corner_radius(6.0)
                        .inner_margin(Vec2::new(10.0, 0.0))
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.x = hint_gap;
                            ui.allocate_ui_with_layout(
                                Vec2::new(0.0, button_h),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.set_min_height(button_h);
                                    if let Some((h, false)) = &hint { ui.label(h.clone()); }
                                    let mut display = cfg.checked;
                                    if b.rtl {
                                        // Right-to-left: the box goes right of its text.
                                        ui.label(text);
                                        ui.add(egui::Checkbox::without_text(&mut display));
                                    } else {
                                        ui.checkbox(&mut display, text);
                                    }
                                    if let Some((h, true)) = &hint { ui.label(h.clone()); }
                                },
                            );
                        })
                        .response,
                    ButtonKind::Button => {
                        ui.spacing_mut().button_padding = Vec2::new(10.0, 0.0);
                        ui.add(hint_button(text, hint, hint_gap)
                            .fill(bg)
                            .corner_radius(6.0)
                            .min_size(Vec2::new(0.0, button_h)))
                    }
                };
                if !cfg.tooltip.is_empty() {
                    resp.on_hover_text(&cfg.tooltip);
                }
            });
            match response {
                PieMenuResponse::Selected(_) => {
                    // The full route from the root: "Paste Special › As Text".
                    let route = self.menu.selected_path().to_vec();
                    let name = route_configs(&self.button_configs, &route)
                        .map(|r| r.iter().map(|c| c.label.as_str()).collect::<Vec<_>>().join(" › "))
                        .unwrap_or_default();
                    if let Some(cfg) = config_at_mut(&mut self.button_configs, &route) {
                        self.last_action = if cfg.kind == ButtonKind::Checkbox {
                            cfg.checked = !cfg.checked;
                            format!("Selected: {name} ({})", if cfg.checked { "on" } else { "off" })
                        } else {
                            format!("Selected: {name}")
                        };
                    }
                }
                PieMenuResponse::Dismissed => {
                    // While pinned only Esc gets here, and it unpins.
                    self.pinned = false;
                    self.last_action = "Dismissed.".to_string();
                }
                PieMenuResponse::QuickTap => self.last_action = "Quick tap — default action.".to_string(),
                PieMenuResponse::DoubleTap => self.last_action = "Double tap — alternate action.".to_string(),
                PieMenuResponse::None => {}
            }
        });
    }
}
