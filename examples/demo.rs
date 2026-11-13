use eframe::egui;
use egui::{Color32, Rect, RichText, Vec2};
use egui_pie_menu::dev_tool::{
    BindTarget, Bindings, Selection, Variables, button_tree_marked, SettingsField, SettingsGroup, TreeEdit, binding_details, binding_toggle, button_inspector_bound,
    field_editor_bound, field_label, preview_target, relevant_fields, selection_kind,
    set_menu_sub_pie_style,
};
use egui_pie_menu::catalog::{MenuCatalog, MenuDefinition};
use egui_pie_menu::{
    Actions, ButtonIds, ButtonView, IdKey, unhandled_color, HintDigits, Mnemonic, PieButton, PieButtonKind, PieDirection, PieMenu, PieMenuResponse, PieMenuSettings,
    SubPieStyle,
};
use std::collections::HashMap;

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

/// The menu title in each of `LANGS`.
const TITLES: [&str; 5] = ["Edit", "Правка", "עריכה", "تحرير", "ویرایش"];

/// Translation keys of the root labels (N clockwise to NW), for the `tr("..")` variables.
const LABEL_KEYS: [&str; 8] = ["copy", "paste_special", "redo", "save_as", "delete", "cut", "undo", "word_wrap"];

/// What the demo offers the dev tool to bind to, as an app would: `theme.*` from egui's
/// current visuals (so the Dark / Light switch changes them) and `i18n.*` for the labels and
/// the title in the current language, which generated code reads with `tr("..")`.
fn app_variables(visuals: &egui::Visuals, lang: usize) -> Variables {
    let mut vars = Variables::new();
    vars.color("theme.accent", visuals.selection.bg_fill)
        .color("theme.text", visuals.strong_text_color())
        .color("theme.weak_text", visuals.weak_text_color())
        .color("theme.panel", visuals.panel_fill)
        .color("theme.faint", visuals.faint_bg_color)
        .color("theme.warn", visuals.warn_fg_color)
        .stroke("theme.outline", egui::Stroke::new(2.0, visuals.selection.stroke.color))
        .font("theme.title_font", egui::FontId::proportional(18.0));
    vars.text("i18n.edit", TITLES[lang]).code("tr(\"edit\")");
    for (key, label) in LABEL_KEYS.iter().zip(LANGS[lang].labels) {
        vars.text(format!("i18n.{key}"), label).code(format!("tr(\"{key}\")"));
    }
    vars
}

/// The actions the demo "handles": the ids of the example menus in `examples/menus`, except
/// `plugin.share.print`, which the example plugin offers but the demo does not handle (to show
/// how such a button looks). A program registers its own (see [`demo_actions`]).
const ACTIONS: &[&str] = &[
    "edit.copy",
    "edit.select_all", // not in the example menus: user.toml adds a button for it
    "edit.paste_special",
    "edit.redo",
    "edit.cut",
    "edit.undo",
    "format.bold",
    "format.italic",
    "format.underline",
    "paste.values",
    "paste.formatting",
    "paste.link",
    "plugin.share",
    "plugin.paste_image",
    "plugin.share.email",
    "plugin.share.chat",
    "plugin.share.link",
];

/// The demo's actions with their state, as a program would register them: Undo and Redo
/// only while the pretend history allows, the Share plugin's only while it is switched on.
fn demo_actions(history: &History, share_plugin: bool) -> Actions {
    let mut actions = Actions::new();
    for id in ACTIONS {
        actions.add(*id);
    }
    actions.add("edit.undo").enabled(history.done > 0);
    actions.add("edit.redo").enabled(history.undone > 0);
    for id in ACTIONS.iter().filter(|id| id.starts_with("plugin.share")) {
        actions.add(*id).visible(share_plugin);
    }
    actions
}

/// A pretend undo history, run by the menu's actions (by id): Undo and Redo are enabled only
/// while there is something to undo or redo ([`demo_actions`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct History {
    /// Actions that can be undone.
    done: usize,
    /// Undone actions that can be redone.
    undone: usize,
}

impl History {
    /// Follows an action picked from the menu.
    fn run(&mut self, id: &str) {
        match id {
            "edit.undo" if self.done > 0 => {
                self.done -= 1;
                self.undone += 1;
            }
            "edit.redo" if self.undone > 0 => {
                self.undone -= 1;
                self.done += 1;
            }
            "edit.undo" | "edit.redo" => {}
            _ => {
                self.done += 1;
                self.undone = 0;
            }
        }
    }
}

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

    /// The buttons: the library's tree (sub-pies, fan-outs and clusters inside), the source of
    /// truth for the menu and for the generated code.
    buttons: Vec<PieButton>,
    /// What the demo adds to each button for drawing it, by path (see [`Decor`]).
    decor: DecorMap,
    /// The node selected in the button tree.
    selection: Selection,
    /// Values bound to expressions of your app (the chain buttons): the menu code uses the
    /// expression, the demo goes on showing the value.
    bindings: Bindings,
    /// The `id` of each button, by path (the "Id" row of the inspector; saved in the TOML).
    ids: ButtonIds,
    /// The menu's name in the TOML files.
    menu_name: String,
    /// The TOML files to open, separated by commas (merged in order); Save writes the first.
    toml_files: String,
    /// What the last Open or Save did.
    toml_note: String,
    /// The pretend undo history behind `app.can_undo` / `app.can_redo`.
    history: History,
    /// The "Share plugin on" switch: the plugin's actions are visible only while it is on.
    share_plugin: bool,
    /// The buttons and cluster items whose id the demo does not handle (from the last
    /// `Actions::apply`): marked in the tree and drawn in the error colour.
    unhandled: Vec<IdKey>,
    /// What the canvas shows.
    mode: Mode,
    /// The last tree without problems: what Configure mode previews.
    snapshot: Vec<PieButton>,
    /// The preview menus of Configure mode, one per level of the tree (by the path of the
    /// sub-pie button whose children form it; empty: the root level).
    previews: HashMap<Vec<usize>, PieMenu>,

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
        let (defaults, _) = default_tree();
        for button in &mut self.buttons {
            // The root buttons are labelled by direction (N clockwise to NW).
            let d = dir_index(button.direction);
            button.label = l.labels[d].to_string();
            if button.is_cluster() {
                continue;
            }
            button.mnemonic = if self.lang == 0 {
                defaults.iter().find(|b| b.direction == button.direction).map_or(Mnemonic::Auto, |b| b.mnemonic)
            } else {
                Mnemonic::Auto
            };
        }
        self.menu.settings.mirror_horizontal = l.rtl;
        self.menu.settings.number_hints.digits = l.digits;
    }

    /// The menu being edited, as a definition for the TOML files.
    fn definition(&self) -> MenuDefinition {
        MenuDefinition {
            name: self.menu_name.clone(),
            title: (!self.title.is_empty()).then(|| self.title.clone()),
            settings: self.menu.settings.clone(),
            buttons: self.buttons.clone(),
            ids: self.ids.clone(),
            bindings: self.bindings.clone(),
            ..MenuDefinition::default()
        }
    }

    /// Replaces the menu being edited with `def` (the demo's own decorations start afresh).
    fn load_definition(&mut self, def: MenuDefinition) {
        self.menu_name = def.name;
        self.title = def.title.unwrap_or_default();
        self.menu.settings = def.settings;
        self.buttons = def.buttons;
        self.ids = def.ids;
        let variables = self.bindings.variables().clone();
        self.bindings = def.bindings;
        self.bindings.set_variables(variables);
        self.decor.clear();
        self.selection = Selection::root();
        self.previews.clear();
    }

    /// The comma-separated file names of the TOML tab.
    fn toml_paths(&self) -> Vec<String> {
        self.toml_files.split(',').map(str::trim).filter(|p| !p.is_empty()).map(str::to_string).collect()
    }

    /// Opens every file of the TOML tab into one catalog (later files add to and override
    /// earlier ones) and builds the menu named in the tab (else the first one).
    fn open_toml(&mut self) {
        let mut catalog = MenuCatalog::new();
        for path in self.toml_paths() {
            if let Err(e) = catalog.add_file(&path) {
                self.toml_note = e.to_string();
                return;
            }
        }
        let names: Vec<String> = catalog.menu_names().map(str::to_string).collect();
        let Some(name) = names.iter().find(|n| **n == self.menu_name).or(names.first()).cloned() else {
            self.toml_note = "The files define no menu.".to_string();
            return;
        };
        match catalog.build(&name) {
            Ok(def) => {
                // What the program would log: references it does not answer.
                let vars = app_variables(&egui::Visuals::dark(), self.lang);
                let problems = def.check_references(&vars, &demo_actions(&self.history, self.share_plugin));
                self.load_definition(def);
                self.toml_note = format!("Opened `{name}` (the files define: {}).", names.join(", "));
                if problems.is_empty() {
                    self.toml_note += " Every reference is known.";
                } else {
                    for p in problems {
                        self.toml_note += &format!("\n\u{26a0} {p}");
                    }
                }
            }
            Err(e) => self.toml_note = e.to_string(),
        }
    }

    /// Writes the menu to the first file of the TOML tab.
    fn save_toml(&mut self) {
        let Some(path) = self.toml_paths().into_iter().next() else {
            self.toml_note = "No file name.".to_string();
            return;
        };
        self.toml_note = match std::fs::write(&path, self.definition().to_toml()) {
            Ok(()) => format!("Saved `{}` to {path}.", self.menu_name),
            Err(e) => format!("{path}: {e}"),
        };
    }

    /// The menu name, the files, Open and Save, above the TOML.
    fn toml_files_row(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("toml_files").num_columns(2).show(ui, |ui| {
            ui.label("Menu").on_hover_text("The menu's name in the files ([menu.<name>]); Open builds this one if the files have it.");
            ui.add(egui::TextEdit::singleline(&mut self.menu_name).desired_width(140.0));
            ui.end_row();
            ui.label("Files").on_hover_text("Separated by commas. Open merges them in order; Save writes the first.");
            ui.add(egui::TextEdit::singleline(&mut self.toml_files).desired_width(f32::INFINITY));
            ui.end_row();
        });
        ui.horizontal(|ui| {
            if ui.button("Open").on_hover_text("Replaces the menu with the one built from the files").clicked() {
                self.open_toml();
            }
            if ui.button("Save").on_hover_text("Writes the menu below to the first file").clicked() {
                self.save_toml();
            }
            ui.checkbox(&mut self.share_plugin, "Share plugin on")
                .on_hover_text("The demo's plugin.share* actions are visible only while this is on: the example plugin's Share button comes and goes.");
        });
        if !self.toml_note.is_empty() {
            ui.label(RichText::new(&self.toml_note).small());
        }
        ui.separator();
    }

    fn new(ctx: &egui::Context) -> Self {
        let (buttons, decor) = default_tree();

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
            buttons,
            decor,
            selection: Selection::root(),
            bindings: Bindings::new(),
            ids: ButtonIds::new(),
            menu_name: "edit".to_string(),
            toml_files: "examples/menus/edit.toml, examples/menus/plugin.toml".to_string(),
            toml_note: String::new(),
            history: History::default(),
            share_plugin: true,
            unhandled: Vec::new(),
            mode: Mode::Demo,
            snapshot: Vec::new(),
            previews: HashMap::new(),
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
    /// The menu as a TOML file (`MenuDefinition::to_toml`), with Open and Save.
    Toml,
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
    /// The marker for a button that opens a sub-pie (`opens_sub_pie`) or goes back ("" if none).
    fn marker(&self, button: &PieButton, opens_sub_pie: bool) -> &str {
        match (opens_sub_pie, button.is_back()) {
            (true, _) => &self.sub_pie,
            (_, true) => &self.back,
            _ => "",
        }
    }
}

