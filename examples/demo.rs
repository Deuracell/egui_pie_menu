use eframe::egui;
use egui::{Color32, Rect, RichText, Vec2};
use egui_pie_menu::dev_tool::{SettingsField, SettingsGroup, field_editor, field_label};
use egui_pie_menu::{ButtonProblem, ClusterItem, HintDigits, PieButton, PieDirection, PieMenu, PieMenuResponse, PieMenuSettings, SubPieStyle};

fn main() -> eframe::Result {
    eframe::run_native(
        "egui_pie_menu demo",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1600.0, 900.0]).with_min_inner_size([900.0, 600.0]),
            ..Default::default()
        },
        Box::new(|cc| {
            let font_note = load_system_fonts(&cc.egui_ctx);
            let mut demo = Demo::new(&cc.egui_ctx);
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

/// The tabs of the left-hand panel.
#[derive(Clone, Copy, PartialEq, Eq)]
enum LeftTab {
    /// The menu-wide settings (library `group_editor` sections plus the demo's own rows).
    Menu,
    /// The buttons.
    Buttons,
}

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
    /// The root menu's title ("Menu title" field; empty: no title).
    title: String,
    /// The right-hand code panel is shown.
    show_code: bool,
    /// Which tab of the code panel is selected.
    code_tab: CodeTab,
    /// Which tab of the left panel is selected.
    left_tab: LeftTab,

    // Button editor: the source of truth; `buttons` is rebuilt from it every frame.
    button_configs: Vec<ButtonConfig>,
    /// The root level, sub-pies included (see `build_buttons`).
    buttons: Vec<PieButton>,

    // Search query of the left panel (both tabs)
    search: String,
    /// A message from the last menu-wide change (e.g. which sub-pies a fan-out style left
    /// replace-style), shown on the Menu tab until dismissed.
    notice: Option<String>,

    /// Height of every pie button in the demo (both kinds), in px.
    button_height: f32,
    // Number hints (drawn inside the buttons) on/off
    show_hints: bool,
    /// Marker for sub-pie and back buttons that have no icon of their own.
    icons: MarkerIcons,
    /// The embedded image icons, uploaded once as textures (see `IMAGE_ICONS`).
    images: Vec<egui::TextureHandle>,
    sub_pie_look: SubPieLook,
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

    fn new(ctx: &egui::Context) -> Self {
        let button_configs = default_button_configs();
        let buttons = build_buttons(&button_configs, SubPieStyle::Replace, false);

        let mut menu = PieMenu::new().with_title("Edit");
        menu.settings.input.auto_mnemonics = true; // so "Auto" buttons get a letter

        Self {
            lang: 0,
            font_note: String::new(),
            menu,
            last_action: String::new(),
            pinned: false,
            space_open: false,
            title: "Edit".to_string(),
            show_code: true,
            code_tab: CodeTab::Menu,
            left_tab: LeftTab::Menu,
            button_configs,
            buttons,
            search: String::new(),
            notice: None,
            button_height: DEFAULT_BUTTON_HEIGHT,
            show_hints: false,
            icons: MarkerIcons::default(),
            images: load_icon_images(ctx),
            sub_pie_look: SubPieLook::default(),
        }
    }
}

/// The tabs of the right-hand code panel.
#[derive(Clone, Copy, PartialEq)]
enum CodeTab {
    /// The library-level code (`menu_to_code`).
    Menu,
    /// A sketch of the render closure (`draw_code`).
    Draw,
}

/// The demo's pie buttons are drawn by the demo, so their height is a demo setting.
const DEFAULT_BUTTON_HEIGHT: f32 = 24.0;

/// Where a sub-pie or back marker is drawn on its button.
#[derive(Clone, Copy, PartialEq)]
enum MarkerPlacement {
    /// At the button's far end: the right edge, or the left when the layout is mirrored.
    FarEnd,
    /// Before the label, ahead of the button's icon (the opposite end to `FarEnd`).
    Start,
}

/// How a button that opens a sub-pie is drawn, compared with a plain action button.
#[derive(Clone, Copy, PartialEq, Default)]
enum SubPieLook {
    Same,
    #[default]
    Outlined,
    Dimmer,
}

/// The marker of a sub-pie button and of a back button. It has a slot of its own, so a
/// button's icon never replaces it. Editable in the "Sub-pies" section; empty = no marker.
#[derive(Clone)]
struct MarkerIcons {
    sub_pie: String,
    back: String,
    placement: MarkerPlacement,
}

impl Default for MarkerIcons {
    fn default() -> Self {
        Self { sub_pie: "▶".into(), back: "◀".into(), placement: MarkerPlacement::FarEnd }
    }
}

impl MarkerIcons {
    /// The marker for `cfg`'s kind of button ("" if none).
    fn marker(&self, cfg: &ButtonConfig) -> &str {
        match (cfg.submenu, cfg.back) {
            (true, _) => &self.sub_pie,
            (_, true) => &self.back,
            _ => "",
        }
    }
}

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

/// How a cluster button's items are laid out inside it (the demo's own choice: the library
/// leaves the layout to the render closure).
#[derive(Clone, Copy, PartialEq)]
enum ClusterLayout {
    Row,
    Column,
    /// Two columns.
    Grid,
}

/// One item of a cluster: its label and an optional explicit mnemonic key (empty: Auto).
#[derive(Clone)]
struct ClusterItemConfig {
    label: String,
    key: String,
}

/// A cluster button (`PieButton::cluster`): several clickable items in one direction.
/// An item's number is its index in `items`.
#[derive(Clone)]
struct ClusterConfig {
    items: Vec<ClusterItemConfig>,
    layout: ClusterLayout,
    /// Gap between the items, in px.
    spacing: f32,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        let item = |label: &str| ClusterItemConfig { label: label.into(), key: String::new() };
        Self { items: vec![item("Left"), item("Middle"), item("Right")], layout: ClusterLayout::Row, spacing: 4.0 }
    }
}

/// The kind of icon a button shows before its label.
#[derive(Clone, Copy, PartialEq)]
enum IconKind {
    /// The button's `icon` text (empty: no icon).
    Text,
    /// A procedural icon painted with `egui::Painter`.
    Vector(VectorIcon),
    /// An embedded image (index into `IMAGE_ICONS`), shown as a texture.
    Image(usize),
}

/// The built-in painted icons.
#[derive(Clone, Copy, PartialEq)]
enum VectorIcon { Copy, Paste, Cut, Undo, Redo, Save, Trash, Check }

const ALL_VECTOR_ICONS: &[VectorIcon] = &[
    VectorIcon::Copy, VectorIcon::Paste, VectorIcon::Cut, VectorIcon::Undo,
    VectorIcon::Redo, VectorIcon::Save, VectorIcon::Trash, VectorIcon::Check,
];