// ── Button model: the library's `PieButton` tree plus the demo's decorations ─────────────

/// What the demo draws for a plain button: an ordinary button or a checkbox row.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ButtonKind {
    Button,
    Checkbox,
}

/// How a cluster button's items are laid out inside it (the demo's own choice: the library
/// leaves the layout to the render closure).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ClusterLayout {
    Row,
    Column,
    /// Two columns.
    Grid,
}

/// How a cluster button is drawn: the layout of its items and the gap between them.
#[derive(Clone, Copy, PartialEq, Debug)]
struct ClusterLook {
    layout: ClusterLayout,
    /// Gap between the items, in px.
    spacing: f32,
}

impl Default for ClusterLook {
    fn default() -> Self {
        Self { layout: ClusterLayout::Row, spacing: 4.0 }
    }
}

const DEFAULT_COLOR: Color32 = Color32::from_rgb(70, 130, 180);

/// What the demo adds to a button of the tree: how it is drawn, which the library leaves to
/// the render closure (colour, icon, tooltip, checkbox, cluster layout) and the checkbox state.
/// Kept in a [`DecorMap`] by the button's path.
#[derive(Clone, PartialEq, Debug)]
struct Decor {
    /// The icon text (e.g. an emoji) shown before the label when `icon_kind` is `Text`.
    icon: String,
    /// Which kind of icon the button shows: the text above, a painted vector icon or an image.
    icon_kind: IconKind,
    tooltip: String,
    /// Drawn as a button or as a checkbox (plain buttons only).
    kind: ButtonKind,
    /// The checkbox state when `kind` is `Checkbox` (toggled when the button is selected).
    checked: bool,
    color: Color32,
    /// The layout of a cluster button's items.
    cluster: ClusterLook,
}

impl Decor {
    fn new(color: Color32) -> Self {
        Self {
            icon: String::new(),
            icon_kind: IconKind::Text,
            tooltip: String::new(),
            kind: ButtonKind::Button,
            checked: false,
            color,
            cluster: ClusterLook::default(),
        }
    }
}

/// The decorations of the tree by button path (indices from the root level down).
type DecorMap = HashMap<Vec<usize>, Decor>;

/// The decoration of the button at `path`. A button without an entry (a new one, or one the
/// library brought back when converting a button's kind) gets plain defaults and its nearest
/// decorated ancestor's colour.
fn decor_at(map: &DecorMap, path: &[usize]) -> Decor {
    if let Some(d) = map.get(path) {
        return d.clone();
    }
    let color = (1..path.len()).rev().find_map(|n| map.get(&path[..n])).map_or(DEFAULT_COLOR, |d| d.color);
    Decor::new(color)
}

/// Mutable access to the decoration at `path`, creating the default one first.
fn decor_mut<'a>(map: &'a mut DecorMap, path: &[usize]) -> &'a mut Decor {
    if !map.contains_key(path) {
        let d = decor_at(map, path);
        map.insert(path.to_vec(), d);
    }
    map.get_mut(path).expect("just inserted")
}

/// Follows a change to the tree's shape ([`TreeEdit`]) so every decoration stays with its button.
/// A removed button takes its subtree's decorations with it and its later siblings move down;
/// a button whose children were dropped loses theirs; a new button starts clean.
fn rekey(map: &mut DecorMap, edit: &TreeEdit) {
    match edit {
        TreeEdit::Removed(path) => {
            let Some((last, parent)) = path.split_last() else { return };
            let depth = parent.len();
            for (mut key, decor) in std::mem::take(map) {
                if key.starts_with(path) {
                    continue;
                }
                if key.len() > depth && key.starts_with(parent) && key[depth] > *last {
                    key[depth] -= 1;
                }
                map.insert(key, decor);
            }
        }
        TreeEdit::Dropped(path) => map.retain(|key, _| !(key.len() > path.len() && key.starts_with(path))),
        TreeEdit::Added(path) => map.retain(|key, _| !key.starts_with(path)),
    }
}

/// The kind of icon a button shows before its label.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum IconKind {
    /// The button's `icon` text (empty: no icon).
    Text,
    /// A procedural icon painted with `egui::Painter`.
    Vector(VectorIcon),
    /// An embedded image (index into `IMAGE_ICONS`), shown as a texture.
    Image(usize),
}

/// The built-in painted icons.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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

const DIR_NAMES: [&str; 8] = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];

/// The index of `d` in `PieDirection::ALL` (N clockwise to NW).
fn dir_index(d: PieDirection) -> usize {
    PieDirection::ALL.iter().position(|x| *x == d).unwrap_or(0)
}

/// (label, icon, tooltip, mnemonic key, colour) of a default button.
type ButtonSpec = (&'static str, &'static str, &'static str, Option<char>, (u8, u8, u8));

/// The demo's standard buttons (one per direction, N clockwise to NW) and their decorations.
/// "Paste Special" (NE) opens a sub-pie with a back button in the opposite direction (SW).
fn default_tree() -> (Vec<PieButton>, DecorMap) {
    let mut decor = DecorMap::new();
    let mut buttons = Vec::new();
    // (label, icon, tooltip, mnemonic key, colour), N clockwise to NW.
    #[rustfmt::skip]
    let rows: [ButtonSpec; 8] = [
        ("Copy",               "",   "Ctrl+C",       Some('c'), ( 70, 130, 180)),
        ("Paste Special",      "",   "Ctrl+Shift+V", Some('p'), ( 70, 130, 180)),
        ("Redo",               "",   "Ctrl+Y",       Some('r'), (100, 160, 100)),
        ("Save As…",           "",   "Ctrl+Shift+S", Some('s'), (180, 140,  60)),
        ("Delete Permanently", "🗑", "Del",          Some('d'), (180,  70,  70)),
        ("Cut",                "",   "Ctrl+X",       Some('t'), (180, 100,  60)),
        ("Undo Last Change",   "",   "Ctrl+Z",       Some('u'), (100, 160, 100)),
        ("Word wrap",          "",   "",             None,      (130,  90, 180)),
    ];
    for (i, (label, icon, tooltip, key, (r, g, b))) in rows.into_iter().enumerate() {
        let color = Color32::from_rgb(r, g, b);
        let mut button = PieButton::new(PieDirection::ALL[i]).with_label(label);
        if let Some(k) = key {
            button = button.with_mnemonic(k);
        }
        let mut d = Decor::new(color);
        d.icon = icon.into();
        d.tooltip = tooltip.into();
        if i == 7 {
            d.kind = ButtonKind::Checkbox;
        }
        decor.insert(vec![i], d);
        if i == 1 {
            // Five children in the directions a fan-out may use, so the sub-pie works in both
            // styles, and a back button opposite the opening direction.
            let dir = PieDirection::ALL[i];
            let labels = ["As Text", "Values Only", "Formatting", "As Link", "As Image"];
            let slots = dir.fan_out_slots();
            let children: Vec<PieButton> = (0..8)
                .filter_map(|k| {
                    let at = PieDirection::ALL[k];
                    if at == dir.opposite() {
                        Some(PieButton::back(at))
                    } else {
                        slots.iter().position(|s| *s == at).map(|n| PieButton::new(at).with_label(labels[n]))
                    }
                })
                .collect();
            for k in 0..children.len() {
                decor.insert(vec![i, k], Decor::new(color));
            }
            button = button.with_children(children);
        }
        buttons.push(button);
    }
    (buttons, decor)
}

/// Which icon set the "Sample icons" row assigns to the root buttons.
enum SampleIcons {
    Vector,
    Image,
    Reset,
}

/// Assigns icons to the root buttons by direction (N clockwise to NW: copy, paste, redo,
/// save, delete, cut, undo, word wrap), or restores the defaults' icons.
fn sample_icons(buttons: &[PieButton], decor: &mut DecorMap, set: SampleIcons) {
    let (_, defaults) = default_tree();
    let vector = [
        VectorIcon::Copy, VectorIcon::Paste, VectorIcon::Redo, VectorIcon::Save,
        VectorIcon::Trash, VectorIcon::Cut, VectorIcon::Undo, VectorIcon::Check,
    ];
    for (i, button) in buttons.iter().enumerate() {
        let d = dir_index(button.direction);
        let entry = decor_mut(decor, &[i]);
        match set {
            SampleIcons::Vector => entry.icon_kind = IconKind::Vector(vector[d]),
            SampleIcons::Image => entry.icon_kind = IconKind::Image(d % IMAGE_ICONS.len()),
            SampleIcons::Reset => {
                let original = defaults.get(&vec![d]);
                entry.icon_kind = original.map_or(IconKind::Text, |o| o.icon_kind);
                entry.icon = original.map(|o| o.icon.clone()).unwrap_or_default();
            }
        }
    }
}

/// The button at `path` (indices from the root level), if there is one.
fn button_at<'a>(buttons: &'a [PieButton], path: &[usize]) -> Option<&'a PieButton> {
    let (last, parents) = path.split_last()?;
    let mut level = buttons;
    for &i in parents {
        level = level.get(i)?.children();
    }
    level.get(*last)
}

/// "Paste Special › As Text": the labels along `route` (a path from the root).
fn names_along(buttons: &[PieButton], route: &[usize]) -> String {
    (1..=route.len())
        .map_while(|n| button_at(buttons, &route[..n]).map(|b| b.label.as_str()))
        .collect::<Vec<_>>()
        .join(" › ")
}

/// Whether the demo draws `button` as a checkbox: only plain buttons can be.
fn is_checkbox(button: &PieButton, decor: &Decor) -> bool {
    decor.kind == ButtonKind::Checkbox && matches!(button.kind, PieButtonKind::Action)
}

/// Problems (shared key / direction, bad fan-out children) of every level of the tree, each
/// prefixed with its sub-pie's name. Empty: the library will accept the tree (debug builds
/// assert on problems when a level opens).
fn tree_problems(settings: &PieMenuSettings, buttons: &[PieButton]) -> Vec<String> {
    fn walk(menu: &PieMenu, level: &[PieButton], name: &str, out: &mut Vec<String>) {
        for p in menu.check_buttons(level) {
            out.push(if name.is_empty() { p.to_string() } else { format!("{name}: {p}") });
        }
        for b in level.iter().filter(|b| !b.children().is_empty()) {
            let name = if name.is_empty() { b.label.clone() } else { format!("{name} › {}", b.label) };
            walk(menu, b.children(), &name, out);
        }
    }
    let menu = PieMenu::new().with_settings(settings.clone());
    let mut out = Vec::new();
    walk(&menu, buttons, "", &mut out);
    out
}

/// The words a search of the Buttons tab matches a button against: its direction (short and
/// long), label, kind and, for a cluster, its items' labels.
fn button_search_text(b: &PieButton, settings: &PieMenuSettings) -> String {
    let kind = match &b.kind {
        PieButtonKind::Action => "action button",
        PieButtonKind::Back => "back button",
        PieButtonKind::Cluster(_) => "cluster",
        PieButtonKind::SubPie { style, .. } => match style.unwrap_or(settings.sub_pies.style) {
            SubPieStyle::FanOut => "fan-out fanout sub-pie",
            SubPieStyle::Replace => "sub-pie subpie",
        },
    };
    let mut text = format!("{} {:?} {} {kind}", DIR_NAMES[dir_index(b.direction)], b.direction, b.label);
    for item in b.cluster_items() {
        text.push(' ');
        text += &item.label;
    }
    text.to_lowercase()
}

/// Whether a button matches the (lower-case, non-empty) search `query`.
fn button_matches(b: &PieButton, settings: &PieMenuSettings, query: &str) -> bool {
    button_search_text(b, settings).contains(query)
}

/// How many buttons of the tree (sub-pies included) match a search (0 for an empty query):
/// the number in the badge of the Buttons tab while the Menu tab is showing.
fn buttons_hit_count(buttons: &[PieButton], settings: &PieMenuSettings, query: &str) -> usize {
    if query.is_empty() {
        return 0;
    }
    buttons
        .iter()
        .map(|b| usize::from(button_matches(b, settings, query)) + buttons_hit_count(b.children(), settings, query))
        .sum()
}

/// A sketch of the render closure that draws the demo's buttons, for the "Draw code" tab:
/// one `match` arm per button of the tree (`b.path` is the button's route from the root, whichever
/// ring it is in). It covers what the library cannot know: colours, tooltips, icons, the
/// checkbox kind and cluster layouts. Hover colours, scale and number hints are left out.
fn draw_code(buttons: &[PieButton], decor: &DecorMap) -> String {
    let mut out = String::from(
        "// Sketch of how the demo draws its buttons: colours, tooltips, icons and cluster\n\
         // layouts are the demo's own choices, not library settings.\n\
         |ui, b| match b.path {\n",
    );
    draw_arms(buttons, decor, &mut Vec::new(), &mut out);
    out += "    _ => {}\n}\n";
    out
}

/// Appends the arms of one level (and, recursively, of its sub-pies) to `out`.
fn draw_arms(level: &[PieButton], decor: &DecorMap, path: &mut Vec<usize>, out: &mut String) {
    for (n, button) in level.iter().enumerate() {
        path.push(n);
        let d = decor_at(decor, path);
        let route: Vec<String> = path.iter().map(|p| p.to_string()).collect();
        let label = button.label.replace('\n', " ");
        out.push_str(&format!("    [{}] => {{ // {label}\n", route.join(", ")));
        let mut body: Vec<String> = Vec::new();
        let fill = format!("egui::Color32::from_rgb({}, {}, {})", d.color.r(), d.color.g(), d.color.b());
        let tooltip = if d.tooltip.is_empty() { String::new() } else { format!("\n    .on_hover_text({:?})", d.tooltip) };
        if button.is_cluster() {
            let v = format!("egui::vec2({:?}, {:?})", d.cluster.spacing, d.cluster.spacing);
            body.push(format!("// a frame filled with {fill} around the items"));
            body.push(format!("ui.spacing_mut().item_spacing = {v};"));
            let item = |number: usize| format!("b.item({number}, ui.button(b.item_label({number})));");
            let numbers: Vec<usize> = button.cluster_items().iter().map(|i| i.number).collect();
            match d.cluster.layout {
                ClusterLayout::Row | ClusterLayout::Column => {
                    body.push(format!("ui.{}(|ui| {{", if d.cluster.layout == ClusterLayout::Row { "horizontal" } else { "vertical" }));
                    body.extend(numbers.iter().map(|&n| format!("    {}", item(n))));
                    body.push("});".to_string());
                }
                ClusterLayout::Grid => {
                    body.push(format!("egui::Grid::new(\"cluster\").num_columns(2).spacing({v}).show(ui, |ui| {{"));
                    for (k, &number) in numbers.iter().enumerate() {
                        body.push(format!("    {}", item(number)));
                        if k % 2 == 1 {
                            body.push("    ui.end_row();".to_string());
                        }
                    }
                    body.push("});".to_string());
                }
            }
        } else {
            match d.icon_kind {
                IconKind::Vector(v) => body.push(format!("// icon: {} (painted vector icon, see VectorIcon::paint in the demo)", v.name())),
                IconKind::Image(k) => body.push(format!("// icon: {} (embedded image, tinted with the text colour)", IMAGE_ICONS[k % IMAGE_ICONS.len()].0)),
                IconKind::Text => {}
            }
            let mut text = "b.label_text()".to_string();
            if d.icon_kind == IconKind::Text && !d.icon.is_empty() {
                text += &format!(".prefix({:?})", format!("{}  ", d.icon));
            }
            if !button.children().is_empty() {
                body.push("// opens a sub-pie (the demo adds a marker at the far end)".to_string());
            }
            if is_checkbox(button, &d) {
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
        draw_arms(button.children(), decor, path, out);
        path.pop();
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

// ── Configure mode ──────────────────────────────────────────────────────────

/// What the canvas shows.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    /// The real menu, opened with right-click or Space, as an app would use it.
    Demo,
    /// A preview of the selected node of the button tree, driven by the editors.
    Configure,
}

/// The level of the tree whose buttons are the children of the button at `parent_path` (empty:
/// the root level).
fn level_at<'a>(buttons: &'a [PieButton], parent_path: &[usize]) -> Option<&'a [PieButton]> {
    if parent_path.is_empty() { Some(buttons) } else { button_at(buttons, parent_path).map(PieButton::children) }
}

/// The settings a preview menu is drawn with: the real ones, except for what would make the
/// preview react to input it should not: no hover timer for sub-pies, no mnemonic keys, nothing
/// that dismisses a pinned menu. `in_sub_pie`: the preview shows the children of a sub-pie as its
/// ring, which the real menu draws with the sub-pie hover emphasis.
fn preview_settings(real: &PieMenuSettings, in_sub_pie: bool) -> PieMenuSettings {
    let mut s = real.clone();
    s.sub_pies.hover_delay = None;
    s.input.use_mnemonic_keys = false;
    s.input.pinned_dismiss_inputs = Vec::new();
    if in_sub_pie {
        s.animations.hover_emphasis = s.animations.sub_pie_hover_emphasis;
    }
    s
}

/// Settings whose effect needs a pointer, a click, time or a descent into a sub-pie, so the
/// preview cannot show them: they are judged in Demo mode only.
fn judged_in_demo_only(field: SettingsField) -> bool {
    use SettingsField as F;
    matches!(
        field,
        F::Reveal
            | F::RevealDuration
            | F::SlideStartRadius
            | F::SlideStartShapeFactor
            | F::SlideFade
            | F::HoverEmphasis
            | F::SubPieHoverEmphasis
            | F::HoverDuration
            | F::SubPieCenter
            | F::SubPieHoverDelay
            | F::FanOutDelay
            | F::FanOutOpenerAction
    )
}

/// The menu-wide settings that concern the selected node and are judged in Demo mode only.
fn demo_only_fields(settings: &PieMenuSettings, buttons: &[PieButton], selection: &Selection) -> Vec<SettingsField> {
    relevant_fields(settings, selection_kind(buttons, settings, selection)).into_iter().filter(|f| judged_in_demo_only(*f)).collect()
}

/// The note the inspector shows in Configure mode for those settings ("" when none apply).
fn demo_only_note(fields: &[SettingsField]) -> String {
    if fields.is_empty() {
        return String::new();
    }
    let names: Vec<&str> = fields.iter().map(|f| field_label(*f)).collect();
    format!("The preview does not show these settings; judge them in Demo mode: {}.", names.join(", "))
}

impl Demo {
    /// The Configure canvas: a preview of what the selected node of the tree is about, below
    /// the toolbar. It draws separate preview menus (one per level of the tree, pinned at the
    /// centre of the canvas), never `self.menu`, from the last tree that had no problems.
    fn configure_canvas(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, viewport: Rect) {
        // The preview always draws the last problem-free tree: debug builds assert on problems.
        let mut settings = preview_settings(&self.menu.settings, false);
        if tree_problems(&settings, &self.buttons).is_empty() {
            self.snapshot.clone_from(&self.buttons);
        }
        if self.snapshot.is_empty() || !tree_problems(&settings, &self.snapshot).is_empty() {
            ui.label(RichText::new("The preview is paused until the button problems are fixed.").color(Color32::LIGHT_RED));
            return;
        }
        let target = preview_target(&self.snapshot, &settings, &self.selection);
        let Some(level) = level_at(&self.snapshot, &target.level_path) else { return };
        if !target.level_path.is_empty() {
            settings = preview_settings(&self.menu.settings, true);
        }

        // Breadcrumb: "Menu › Paste Special" for the children of a sub-pie; each part selects it.
        if !target.level_path.is_empty() {
            ui.horizontal(|ui| {
                if ui.link("Menu").clicked() {
                    self.selection = Selection::root();
                }
                for n in 1..=target.level_path.len() {
                    ui.label("›");
                    let label = button_at(&self.snapshot, &target.level_path[..n]).map_or("?", |b| b.label.as_str());
                    if ui.link(label).clicked() {
                        self.selection = Selection::button(&target.level_path[..n]);
                    }
                }
            });
        }
        let canvas = Rect::from_min_max(egui::pos2(viewport.min.x, ui.cursor().min.y), viewport.max);
        let center = canvas.center();

        // The preview menu of this level, pinned at the centre, with the fan of the target open.
        let id = ("preview", target.level_path.clone());
        let preview = self.previews.entry(target.level_path.clone()).or_insert_with(|| PieMenu::new().with_id(id));
        preview.settings = settings;
        preview.title = match button_at(&self.snapshot, &target.level_path) {
            Some(parent) => Some(parent.label.clone()),
            None => (!self.title.is_empty()).then(|| self.title.clone()),
        };
        preview.pointer_area = None;
        if !preview.is_pinned() || !preview.path().is_empty() {
            preview.pin(center);
        }
        preview.position = center;
        preview.set_open_fan_out(target.fan_trigger);

        // Number hints still show, but the digits do nothing.
        ctx.input_mut(|i| {
            i.events.retain(|e| {
                !matches!(e, egui::Event::Key { key: egui::Key::Num1 | egui::Key::Num2 | egui::Key::Num3 | egui::Key::Num4
                    | egui::Key::Num5 | egui::Key::Num6 | egui::Key::Num7 | egui::Key::Num8 | egui::Key::Num9, .. })
            });
        });
        let look = Look {
            decor: &self.decor,
            images: &self.images,
            icons: &self.icons,
            sub_pie_look: self.sub_pie_look,
            button_h: self.button_height,
            hint_gap: self.menu.settings.number_hints.spacing,
            show_hints: self.show_hints,
            unhandled: &self.unhandled,
        };
        let selection = self.selection.clone();
        let mut clicked = None;
        // No pointer: nothing is hovered and nothing is selected by the menu itself; a click is
        // taken by the overlay of `draw_button`, which selects the node in the tree.
        preview.show_with_pointer(ctx, level, None, true, |ui, b| {
            let mut path = target.level_path.clone();
            path.extend_from_slice(b.path);
            if let Some(s) = draw_button(ui, b, &path, &look, Some(&selection)) {
                clicked = Some(s);
            }
        });
        if let Some(s) = clicked {
            self.selection = s;
        }
    }
}

// ── Drawing the buttons ─────────────────────────────────────────────────────

/// What `draw_button` needs besides the button itself.
struct Look<'a> {
    decor: &'a DecorMap,
    images: &'a [egui::TextureHandle],
    icons: &'a MarkerIcons,
    sub_pie_look: SubPieLook,
    /// The minimum height of a button, in px.
    button_h: f32,
    /// The gap between a number hint and the label (`number_hints.spacing`).
    hint_gap: f32,
    show_hints: bool,
    /// Buttons and items without an action in the demo, drawn in the error colour.
    unhandled: &'a [IdKey],
}