impl VectorIcon {
    fn name(self) -> &'static str {
        match self {
            Self::Copy => "Copy", Self::Paste => "Paste", Self::Cut => "Cut", Self::Undo => "Undo",
            Self::Redo => "Redo", Self::Save => "Save", Self::Trash => "Trash", Self::Check => "Check",
        }
    }

    /// Paints the icon into `rect` with shapes only: no fonts, no textures. Coordinates are
    /// fractions of the rect, so it scales to any size.
    fn paint(self, painter: &egui::Painter, rect: Rect, color: Color32, width: f32) {
        use egui::{Shape, Stroke, StrokeKind, pos2};
        let st = Stroke::new(width, color);
        let p = |x: f32, y: f32| pos2(rect.min.x + x * rect.width(), rect.min.y + y * rect.height());
        let r = |x0: f32, y0: f32, x1: f32, y1: f32| Rect::from_min_max(p(x0, y0), p(x1, y1));
        let line = |pts: &[(f32, f32)]| {
            painter.add(Shape::line(pts.iter().map(|&(x, y)| p(x, y)).collect(), st));
        };
        let rad = 0.12 * rect.width();
        // Points on an arc around (cx, cy), angles in radians (egui's y points down).
        let arc = |cx: f32, cy: f32, rr: f32, a0: f32, a1: f32| -> Vec<(f32, f32)> {
            (0..=12).map(|i| {
                let a = a0 + (a1 - a0) * i as f32 / 12.0;
                (cx + rr * a.cos(), cy + rr * a.sin())
            }).collect()
        };
        match self {
            Self::Copy => {
                painter.rect_stroke(r(0.08, 0.08, 0.6, 0.6), rad, st, StrokeKind::Inside);
                painter.rect_stroke(r(0.4, 0.4, 0.92, 0.92), rad, st, StrokeKind::Inside);
            }
            Self::Paste => {
                painter.rect_stroke(r(0.12, 0.16, 0.88, 0.94), rad, st, StrokeKind::Inside);
                painter.rect_filled(r(0.34, 0.04, 0.66, 0.26), rad * 0.5, color);
                line(&[(0.3, 0.5), (0.7, 0.5)]);
                line(&[(0.3, 0.72), (0.7, 0.72)]);
            }
            Self::Cut => {
                // Two crossing blades with a ring handle each.
                line(&[(0.3, 0.7), (0.78, 0.04)]);
                line(&[(0.7, 0.7), (0.22, 0.04)]);
                painter.circle_stroke(p(0.26, 0.82), 0.14 * rect.width(), st);
                painter.circle_stroke(p(0.74, 0.82), 0.14 * rect.width(), st);
            }
            Self::Undo | Self::Redo => {
                // Undo: an arc from the top running clockwise round to the lower right, with
                // an arrowhead at the top pointing left. Redo is the mirror image.
                let m = |(x, y): (f32, f32)| if self == Self::Redo { (1.0 - x, y) } else { (x, y) };
                let pi = std::f32::consts::PI;
                let pts: Vec<_> = arc(0.5, 0.58, 0.34, -0.5 * pi, 0.2 * pi).into_iter().map(m).collect();
                line(&pts);
                let tip = (0.5, 0.24);
                line(&[m((tip.0 + 0.2, tip.1 - 0.18)), m(tip), m((tip.0 + 0.2, tip.1 + 0.18))]);
            }
            Self::Save => {
                painter.rect_stroke(r(0.1, 0.1, 0.9, 0.9), rad * 0.5, st, StrokeKind::Inside);
                painter.rect_stroke(r(0.28, 0.1, 0.72, 0.38), 0.0, st, StrokeKind::Inside);
                painter.rect_stroke(r(0.26, 0.58, 0.74, 0.9), 0.0, st, StrokeKind::Inside);
            }
            Self::Trash => {
                line(&[(0.12, 0.22), (0.88, 0.22)]);
                line(&[(0.38, 0.22), (0.38, 0.1), (0.62, 0.1), (0.62, 0.22)]);
                line(&[(0.2, 0.22), (0.27, 0.92), (0.73, 0.92), (0.8, 0.22)]);
                line(&[(0.42, 0.4), (0.42, 0.76)]);
                line(&[(0.58, 0.4), (0.58, 0.76)]);
            }
            Self::Check => line(&[(0.1, 0.55), (0.38, 0.82), (0.9, 0.2)]),
        }
    }
}

/// The embedded image icons: 24x24 white-on-transparent PNGs, tinted when drawn.
const IMAGE_ICONS: &[(&str, &[u8])] = &[
    ("Circle", include_bytes!("assets/circle.png")),
    ("Notch", include_bytes!("assets/notch.png")),
    ("Triangle", include_bytes!("assets/triangle.png")),
    ("Heart", include_bytes!("assets/heart.png")),
];

/// Decodes the embedded PNGs (with the `image` crate) and uploads each once as a texture.
fn load_icon_images(ctx: &egui::Context) -> Vec<egui::TextureHandle> {
    IMAGE_ICONS
        .iter()
        .map(|(name, bytes)| {
            let img = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
                .expect("embedded icon is a valid PNG")
                .to_rgba8();
            let size = [img.width() as usize, img.height() as usize];
            let color = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
            ctx.load_texture(format!("icon-{name}"), color, egui::TextureOptions::LINEAR)
        })
        .collect()
}

/// Draws a button's non-text icon (vector or image) at `size`, in `color`.
fn add_icon(ui: &mut egui::Ui, kind: IconKind, images: &[egui::TextureHandle], size: f32, color: Color32) {
    match kind {
        IconKind::Text => {}
        IconKind::Vector(v) => {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
            v.paint(ui.painter(), rect, color, (size / 12.0).max(1.0));
        }
        IconKind::Image(i) => {
            if let Some(tex) = images.get(i) {
                ui.add(egui::Image::from_texture(tex).fit_to_exact_size(Vec2::splat(size)).tint(color));
            }
        }
    }
}

#[derive(Clone)]
struct ButtonConfig {
    label: String,
    /// The icon text (e.g. an emoji) shown before the label when `icon_kind` is `Text`.
    icon: String,
    /// Which kind of icon the button shows: the text above, a painted vector icon or an image.
    icon_kind: IconKind,
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
    /// How this button's sub-pie is shown; `None` follows the menu's style.
    style: Option<SubPieStyle>,
    /// A cluster of items instead of a single action (excludes `submenu` and `back`).
    cluster: Option<ClusterConfig>,
}

impl ButtonConfig {
    /// A plain button with a generated mnemonic.
    fn new(label: impl Into<String>, color: Color32) -> Self {
        Self {
            label: label.into(),
            icon: String::new(),
            icon_kind: IconKind::Text,
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
            style: None,
            cluster: None,
        }
    }

    /// Whether this button's sub-pie fans out, given the menu's style.
    fn fans(&self, menu_style: SubPieStyle) -> bool {
        self.submenu && self.style.unwrap_or(menu_style) == SubPieStyle::FanOut
    }

    /// Eight sub-pie buttons in this button's colour for a parent sitting in direction
    /// `dir` (index into `DIR_NAMES`). Five are present, labelled from `labels`, in the
    /// directions a fan-out may use (`PieDirection::fan_out_slots`), so the same sub-pie
    /// works in both styles. Opposite the parent's direction sits a back button, pointing
    /// back the way the sub-pie was entered; a fan-out leaves it out (see `shown`).
    fn sub_pie(&self, dir: usize, labels: [&str; 5]) -> Vec<ButtonConfig> {
        let slots = PieDirection::ALL[dir].fan_out_slots();
        let back_dir = (dir + 4) % 8;
        (0..8)
            .map(|i| {
                if i == back_dir {
                    return ButtonConfig { back: true, ..ButtonConfig::new("Back", self.color) };
                }
                match slots.iter().position(|d| *d == PieDirection::ALL[i]) {
                    Some(slot) => ButtonConfig::new(labels[slot], self.color),
                    None => ButtonConfig { present: false, ..ButtonConfig::new(format!("{} {}", self.label, DIR_NAMES[i]), self.color) },
                }
            })
            .collect()
    }

    /// Generic sub-pie buttons for a button (in direction `dir`) that had none yet.
    fn default_sub_pie(&self, dir: usize) -> Vec<ButtonConfig> {
        let l = |n: usize| format!("{} {n}", self.label);
        self.sub_pie(dir, [&l(1), &l(2), &l(3), &l(4), &l(5)])
    }
}

/// Which icon set the "Icons" section assigns to the default buttons.
enum SampleIcons { Vector, Image, Reset }

/// Assigns icons to the eight root buttons (N clockwise to NW: copy, paste, redo, save,
/// delete, cut, undo, word wrap), or restores the defaults' icons.
fn sample_icons(configs: &mut [ButtonConfig], set: SampleIcons) {
    let defaults = default_button_configs();
    let vector = [VectorIcon::Copy, VectorIcon::Paste, VectorIcon::Redo, VectorIcon::Save,
                  VectorIcon::Trash, VectorIcon::Cut, VectorIcon::Undo, VectorIcon::Check];
    for (i, cfg) in configs.iter_mut().enumerate().take(8) {
        match set {
            SampleIcons::Vector => cfg.icon_kind = IconKind::Vector(vector[i]),
            SampleIcons::Image => cfg.icon_kind = IconKind::Image(i % IMAGE_ICONS.len()),
            SampleIcons::Reset => {
                cfg.icon_kind = defaults[i].icon_kind;
                cfg.icon = defaults[i].icon.clone();
            }
        }
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
    paste.children = paste.sub_pie(1, ["As Text", "Values Only", "Formatting", "As Link", "As Image"]);
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

/// Config indices of a level's buttons, in button order: the present configs. A fan-out
/// (`in_fan`) also leaves out back buttons: it has no room for one (the direction back to
/// the parent is blocked) and closes by moving the pointer back instead.
fn shown(configs: &[ButtonConfig], in_fan: bool) -> impl Iterator<Item = usize> + '_ {
    (0..configs.len()).filter(move |&i| configs[i].present && !(in_fan && configs[i].back))
}

/// Builds one level's buttons from its shown configs (see [`shown`]), sub-pies included.
/// `menu_style` is the menu's sub-pie style and `in_fan` whether this level is a fan-out.
/// The menu resolves the mnemonics itself.
fn build_buttons(configs: &[ButtonConfig], menu_style: SubPieStyle, in_fan: bool) -> Vec<PieButton> {
    shown(configs, in_fan)
        .map(|i| {
            let c = &configs[i];
            let mut b = PieButton::new(PieDirection::ALL[i]).with_label(c.label.clone()).with_enabled(c.enabled);
            if let Some(cl) = &c.cluster {
                // A cluster has no mnemonic or number key of its own; its items declare theirs.
                let items = cl.items.iter().enumerate().map(|(n, it)| {
                    let item = ClusterItem::new(n, it.label.clone());
                    match it.key.chars().next() {
                        Some(k) => item.with_mnemonic(k),
                        None => item,
                    }
                });
                return PieButton::cluster(b.direction).with_label(b.label).with_enabled(b.enabled).with_items(items.collect());
            }
            if c.back {
                b = PieButton { label: b.label, enabled: b.enabled, ..PieButton::back(b.direction) };
            }
            if c.submenu {
                b = b
                    .with_children(build_buttons(&c.children, menu_style, c.fans(menu_style)))
                    .with_sub_pie_style(c.style);
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

/// A sketch of the render closure that draws the demo's buttons, for the "Draw code" tab:
/// one `match` arm per shown button (`b.path` indexes the shown buttons of each level, see
/// [`shown`]). It covers what the library cannot know: colours, tooltips, icons, the checkbox
/// kind and cluster layouts. Hover colours, scale and number hints are left out.
fn draw_code(configs: &[ButtonConfig], menu_style: SubPieStyle) -> String {
    let mut out = String::from(
        "// Sketch of how the demo draws its buttons: colours, tooltips, icons and cluster\n\
         // layouts are the demo's own choices, not library settings.\n\
         |ui, b| match b.path {\n",
    );
    draw_arms(configs, menu_style, false, &mut Vec::new(), &mut out);
    out += "    _ => {}\n}\n";
    out
}

/// Appends the arms of one level (and, recursively, of its sub-pies) to `out`.
fn draw_arms(configs: &[ButtonConfig], menu_style: SubPieStyle, in_fan: bool, path: &mut Vec<usize>, out: &mut String) {
    for (n, i) in shown(configs, in_fan).enumerate() {
        let c = &configs[i];
        path.push(n);
        let route: Vec<String> = path.iter().map(|p| p.to_string()).collect();
        let label = c.label.replace('\n', " ");
        out.push_str(&format!("    [{}] => {{ // {label}\n", route.join(", ")));
        let mut body: Vec<String> = Vec::new();
        let fill = format!("egui::Color32::from_rgb({}, {}, {})", c.color.r(), c.color.g(), c.color.b());
        let tooltip = if c.tooltip.is_empty() { String::new() } else { format!("\n    .on_hover_text({:?})", c.tooltip) };
        if let Some(cl) = &c.cluster {
            let v = format!("egui::vec2({:?}, {:?})", cl.spacing, cl.spacing);
            body.push(format!("// a frame filled with {fill} around the items"));
            body.push(format!("ui.spacing_mut().item_spacing = {v};"));
            let item = |n: usize| format!("b.item({n}, ui.button(b.item_label({n})));");
            let count = cl.items.len();
            match cl.layout {
                ClusterLayout::Row | ClusterLayout::Column => {
                    body.push(format!("ui.{}(|ui| {{", if cl.layout == ClusterLayout::Row { "horizontal" } else { "vertical" }));
                    body.extend((0..count).map(|n| format!("    {}", item(n))));
                    body.push("});".to_string());
                }
                ClusterLayout::Grid => {
                    body.push(format!("egui::Grid::new(\"cluster\").num_columns(2).spacing({v}).show(ui, |ui| {{"));
                    for n in 0..count {
                        body.push(format!("    {}", item(n)));
                        if n % 2 == 1 {
                            body.push("    ui.end_row();".to_string());
                        }
                    }
                    body.push("});".to_string());
                }
            }
        } else {
            match c.icon_kind {
                IconKind::Vector(v) => body.push(format!("// icon: {} (painted vector icon, see VectorIcon::paint in the demo)", v.name())),
                IconKind::Image(k) => body.push(format!("// icon: {} (embedded image, tinted with the text colour)", IMAGE_ICONS[k % IMAGE_ICONS.len()].0)),
                IconKind::Text => {}
            }
            let mut text = "b.label_text()".to_string();
            if c.icon_kind == IconKind::Text && !c.icon.is_empty() {
                text += &format!(".prefix({:?})", format!("{}  ", c.icon));
            }
            if c.submenu {
                body.push("// opens a sub-pie (the demo adds a marker at the far end)".to_string());
            }
            if c.kind == ButtonKind::Checkbox {
                body.push("// the demo toggles `checked` when the button is selected".to_string());
                body.push(format!("ui.checkbox(&mut checked, {text}){tooltip};"));
            } else {
                body.push(format!("ui.add(egui::Button::new({text}).fill({fill})){tooltip};"));
            }
        }
        for line in body.iter().flat_map(|l| l.lines()) {
            out.push_str(&format!("        {line}\n"));
        }
        out.push_str("    }\n");
        if c.submenu {
            draw_arms(&c.children, menu_style, c.fans(menu_style), path, out);
        }
        path.pop();
    }
}

/// The configs along `route` (button indices from the root, as in `ButtonView::path` and
/// `PieMenu::selected_path`), or `None` if the route no longer matches the editor.
fn route_configs<'a>(
    mut configs: &'a [ButtonConfig],
    route: &[usize],
    menu_style: SubPieStyle,
) -> Option<Vec<&'a ButtonConfig>> {
    let mut out = Vec::new();
    let mut in_fan = false;
    for (step, &n) in route.iter().enumerate() {
        let cfg = &configs[shown(configs, in_fan).nth(n)?];
        out.push(cfg);
        if step + 1 < route.len() {
            if !cfg.submenu {
                return None;
            }
            in_fan = cfg.fans(menu_style);
            configs = &cfg.children;
        }
    }
    Some(out)
}

/// The config of the button at the end of `route`, for changing it.
fn config_at_mut<'a>(
    mut configs: &'a mut [ButtonConfig],
    route: &[usize],
    menu_style: SubPieStyle,
) -> Option<&'a mut ButtonConfig> {
    let (&last, parents) = route.split_last()?;
    let mut in_fan = false;
    for &n in parents {
        let i = shown(configs, in_fan).nth(n)?;
        in_fan = configs[i].fans(menu_style);
        configs = &mut configs[i].children;
    }
    let i = shown(configs, in_fan).nth(last)?;
    Some(&mut configs[i])
}

/// Problems (shared key / direction, bad fan-out children) of every level, each prefixed
/// with its sub-pie's name. A fan-out's children are checked as part of their parent's level.
fn collect_problems(menu: &PieMenu, configs: &[ButtonConfig], level: &str, out: &mut Vec<String>) {
    let style = menu.settings.sub_pies.style;
    for p in menu.check_buttons(&build_buttons(configs, style, false)) {
        out.push(if level.is_empty() { p.to_string() } else { format!("{level}: {p}") });
    }
    for c in configs.iter().filter(|c| c.present && c.submenu && !c.fans(style)) {
        let name = if level.is_empty() { c.label.clone() } else { format!("{level} › {}", c.label) };
        collect_problems(menu, &c.children, &name, out);
    }
}

// ── Menu tab rows ───────────────────────────────────────────────────────────

/// A row of the Menu tab that is not a library settings field: it feeds the demo's own
/// drawing (the "Draw code") or the demo itself, and carries a small "demo" tag where it is
/// not part of the menu code.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum DemoRow {
    /// `PieMenu::title` (it is in the menu code, so no tag).
    MenuTitle,
    ShowHints,
    ButtonHeight,
    SubPieLook,
    SampleIcons,
    MarkerPosition,
    SubPieMarker,
    BackMarker,
    /// The language picker and the font note (not tagged: it sets library settings too).
    Language,
}

impl DemoRow {
    const ALL: [DemoRow; 9] = [
        DemoRow::MenuTitle,
        DemoRow::ShowHints,
        DemoRow::ButtonHeight,
        DemoRow::SubPieLook,
        DemoRow::SampleIcons,
        DemoRow::MarkerPosition,
        DemoRow::SubPieMarker,
        DemoRow::BackMarker,
        DemoRow::Language,
    ];