/// The outline colour of the selected button in Configure mode.
const SELECTED_OUTLINE: Color32 = Color32::from_rgb(255, 190, 60);

fn outline(ui: &egui::Ui, rect: Rect) {
    ui.painter().rect_stroke(rect.expand(2.0), 7.0, egui::Stroke::new(2.0, SELECTED_OUTLINE), egui::StrokeKind::Outside);
}

/// The render closure's body: draws one button the way the demo does. `path` is the button's
/// route from the root of the whole tree. With `select` (Configure mode: the current
/// selection) the button also gets an outline when it is the selected one, and a click on it
/// (or on a cluster's item) is returned as the new selection.
fn draw_button(ui: &mut egui::Ui, b: &ButtonView, path: &[usize], look: &Look, select: Option<&Selection>) -> Option<Selection> {
    // No action in the program (for a cluster: for one of its items): the error colour, the
    // demo's choice of how to show it.
    let text_color = if look.unhandled.iter().any(|k| k.path == path) { unhandled_color(ui) } else { b.text_color() };
    let d = decor_at(look.decor, path);
    let hovered = b.hovered;
    let mut clicked = None;
    // Hover emphasis enlarges the content, not the layer (which would blur the
    // text): scale the minimum height and the padding along with the label.
    let button_h = look.button_h * b.scale();
    let pad_x = 10.0 * b.scale();
    let is_cluster = b.button.is_cluster();
    // The pie's own highlight is the only hover feedback: egui's per-widget
    // hover/press outlines would add a second, inconsistent one on top.
    // (An app drawing plain widgets can call `b.use_pie_hover(ui)` instead.)
    // Not in a cluster: it is never highlighted, and its items are plain
    // widgets with egui's native hover and press look.
    if !is_cluster {
        let w = &mut ui.visuals_mut().widgets;
        let (stroke, expansion) = (w.inactive.bg_stroke, w.inactive.expansion);
        w.hovered.bg_stroke = stroke;
        w.active.bg_stroke = stroke;
        w.hovered.expansion = expansion;
        w.active.expansion = expansion;
    }
    let bg = if !b.enabled {
        d.color.gamma_multiply(0.25)
    } else if hovered {
        d.color
    } else {
        d.color.gamma_multiply(0.6)
    };
    // Buttons that open a sub-pie can look different from actions.
    let (bg, stroke) = if b.opens_sub_pie() && !b.button.is_back() {
        match look.sub_pie_look {
            SubPieLook::Same => (bg, egui::Stroke::NONE),
            SubPieLook::Outlined => (bg.gamma_multiply(0.3), egui::Stroke::new(1.5, bg)),
            SubPieLook::Dimmer => (bg.gamma_multiply(0.55), egui::Stroke::NONE),
        }
    } else {
        (bg, egui::Stroke::NONE)
    };
    // A cluster: its items in the chosen layout, inside the usual coloured frame.
    // Each is an `egui::Button` registered with `b.item(number, ..)`; the label underlines
    // the item's mnemonic.
    if is_cluster {
        let items = b.button.cluster_items();
        let selected_item = select.filter(|s| s.path == path).map(|s| s.item);
        let mut item_clicked = None;
        let mut item = |ui: &mut egui::Ui, k: usize| {
            let it = &items[k];
            let button = egui::Button::new(b.item_label(it.number)).min_size(Vec2::new(0.0, button_h));
            let resp = b.item(it.number, ui.add_enabled(it.enabled, button));
            if select.is_some() {
                if resp.clicked() {
                    item_clicked = Some(k);
                }
                if selected_item == Some(Some(k)) {
                    outline(ui, resp.rect);
                }
            }
        };
        let spacing = d.cluster.spacing;
        let count = items.len();
        let frame = egui::Frame::new()
            .fill(bg)
            .corner_radius(6.0)
            .inner_margin(Vec2::new(pad_x, 3.0 * b.scale()))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::splat(spacing);
                ui.spacing_mut().button_padding = Vec2::new(pad_x * 0.6, 0.0);
                match d.cluster.layout {
                    ClusterLayout::Row => {
                        ui.horizontal(|ui| (0..count).for_each(|k| item(ui, k)));
                    }
                    ClusterLayout::Column => {
                        ui.vertical(|ui| (0..count).for_each(|k| item(ui, k)));
                    }
                    ClusterLayout::Grid => {
                        egui::Grid::new(("cluster_grid", b.idx)).spacing(Vec2::splat(spacing)).show(ui, |ui| {
                            for k in 0..count {
                                item(ui, k);
                                if k % 2 == 1 {
                                    ui.end_row();
                                }
                            }
                        });
                    }
                }
            });
        if select.is_some() {
            let rect = frame.response.rect;
            if let Some(k) = item_clicked {
                clicked = Some(Selection::item(path, k));
            } else if ui.input(|i| i.pointer.primary_clicked()) && ui.rect_contains_pointer(rect) {
                // The frame around the items: selects the cluster button itself.
                clicked = Some(Selection::button(path));
            }
            if select.is_some_and(|s| s.path == path && s.item.is_none()) {
                outline(ui, rect);
            }
        }
        return clicked;
    }
    // Label and digit colours come from the library: egui's widget colours for the hover
    // state, or the forced colour; disabled is greyed out.
    let hint = b.number_hint.as_ref().filter(|_| look.show_hints).map(|h| (RichText::new(&h.text).color(h.color), h.on_right));

    // Icon + label, with the mnemonic underlined.
    let mut label = b.label_text().color(text_color);
    // Sub-pie and back buttons carry a marker (from the "Sub-pies" section) in a slot of its
    // own: at the far end, or before the icon. The icon is the button's own and never
    // replaces it.
    let marker = look.icons.marker(b.button, b.opens_sub_pie());
    let far_end = look.icons.placement == MarkerPlacement::FarEnd;
    let mut prefix = String::new();
    if !marker.is_empty() && !far_end {
        prefix += &format!("{marker}  ");
    }
    if d.icon_kind == IconKind::Text && !d.icon.is_empty() {
        prefix += &format!("{}  ", d.icon);
    }
    if !prefix.is_empty() {
        label = label.prefix(prefix);
    }
    // Far end: in the label's colour, set apart from the text by a little space.
    let far_marker = (!marker.is_empty() && far_end).then(|| {
        let text = if b.rtl { format!("{marker}  ") } else { format!("  {marker}") };
        RichText::new(text).color(text_color)
    });
    let text: egui::WidgetText = label.into();
    // A painted or image icon, about the text height; `b.scale()` keeps it crisp under hover
    // emphasis.
    let icon_size = ui.text_style_height(&egui::TextStyle::Body) * b.scale();
    let graphic = d.icon_kind != IconKind::Text;
    let icon_color = text_color;
    let hint_gap = look.hint_gap;
    let images = look.images;

    let resp = if is_checkbox(b.button, &d) {
        // Display only: the toggle happens on Selected. Both kinds get the same fixed height
        // ("Button height"): no vertical padding, content centred in a row of exactly that height.
        egui::Frame::new()
            .fill(bg)
            .stroke(stroke)
            .corner_radius(6.0)
            .inner_margin(Vec2::new(pad_x, 0.0))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.x = hint_gap;
                ui.allocate_ui_with_layout(Vec2::new(0.0, button_h), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.set_min_height(button_h);
                    // Mirrored: the far end is the left.
                    if b.rtl && let Some(m) = &far_marker {
                        ui.label(m.clone());
                    }
                    if let Some((h, false)) = &hint {
                        ui.label(h.clone());
                    }
                    let mut display = d.checked;
                    if b.rtl {
                        // Right-to-left: the box goes right of its text.
                        ui.label(text);
                        add_icon(ui, d.icon_kind, images, icon_size, icon_color);
                        ui.add(egui::Checkbox::without_text(&mut display));
                    } else {
                        ui.add(egui::Checkbox::without_text(&mut display));
                        add_icon(ui, d.icon_kind, images, icon_size, icon_color);
                        ui.label(text);
                    }
                    if let Some((h, true)) = &hint {
                        ui.label(h.clone());
                    }
                    if !b.rtl && let Some(m) = &far_marker {
                        ui.label(m.clone());
                    }
                });
            })
            .response
    } else if graphic {
        // A painted or image icon: same look and height as a button, laid out by hand (the
        // icon before the label, after it when mirrored).
        egui::Frame::new()
            .fill(bg)
            .stroke(stroke)
            .corner_radius(6.0)
            .inner_margin(Vec2::new(pad_x, 0.0))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.x = hint_gap.max(4.0 * b.scale());
                ui.allocate_ui_with_layout(Vec2::new(0.0, button_h), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.set_min_height(button_h);
                    if b.rtl && let Some(m) = &far_marker {
                        ui.label(m.clone());
                    }
                    if let Some((h, false)) = &hint {
                        ui.label(h.clone());
                    }
                    if b.rtl {
                        ui.label(text);
                        add_icon(ui, d.icon_kind, images, icon_size, icon_color);
                    } else {
                        add_icon(ui, d.icon_kind, images, icon_size, icon_color);
                        ui.label(text);
                    }
                    if let Some((h, true)) = &hint {
                        ui.label(h.clone());
                    }
                    if !b.rtl && let Some(m) = &far_marker {
                        ui.label(m.clone());
                    }
                });
            })
            .response
    } else {
        ui.spacing_mut().button_padding = Vec2::new(pad_x, 0.0);
        let mut button = hint_button(text, hint, hint_gap)
            .fill(bg)
            .stroke(stroke)
            .corner_radius(6.0)
            .min_size(Vec2::new(0.0, button_h));
        if let Some(m) = far_marker {
            // egui lays text out left to right, so a mirrored layout's far end is the left edge.
            button = if b.rtl { button.left_text(m) } else { button.right_text(m) };
        }
        ui.add(button)
    };
    let rect = resp.rect;
    match select {
        Some(selection) => {
            // Configure mode: an overlay added last, so it is the topmost widget of this button
            // and takes the click (and the tooltip) from the widgets underneath.
            let overlay = ui.interact(rect, ui.id().with(("select", path)), egui::Sense::click());
            if overlay.clicked() {
                clicked = Some(Selection::button(path));
            }
            if selection.path == path && selection.item.is_none() {
                outline(ui, rect);
            }
            if !d.tooltip.is_empty() {
                overlay.on_hover_text(&d.tooltip);
            }
        }
        None => {
            if !d.tooltip.is_empty() {
                resp.on_hover_text(&d.tooltip);
            }
        }
    }
    clicked
}

// ── The Appearance group of the inspector ───────────────────────────────────