    fn group(self) -> SettingsGroup {
        match self {
            DemoRow::MenuTitle => SettingsGroup::Title,
            DemoRow::ShowHints => SettingsGroup::KeysAndHints,
            DemoRow::ButtonHeight | DemoRow::SubPieLook | DemoRow::SampleIcons => SettingsGroup::ButtonLook,
            DemoRow::MarkerPosition | DemoRow::SubPieMarker | DemoRow::BackMarker => SettingsGroup::SubPies,
            DemoRow::Language => SettingsGroup::Language,
        }
    }

    fn name(self) -> &'static str {
        match self {
            DemoRow::MenuTitle => "Menu title",
            DemoRow::ShowHints => "Show number hints",
            DemoRow::ButtonHeight => "Button height",
            DemoRow::SubPieLook => "Sub-pie button look",
            DemoRow::SampleIcons => "Sample icons",
            DemoRow::MarkerPosition => "Marker position",
            DemoRow::SubPieMarker => "Sub-pie marker",
            DemoRow::BackMarker => "Back marker",
            DemoRow::Language => "Language",
        }
    }

    /// Extra words the search matches besides the name.
    fn keywords(self) -> &'static str {
        match self {
            DemoRow::MenuTitle => "title text centre root",
            DemoRow::ShowHints => "digits keys numbers",
            DemoRow::ButtonHeight => "size",
            DemoRow::SubPieLook => "outline dim folder",
            DemoRow::SampleIcons => "icon icons vector image images procedural painted texture png reset",
            DemoRow::MarkerPosition => "sub-pie back far end start",
            DemoRow::SubPieMarker => "arrow icon",
            DemoRow::BackMarker => "arrow icon",
            DemoRow::Language => "rtl right to left mirror hebrew arabic persian farsi russian unicode digits font",
        }
    }

    /// Whether the row carries the "demo" tag: it feeds the Draw code, not the menu code.
    fn tagged(self) -> bool {
        !matches!(self, DemoRow::MenuTitle | DemoRow::Language)
    }

    /// Like `SettingsField::is_relevant`: the hint row only matters while number keys are on.
    fn is_relevant(self, s: &PieMenuSettings) -> bool {
        match self {
            DemoRow::ShowHints => s.input.use_number_keys,
            _ => true,
        }
    }
}

/// One row of the Menu tab: a library field or one of the demo's own.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Row {
    Field(SettingsField),
    Demo(DemoRow),
}

/// The rows of one section: the fields that apply (see `SettingsGroup::relevant_fields`),
/// then the demo's own rows of that group.
fn group_rows(group: SettingsGroup, settings: &PieMenuSettings) -> Vec<Row> {
    let mut rows: Vec<Row> = group.relevant_fields(settings).into_iter().map(Row::Field).collect();
    rows.extend(DemoRow::ALL.into_iter().filter(|r| r.group() == group && r.is_relevant(settings)).map(Row::Demo));
    rows
}

/// Whether a row matches the (lower-case, non-empty) search `query`: its name, its settings
/// path (`layout.radius`) or its keywords contain it.
fn row_matches(row: Row, query: &str) -> bool {
    let has = |text: &str| text.to_lowercase().contains(query);
    match row {
        Row::Field(f) => has(field_label(f)) || has(f.path()),
        Row::Demo(r) => has(r.name()) || has(r.keywords()),
    }
}

/// The rows of a section that a search shows: all of them while the query is empty or matches
/// the section's title, else only the matching ones.
fn visible_rows(group: SettingsGroup, settings: &PieMenuSettings, query: &str) -> Vec<Row> {
    let rows = group_rows(group, settings);
    if query.is_empty() || group.title().to_lowercase().contains(query) {
        return rows;
    }
    rows.into_iter().filter(|r| row_matches(*r, query)).collect()
}

/// How many rows of the Menu tab a search shows (0 for an empty query): the number in the
/// badge of the tab while the other tab is showing.
fn menu_hit_count(settings: &PieMenuSettings, query: &str) -> usize {
    if query.is_empty() {
        return 0;
    }
    SettingsGroup::ALL.into_iter().map(|g| visible_rows(g, settings, query).len()).sum()
}

/// A small "demo" tag for rows that feed only the demo's own drawing.
fn demo_tag(ui: &mut egui::Ui) {
    ui.label(RichText::new("demo").small().weak())
        .on_hover_text("Used by the demo's Draw code (how this demo draws its buttons); not part of the menu code.");
}

/// A drag value in pixels with a reset button beside it. Returns true if the value changed.
fn demo_px(ui: &mut egui::Ui, value: &mut f32, default: f32, range: std::ops::RangeInclusive<f32>) -> bool {
    let mut changed = ui
        .add(egui::DragValue::new(value).range(range).speed(1.0).fixed_decimals(0).suffix(" px"))
        .changed();
    if ui.add_enabled(*value != default, egui::Button::new("\u{21ba}").small()).on_hover_text("Reset to default").clicked() {
        *value = default;
        changed = true;
    }
    if changed {
        *value = value.round();
    }
    changed
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

/// Turns the first absent direction slot of a level into a sub-pie (`fan`: a fan-out) with
/// the default five children plus back button, and returns its index; `None` when all
/// slots are present. The style is stored only when it differs from `menu_style`.
fn add_sub_pie(configs: &mut [ButtonConfig], fan: bool, menu_style: SubPieStyle) -> Option<usize> {
    let i = configs.iter().position(|c| !c.present)?;
    let color = configs.iter().find(|c| c.present).map_or(Color32::from_rgb(70, 130, 180), |c| c.color);
    let wanted = if fan { SubPieStyle::FanOut } else { SubPieStyle::Replace };
    let label = if fan { "Fan-out" } else { "Sub-pie" };
    configs[i] = ButtonConfig {
        submenu: true,
        style: (wanted != menu_style).then_some(wanted),
        ..ButtonConfig::new(label, color)
    };
    configs[i].children = configs[i].default_sub_pie(i);
    Some(i)
}

/// What the editor of a fan-out's children needs from the parent level.
struct FanInfo {
    /// Direction index of the button that opens the fan.
    dir: usize,
    /// The problems among the fan's children (from the parent's `ButtonProblem::InFanOut`).
    problems: Vec<ButtonProblem>,
}

/// The button editor for one level: a collapsible entry per direction. Buttons with a
/// sub-pie nest the same editor for their children; `fan` is set when this level is a
/// fan-out.
fn button_list_editor(
    ui: &mut egui::Ui,
    menu: &PieMenu,
    icons: &MarkerIcons,
    configs: &mut [ButtonConfig],
    fan: Option<FanInfo>,
) {
    let style = menu.settings.sub_pies.style;
    if fan.is_none() {
        ui.horizontal(|ui| {
            let free = configs.iter().any(|c| !c.present);
            ui.label("Add:");
            for (name, is_fan) in [("sub-pie", false), ("fan-out", true)] {
                if ui.add_enabled(free, egui::Button::new(name)).clicked() {
                    add_sub_pie(configs, is_fan, style);
                }
            }
            if !free {
                ui.label(RichText::new("all 8 directions are used: untick one first").weak().small());
            }
        });
    }
    // This level's buttons as the menu will see them: the mnemonic in effect per config
    // (configs without a button have none) and the configs with a problem, shown in red.
    let buttons = build_buttons(configs, style, fan.is_some());
    let cluster_keys = menu.cluster_mnemonics(&buttons);
    let config_of: Vec<usize> = shown(configs, fan.is_some()).collect();
    let mut button_of: Vec<Option<usize>> = vec![None; configs.len()];
    let mut mnemonics: Vec<Option<char>> = vec![None; configs.len()];
    for (bi, m) in menu.mnemonics(&buttons).into_iter().enumerate() {
        mnemonics[config_of[bi]] = m;
        button_of[config_of[bi]] = Some(bi);
    }
    let problems = match &fan {
        Some(fan) => fan.problems.clone(),
        None => menu.check_buttons(&buttons),
    };
    let mut bad = vec![false; configs.len()];
    for p in &problems {
        for i in p.buttons() {
            bad[config_of[i]] = true;
        }
    }
    for (i, cfg) in configs.iter_mut().enumerate() {
        let assigned = mnemonics[i];
        // In a fan-out: the three directions facing the parent cannot be used, and a back
        // button is left out.
        let note = match &fan {
            Some(f) if PieDirection::ALL[f.dir].blocks_fan_out(PieDirection::ALL[i]) => "  (faces the parent)",
            Some(_) if cfg.back => "  (not in a fan-out)",
            _ => "",
        };
        let marker = match (cfg.submenu, cfg.back) {
            (true, _) if !icons.sub_pie.is_empty() => format!("  {}", icons.sub_pie),
            (_, true) if !icons.back.is_empty() => format!("  {}", icons.back),
            _ => String::new(),
        };
        let title = format!("{} · {}{marker}{note}", DIR_NAMES[i], cfg.label);
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
                let blocked = matches!(&fan, Some(f) if PieDirection::ALL[f.dir].blocks_fan_out(PieDirection::ALL[i]));
                ui.add_enabled(!blocked || cfg.present, egui::Checkbox::new(&mut cfg.present, ""))
                    .on_hover_text("Include this button in the menu (untick to leave the direction empty)");
                ui.add_enabled(cfg.present, egui::Label::new(title));
            })
            .body(|ui| ui.add_enabled_ui(cfg.present, |ui| {
                egui::Grid::new(("button_grid", i)).num_columns(2).show(ui, |ui| {
                    ui.label("Label");
                    ui.add(egui::TextEdit::singleline(&mut cfg.label).desired_width(130.0));
                    ui.end_row();

                    ui.label("Icon");
                    ui.horizontal(|ui| {
                        let kind_name = match cfg.icon_kind {
                            IconKind::Text if cfg.icon.is_empty() => "None",
                            IconKind::Text => "Text",
                            IconKind::Vector(_) => "Vector",
                            IconKind::Image(_) => "Image",
                        };
                        egui::ComboBox::from_id_salt(("icon_kind", i)).selected_text(kind_name).width(70.0).show_ui(ui, |ui| {
                            if ui.selectable_label(kind_name == "None", "None").clicked() {
                                cfg.icon_kind = IconKind::Text;
                                cfg.icon.clear();
                            }
                            if ui.selectable_label(kind_name == "Text", "Text").clicked() {
                                cfg.icon_kind = IconKind::Text;
                                if cfg.icon.is_empty() { cfg.icon = "★".into(); }
                            }
                            if ui.selectable_label(kind_name == "Vector", "Vector").clicked()
                                && !matches!(cfg.icon_kind, IconKind::Vector(_)) {
                                cfg.icon_kind = IconKind::Vector(VectorIcon::Copy);
                            }
                            if ui.selectable_label(kind_name == "Image", "Image").clicked()
                                && !matches!(cfg.icon_kind, IconKind::Image(_)) {
                                cfg.icon_kind = IconKind::Image(0);
                            }
                        });
                        match &mut cfg.icon_kind {
                            IconKind::Text => {
                                ui.add(egui::TextEdit::singleline(&mut cfg.icon).desired_width(40.0))
                                    .on_hover_text("Shown before the label, e.g. an emoji");
                            }
                            IconKind::Vector(v) => {
                                let col = ui.visuals().text_color();
                                let (rect, _) = ui.allocate_exact_size(Vec2::splat(16.0), egui::Sense::hover());
                                v.paint(ui.painter(), rect, col, 1.3);
                                egui::ComboBox::from_id_salt(("icon_vector", i)).selected_text(v.name()).show_ui(ui, |ui| {
                                    for &o in ALL_VECTOR_ICONS {
                                        ui.horizontal(|ui| {
                                            let (rect, _) = ui.allocate_exact_size(Vec2::splat(14.0), egui::Sense::hover());
                                            o.paint(ui.painter(), rect, col, 1.2);
                                            ui.selectable_value(v, o, o.name());
                                        });
                                    }
                                });
                            }
                            IconKind::Image(n) => {
                                egui::ComboBox::from_id_salt(("icon_image", i)).selected_text(IMAGE_ICONS[*n % IMAGE_ICONS.len()].0).show_ui(ui, |ui| {
                                    for (k, (name, _)) in IMAGE_ICONS.iter().enumerate() {
                                        ui.selectable_value(n, k, *name);
                                    }
                                });
                            }
                        }
                    });
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

                    ui.label("Cluster");
                    ui.vertical(|ui| {
                        let mut on = cfg.cluster.is_some();
                        if ui.checkbox(&mut on, "")
                            .on_hover_text("Several clickable items in this one direction. The direction itself highlights nothing and has no number key; a click, or releasing the key over an item, selects it. Try it: tick this, then open the menu. Excludes a sub-pie and a back button.")
                            .changed()
                        {
                            cfg.cluster = on.then(ClusterConfig::default);
                            if on {
                                cfg.back = false;
                                cfg.submenu = false;
                            }
                        }
                        let keys = button_of[i].and_then(|bi| cluster_keys.get(bi));
                        let Some(cl) = &mut cfg.cluster else { return };
                        egui::ComboBox::from_id_salt(("cluster_layout", i))
                            .width(80.0)
                            .selected_text(match cl.layout {
                                ClusterLayout::Row => "Row",
                                ClusterLayout::Column => "Column",
                                ClusterLayout::Grid => "Grid (2 columns)",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut cl.layout, ClusterLayout::Row, "Row");
                                ui.selectable_value(&mut cl.layout, ClusterLayout::Column, "Column");
                                ui.selectable_value(&mut cl.layout, ClusterLayout::Grid, "Grid (2 columns)");
                            });
                        ui.horizontal(|ui| {
                            ui.label("Spacing");
                            demo_px(ui, &mut cl.spacing, 4.0, 0.0..=60.0);
                        });
                        let mut remove = None;
                        for (n, item) in cl.items.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                ui.add(egui::TextEdit::singleline(&mut item.label).desired_width(80.0));
                                ui.add(egui::TextEdit::singleline(&mut item.key).char_limit(1).desired_width(16.0))
                                    .on_hover_text("Mnemonic key for this item; empty: Auto");
                                let got = keys.and_then(|k| k.get(n)).copied().flatten();
                                ui.label(RichText::new(format!("→ {}", got.map_or("none".to_string(), |c| c.to_string()))).weak())
                                    .on_hover_text("The mnemonic this item actually has");
                                if ui.small_button("✖").on_hover_text("Remove this item").clicked() {
                                    remove = Some(n);
                                }
                            });
                        }
                        if let Some(n) = remove {
                            cl.items.remove(n);
                        }
                        if ui.button("+ Add item").clicked() {
                            let label = format!("Item {}", cl.items.len() + 1);
                            cl.items.push(ClusterItemConfig { label, key: String::new() });
                        }
                    });
                    ui.end_row();

                    ui.label("Back button");
                    ui.add_enabled(cfg.cluster.is_none(), egui::Checkbox::without_text(&mut cfg.back))
                        .on_hover_text("Inside a sub-pie, selecting this button returns to the parent pie instead of selecting anything");
                    ui.end_row();

                    ui.label("Sub-pie");
                    ui.add_enabled_ui(cfg.cluster.is_none(), |ui| ui.horizontal(|ui| {
                        if ui.checkbox(&mut cfg.submenu, "")
                            .on_hover_text("Selecting this button opens its own pie of buttons")
                            .changed()
                            && cfg.submenu
                            && cfg.children.is_empty()
                        {
                            cfg.children = cfg.default_sub_pie(i);
                        }
                        // How it is shown: the menu's style (Sub-pies section) or its own.
                        let label = |s: Option<SubPieStyle>| match s {
                            None => "Menu style",
                            Some(SubPieStyle::Replace) => "Replace",
                            Some(SubPieStyle::FanOut) => "Fan-out",
                        };
                        ui.add_enabled_ui(cfg.submenu, |ui| {
                            egui::ComboBox::from_id_salt(("sub_pie_style", i))
                                .width(80.0)
                                .selected_text(label(cfg.style))
                                .show_ui(ui, |ui| {
                                    for s in [None, Some(SubPieStyle::Replace), Some(SubPieStyle::FanOut)] {
                                        ui.selectable_value(&mut cfg.style, s, label(s));
                                    }
                                });
                        });
                    }));
                    ui.end_row();
                });
                if cfg.submenu {
                    // The same editor again, for this button's sub-pie (to any depth). A
                    // fan-out's children get their problems from this level's check.
                    let child_fan = cfg.fans(style).then(|| FanInfo {
                        dir: i,
                        problems: problems
                            .iter()
                            .filter_map(|p| match p {
                                ButtonProblem::InFanOut { parent, problem } if Some(*parent) == button_of[i] => {
                                    Some((**problem).clone())
                                }
                                _ => None,
                            })
                            .collect(),
                    });
                    let heading = if child_fan.is_some() { "Fan-out buttons" } else { "Sub-pie buttons" };
                    egui::CollapsingHeader::new(heading)
                        .id_salt(("sub_pie", i))
                        .show(ui, |ui| button_list_editor(ui, menu, icons, &mut cfg.children, child_fan));
                }
            }));
    }
}