/// The demo's own controls for the selected button, drawn by the inspector's `extras`: icon,
/// tooltip, colour, button or checkbox, and a cluster's layout. Returns true if `d` changed.
fn appearance_editor(ui: &mut egui::Ui, button: &PieButton, d: &mut Decor) -> bool {
    let mut changed = false;
    egui::CollapsingHeader::new(RichText::new("Appearance").strong()).id_salt("pie_demo_appearance").default_open(true).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("demo").small().weak()).on_hover_text(
                "How the demo draws this button: used by its Draw code, not part of the menu code.",
            );
        });
        egui::Grid::new("pie_demo_appearance_grid").num_columns(2).show(ui, |ui| {
            if !button.is_cluster() {
                ui.label("Icon");
                ui.horizontal(|ui| {
                    let kind_name = match d.icon_kind {
                        IconKind::Text if d.icon.is_empty() => "None",
                        IconKind::Text => "Text",
                        IconKind::Vector(_) => "Vector",
                        IconKind::Image(_) => "Image",
                    };
                    egui::ComboBox::from_id_salt("pie_demo_icon_kind").selected_text(kind_name).width(70.0).show_ui(ui, |ui| {
                        if ui.selectable_label(kind_name == "None", "None").clicked() {
                            d.icon_kind = IconKind::Text;
                            d.icon.clear();
                            changed = true;
                        }
                        if ui.selectable_label(kind_name == "Text", "Text").clicked() {
                            d.icon_kind = IconKind::Text;
                            if d.icon.is_empty() {
                                d.icon = "★".into();
                            }
                            changed = true;
                        }
                        if ui.selectable_label(kind_name == "Vector", "Vector").clicked() && !matches!(d.icon_kind, IconKind::Vector(_)) {
                            d.icon_kind = IconKind::Vector(VectorIcon::Copy);
                            changed = true;
                        }
                        if ui.selectable_label(kind_name == "Image", "Image").clicked() && !matches!(d.icon_kind, IconKind::Image(_)) {
                            d.icon_kind = IconKind::Image(0);
                            changed = true;
                        }
                    });
                    match &mut d.icon_kind {
                        IconKind::Text => {
                            changed |= ui
                                .add(egui::TextEdit::singleline(&mut d.icon).desired_width(40.0))
                                .on_hover_text("Shown before the label, e.g. an emoji")
                                .changed();
                        }
                        IconKind::Vector(v) => {
                            let col = ui.visuals().text_color();
                            let (rect, _) = ui.allocate_exact_size(Vec2::splat(16.0), egui::Sense::hover());
                            v.paint(ui.painter(), rect, col, 1.3);
                            egui::ComboBox::from_id_salt("pie_demo_icon_vector").selected_text(v.name()).show_ui(ui, |ui| {
                                for &o in ALL_VECTOR_ICONS {
                                    ui.horizontal(|ui| {
                                        let (rect, _) = ui.allocate_exact_size(Vec2::splat(14.0), egui::Sense::hover());
                                        o.paint(ui.painter(), rect, col, 1.2);
                                        changed |= ui.selectable_value(v, o, o.name()).changed();
                                    });
                                }
                            });
                        }
                        IconKind::Image(n) => {
                            egui::ComboBox::from_id_salt("pie_demo_icon_image")
                                .selected_text(IMAGE_ICONS[*n % IMAGE_ICONS.len()].0)
                                .show_ui(ui, |ui| {
                                    for (k, (name, _)) in IMAGE_ICONS.iter().enumerate() {
                                        changed |= ui.selectable_value(n, k, *name).changed();
                                    }
                                });
                        }
                    }
                });
                ui.end_row();

                ui.label("Tooltip");
                changed |= ui.add(egui::TextEdit::singleline(&mut d.tooltip).desired_width(130.0)).changed();
                ui.end_row();
            }

            ui.label("Colour");
            changed |= ui.color_edit_button_srgba(&mut d.color).changed();
            ui.end_row();

            if matches!(button.kind, PieButtonKind::Action) {
                ui.label("Drawn as");
                ui.horizontal(|ui| {
                    changed |= ui.radio_value(&mut d.kind, ButtonKind::Button, "Button").changed();
                    changed |= ui.radio_value(&mut d.kind, ButtonKind::Checkbox, "Checkbox").changed();
                });
                ui.end_row();
            }

            if button.is_cluster() {
                ui.label("Layout");
                egui::ComboBox::from_id_salt("pie_demo_cluster_layout")
                    .width(110.0)
                    .selected_text(match d.cluster.layout {
                        ClusterLayout::Row => "Row",
                        ClusterLayout::Column => "Column",
                        ClusterLayout::Grid => "Grid (2 columns)",
                    })
                    .show_ui(ui, |ui| {
                        for (layout, name) in [
                            (ClusterLayout::Row, "Row"),
                            (ClusterLayout::Column, "Column"),
                            (ClusterLayout::Grid, "Grid (2 columns)"),
                        ] {
                            changed |= ui.selectable_value(&mut d.cluster.layout, layout, name).changed();
                        }
                    });
                ui.end_row();

                ui.label("Spacing");
                ui.horizontal(|ui| {
                    changed |= demo_px(ui, &mut d.cluster.spacing, ClusterLook::default().spacing, 0.0..=60.0);
                });
                ui.end_row();
            }
        });
    });
    changed
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
                    let from_app = self.bindings.variable_for(&BindTarget::Title).is_some();
                    ui.add_enabled(!from_app, egui::TextEdit::singleline(&mut self.title).desired_width(120.0));
                    binding_toggle(ui, &mut self.bindings, &BindTarget::Title);
                });
                binding_details(ui, &mut self.bindings, &BindTarget::Title);
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
                        sample_icons(&self.buttons, &mut self.decor, SampleIcons::Vector);
                    }
                    if ui.button("Image").clicked() {
                        sample_icons(&self.buttons, &mut self.decor, SampleIcons::Image);
                    }
                    if ui.button("Reset").clicked() {
                        sample_icons(&self.buttons, &mut self.decor, SampleIcons::Reset);
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
        let style_before = self.menu.settings.sub_pies.style;
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
                                field_editor_bound(ui, &mut self.menu.settings, f, &mut self.bindings);
                                if self.mode == Mode::Configure && judged_in_demo_only(f) {
                                    ui.label(RichText::new("judged in Demo mode only").small().weak())
                                        .on_hover_text("The Configure preview has no pointer, clock or navigation: switch to Demo mode to see this setting at work.");
                                }
                            }
                            Row::Demo(r) => self.demo_row(ui, r),
                        }
                    }
                });
        }
        if shown == 0 {
            ui.label(RichText::new("No matching settings.").italics().color(ui.visuals().weak_text_color()));
        }
        self.follow_menu_style(style_before);
    }

    /// The editors only set `sub_pies.style`; keeping the buttons legal under a fan-out style is
    /// the caller's job: sub-pies that cannot fan out are pinned to replace style, and the
    /// notice says which. `before` is the style at the start of the frame.
    fn follow_menu_style(&mut self, before: SubPieStyle) {
        let style = self.menu.settings.sub_pies.style;
        if style != before && style == SubPieStyle::FanOut {
            self.notice = set_menu_sub_pie_style(&mut self.menu.settings, &mut self.buttons, style);
        }
    }

    /// The Buttons tab: the library's tree above (a search shows only the buttons that match),
    /// the inspector of the selected node below, with the demo's Appearance group among its
    /// own controls.
    fn buttons_tab(&mut self, ui: &mut egui::Ui, query: &str) {
        ui.horizontal(|ui| {
            if ui.button("Reset buttons").clicked() {
                (self.buttons, self.decor) = default_tree();
                self.selection = Selection::root();
                // Ids and label bindings belonged to the old tree's paths.
                self.ids = ButtonIds::new();
                self.bindings.retain(|target, _| target.path().is_none());
            }
            ui.label(RichText::new(format!("{} root buttons", self.buttons.len())).weak().small());
        });
        let tree_height = (ui.available_height() * 0.4).max(100.0);
        egui::Panel::top("buttons_tree_panel")
            .resizable(true)
            .default_size(tree_height)
            .min_size(60.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().id_salt("buttons_tree_scroll").auto_shrink([false, false]).show(ui, |ui| {
                    let settings = &self.menu.settings;
                    // Buttons without an action in the demo are marked like problems.
                    let marked: Vec<Vec<usize>> = self.unhandled.iter().map(|k| k.path.clone()).collect();
                    let filter = |b: &PieButton| button_matches(b, settings, query);
                    let filter: Option<&dyn Fn(&PieButton) -> bool> = if query.is_empty() { None } else { Some(&filter) };
                    button_tree_marked(ui, settings, &self.buttons, &mut self.selection, filter, &marked);
                });
            });
        egui::ScrollArea::vertical().id_salt("buttons_inspector_scroll").auto_shrink([false, false]).show(ui, |ui| {
            let decor = &mut self.decor;
            let note = if self.mode == Mode::Configure {
                demo_only_note(&demo_only_fields(&self.menu.settings, &self.buttons, &self.selection))
            } else {
                String::new()
            };
            let bindings = &mut self.bindings;
            let ids = &mut self.ids;
            let actions = demo_actions(&self.history, self.share_plugin);
            let out = button_inspector_bound(ui, &mut self.menu.settings, &mut self.buttons, &mut self.selection, bindings, ids, &actions, |ui, path, button| {
                // Read first, write only on a change: the inspector may have reshaped the
                // tree this frame, and the table follows only once it returns.
                if !note.is_empty() {
                    ui.label(RichText::new(&note).small().weak());
                }
                let mut d = decor_at(decor, path);
                if appearance_editor(ui, button, &mut d) {
                    decor.insert(path.to_vec(), d);
                }
            });
            // The menu-wide section of the inspector edits `menu.settings` itself (and calls
            // `set_menu_sub_pie_style` for a fan-out style, showing its own notice), and nothing
            // in the demo mirrors a setting, so `out.settings_changed` needs no re-sync. Only
            // changes to the tree's shape need following.
            for edit in &out.edits {
                rekey(&mut self.decor, edit);
            }
        });
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
        // The app's values the dev tool can bind to; what is bound to one shows its value.
        self.bindings.set_variables(app_variables(ui.visuals(), self.lang));
        self.bindings.apply(&mut self.menu.settings, &mut self.buttons);
        self.bindings.apply_title(&mut self.title);
        // The program's state, by action id: enabled / visible, and what it does not handle.
        self.unhandled = demo_actions(&self.history, self.share_plugin).apply(&self.ids, &mut self.buttons);
        egui::Panel::left("settings").min_size(220.0).show(ui, |ui| {
            ui.add_space(4.0);
            let query = self.search.trim().to_lowercase();
            // Two tabs; the one that is not showing carries the number of search hits.
            let hits = [
                (LeftTab::Menu, "Menu", menu_hit_count(&self.menu.settings, &query)),
                (LeftTab::Buttons, "Buttons", buttons_hit_count(&self.buttons, &self.menu.settings, &query)),
            ];
            ui.horizontal(|ui| {
                for (tab, name, hits) in hits {
                    let label = if !query.is_empty() && self.left_tab != tab { format!("{name} ({hits})") } else { name.to_string() };
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
            match self.left_tab {
                LeftTab::Menu => {
                    egui::ScrollArea::vertical().show(ui, |ui| self.menu_tab(ui, &query));
                }
                LeftTab::Buttons => self.buttons_tab(ui, &query),
            }
        });

        // The generated code, live. Declared before the central panel, so the central panel
        // (and with it the menu's viewport) shrinks to what both side panels leave.
        if self.show_code {
            egui::Panel::right("code").resizable(true).default_size(440.0).min_size(260.0).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.code_tab, CodeTab::Menu, "Menu code");
                    ui.selectable_value(&mut self.code_tab, CodeTab::Draw, "Draw code");
                    ui.selectable_value(&mut self.code_tab, CodeTab::Toml, "TOML");
                });
                ui.separator();
                match self.code_tab {
                    CodeTab::Menu => {
                        let title = (!self.title.is_empty()).then_some(self.title.as_str());
                        let code = egui_pie_menu::dev_tool::menu_to_code_bound(title, &self.menu.settings, &self.buttons, &self.bindings);
                        egui_pie_menu::dev_tool::code_view(ui, &code);
                    }
                    CodeTab::Draw => {
                        let code = draw_code(&self.buttons, &self.decor);
                        egui_pie_menu::dev_tool::code_view(ui, &code);
                    }
                    CodeTab::Toml => {
                        self.toml_files_row(ui);
                        let code = self.definition().to_toml();
                        egui_pie_menu::dev_tool::code_view(ui, &code);
                    }
                }
            });
        }

        // Problems (shared key / direction) in any level are listed in a window and marked in
        // the tree; the menu stays closed until they are fixed, because debug builds assert on
        // them when the level opens. The menu needs at least one button.
        let problems = tree_problems(&self.menu.settings, &self.buttons);
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
                    ui.label("The menu is disabled until this is fixed (see the Buttons tab).");
                });
        }

        let configuring = self.mode == Mode::Configure;
        if configuring {
            // The real menu is not part of Configure mode.
            self.menu.close();
        }
        egui::CentralPanel::default().show(ui, |ui| {
            // The panel's own area (not its clip rect, which can be larger), so the
            // pinned menu, the result text and the pointer checks share one centre.
            let viewport = ui.max_rect();
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.mode, Mode::Demo, "Demo")
                    .on_hover_text("The real menu: right-click or hold Space to open it, as in an app");
                ui.selectable_value(&mut self.mode, Mode::Configure, "Configure")
                    .on_hover_text("A preview of what the selected node of the button tree is about; click a button to select it");
                ui.separator();
                ui.add_enabled(!configuring, egui::Checkbox::new(&mut self.pinned, "Pin pie-menu"))
                    .on_disabled_hover_text("Configure mode pins its own previews");
                ui.checkbox(&mut self.show_code, "Code panel");
                ui.separator();
                // egui's own theme switch (System / Dark / Light), so the default text colours,
                // which come from the theme, can be checked in both.
                egui::widgets::global_theme_preference_buttons(ui);
                ui.separator();
                ui.label(RichText::new(&self.last_action).weak());
            });
            if configuring {
                ui.label(RichText::new("Configure: the canvas previews the selected node (no pointer, number keys do nothing). Click a button to select it. Hover, reveal and navigation settings are judged in Demo mode.").small().weak());
                self.configure_canvas(ctx, ui, viewport);
                return;
            }
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

            // The title is the open (replace-style) sub-pie's parent, or the "Menu title" at the root.
            self.menu.title = match button_at(&self.buttons, self.menu.path()) {
                Some(parent) => Some(parent.label.clone()),
                None => (!self.title.is_empty()).then(|| self.title.clone()),
            };
            let look = Look {
                decor: &self.decor,
                images: &self.images,
                icons: &self.icons,
                sub_pie_look: self.sub_pie_look,
                button_h: self.button_height,
                hint_gap: self.menu.settings.number_hints.spacing,
                show_hints: self.show_hints,
                unhandled: &self.unhandled,
            };

            // Does nothing while the menu is closed; closes itself on any response but
            // `None` (a pinned menu stays open after a selection). `b.path` is the button's
            // route from the root, whichever ring it is in (the level shown, or a fan-out
            // open beside it): it indexes the tree directly.
            let response = self.menu.show(ctx, &self.buttons, key_down, |ui, b| {
                draw_button(ui, b, b.path, &look, None);
            });
            match response {
                PieMenuResponse::Selected(_) => {
                    // The full route from the root: "Paste Special › As Text".
                    let route = self.menu.selected_path().to_vec();
                    let picked = self.ids.selected(&self.buttons, &route).map(str::to_string);
                    if let Some(id) = &picked {
                        self.history.run(id);
                    }
                    let id = picked.map(|id| format!("  [{id}]")).unwrap_or_default();
                    // A cluster's item: the last element is its number, not a button index.
                    let item = route.split_last().and_then(|(&n, up)| {
                        let cluster = button_at(&self.buttons, up).filter(|b| b.is_cluster())?;
                        let label = cluster.cluster_items().iter().find(|i| i.number == n)?.label.clone();
                        Some((up.to_vec(), label))
                    });
                    if let Some((up, label)) = item {
                        self.last_action = format!("Selected: {} › {label}{id}", names_along(&self.buttons, &up));
                        return;
                    }
                    let name = names_along(&self.buttons, &route);
                    let checkbox = button_at(&self.buttons, &route).is_some_and(|b| is_checkbox(b, &decor_at(&self.decor, &route)));
                    self.last_action = if checkbox {
                        let d = decor_mut(&mut self.decor, &route);
                        d.checked = !d.checked;
                        format!("Selected: {name} ({}){id}", if d.checked { "on" } else { "off" })
                    } else {
                        format!("Selected: {name}{id}")
                    };
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
    use egui_pie_menu::ClusterItem;

    fn labels(map: &DecorMap, paths: &[&[usize]]) -> Vec<Option<String>> {
        paths.iter().map(|p| map.get(*p).map(|d| d.tooltip.clone())).collect()
    }

    /// A table with a recognisable tooltip at every path given.
    fn table(paths: &[&[usize]]) -> DecorMap {
        paths
            .iter()
            .map(|p| {
                let mut d = Decor::new(DEFAULT_COLOR);
                d.tooltip = format!("{p:?}");
                (p.to_vec(), d)
            })
            .collect()
    }

    #[test]
    fn default_tree_is_clean_in_both_styles() {
        let (buttons, decor) = default_tree();
        let mut s = PieMenuSettings::default();
        s.input.auto_mnemonics = true;
        assert!(tree_problems(&s, &buttons).is_empty(), "{:?}", tree_problems(&s, &buttons));
        assert_eq!(buttons.len(), 8);
        // Fan-out as the menu-wide style: the library pins what cannot fan out.
        let mut fan = buttons.clone();
        let mut s = s;
        let notice = egui_pie_menu::dev_tool::set_menu_sub_pie_style(&mut s, &mut fan, SubPieStyle::FanOut);
        assert!(notice.is_some_and(|n| n.contains("Paste Special")), "the back button faces the parent");
        assert!(tree_problems(&s, &fan).is_empty());
        // Every button of the tree has a decoration.
        fn walk(level: &[PieButton], decor: &DecorMap, path: &mut Vec<usize>) {
            for (i, b) in level.iter().enumerate() {
                path.push(i);
                assert!(decor.contains_key(path.as_slice()), "no decor for {path:?}");
                walk(b.children(), decor, path);
                path.pop();
            }
        }
        walk(&buttons, &decor, &mut Vec::new());
        assert_eq!(decor.len(), 8 + 6);
        assert_eq!(buttons[1].children().len(), 6, "five children and the back button");
        assert!(buttons[1].children().iter().any(|c| c.is_back()));
        assert_eq!(decor[&vec![7]].kind, ButtonKind::Checkbox);
    }

    #[test]
    fn removing_a_button_shifts_its_siblings_and_drops_its_subtree() {
        let mut m = table(&[&[0], &[1], &[1, 0], &[1, 1], &[2], &[2, 0], &[3]]);
        rekey(&mut m, &TreeEdit::Removed(vec![1]));
        assert_eq!(labels(&m, &[&[0], &[1], &[2]]), vec![Some("[0]".into()), Some("[2]".into()), Some("[3]".into())]);
        assert_eq!(labels(&m, &[&[1, 0]]), vec![Some("[2, 0]".into())], "a sibling's subtree moves with it");
        assert_eq!(m.len(), 4, "[0], [2], [2,0] and [3] remain");
        // One level down: removing [1, 0] shifts [1, 1] only.
        let mut m = table(&[&[1], &[1, 0], &[1, 1], &[1, 2], &[2, 1]]);
        rekey(&mut m, &TreeEdit::Removed(vec![1, 0]));
        assert_eq!(labels(&m, &[&[1, 0], &[1, 1], &[2, 1]]), vec![Some("[1, 1]".into()), Some("[1, 2]".into()), Some("[2, 1]".into())]);
        assert_eq!(m.len(), 4);
        // Removing the last sibling shifts nothing.
        let mut m = table(&[&[0], &[1], &[1, 0]]);
        rekey(&mut m, &TreeEdit::Removed(vec![1]));
        assert_eq!(labels(&m, &[&[0], &[1]]), vec![Some("[0]".into()), None]);
    }

    #[test]
    fn dropping_children_keeps_the_button_and_forgets_below() {
        let mut m = table(&[&[1], &[1, 0], &[1, 1, 0], &[2], &[2, 0]]);
        rekey(&mut m, &TreeEdit::Dropped(vec![1]));
        assert_eq!(labels(&m, &[&[1], &[1, 0], &[1, 1, 0], &[2], &[2, 0]]), vec![
            Some("[1]".into()), None, None, Some("[2]".into()), Some("[2, 0]".into())
        ]);
    }

    #[test]
    fn a_new_button_starts_clean_and_inherits_its_ancestors_colour() {
        let mut m = table(&[&[0], &[0, 0]]);
        m.get_mut(&vec![0]).expect("present").color = Color32::from_rgb(1, 2, 3);
        // A stale entry at the new path is cleared.
        rekey(&mut m, &TreeEdit::Added(vec![0, 0]));
        assert!(!m.contains_key(&vec![0, 0]));
        // A path nobody decorated (a restored child, a new button) falls back to defaults.
        let d = decor_at(&m, &[0, 5]);
        assert_eq!(d.color, Color32::from_rgb(1, 2, 3));
        assert_eq!(d.kind, ButtonKind::Button);
        assert!(d.tooltip.is_empty());
        assert_eq!(decor_at(&DecorMap::new(), &[4]).color, DEFAULT_COLOR);
        // decor_mut stores the derived default.
        assert_eq!(decor_mut(&mut m, &[0, 5]).color, Color32::from_rgb(1, 2, 3));
        assert!(m.contains_key(&vec![0, 5]));
    }

    #[test]
    fn the_pretend_history() {
        let mut h = History::default();
        h.run("edit.undo");
        assert_eq!(h, History::default(), "nothing to undo");
        h.run("edit.copy");
        h.run("edit.cut");
        h.run("edit.undo");
        assert_eq!(h, History { done: 1, undone: 1 });
        h.run("edit.redo");
        assert_eq!(h, History { done: 2, undone: 0 });
        h.run("edit.undo");
        h.run("paste.values");
        assert_eq!(h, History { done: 2, undone: 0 }, "a new action drops the redo");
    }

    #[test]
    fn app_variables_cover_every_language() {
        assert_eq!(TITLES.len(), LANGS.len());
        for lang in 0..LANGS.len() {
            let vars = app_variables(&egui::Visuals::dark(), lang);
            assert_eq!(vars.get("i18n.edit"), Some(&egui_pie_menu::dev_tool::VarValue::Text(TITLES[lang].to_string())));
            assert_eq!(vars.get("i18n.undo"), Some(&egui_pie_menu::dev_tool::VarValue::Text(LANGS[lang].labels[6].to_string())));
            assert_eq!(vars.code_for("i18n.undo"), "tr(\"undo\")", "generated code reads it with tr");
            assert_eq!(vars.code_for("theme.accent"), "theme.accent");
        }
        let (light, dark) = (app_variables(&egui::Visuals::light(), 0), app_variables(&egui::Visuals::dark(), 0));
        assert_ne!(light.get("theme.panel"), dark.get("theme.panel"), "the theme switch changes the theme variables");
    }

    #[test]
    fn rekey_follows_a_real_edit_sequence() {
        // The same bookkeeping the library's inspector reports: remove, then add.
        let (mut buttons, mut decor) = default_tree();
        let s = PieMenuSettings::default();
        assert!(s.input.use_number_keys);
        decor.get_mut(&vec![2]).expect("redo").tooltip = "redo!".into();
        buttons.remove(0);
        rekey(&mut decor, &TreeEdit::Removed(vec![0]));
        assert_eq!(button_at(&buttons, &[1]).map(|b| b.label.as_str()), Some("Redo"));
        assert_eq!(decor[&vec![1]].tooltip, "redo!", "Redo moved from index 2 to 1");
        assert_eq!(decor[&vec![0, 0]].color, decor[&vec![0]].color, "Paste Special's children moved to [0, k]");
        assert_eq!(decor.len(), 7 + 6);
    }

    #[test]
    fn buttons_search_counts_labels_directions_and_kinds() {
        let (buttons, _) = default_tree();
        let s = PieMenuSettings::default();
        assert_eq!(buttons_hit_count(&buttons, &s, ""), 0);
        assert_eq!(buttons_hit_count(&buttons, &s, "copy"), 1);
        assert_eq!(buttons_hit_count(&buttons, &s, "as text"), 1, "a sub-pie child counts");
        // "back" is the kind of one button; "sub-pie" the kind of Paste Special.
        assert_eq!(buttons_hit_count(&buttons, &s, "back"), 1);
        assert_eq!(buttons_hit_count(&buttons, &s, "sub-pie"), 1);
        assert_eq!(buttons_hit_count(&buttons, &s, "fan-out"), 0);
        let mut fan = s.clone();
        fan.sub_pies.style = SubPieStyle::FanOut;
        assert_eq!(buttons_hit_count(&buttons, &fan, "fan-out"), 1, "the menu's style decides");
        // A direction name matches a root button and the children in that direction.
        assert!(buttons_hit_count(&buttons, &s, "northeast") >= 1);
        assert_eq!(buttons_hit_count(&buttons, &s, "zzz"), 0);
        assert!(!button_matches(&buttons[0], &s, "ctrl"), "tooltips are not searched");
    }

    /// Runs `f` for a few frames in a headless egui context (layout and id clashes that panic
    /// show up here; how it looks does not).
    fn frames(demo: &mut Demo, mut f: impl FnMut(&mut Demo, &mut egui::Ui)) {
        let ctx = egui::Context::default();
        for _ in 0..3 {
            let mut out = ctx.run_ui(Default::default(), |ui| f(demo, ui));
            out.textures_delta.clear();
        }
    }

    #[test]
    fn the_tabs_draw_without_panicking() {
        let ctx = egui::Context::default();
        let mut demo = Demo::new(&ctx);
        frames(&mut demo, |d, ui| d.menu_tab(ui, ""));
        frames(&mut demo, |d, ui| d.menu_tab(ui, "radius"));
        frames(&mut demo, |d, ui| d.buttons_tab(ui, ""));
        // Every node of the tree selected in turn, with the inspector and the Appearance group.
        for sel in [Selection::root(), Selection::button([1]), Selection::button([1, 0]), Selection::button([7])] {
            demo.selection = sel;
            frames(&mut demo, |d, ui| d.buttons_tab(ui, ""));
        }
        frames(&mut demo, |d, ui| d.buttons_tab(ui, "paste"));
        assert!(tree_problems(&demo.menu.settings, &demo.buttons).is_empty());
    }

    #[test]
    fn toml_open_save_and_open_again() {
        /// What the demo does each frame: the program's variables and actions applied.
        fn refresh(demo: &mut Demo) {
            demo.bindings.set_variables(app_variables(&egui::Visuals::dark(), 0));
            demo.bindings.apply(&mut demo.menu.settings, &mut demo.buttons);
            demo.unhandled = demo_actions(&demo.history, demo.share_plugin).apply(&demo.ids, &mut demo.buttons);
        }
        let ctx = egui::Context::default();
        let mut demo = Demo::new(&ctx);
        frames(&mut demo, |d, ui| d.toml_files_row(ui));
        demo.open_toml();
        assert!(demo.toml_note.starts_with("Opened `edit`"), "{}", demo.toml_note);
        // The one reference the demo does not answer: the plugin's Print, on purpose.
        assert!(
            demo.toml_note.ends_with("\n\u{26a0} the id of \"Share\" \u{203a} \"Print\": `plugin.share.print` is not an action of the program"),
            "{}",
            demo.toml_note
        );
        assert_eq!(demo.buttons.len(), 7, "edit.toml and plugin.toml merged");
        assert_eq!(demo.ids.selected(&demo.buttons, &[6, 2]), Some("plugin.share.chat"));
        assert_eq!(demo.menu.settings.layout.radius, 130.0);
        assert!(demo.bindings.get(&BindTarget::Title).is_some());
        // Drawn with the opened menu: the Id row of a button, the marked tree.
        refresh(&mut demo);
        assert_eq!(demo.unhandled, [IdKey { path: vec![6, 4], item: None }]);
        assert!(!demo.buttons[6].children()[4].enabled, "Print has no action: disabled");
        demo.selection = Selection::button([4]);
        frames(&mut demo, |d, ui| d.buttons_tab(ui, ""));
        // Undo is enabled only when the history allows it.
        assert_eq!(demo.ids.button(&[4]), Some("edit.undo"));
        assert!(!demo.buttons[4].enabled, "nothing to undo yet");
        demo.history.run("edit.copy");
        refresh(&mut demo);
        assert!(demo.buttons[4].enabled);
        // The plugin's buttons follow the "Share plugin on" switch.
        assert!(demo.buttons[6].visible);
        demo.share_plugin = false;
        refresh(&mut demo);
        assert!(!demo.buttons[6].visible);
        demo.share_plugin = true;
        refresh(&mut demo);
        // Save to a file of its own and open it alone: the same menu.
        let path = std::env::temp_dir().join(format!("pie_demo_{}.toml", std::process::id()));
        demo.toml_files = path.display().to_string();
        demo.ids.set(IdKey { path: vec![0], item: None }, "edit.copy_renamed");
        refresh(&mut demo);
        assert!(!demo.buttons[0].enabled, "a renamed id the demo does not handle");
        let before = demo.definition();
        demo.save_toml();
        assert!(demo.toml_note.starts_with("Saved"), "{}", demo.toml_note);
        assert!(!std::fs::read_to_string(&path).unwrap().contains("enabled"), "state is not saved");
        demo.buttons.clear();
        demo.open_toml();
        refresh(&mut demo);
        let after = demo.definition();
        std::fs::remove_file(&path).ok();
        assert_eq!(format!("{:?}", after.buttons), format!("{:?}", before.buttons));
        assert_eq!(after.ids, before.ids);
        assert_eq!(after.bindings, before.bindings);
        // With the user's file as well: its removals and changes.
        demo.toml_files = "examples/menus/edit.toml, examples/menus/plugin.toml, examples/menus/user.toml".to_string();
        demo.open_toml();
        refresh(&mut demo);
        assert!(demo.ids.find("edit.paste_special").is_none());
        assert!(demo.ids.find("edit.select_all").is_some_and(|k| demo.buttons[k.path[0]].enabled), "an action the demo handles");
        // A missing file is reported, and the menu stays.
        let count = demo.buttons.len();
        demo.toml_files = "no/such/file.toml".to_string();
        demo.open_toml();
        assert!(demo.toml_note.starts_with("no/such/file.toml: "), "{}", demo.toml_note);
        assert_eq!(demo.buttons.len(), count);
    }

    #[test]
    fn configure_mode_previews_every_node_without_panicking() {
        let ctx = egui::Context::default();
        let mut demo = Demo::new(&ctx);
        demo.mode = Mode::Configure;
        let viewport = Rect::from_min_size(egui::pos2(0.0, 0.0), Vec2::new(800.0, 600.0));
        let paths: Vec<Selection> = vec![
            Selection::root(),
            Selection::button([0]),
            Selection::button([1]),
            Selection::button([1, 0]),
            Selection::button([7]),
        ];
        for sel in paths {
            demo.selection = sel;
            frames(&mut demo, |d, ui| {
                let ctx = ui.ctx().clone();
                d.configure_canvas(&ctx, ui, viewport);
            });
        }
        // One preview menu per level that was shown, never the real menu.
        assert!(demo.previews.contains_key(&vec![]) && demo.previews.contains_key(&vec![1]));
        assert!(demo.previews.values().all(PieMenu::is_pinned));
        assert!(!demo.menu.is_open());
        // A tree with a problem is not previewed: the last clean one stays.
        let clean = demo.snapshot.len();
        demo.buttons.push(PieButton::new(PieDirection::North)); // the North slot is taken
        frames(&mut demo, |d, ui| {
            let ctx = ui.ctx().clone();
            d.configure_canvas(&ctx, ui, viewport);
        });
        assert_eq!(demo.snapshot.len(), clean);
        assert!(!tree_problems(&demo.menu.settings, &demo.buttons).is_empty());
    }

    /// A click on a preview button selects it in the tree (the overlay of `draw_button`).
    /// Headless: the pointer goes to where the North button of the ring is drawn.
    #[test]
    fn clicking_a_preview_button_selects_it() {
        let ctx = egui::Context::default();
        let mut demo = Demo::new(&ctx);
        demo.mode = Mode::Configure;
        demo.selection = Selection::root();
        let viewport = Rect::from_min_size(egui::pos2(0.0, 0.0), Vec2::new(800.0, 600.0));
        let frame = |demo: &mut Demo, events: Vec<egui::Event>| {
            let input = egui::RawInput { events, ..Default::default() };
            let mut out = ctx.run_ui(input, |ui| {
                let ctx = ui.ctx().clone();
                demo.configure_canvas(&ctx, ui, viewport);
            });
            out.textures_delta.clear();
        };
        for _ in 0..5 {
            frame(&mut demo, vec![]);
        }
        let center = demo.previews[&vec![]].position;
        let at = egui::pos2(center.x, center.y - demo.menu.settings.layout.radius);
        let button = |pressed| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        frame(&mut demo, vec![egui::Event::PointerMoved(at)]);
        frame(&mut demo, vec![button(true)]);
        frame(&mut demo, vec![button(false)]);
        assert_eq!(demo.selection, Selection::button([0]), "the North button is Copy");

        // "Word wrap" (NW) is drawn as a checkbox row whose inner checkbox also takes clicks:
        // the overlay on top still selects the button, wherever on it the click lands.
        let mut hit = Vec::new();
        for ix in 0..16 {
            for iy in 0..16 {
                let at = egui::pos2(center.x - 170.0 + 8.0 * ix as f32, center.y - 130.0 + 8.0 * iy as f32);
                demo.selection = Selection::root();
                let button = |pressed| egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                };
                frame(&mut demo, vec![egui::Event::PointerMoved(at)]);
                frame(&mut demo, vec![button(true)]);
                frame(&mut demo, vec![button(false)]);
                if demo.selection == Selection::button([7]) {
                    hit.push(at);
                }
            }
        }
        assert!(hit.len() >= 4, "clicks on the checkbox button select it: {} hits", hit.len());
    }

    /// A cluster: a click on one of its items selects that item, a click on its frame the button.
    #[test]
    fn clicking_a_preview_cluster_selects_item_or_button() {
        let ctx = egui::Context::default();
        let mut demo = Demo::new(&ctx);
        demo.mode = Mode::Configure;
        demo.buttons[2] = PieButton::cluster(PieDirection::East)
            .with_label("Format")
            .with_items(vec![ClusterItem::new(0, "Bold"), ClusterItem::new(1, "Italic")]);
        demo.decor.remove(&vec![2]);
        let viewport = Rect::from_min_size(egui::pos2(0.0, 0.0), Vec2::new(800.0, 600.0));
        let frame = |demo: &mut Demo, events: Vec<egui::Event>| {
            let input = egui::RawInput { events, ..Default::default() };
            let mut out = ctx.run_ui(input, |ui| {
                let ctx = ui.ctx().clone();
                demo.configure_canvas(&ctx, ui, viewport);
            });
            out.textures_delta.clear();
        };
        for _ in 0..5 {
            frame(&mut demo, vec![]);
        }
        let center = demo.previews[&vec![]].position;
        let (mut items, mut frames_hit) = (std::collections::BTreeSet::new(), 0);
        for ix in 0..24 {
            for iy in 0..10 {
                let at = egui::pos2(center.x + 20.0 + 8.0 * ix as f32, center.y - 40.0 + 8.0 * iy as f32);
                demo.selection = Selection::root();
                let button = |pressed| egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                };
                frame(&mut demo, vec![egui::Event::PointerMoved(at)]);
                frame(&mut demo, vec![button(true)]);
                frame(&mut demo, vec![button(false)]);
                match (&demo.selection.path[..], demo.selection.item) {
                    ([2], Some(k)) => {
                        items.insert(k);
                    }
                    ([2], None) => frames_hit += 1,
                    _ => {}
                }
            }
        }
        assert_eq!(items.into_iter().collect::<Vec<_>>(), vec![0, 1], "both items can be clicked");
        assert!(frames_hit > 0, "the frame around the items selects the cluster itself");
    }

    #[test]
    fn a_fan_out_style_pins_what_cannot_fan_and_shows_the_notice() {
        let ctx = egui::Context::default();
        let mut demo = Demo::new(&ctx);
        // No change of style: nothing happens.
        demo.follow_menu_style(SubPieStyle::Replace);
        assert!(demo.notice.is_none());
        // The combo box sets the field; the demo then follows.
        demo.menu.settings.sub_pies.style = SubPieStyle::FanOut;
        demo.follow_menu_style(SubPieStyle::Replace);
        let notice = demo.notice.clone().expect("Paste Special cannot fan out: its back button faces the parent");
        assert!(notice.contains("Paste Special"), "{notice}");
        assert!(matches!(&demo.buttons[1].kind, PieButtonKind::SubPie { style: Some(SubPieStyle::Replace), .. }));
        assert!(tree_problems(&demo.menu.settings, &demo.buttons).is_empty());
        // Back to replace: the pins stay, no new notice.
        demo.notice = None;
        demo.menu.settings.sub_pies.style = SubPieStyle::Replace;
        demo.follow_menu_style(SubPieStyle::FanOut);
        assert!(demo.notice.is_none());
    }

    #[test]
    fn names_and_lookup_follow_the_tree() {
        let (buttons, _) = default_tree();
        assert_eq!(names_along(&buttons, &[1, 0]), "Paste Special › ".to_string() + &button_at(&buttons, &[1, 0]).expect("child").label);
        assert_eq!(names_along(&buttons, &[0]), "Copy");
        assert!(button_at(&buttons, &[]).is_none());
        assert!(button_at(&buttons, &[9]).is_none());
        assert!(button_at(&buttons, &[0, 0]).is_none(), "Copy has no children");
    }

    #[test]
    fn sample_icons_follow_directions_and_reset_restores() {
        let (buttons, mut decor) = default_tree();
        sample_icons(&buttons, &mut decor, SampleIcons::Vector);
        assert_eq!(decor[&vec![0]].icon_kind, IconKind::Vector(VectorIcon::Copy));
        assert_eq!(decor[&vec![4]].icon_kind, IconKind::Vector(VectorIcon::Trash));
        sample_icons(&buttons, &mut decor, SampleIcons::Image);
        assert!(matches!(decor[&vec![3]].icon_kind, IconKind::Image(3)));
        sample_icons(&buttons, &mut decor, SampleIcons::Reset);
        assert_eq!(decor[&vec![0]].icon_kind, IconKind::Text);
        assert_eq!(decor[&vec![4]].icon, "🗑");
    }

    #[test]
    fn draw_code_covers_the_tree() {
        let (buttons, decor) = default_tree();
        let code = draw_code(&buttons, &decor);
        assert!(code.contains("[0] => { // Copy"));
        assert!(code.contains(".on_hover_text(\"Ctrl+C\")"));
        assert!(code.contains("from_rgb(70, 130, 180)"));
        assert!(code.contains("[1, 0] => {"), "sub-pie children get nested paths");
        assert!(code.contains("prefix(\"🗑  \")"));
        assert!(code.contains("ui.checkbox(&mut checked"));
        assert!(code.contains("// Back"), "the sub-pie's back button is drawn");
        // A cluster: its items by number, in the chosen layout.
        let cluster = PieButton::cluster(PieDirection::North)
            .with_label("Format")
            .with_items(vec![ClusterItem::new(7, "Bold"), ClusterItem::new(9, "Italic"), ClusterItem::new(12, "Underline")]);
        let mut decor = DecorMap::new();
        decor_mut(&mut decor, &[0]).cluster = ClusterLook { layout: ClusterLayout::Grid, spacing: 6.0 };
        let code = draw_code(&[cluster], &decor);
        assert!(code.contains("b.item(9, ui.button(b.item_label(9)));"));
        assert!(code.contains("egui::Grid::new(\"cluster\")"));
        assert!(code.contains("ui.end_row();"));
    }

    #[test]
    fn a_checkbox_is_only_a_plain_button() {
        let mut d = Decor::new(DEFAULT_COLOR);
        d.kind = ButtonKind::Checkbox;
        assert!(is_checkbox(&PieButton::new(PieDirection::North), &d));
        assert!(!is_checkbox(&PieButton::back(PieDirection::North), &d));
        assert!(!is_checkbox(&PieButton::new(PieDirection::North).with_children(vec![PieButton::new(PieDirection::South)]), &d));
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
    fn preview_settings_override_only_what_reacts_to_input() {
        let mut real = PieMenuSettings::default();
        real.sub_pies.hover_delay = Some(std::time::Duration::from_millis(400));
        real.animations.sub_pie_hover_emphasis.scale = 1.3;
        real.layout.radius = 123.0;
        let p = preview_settings(&real, false);
        assert_eq!(p.sub_pies.hover_delay, None);
        assert!(!p.input.use_mnemonic_keys);
        assert!(p.input.pinned_dismiss_inputs.is_empty());
        assert_eq!(p.layout.radius, 123.0, "everything else follows the real settings");
        assert_eq!(p.animations.hover_emphasis, real.animations.hover_emphasis);
        // The children of a sub-pie are drawn with the sub-pie hover emphasis.
        assert_eq!(preview_settings(&real, true).animations.hover_emphasis, real.animations.sub_pie_hover_emphasis);
        // The real settings are untouched.
        assert!(real.sub_pies.hover_delay.is_some() && real.input.use_mnemonic_keys);
    }

    #[test]
    fn the_preview_level_follows_the_selection() {
        let (buttons, _) = default_tree();
        let s = PieMenuSettings::default();
        let level_of = |sel: Selection| {
            let t = preview_target(&buttons, &s, &sel);
            (t.level_path.clone(), level_at(&buttons, &t.level_path).map(<[PieButton]>::len), t.fan_trigger)
        };
        assert_eq!(level_of(Selection::root()), (vec![], Some(8), None));
        assert_eq!(level_of(Selection::button([0])), (vec![], Some(8), None), "a plain button: its own ring");
        assert_eq!(level_of(Selection::button([1])), (vec![1], Some(6), None), "a replace sub-pie shows its children");
        assert_eq!(level_of(Selection::button([1, 0])), (vec![1], Some(6), None), "a child: the same ring");
        // Under a fan-out style the parent ring stays and the fan opens.
        let mut fan = s.clone();
        fan.sub_pies.style = SubPieStyle::FanOut;
        let mut buttons = buttons;
        buttons[1] = PieButton::new(PieDirection::NorthEast).with_label("Fan").with_fan_out(vec![
            PieButton::new(PieDirection::NorthEast).with_label("A"),
            PieButton::new(PieDirection::East).with_label("B"),
        ]);
        let t = preview_target(&buttons, &fan, &Selection::button([1]));
        assert_eq!((t.level_path, t.fan_trigger), (vec![], Some(1)));
        let t = preview_target(&buttons, &fan, &Selection::button([1, 1]));
        assert_eq!((t.level_path, t.fan_trigger, t.highlight), (vec![], Some(1), Some(1)));
    }

    #[test]
    fn demo_only_settings_are_listed_for_the_selected_node() {
        let (buttons, _) = default_tree();
        let s = PieMenuSettings::default();
        let fields = demo_only_fields(&s, &buttons, &Selection::button([1]));
        assert!(fields.contains(&SettingsField::SubPieCenter) && fields.contains(&SettingsField::SubPieHoverDelay));
        assert!(fields.iter().all(|f| judged_in_demo_only(*f)));
        // A root action: only the root hover emphasis and easing.
        let fields = demo_only_fields(&s, &buttons, &Selection::button([0]));
        assert_eq!(fields, vec![SettingsField::HoverEmphasis, SettingsField::HoverDuration]);
        assert!(demo_only_note(&fields).contains("Demo mode"));
        // The menu itself: the ring settings are shown by the preview, the hover ones are not.
        let fields = demo_only_fields(&s, &buttons, &Selection::root());
        assert!(!fields.contains(&SettingsField::LayoutRadius));
        assert_eq!(demo_only_note(&[]), "");
        // Layout and colours are judged in the preview.
        assert!(!judged_in_demo_only(SettingsField::LayoutRadius));
        assert!(!judged_in_demo_only(SettingsField::TextColor));
    }

    #[test]
    fn hint_rows_follow_number_keys() {
        let mut s = PieMenuSettings::default();
        assert!(group_rows(SettingsGroup::KeysAndHints, &s).contains(&Row::Demo(DemoRow::ShowHints)));
        s.input.use_number_keys = false;
        assert!(!group_rows(SettingsGroup::KeysAndHints, &s).contains(&Row::Demo(DemoRow::ShowHints)));
    }
}