impl Demo {
    /// One of the demo's own rows of the Menu tab.
    fn demo_row(&mut self, ui: &mut egui::Ui, row: DemoRow) {
        let name = |ui: &mut egui::Ui, hover: &str| {
            ui.label(row.name()).on_hover_text(hover);
            if row.tagged() {
                demo_tag(ui);
            }
        };
        match row {
            DemoRow::MenuTitle => {
                ui.horizontal(|ui| {
                    name(ui, "Shown in the centre of the root pie. Empty: no title. Sub-pies show their parent's label.");
                    ui.add(egui::TextEdit::singleline(&mut self.title).desired_width(120.0));
                });
            }
            DemoRow::ShowHints => {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.show_hints, row.name())
                        .on_hover_text("Draw the number-key hints inside the demo's buttons (the library hands them to the render closure).");
                    demo_tag(ui);
                });
            }
            DemoRow::ButtonHeight => {
                ui.horizontal_wrapped(|ui| {
                    name(ui, "Height of every pie button. Content taller than this (e.g. a large icon) still makes a button taller.");
                    demo_px(ui, &mut self.button_height, DEFAULT_BUTTON_HEIGHT, 20.0..=60.0);
                });
            }
            DemoRow::SubPieLook => {
                ui.horizontal_wrapped(|ui| {
                    name(ui, "How buttons that open a sub-pie are drawn, to tell them from actions. Uses ButtonView::opens_sub_pie.");
                    let look_label = |l: SubPieLook| match l {
                        SubPieLook::Same => "Same as the others",
                        SubPieLook::Outlined => "Outlined, paler fill",
                        SubPieLook::Dimmer => "Dimmer fill",
                    };
                    egui::ComboBox::from_id_salt("sub_pie_look")
                        .selected_text(look_label(self.sub_pie_look))
                        .show_ui(ui, |ui| {
                            for l in [SubPieLook::Same, SubPieLook::Outlined, SubPieLook::Dimmer] {
                                ui.selectable_value(&mut self.sub_pie_look, l, look_label(l));
                            }
                        });
                });
            }
            DemoRow::SampleIcons => {
                ui.horizontal_wrapped(|ui| {
                    name(ui, "Assign painted or image icons to the eight root buttons, or restore the defaults.");
                    if ui.button("Vector").clicked() {
                        sample_icons(&mut self.button_configs, SampleIcons::Vector);
                    }
                    if ui.button("Image").clicked() {
                        sample_icons(&mut self.button_configs, SampleIcons::Image);
                    }
                    if ui.button("Reset").clicked() {
                        sample_icons(&mut self.button_configs, SampleIcons::Reset);
                    }
                });
            }
            DemoRow::MarkerPosition => {
                ui.horizontal_wrapped(|ui| {
                    name(ui, "Where the sub-pie and back markers sit on their buttons. Separate from the button's icon, which never replaces them.");
                    let place_label = |p: MarkerPlacement| match p {
                        MarkerPlacement::FarEnd => "Far end of the button",
                        MarkerPlacement::Start => "Start, before the icon",
                    };
                    egui::ComboBox::from_id_salt("marker_placement")
                        .selected_text(place_label(self.icons.placement))
                        .show_ui(ui, |ui| {
                            for p in [MarkerPlacement::FarEnd, MarkerPlacement::Start] {
                                ui.selectable_value(&mut self.icons.placement, p, place_label(p));
                            }
                        });
                });
            }
            DemoRow::SubPieMarker => {
                ui.horizontal(|ui| {
                    name(ui, "Drawn on a button that opens a sub-pie, whatever its icon. Empty: none.");
                    ui.add(egui::TextEdit::singleline(&mut self.icons.sub_pie).desired_width(50.0));
                });
            }
            DemoRow::BackMarker => {
                ui.horizontal(|ui| {
                    name(ui, "Drawn on a back button, whatever its icon. Empty: none.");
                    ui.add(egui::TextEdit::singleline(&mut self.icons.back).desired_width(50.0));
                });
            }
            DemoRow::Language => {
                ui.horizontal(|ui| {
                    name(ui, "Button labels, mirroring (RTL) and digit style of the sample menu.");
                    let mut changed = false;
                    egui::ComboBox::from_id_salt("lang")
                        .selected_text(LANGS[self.lang].name)
                        .show_ui(ui, |ui| {
                            for (i, l) in LANGS.iter().enumerate() {
                                changed |= ui.selectable_value(&mut self.lang, i, l.name).changed();
                            }
                        });
                    if changed {
                        self.apply_language();
                    }
                });
                ui.label(RichText::new("egui has no bidi: keep each label single-script.").small());
                if !self.font_note.is_empty() {
                    ui.label(RichText::new(&self.font_note).small());
                }
            }
        }
    }

    /// The Menu tab: one collapsing section per library `SettingsGroup`, drawn with
    /// `field_editor`, plus the demo's own rows. A search (`query`, lower-case) hides the
    /// rows and sections that do not match.
    fn menu_tab(&mut self, ui: &mut egui::Ui, query: &str) {
        if let Some(text) = self.notice.clone() {
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(Color32::ORANGE, text);
                if ui.small_button("dismiss").clicked() {
                    self.notice = None;
                }
            });
        }
        let mut shown = 0;
        for group in SettingsGroup::ALL {
            let rows = visible_rows(group, &self.menu.settings, query);
            if rows.is_empty() {
                continue;
            }
            shown += 1;
            // Collapsible; Ring and Button look start open. A search opens every matching section.
            let default_open = matches!(group, SettingsGroup::Ring | SettingsGroup::ButtonLook);
            egui::CollapsingHeader::new(RichText::new(group.title()).strong().size(15.0))
                .id_salt(("menu_group", group))
                .default_open(default_open)
                .open((!query.is_empty()).then_some(true))
                .show(ui, |ui| {
                    for row in rows {
                        match row {
                            Row::Field(f) => {
                                field_editor(ui, &mut self.menu.settings, f);
                            }
                            Row::Demo(r) => self.demo_row(ui, r),
                        }
                    }
                });
        }
        if shown == 0 {
            ui.label(RichText::new("No matching settings.").italics().color(ui.visuals().weak_text_color()));
        }
    }

    /// The Buttons tab (the button list of the old demo until the tree takes over).
    fn buttons_tab(&mut self, ui: &mut egui::Ui) {
        if ui.button("Reset buttons").clicked() {
            self.button_configs = default_button_configs();
        }
        button_list_editor(ui, &self.menu, &self.icons, &mut self.button_configs, None);
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
        self.buttons = build_buttons(&self.button_configs, self.menu.settings.sub_pies.style, false);
        // Problems (shared key / direction) in any level are shown in red in the editor;
        // the menu stays closed until they are fixed, because debug builds assert on them
        // when the level opens.
        let mut problems = Vec::new();
        collect_problems(&self.menu, &self.button_configs, "", &mut problems);
        egui::Panel::left("settings").min_size(220.0).show(ui, |ui| {
            ui.add_space(4.0);
            let query = self.search.trim().to_lowercase();
            // Two tabs; the one that is not showing carries the number of search hits.
            let menu_hits = menu_hit_count(&self.menu.settings, &query);
            ui.horizontal(|ui| {
                for (tab, name, hits) in [(LeftTab::Menu, "Menu", menu_hits), (LeftTab::Buttons, "Buttons", 0)] {
                    let label = if !query.is_empty() && self.left_tab != tab && tab == LeftTab::Menu {
                        format!("{name} ({hits})")
                    } else {
                        name.to_string()
                    };
                    ui.selectable_value(&mut self.left_tab, tab, label);
                }
            });
            // ── Search ───────────────────────────────────────────────
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
                        .hint_text("🔍 Search menu and buttons…")
                        .desired_width(f32::INFINITY),
                );
            });
            ui.separator();

            // Solid (non-floating) scrollbar, so it takes its own space instead of covering
            // the reset buttons at the right edge of each row.
            ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
            egui::ScrollArea::vertical().show(ui, |ui| match self.left_tab {
                LeftTab::Menu => self.menu_tab(ui, &query),
                LeftTab::Buttons => self.buttons_tab(ui),
            });
        });

        // The generated code, live. Declared before the central panel, so the central panel
        // (and with it the menu's viewport) shrinks to what both side panels leave.
        if self.show_code {
            egui::Panel::right("code").resizable(true).default_size(440.0).min_size(260.0).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.code_tab, CodeTab::Menu, "Menu code");
                    ui.selectable_value(&mut self.code_tab, CodeTab::Draw, "Draw code");
                });
                ui.separator();
                match self.code_tab {
                    CodeTab::Menu => {
                        let title = (!self.title.is_empty()).then_some(self.title.as_str());
                        let code = egui_pie_menu::dev_tool::menu_to_code(title, &self.menu.settings, &self.buttons);
                        egui_pie_menu::dev_tool::code_view(ui, &code);
                    }
                    CodeTab::Draw => {
                        let code = draw_code(&self.button_configs, self.menu.settings.sub_pies.style);
                        egui_pie_menu::dev_tool::code_view(ui, &code);
                    }
                }
            });
        }

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
                ui.checkbox(&mut self.show_code, "Code panel");
                ui.separator();
                // egui's own theme switch (System / Dark / Light), so the default text colours,
                // which come from the theme, can be checked in both.
                egui::widgets::global_theme_preference_buttons(ui);
                ui.separator();
                ui.label(RichText::new(&self.last_action).weak());
            });
            ui.label(RichText::new("Right-click or hold Space to open  ·  Number keys 1–9 to pick  ·  Esc / 5 to dismiss  ·  Sub-pie buttons carry a marker at their far end (Backspace or centre click: back; a fan-out closes when the pointer returns)  ·  Pinned: only Esc unpins").small().weak());
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

            // The title is the open (replace-style) sub-pie's parent, or the "Menu title" at the root.
            let style = self.menu.settings.sub_pies.style;
            let parent = route_configs(&self.button_configs, self.menu.path(), style).and_then(|r| r.last().copied());
            self.menu.title = match parent {
                Some(c) => Some(c.label.clone()),
                None => (!self.title.is_empty()).then(|| self.title.clone()),
            };
            let configs = &self.button_configs;
            let icons = &self.icons;
            let images = &self.images;
            let sub_pie_look = self.sub_pie_look;

            // Does nothing while the menu is closed; closes itself on any response but
            // `None` (a pinned menu stays open after a selection).
            let response = self.menu.show(ctx, &self.buttons, key_down, |ui, b| {
                // `b.path` is the button's route from the root, whichever ring it is in
                // (the level shown, or a fan-out open beside it).
                let Some(cfg) = route_configs(configs, b.path, style).and_then(|r| r.last().copied()) else {
                    return;
                };
                let hovered = b.hovered;
                // Hover emphasis enlarges the content, not the layer (which would blur the
                // text): scale the minimum height and the padding along with the label.
                let button_h = button_h * b.scale();
                let pad_x = 10.0 * b.scale();
                // The pie's own highlight is the only hover feedback: egui's per-widget
                // hover/press outlines would add a second, inconsistent one on top.
                // (An app drawing plain widgets can call `b.use_pie_hover(ui)` instead.)
                // Not in a cluster: it is never highlighted, and its items are plain
                // widgets with egui's native hover and press look.
                if cfg.cluster.is_none() {
                    let w = &mut ui.visuals_mut().widgets;
                    let (stroke, expansion) = (w.inactive.bg_stroke, w.inactive.expansion);
                    w.hovered.bg_stroke = stroke;
                    w.active.bg_stroke = stroke;
                    w.hovered.expansion = expansion;
                    w.active.expansion = expansion;
                }
                let bg = if !b.enabled {
                    cfg.color.gamma_multiply(0.25)
                } else if hovered {
                    cfg.color
                } else {
                    cfg.color.gamma_multiply(0.6)
                };
                // Buttons that open a sub-pie can look different from actions.
                let (bg, stroke) = if b.opens_sub_pie() && !cfg.back {
                    match sub_pie_look {
                        SubPieLook::Same => (bg, egui::Stroke::NONE),
                        SubPieLook::Outlined => (bg.gamma_multiply(0.3), egui::Stroke::new(1.5, bg)),
                        SubPieLook::Dimmer => (bg.gamma_multiply(0.55), egui::Stroke::NONE),
                    }
                } else {
                    (bg, egui::Stroke::NONE)
                };
                // A cluster: its items in the chosen layout, inside the usual coloured frame.
                // Each is an `egui::Button` registered with `b.item(n, ..)` (n = its index in
                // the config list); the label underlines the item's mnemonic.
                if let Some(cl) = &cfg.cluster {
                    let item = |ui: &mut egui::Ui, n: usize| {
                        let button = egui::Button::new(b.item_label(n)).min_size(Vec2::new(0.0, button_h));
                        b.item(n, ui.add(button));
                    };
                    egui::Frame::new()
                        .fill(bg)
                        .corner_radius(6.0)
                        .inner_margin(Vec2::new(pad_x, 3.0 * b.scale()))
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing = Vec2::splat(cl.spacing);
                            ui.spacing_mut().button_padding = Vec2::new(pad_x * 0.6, 0.0);
                            let count = cl.items.len();
                            match cl.layout {
                                ClusterLayout::Row => {
                                    ui.horizontal(|ui| (0..count).for_each(|n| item(ui, n)));
                                }
                                ClusterLayout::Column => {
                                    ui.vertical(|ui| (0..count).for_each(|n| item(ui, n)));
                                }
                                ClusterLayout::Grid => {
                                    egui::Grid::new(("cluster_grid", b.idx))
                                        .spacing(Vec2::splat(cl.spacing))
                                        .show(ui, |ui| {
                                            for n in 0..count {
                                                item(ui, n);
                                                if n % 2 == 1 {
                                                    ui.end_row();
                                                }
                                            }
                                        });
                                }
                            }
                        });
                    return;
                }
                // Label and digit colours come from the library: egui's widget colours
                // for the hover state, or the forced colour; disabled is greyed out.
                let hint = b.number_hint.as_ref().filter(|_| self.show_hints).map(|h| {
                    (RichText::new(&h.text).color(h.color), h.on_right)
                });

                // Icon + label, with the mnemonic underlined.
                let mut label = b.label_text();
                // Sub-pie and back buttons carry a marker (from the "Sub-pies" section) in a
                // slot of its own: at the far end, or before the icon. The icon is the
                // button's own and never replaces it.
                let marker = icons.marker(cfg);
                let far_end = icons.placement == MarkerPlacement::FarEnd;
                let mut prefix = String::new();
                if !marker.is_empty() && !far_end {
                    prefix += &format!("{marker}  ");
                }
                if cfg.icon_kind == IconKind::Text && !cfg.icon.is_empty() {
                    prefix += &format!("{}  ", cfg.icon);
                }
                if !prefix.is_empty() {
                    label = label.prefix(prefix);
                }
                // Far end: in the label's colour, set apart from the text by a little space.
                let far_marker = (!marker.is_empty() && far_end).then(|| {
                    let text = if b.rtl { format!("{marker}  ") } else { format!("  {marker}") };
                    RichText::new(text).color(b.text_color())
                });
                let text: egui::WidgetText = label.into();
                // A painted or image icon, about the text height; `b.scale()` keeps it crisp
                // under hover emphasis.
                let icon_size = ui.text_style_height(&egui::TextStyle::Body) * b.scale();
                let graphic = cfg.icon_kind != IconKind::Text;
                let icon_color = b.text_color();

                let resp = match cfg.kind {
                    // Display only: the toggle happens on Selected.
                    // Both kinds get the same fixed height ("Button height"): no vertical
                    // padding, content centred in a row of exactly that height.
                    ButtonKind::Checkbox => egui::Frame::new()
                        .fill(bg)
                        .stroke(stroke)
                        .corner_radius(6.0)
                        .inner_margin(Vec2::new(pad_x, 0.0))
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.x = hint_gap;
                            ui.allocate_ui_with_layout(
                                Vec2::new(0.0, button_h),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.set_min_height(button_h);
                                    // Mirrored: the far end is the left.
                                    if b.rtl && let Some(m) = &far_marker { ui.label(m.clone()); }
                                    if let Some((h, false)) = &hint { ui.label(h.clone()); }
                                    let mut display = cfg.checked;
                                    if b.rtl {
                                        // Right-to-left: the box goes right of its text.
                                        ui.label(text);
                                        add_icon(ui, cfg.icon_kind, images, icon_size, icon_color);
                                        ui.add(egui::Checkbox::without_text(&mut display));
                                    } else {
                                        ui.add(egui::Checkbox::without_text(&mut display));
                                        add_icon(ui, cfg.icon_kind, images, icon_size, icon_color);
                                        ui.label(text);
                                    }
                                    if let Some((h, true)) = &hint { ui.label(h.clone()); }
                                    if !b.rtl && let Some(m) = &far_marker { ui.label(m.clone()); }
                                },
                            );
                        })
                        .response,
                    // A painted or image icon: same look and height as a button, laid out by
                    // hand (the icon before the label, after it when mirrored).
                    ButtonKind::Button if graphic => egui::Frame::new()
                        .fill(bg)
                        .stroke(stroke)
                        .corner_radius(6.0)
                        .inner_margin(Vec2::new(pad_x, 0.0))
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.x = hint_gap.max(4.0 * b.scale());
                            ui.allocate_ui_with_layout(
                                Vec2::new(0.0, button_h),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.set_min_height(button_h);
                                    if b.rtl && let Some(m) = &far_marker { ui.label(m.clone()); }
                                    if let Some((h, false)) = &hint { ui.label(h.clone()); }
                                    if b.rtl {
                                        ui.label(text);
                                        add_icon(ui, cfg.icon_kind, images, icon_size, icon_color);
                                    } else {
                                        add_icon(ui, cfg.icon_kind, images, icon_size, icon_color);
                                        ui.label(text);
                                    }
                                    if let Some((h, true)) = &hint { ui.label(h.clone()); }
                                    if !b.rtl && let Some(m) = &far_marker { ui.label(m.clone()); }
                                },
                            );
                        })
                        .response,
                    ButtonKind::Button => {
                        ui.spacing_mut().button_padding = Vec2::new(pad_x, 0.0);
                        let mut button = hint_button(text, hint, hint_gap)
                            .fill(bg)
                            .stroke(stroke)
                            .corner_radius(6.0)
                            .min_size(Vec2::new(0.0, button_h));
                        if let Some(m) = far_marker {
                            // egui lays text out left to right, so a mirrored layout's far
                            // end is the left edge.
                            button = if b.rtl { button.left_text(m) } else { button.right_text(m) };
                        }
                        ui.add(button)
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
                    // A cluster's item: the last element is its number, not a button index.
                    let item = route.split_last().and_then(|(&n, up)| {
                        let cfg = route_configs(&self.button_configs, up, style)?.last().copied()?;
                        Some((up.to_vec(), cfg.cluster.as_ref()?.items.get(n)?.label.clone()))
                    });
                    if let Some((up, label)) = item {
                        let name = route_configs(&self.button_configs, &up, style)
                            .map(|r| r.iter().map(|c| c.label.as_str()).collect::<Vec<_>>().join(" › "))
                            .unwrap_or_default();
                        self.last_action = format!("Selected: {name} › {label}");
                        return;
                    }
                    let name = route_configs(&self.button_configs, &route, style)
                        .map(|r| r.iter().map(|c| c.label.as_str()).collect::<Vec<_>>().join(" › "))
                        .unwrap_or_default();
                    if let Some(cfg) = config_at_mut(&mut self.button_configs, &route, style) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_sub_pie_uses_the_first_absent_slot() {
        let mut cfgs = default_button_configs();
        assert_eq!(add_sub_pie(&mut cfgs, false, SubPieStyle::Replace), None);
        cfgs[3].present = false;
        cfgs[5].present = false;
        assert_eq!(add_sub_pie(&mut cfgs, false, SubPieStyle::Replace), Some(3));
        let c = &cfgs[3];
        assert!(c.present && c.submenu && c.style.is_none());
        assert_eq!(c.label, "Sub-pie");
        let slots = PieDirection::SouthEast.fan_out_slots();
        for (k, ch) in c.children.iter().enumerate() {
            let allowed = slots.contains(&PieDirection::ALL[k]);
            assert_eq!(ch.present, allowed || ch.back, "child {k}");
        }
        assert_eq!(c.children.iter().filter(|c| c.present && !c.back).count(), 5);
        let backs: Vec<usize> = (0..8).filter(|&k| c.children[k].back).collect();
        assert_eq!(backs, vec![7]);
        // Next absent slot; style overrides.
        assert_eq!(add_sub_pie(&mut cfgs, true, SubPieStyle::Replace), Some(5));
        assert_eq!(cfgs[5].style, Some(SubPieStyle::FanOut));
        assert_eq!(cfgs[5].label, "Fan-out");
        cfgs[0].present = false;
        assert_eq!(add_sub_pie(&mut cfgs, true, SubPieStyle::FanOut), Some(0));
        assert_eq!(cfgs[0].style, None);
        cfgs[1].present = false;
        assert_eq!(add_sub_pie(&mut cfgs, false, SubPieStyle::FanOut), Some(1));
        assert_eq!(cfgs[1].style, Some(SubPieStyle::Replace));
        cfgs[2].present = false;
        assert_eq!(add_sub_pie(&mut cfgs, false, SubPieStyle::Replace), Some(2));
        assert_eq!(cfgs[2].style, None);
    }

    #[test]
    fn draw_code_covers_shown_buttons_and_sub_pies() {
        let code = draw_code(&default_button_configs(), SubPieStyle::Replace);
        assert!(code.contains("[0] => { // Copy"));
        assert!(code.contains(".on_hover_text(\"Ctrl+C\")"));
        assert!(code.contains("from_rgb(70, 130, 180)"));
        assert!(code.contains("[1, 0] => {"), "sub-pie children get nested paths");
        assert!(code.contains("prefix(\"🗑  \")"));
        assert!(code.contains("ui.checkbox(&mut checked"));
        assert!(code.contains("// Back"), "a replace sub-pie keeps its back button");
        let fan = draw_code(&default_button_configs(), SubPieStyle::FanOut);
        assert!(!fan.contains("// Back"), "a fan-out omits back buttons");
    }

    #[test]
    fn every_library_field_is_in_one_menu_section() {
        let s = PieMenuSettings::default();
        let mut seen = Vec::new();
        for g in SettingsGroup::ALL {
            for r in group_rows(g, &s) {
                if let Row::Field(f) = r {
                    assert!(!seen.contains(&f), "{f:?} twice");
                    seen.push(f);
                }
            }
        }
        // Number hints are relevant while number keys are on (the default).
        assert!(s.input.use_number_keys);
        let relevant = SettingsField::ALL.iter().filter(|f| f.is_relevant(&s)).count();
        assert_eq!(seen.len(), relevant);
    }

    #[test]
    fn menu_search_counts_rows_and_sections() {
        let s = PieMenuSettings::default();
        assert_eq!(menu_hit_count(&s, ""), 0);
        // A field label, a settings path, a demo row's keywords.
        assert_eq!(visible_rows(SettingsGroup::Ring, &s, "radius").len(), 2, "ring radius and dead zone radius");
        assert!(visible_rows(SettingsGroup::Ring, &s, "layout.radius").contains(&Row::Field(SettingsField::LayoutRadius)));
        assert!(visible_rows(SettingsGroup::ButtonLook, &s, "vector").contains(&Row::Demo(DemoRow::SampleIcons)));
        // A section title shows the whole section.
        assert_eq!(visible_rows(SettingsGroup::Ring, &s, "ring"), group_rows(SettingsGroup::Ring, &s));
        // No match: nothing; the count adds up over the sections.
        assert!(visible_rows(SettingsGroup::Title, &s, "zzz").is_empty());
        assert_eq!(menu_hit_count(&s, "zzz"), 0);
        let total: usize = SettingsGroup::ALL.iter().map(|g| visible_rows(*g, &s, "radius").len()).sum();
        assert_eq!(menu_hit_count(&s, "radius"), total);
        assert!(total >= 4, "ring, centre, highlight, fan-out radius ...");
    }

    #[test]
    fn hint_rows_follow_number_keys() {
        let mut s = PieMenuSettings::default();
        assert!(group_rows(SettingsGroup::KeysAndHints, &s).contains(&Row::Demo(DemoRow::ShowHints)));
        s.input.use_number_keys = false;
        assert!(!group_rows(SettingsGroup::KeysAndHints, &s).contains(&Row::Demo(DemoRow::ShowHints)));
    }
}
