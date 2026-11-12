//! Embeddable dev tool pieces: editors for [`PieMenuSettings`] and for a [`PieButton`] tree,
//! and generators for ready-to-paste Rust code (in the `configure` closure style).
//!
//! There is no window of its own: put [`settings_editor`] and [`buttons_editor`] in a side
//! panel, show [`menu_to_code`] (or [`settings_to_code`] / [`buttons_to_code`]) with
//! [`code_view`], and paste the result into your project.
use crate::settings::*;
use crate::{ButtonProblem, ClusterItem, Mnemonic, PieButton, PieButtonKind, PieMenu};
use egui::{Color32, FontFamily, FontId, Margin, RichText, Stroke, Ui};
use std::ops::RangeInclusive;
use std::time::Duration;

// ---------------------------------------------------------------------------------------
// Code generation: settings
// ---------------------------------------------------------------------------------------

fn color_code(c: Color32) -> String {
    let [r, g, b, a] = c.to_srgba_unmultiplied();
    format!("egui::Color32::from_rgba_unmultiplied({r}, {g}, {b}, {a})")
}

fn opt_f32_code(v: Option<f32>) -> String {
    match v {
        Some(x) => format!("Some({x:?})"),
        None => "None".to_string(),
    }
}

fn opt_color_code(v: Option<Color32>) -> String {
    match v {
        Some(c) => format!("Some({})", color_code(c)),
        None => "None".to_string(),
    }
}

fn duration_code(d: Duration) -> String {
    if d.subsec_nanos().is_multiple_of(1_000_000) {
        format!("Duration::from_millis({})", d.as_millis())
    } else {
        format!("Duration::from_secs_f32({:?})", d.as_secs_f32())
    }
}

fn opt_duration_code(d: Option<Duration>) -> String {
    match d {
        Some(d) => format!("Some({})", duration_code(d)),
        None => "None".to_string(),
    }
}

fn stroke_code(s: Stroke) -> String {
    format!("egui::Stroke::new({:?}, {})", s.width, color_code(s.color))
}

fn margin_code(m: Margin) -> String {
    format!("egui::Margin {{ left: {}, right: {}, top: {}, bottom: {} }}", m.left, m.right, m.top, m.bottom)
}

fn font_code(f: &FontId) -> String {
    let family = match &f.family {
        FontFamily::Proportional => "egui::FontFamily::Proportional".to_string(),
        FontFamily::Monospace => "egui::FontFamily::Monospace".to_string(),
        FontFamily::Name(name) => format!("egui::FontFamily::Name({:?}.into())", &**name),
    };
    format!("egui::FontId::new({:?}, {family})", f.size)
}

fn emphasis_code(e: HoverEmphasis) -> String {
    format!("HoverEmphasis {{ shift: {:?}, scale: {:?} }}", e.shift, e.scale)
}

fn dismiss_code(list: &[PieMenuDismissInput]) -> String {
    let items: Vec<String> = list
        .iter()
        .map(|d| match d {
            PieMenuDismissInput::Key(k) => format!("PieMenuDismissInput::Key(egui::Key::{k:?})"),
            PieMenuDismissInput::PointerButton(b) => {
                format!("PieMenuDismissInput::PointerButton(egui::PointerButton::{b:?})")
            }
        })
        .collect();
    format!("vec![{}]", items.join(", "))
}

/// Every settings field as `(path, value source)`, paths relative to the settings (`layout.radius`).
fn settings_entries(s: &PieMenuSettings) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = Vec::new();
    macro_rules! e {
        ($path:expr, $value:expr) => {
            v.push(($path.to_string(), $value))
        };
    }
    let l = &s.layout;
    e!("layout.radius", format!("{:?}", l.radius));
    e!("layout.shape_factor", format!("{:?}", l.shape_factor));
    e!("layout.intercardinal_height", opt_f32_code(l.intercardinal_height));
    e!("layout.screen_margin", format!("{:?}", l.screen_margin));
    e!("layout.dead_zone_radius", format!("{:?}", l.dead_zone_radius));

    e!(
        "show_behavior",
        match &s.show_behavior {
            ShowBehavior::Instant => "ShowBehavior::Instant".to_string(),
            ShowBehavior::OnMovement { threshold } => format!("ShowBehavior::OnMovement {{ threshold: {threshold:?} }}"),
        }
    );

    let i = &s.input;
    e!("input.use_number_keys", i.use_number_keys.to_string());
    e!("input.use_mnemonic_keys", i.use_mnemonic_keys.to_string());
    e!("input.auto_mnemonics", i.auto_mnemonics.to_string());
    e!("input.double_tap_window", duration_code(i.double_tap_window));
    e!("input.select_on_primary_click", i.select_on_primary_click.to_string());
    e!("input.dismiss_inputs", dismiss_code(&i.dismiss_inputs));
    e!("input.pinned_dismiss_inputs", dismiss_code(&i.pinned_dismiss_inputs));

    let c = &s.center_indicator;
    e!("center_indicator.background_radius", opt_f32_code(c.background_radius));
    e!("center_indicator.background_stroke", stroke_code(c.background_stroke));
    e!("center_indicator.background_fill_color", color_code(c.background_fill_color));
    e!("center_indicator.highlight_radius", opt_f32_code(c.highlight_radius));
    e!(
        "center_indicator.highlight_shape",
        format!(
            "HighlightShape {{ arc: {}, slice: {}, circle: {} }}",
            c.highlight_shape.arc, c.highlight_shape.slice, c.highlight_shape.circle
        )
    );
    e!("center_indicator.highlight_stroke", stroke_code(c.highlight_stroke));
    e!("center_indicator.highlight_fill_color", color_code(c.highlight_fill_color));
    e!("center_indicator.highlight_angle", format!("{:?}", c.highlight_angle));
    e!("center_indicator.highlight_circle_radius", format!("{:?}", c.highlight_circle_radius));
    e!("center_indicator.highlight_snapping", c.highlight_snapping.to_string());

    let t = &s.title;
    e!("title.show", t.show.to_string());
    e!("title.background_color", color_code(t.background_color));
    e!("title.background_stroke", stroke_code(t.background_stroke));
    e!("title.padding", margin_code(t.padding));
    e!("title.text_color", color_code(t.text_color));
    e!("title.text_font", font_code(&t.text_font));

    let a = &s.animations;
    e!("animations.reveal", format!("RevealAnimation::{:?}", a.reveal));
    e!("animations.reveal_duration", duration_code(a.reveal_duration));
    e!("animations.slide_start_radius", format!("{:?}", a.slide_start_radius));
    e!("animations.slide_start_shape_factor", opt_f32_code(a.slide_start_shape_factor));
    e!("animations.slide_fade", opt_duration_code(a.slide_fade));
    e!("animations.hover_emphasis", emphasis_code(a.hover_emphasis));
    e!("animations.sub_pie_hover_emphasis", emphasis_code(a.sub_pie_hover_emphasis));
    e!("animations.hover_duration", duration_code(a.hover_duration));

    e!("block_background", s.block_background.to_string());

    let p = &s.sub_pies;
    e!("sub_pies.style", format!("SubPieStyle::{:?}", p.style));
    e!("sub_pies.center", format!("SubPieCenter::{:?}", p.center));
    e!("sub_pies.hover_delay", opt_duration_code(p.hover_delay));
    e!("sub_pies.fan_out.delay", duration_code(p.fan_out.delay));
    e!("sub_pies.fan_out.radius", opt_f32_code(p.fan_out.radius));
    e!("sub_pies.fan_out.opener_action", format!("FanOpenerAction::{:?}", p.fan_out.opener_action));

    e!("mirror_horizontal", s.mirror_horizontal.to_string());

    let n = &s.number_hints;
    e!("number_hints.color", opt_color_code(n.color));
    e!("number_hints.spacing", format!("{:?}", n.spacing));
    e!("number_hints.side", format!("HintSide::{:?}", n.side));
    e!("number_hints.digits", format!("HintDigits::{:?}", n.digits));

    e!("text_color", opt_color_code(s.text_color));
    v
}

/// The statements (`s.layout.radius = 120.0;`, without the closure wrapper) for every setting
/// whose generated value differs from [`PieMenuSettings::default`].
pub fn settings_changes(s: &PieMenuSettings) -> Vec<String> {
    let default = settings_entries(&PieMenuSettings::default());
    settings_entries(s)
        .into_iter()
        .zip(default)
        .filter(|(now, was)| now.1 != was.1)
        .map(|((path, value), _)| format!("s.{path} = {value};"))
        .collect()
}

/// The `.configure(|s| { ... })` closure that turns the default settings into `s`, listing only
/// what differs from [`PieMenuSettings::default`]. Empty when nothing differs.
///
/// ```
/// use egui_pie_menu::{PieMenuSettings, dev_tool::settings_to_code};
/// let mut s = PieMenuSettings::default();
/// assert_eq!(settings_to_code(&s), "");
/// s.layout.radius = 120.0;
/// assert_eq!(settings_to_code(&s), ".configure(|s| {\n    s.layout.radius = 120.0;\n})");
/// ```
pub fn settings_to_code(s: &PieMenuSettings) -> String {
    let lines = settings_changes(s);
    if lines.is_empty() {
        return String::new();
    }
    let body: Vec<String> = lines.iter().map(|l| format!("    {l}")).collect();
    format!(".configure(|s| {{\n{}\n}})", body.join("\n"))
}

// ---------------------------------------------------------------------------------------
// Code generation: buttons and the whole menu
// ---------------------------------------------------------------------------------------

fn pad(n: usize) -> String {
    " ".repeat(n)
}

/// `vec![` + one item per line (indented `ind + 4`, trailing commas) + `]` at `ind`.
fn vec_block(items: &[String], ind: usize) -> String {
    if items.is_empty() {
        return "vec![]".to_string();
    }
    let lines: Vec<String> = items.iter().map(|i| format!("{}{i},", pad(ind + 4))).collect();
    format!("vec![\n{}\n{}]", lines.join("\n"), pad(ind))
}

fn mnemonic_code(m: Mnemonic) -> String {
    match m {
        Mnemonic::Auto => String::new(),
        Mnemonic::Key(c) => format!(".with_mnemonic({c:?})"),
        Mnemonic::None => ".without_mnemonic()".to_string(),
    }
}

fn item_code(item: &ClusterItem) -> String {
    let mut s = format!("ClusterItem::new({}, {:?})", item.number, item.label);
    s += &mnemonic_code(item.mnemonic);
    if !item.enabled {
        s += ".with_enabled(false)";
    }
    s
}

/// The expression for one button; `ind` is the indentation of the line it starts on.
fn button_expr(b: &PieButton, ind: usize) -> String {
    let dir = format!("PieDirection::{:?}", b.direction);
    let mut s = match &b.kind {
        PieButtonKind::Back => format!("PieButton::back({dir})"),
        PieButtonKind::Cluster(_) => format!("PieButton::cluster({dir})"),
        _ => format!("PieButton::new({dir})"),
    };
    let default_label = if b.is_back() { "Back" } else { "" };
    if b.label != default_label {
        s += &format!(".with_label({:?})", b.label);
    }
    // A cluster has no mnemonic of its own (`with_items` resets it).
    if !b.is_cluster() {
        s += &mnemonic_code(b.mnemonic);
    }
    if !b.enabled {
        s += ".with_enabled(false)";
    }
    match &b.kind {
        PieButtonKind::SubPie { children, style } => {
            let kids: Vec<String> = children.iter().map(|c| button_expr(c, ind + 4)).collect();
            let block = vec_block(&kids, ind);
            match style {
                None => s += &format!(".with_children({block})"),
                Some(SubPieStyle::FanOut) => s += &format!(".with_fan_out({block})"),
                Some(SubPieStyle::Replace) => {
                    s += &format!(".with_children({block}).with_sub_pie_style(Some(SubPieStyle::Replace))")
                }
            }
        }
        PieButtonKind::Cluster(items) => {
            let lines: Vec<String> = items.iter().map(item_code).collect();
            s += &format!(".with_items({})", vec_block(&lines, ind));
        }
        PieButtonKind::Action | PieButtonKind::Back => {}
    }
    s
}

/// `vec![ .. ]` source that rebuilds `buttons` with the [`PieButton`] builders.
///
/// ```
/// use egui_pie_menu::{PieButton, PieDirection, dev_tool::buttons_to_code};
/// let b = vec![PieButton::new(PieDirection::North).with_label("Copy").with_mnemonic('c')];
/// assert_eq!(
///     buttons_to_code(&b),
///     "vec![\n    PieButton::new(PieDirection::North).with_label(\"Copy\").with_mnemonic('c'),\n]"
/// );
/// ```
pub fn buttons_to_code(buttons: &[PieButton]) -> String {
    let items: Vec<String> = buttons.iter().map(|b| button_expr(b, 4)).collect();
    vec_block(&items, 0)
}

/// A complete snippet: the `use` lines, `let buttons = ..;`, `let menu = PieMenu::new()` with
/// the title and the [`settings_to_code`] closure, and a commented `show` skeleton.
pub fn menu_to_code(title: Option<&str>, settings: &PieMenuSettings, buttons: &[PieButton]) -> String {
    let config = settings_to_code(settings);
    let mut out = String::from("use egui_pie_menu::*;\n");
    if config.contains("Duration::") {
        out += "use std::time::Duration;\n";
    }
    out += "\n";
    if buttons.is_empty() {
        out += "let buttons: Vec<PieButton> = vec![];\n\n";
    } else {
        out += &format!("let buttons = {};\n\n", buttons_to_code(buttons));
    }
    out += "let menu = PieMenu::new()";
    if let Some(t) = title {
        out += &format!("\n    .with_title({t:?})");
    }
    if !config.is_empty() {
        out += &format!("\n    {}", config.replace('\n', "\n    "));
    }
    out += ";\n\n";
    out += "// each frame (key_down: whether your trigger key is held):\n";
    out += "// match menu.show(ctx, &buttons, key_down, |ui, b| {\n";
    out += "//     ui.label(b.label_text());\n";
    out += "// }) {\n";
    out += "//     PieMenuResponse::Selected(idx) => { /* idx, b.path / menu.selected_path() */ }\n";
    out += "//     _ => {}\n";
    out += "// }\n";
    out
}

// ---------------------------------------------------------------------------------------
// Code view
// ---------------------------------------------------------------------------------------

/// A read-only monospace text area filling the available width inside a vertical scroll area,
/// with a "Copy" button above it.
pub fn code_view(ui: &mut Ui, code: &str) {
    if ui.button("Copy").clicked() {
        ui.ctx().copy_text(code.to_owned());
    }
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        let mut text = code;
        ui.add(egui::TextEdit::multiline(&mut text).code_editor().desired_width(f32::INFINITY));
    });
}

// ---------------------------------------------------------------------------------------
// Settings editor
// ---------------------------------------------------------------------------------------

/// A checkbox that switches an optional value on (with `default`) or off, and a slider for it
/// while it is on.
fn optional_slider(ui: &mut Ui, value: &mut Option<f32>, default: f32, range: RangeInclusive<f32>, label: &str) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        let mut on = value.is_some();
        if ui.checkbox(&mut on, label).changed() {
            *value = on.then_some(default);
            changed = true;
        }
        if let Some(v) = value {
            changed |= ui.add(egui::Slider::new(v, range)).changed();
        }
    });
    changed
}

/// A colour button with a label.
fn color_edit(ui: &mut Ui, c: &mut Color32, label: &str) -> bool {
    ui.horizontal(|ui| {
        let changed = ui.color_edit_button_srgba(c).changed();
        ui.label(label);
        changed
    })
    .inner
}

/// A checkbox that switches an optional colour on (with `default`) or off, and a colour
/// button while it is on.
fn optional_color(ui: &mut Ui, value: &mut Option<Color32>, default: Color32, label: &str) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        let mut on = value.is_some();
        if ui.checkbox(&mut on, label).changed() {
            *value = on.then_some(default);
            changed = true;
        }
        if let Some(c) = value {
            changed |= ui.color_edit_button_srgba(c).changed();
        }
    });
    changed
}

/// A slider in milliseconds for a `Duration`.
fn duration_slider(ui: &mut Ui, value: &mut Duration, range: RangeInclusive<u64>, label: &str) -> bool {
    let mut ms = value.as_millis() as u64;
    let changed = ui.add(egui::Slider::new(&mut ms, range).text(label)).changed();
    if changed {
        *value = Duration::from_millis(ms);
    }
    changed
}

/// A checkbox that switches an optional duration on (with `default`) or off, and a slider
/// for it while it is on.
fn optional_duration(ui: &mut Ui, value: &mut Option<Duration>, default: Duration, range: RangeInclusive<u64>, label: &str) -> bool {
    let mut changed = false;
    let mut on = value.is_some();
    if ui.checkbox(&mut on, label).changed() {
        *value = on.then_some(default);
        changed = true;
    }
    if let Some(d) = value {
        changed |= duration_slider(ui, d, range, "ms");
    }
    changed
}

/// Width and colour of a stroke.
fn stroke_edit(ui: &mut Ui, stroke: &mut Stroke, label: &str) -> bool {
    ui.horizontal(|ui| {
        let mut changed = ui.add(egui::DragValue::new(&mut stroke.width).speed(0.1).range(0.0..=64.0)).changed();
        changed |= ui.color_edit_button_srgba(&mut stroke.color).changed();
        ui.label(label);
        changed
    })
    .inner
}

/// The four sides of a margin.
fn margin_edit(ui: &mut Ui, m: &mut Margin, label: &str) -> bool {
    ui.horizontal(|ui| {
        let mut changed = false;
        for (name, side) in [("l", &mut m.left), ("r", &mut m.right), ("t", &mut m.top), ("b", &mut m.bottom)] {
            ui.label(name);
            changed |= ui.add(egui::DragValue::new(side).range(0..=100)).changed();
        }
        ui.label(label);
        changed
    })
    .inner
}

/// Size and family (proportional / monospace) of a font. A named family is kept as it is.
fn font_edit(ui: &mut Ui, f: &mut FontId, label: &str) -> bool {
    ui.horizontal(|ui| {
        let mut changed = ui.add(egui::DragValue::new(&mut f.size).speed(0.2).range(4.0..=96.0)).changed();
        let shown = match &f.family {
            FontFamily::Proportional => "Proportional".to_string(),
            FontFamily::Monospace => "Monospace".to_string(),
            FontFamily::Name(n) => format!("\"{}\"", &**n),
        };
        egui::ComboBox::from_id_salt(("font_family", label)).selected_text(shown).show_ui(ui, |ui| {
            for (name, fam) in [("Proportional", FontFamily::Proportional), ("Monospace", FontFamily::Monospace)] {
                changed |= ui.selectable_value(&mut f.family, fam, name).changed();
            }
        });
        ui.label(label);
        changed
    })
    .inner
}

/// A combo box over `options` for any `Debug` enum.
fn enum_combo<T: Copy + PartialEq + std::fmt::Debug>(ui: &mut Ui, label: &str, value: &mut T, options: &[T]) -> bool {
    let mut changed = false;
    egui::ComboBox::from_label(label).selected_text(format!("{value:?}")).show_ui(ui, |ui| {
        for o in options {
            changed |= ui.selectable_value(value, *o, format!("{o:?}")).changed();
        }
    });
    changed
}

/// The common keys offered when adding a dismiss input.
const DISMISS_KEYS: [egui::Key; 7] = [
    egui::Key::Escape,
    egui::Key::Num5,
    egui::Key::Backspace,
    egui::Key::Space,
    egui::Key::Enter,
    egui::Key::Tab,
    egui::Key::Q,
];

const DISMISS_POINTER: [egui::PointerButton; 3] =
    [egui::PointerButton::Primary, egui::PointerButton::Secondary, egui::PointerButton::Middle];

/// The entries of a dismiss-input list with remove buttons, and an "add" combo.
fn dismiss_edit(ui: &mut Ui, label: &str, list: &mut Vec<PieMenuDismissInput>) -> bool {
    let mut changed = false;
    ui.label(label);
    let mut remove = None;
    for (i, d) in list.iter().enumerate() {
        ui.horizontal(|ui| {
            ui.label(match d {
                PieMenuDismissInput::Key(k) => format!("key {k:?}"),
                PieMenuDismissInput::PointerButton(b) => format!("click {b:?}"),
            });
            if ui.small_button("remove").clicked() {
                remove = Some(i);
            }
        });
    }
    if let Some(i) = remove {
        list.remove(i);
        changed = true;
    }
    egui::ComboBox::from_id_salt(("dismiss_add", label)).selected_text("add...").show_ui(ui, |ui| {
        for k in DISMISS_KEYS {
            if ui.selectable_label(false, format!("key {k:?}")).clicked() {
                list.push(PieMenuDismissInput::Key(k));
                changed = true;
            }
        }
        for b in DISMISS_POINTER {
            if ui.selectable_label(false, format!("click {b:?}")).clicked() {
                list.push(PieMenuDismissInput::PointerButton(b));
                changed = true;
            }
        }
    });
    changed
}

fn slider(ui: &mut Ui, v: &mut f32, range: RangeInclusive<f32>, label: &str) -> bool {
    ui.add(egui::Slider::new(v, range).text(label)).changed()
}

fn group(ui: &mut Ui, name: &str, open: bool, body: impl FnOnce(&mut Ui) -> bool) -> bool {
    egui::CollapsingHeader::new(name).default_open(open).show(ui, body).body_returned.unwrap_or(false)
}

/// Controls for every field of [`PieMenuSettings`], grouped in collapsing headers. Returns
/// `true` if anything changed.
pub fn settings_editor(ui: &mut Ui, s: &mut PieMenuSettings) -> bool {
    let mut changed = false;

    changed |= group(ui, "Layout", true, |ui| {
        let l = &mut s.layout;
        let mut c = slider(ui, &mut l.radius, 20.0..=300.0, "radius");
        c |= slider(ui, &mut l.shape_factor, -1.0..=1.0, "shape_factor");
        c |= optional_slider(ui, &mut l.intercardinal_height, 0.5, 0.0..=1.0, "intercardinal_height");
        c |= slider(ui, &mut l.screen_margin, 0.0..=64.0, "screen_margin");
        c |= slider(ui, &mut l.dead_zone_radius, 0.0..=60.0, "dead_zone_radius");
        c |= ui.checkbox(&mut s.mirror_horizontal, "mirror_horizontal (RTL layout)").changed();
        c
    });

    changed |= group(ui, "Show behaviour", false, |ui| {
        let mut c = false;
        let mut instant = matches!(s.show_behavior, ShowBehavior::Instant);
        if ui.checkbox(&mut instant, "Instant").changed() {
            s.show_behavior = if instant { ShowBehavior::Instant } else { ShowBehavior::OnMovement { threshold: 10.0 } };
            c = true;
        }
        if let ShowBehavior::OnMovement { threshold } = &mut s.show_behavior {
            c |= slider(ui, threshold, 0.0..=50.0, "threshold");
        }
        c
    });

    changed |= group(ui, "Input", false, |ui| {
        let i = &mut s.input;
        let mut c = ui.checkbox(&mut i.use_number_keys, "use_number_keys").changed();
        c |= ui.checkbox(&mut i.use_mnemonic_keys, "use_mnemonic_keys").changed();
        c |= ui.checkbox(&mut i.auto_mnemonics, "auto_mnemonics").changed();
        c |= ui.checkbox(&mut i.select_on_primary_click, "select_on_primary_click").changed();
        c |= duration_slider(ui, &mut i.double_tap_window, 50..=1000, "double_tap_window (ms)");
        c |= dismiss_edit(ui, "dismiss_inputs", &mut i.dismiss_inputs);
        c |= dismiss_edit(ui, "pinned_dismiss_inputs", &mut i.pinned_dismiss_inputs);
        c
    });

    changed |= group(ui, "Sub-pies", false, |ui| {
        let p = &mut s.sub_pies;
        let mut c = enum_combo(ui, "sub_pies.style", &mut p.style, &[SubPieStyle::Replace, SubPieStyle::FanOut]);
        c |= enum_combo(ui, "sub_pies.center", &mut p.center, &[SubPieCenter::Pointer, SubPieCenter::Parent, SubPieCenter::Button]);
        c |= optional_duration(ui, &mut p.hover_delay, Duration::from_millis(400), 0..=2000, "hover_delay");
        c
    });

    changed |= group(ui, "Fan-out", false, |ui| {
        let f = &mut s.sub_pies.fan_out;
        let mut c = duration_slider(ui, &mut f.delay, 0..=2000, "delay (ms)");
        c |= optional_slider(ui, &mut f.radius, 100.0, 20.0..=300.0, "radius");
        c |= enum_combo(ui, "opener_action", &mut f.opener_action, &[FanOpenerAction::BreakOut, FanOpenerAction::Dismiss]);
        c
    });

    changed |= group(ui, "Centre indicator", false, |ui| {
        let ci = &mut s.center_indicator;
        let mut c = optional_slider(ui, &mut ci.background_radius, 15.0, 1.0..=60.0, "background_radius");
        c |= stroke_edit(ui, &mut ci.background_stroke, "background_stroke");
        c |= color_edit(ui, &mut ci.background_fill_color, "background_fill_color");
        c |= optional_slider(ui, &mut ci.highlight_radius, 15.0, 1.0..=60.0, "highlight_radius");
        c |= ui.checkbox(&mut ci.highlight_shape.arc, "highlight arc").changed();
        c |= ui.checkbox(&mut ci.highlight_shape.slice, "highlight slice").changed();
        c |= ui.checkbox(&mut ci.highlight_shape.circle, "highlight circle").changed();
        c |= stroke_edit(ui, &mut ci.highlight_stroke, "highlight_stroke");
        c |= color_edit(ui, &mut ci.highlight_fill_color, "highlight_fill_color");
        c |= slider(ui, &mut ci.highlight_angle, 0.1..=3.2, "highlight_angle");
        c |= slider(ui, &mut ci.highlight_circle_radius, 1.0..=30.0, "highlight_circle_radius");
        c |= ui.checkbox(&mut ci.highlight_snapping, "highlight_snapping").changed();
        c
    });

    changed |= group(ui, "Title", false, |ui| {
        let t = &mut s.title;
        let mut c = ui.checkbox(&mut t.show, "show").changed();
        c |= color_edit(ui, &mut t.background_color, "background_color");
        c |= stroke_edit(ui, &mut t.background_stroke, "background_stroke");
        c |= margin_edit(ui, &mut t.padding, "padding");
        c |= color_edit(ui, &mut t.text_color, "text_color");
        c |= font_edit(ui, &mut t.text_font, "text_font");
        c
    });

    changed |= group(ui, "Animations", false, |ui| {
        let a = &mut s.animations;
        let mut c = enum_combo(ui, "reveal", &mut a.reveal, &[RevealAnimation::None, RevealAnimation::Fade, RevealAnimation::SlideOut]);
        c |= duration_slider(ui, &mut a.reveal_duration, 0..=1000, "reveal_duration (ms)");
        c |= slider(ui, &mut a.slide_start_radius, 0.0..=300.0, "slide_start_radius");
        c |= optional_slider(ui, &mut a.slide_start_shape_factor, -1.0, -1.0..=1.0, "slide_start_shape_factor");
        c |= optional_duration(ui, &mut a.slide_fade, Duration::from_millis(200), 0..=1000, "slide_fade");
        c |= slider(ui, &mut a.hover_emphasis.shift, 0.0..=30.0, "hover_emphasis.shift");
        c |= slider(ui, &mut a.hover_emphasis.scale, 1.0..=1.5, "hover_emphasis.scale");
        c |= slider(ui, &mut a.sub_pie_hover_emphasis.shift, 0.0..=30.0, "sub_pie_hover_emphasis.shift");
        c |= slider(ui, &mut a.sub_pie_hover_emphasis.scale, 1.0..=1.5, "sub_pie_hover_emphasis.scale");
        c |= duration_slider(ui, &mut a.hover_duration, 0..=500, "hover_duration (ms)");
        c
    });

    changed |= group(ui, "Number hints", false, |ui| {
        let n = &mut s.number_hints;
        let mut c = optional_color(ui, &mut n.color, Color32::WHITE, "color");
        c |= slider(ui, &mut n.spacing, 0.0..=40.0, "spacing");
        c |= enum_combo(ui, "side", &mut n.side, &[HintSide::Trailing, HintSide::Outside, HintSide::Left, HintSide::Right]);
        c |= enum_combo(ui, "digits", &mut n.digits, &[HintDigits::Western, HintDigits::ArabicIndic, HintDigits::Persian]);
        c
    });

    changed |= group(ui, "Other", false, |ui| {
        let mut c = ui.checkbox(&mut s.block_background, "block_background").changed();
        c |= optional_color(ui, &mut s.text_color, Color32::WHITE, "text_color");
        c
    });

    changed
}

// ---------------------------------------------------------------------------------------
// Buttons editor
// ---------------------------------------------------------------------------------------

/// What kind of button the kind combo offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KindChoice {
    Action,
    Back,
    SubPie,
    Cluster,
}

impl KindChoice {
    const ALL: [KindChoice; 4] = [KindChoice::Action, KindChoice::Back, KindChoice::SubPie, KindChoice::Cluster];

    fn name(self) -> &'static str {
        match self {
            KindChoice::Action => "Action",
            KindChoice::Back => "Back",
            KindChoice::SubPie => "Sub-pie",
            KindChoice::Cluster => "Cluster",
        }
    }
}

fn kind_choice(b: &PieButton) -> KindChoice {
    match b.kind {
        PieButtonKind::Action => KindChoice::Action,
        PieButtonKind::Back => KindChoice::Back,
        PieButtonKind::SubPie { .. } => KindChoice::SubPie,
        PieButtonKind::Cluster(_) => KindChoice::Cluster,
    }
}

/// Converts `button` to another kind, keeping direction, label, mnemonic and enabled. A new
/// sub-pie starts with one child, a new cluster with one item numbered 0 (and, like
/// [`PieButton::cluster`], no mnemonic). Does nothing when the kind is already `to`.
fn convert_kind(button: &mut PieButton, to: KindChoice) {
    if kind_choice(button) == to {
        return;
    }
    button.kind = match to {
        KindChoice::Action => PieButtonKind::Action,
        KindChoice::Back => PieButtonKind::Back,
        KindChoice::SubPie => {
            PieButtonKind::SubPie { children: vec![PieButton::new(PieDirection::North).with_label("Item")], style: None }
        }
        KindChoice::Cluster => PieButtonKind::Cluster(vec![ClusterItem::new(0, "Item")]),
    };
    if to == KindChoice::Cluster {
        button.mnemonic = Mnemonic::None;
    }
}

/// The first of `candidates` that no button occupies.
fn first_free_direction_in(buttons: &[PieButton], candidates: &[PieDirection]) -> Option<PieDirection> {
    candidates.iter().copied().find(|d| buttons.iter().all(|b| b.direction != *d))
}

/// The first of the eight directions (clockwise from North) that no button occupies.
pub fn first_free_direction(buttons: &[PieButton]) -> Option<PieDirection> {
    first_free_direction_in(buttons, &PieDirection::ALL)
}

/// A ready-made sub-pie at `dir`: five children "Item 1".."Item 5" in the five fan-out slots
/// (valid for both styles) and, for a replace-style sub-pie, a back button opposite `dir`.
fn new_sub_pie(dir: PieDirection, fan: bool) -> PieButton {
    let mut children: Vec<PieButton> = dir
        .fan_out_slots()
        .iter()
        .enumerate()
        .map(|(n, slot)| PieButton::new(*slot).with_label(format!("Item {}", n + 1)))
        .collect();
    if fan {
        PieButton::new(dir).with_label("Fan-out").with_fan_out(children)
    } else {
        children.push(PieButton::back(dir.opposite()));
        PieButton::new(dir).with_label("Sub-pie").with_children(children)
    }
}

/// Mode combo (Auto / Key / None) plus a one-character field while it is a key.
fn mnemonic_edit(ui: &mut Ui, m: &mut Mnemonic, salt: egui::Id) -> bool {
    let mut changed = false;
    let name = match m {
        Mnemonic::Auto => "Auto",
        Mnemonic::Key(_) => "Key",
        Mnemonic::None => "None",
    };
    egui::ComboBox::from_id_salt(salt).selected_text(name).width(70.0).show_ui(ui, |ui| {
        if ui.selectable_label(matches!(m, Mnemonic::Auto), "Auto").clicked() {
            *m = Mnemonic::Auto;
            changed = true;
        }
        if ui.selectable_label(matches!(m, Mnemonic::Key(_)), "Key").clicked() && !matches!(m, Mnemonic::Key(_)) {
            *m = Mnemonic::Key('a');
            changed = true;
        }
        if ui.selectable_label(matches!(m, Mnemonic::None), "None").clicked() {
            *m = Mnemonic::None;
            changed = true;
        }
    });
    if let Mnemonic::Key(c) = m {
        let mut text = c.to_string();
        if ui.add(egui::TextEdit::singleline(&mut text).desired_width(24.0)).changed() {
            // Typing appends: the newest character wins; an emptied field keeps the old key.
            if let Some(n) = text.chars().last().filter(|n| *n != *c) {
                *c = n;
                changed = true;
            }
        }
    }
    changed
}

fn problem_menu(auto_mnemonics: bool) -> PieMenu {
    PieMenu::new().configure(|s| s.input.auto_mnemonics = auto_mnemonics)
}

/// Edits one cluster's items. Returns whether anything changed.
fn items_edit(ui: &mut Ui, items: &mut Vec<ClusterItem>, salt: egui::Id) -> bool {
    let mut changed = false;
    let mut remove = None;
    for (n, item) in items.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            changed |= ui.add(egui::DragValue::new(&mut item.number)).changed();
            changed |= ui.add(egui::TextEdit::singleline(&mut item.label).desired_width(90.0)).changed();
            changed |= mnemonic_edit(ui, &mut item.mnemonic, salt.with(("item_mnemonic", n)));
            changed |= ui.checkbox(&mut item.enabled, "enabled").changed();
            if ui.small_button("remove").clicked() {
                remove = Some(n);
            }
        });
    }
    if let Some(n) = remove {
        items.remove(n);
        changed = true;
    }
    if ui.button("Add item").clicked() {
        let next = items.iter().map(|i| i.number + 1).max().unwrap_or(0);
        items.push(ClusterItem::new(next, "Item"));
        changed = true;
    }
    changed
}

/// One button's controls. Returns `(changed, remove_requested)`.
///
/// `fan_parent` is the direction of the fan-out this button is a child of, if any: the
/// directions facing that parent are blocked.
fn button_body(
    ui: &mut Ui,
    b: &mut PieButton,
    auto: bool,
    level_problems: &[ButtonProblem],
    index: usize,
    fan_parent: Option<PieDirection>,
    salt: egui::Id,
) -> (bool, bool) {
    let mut changed = false;
    let mut remove = false;
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt(salt.with("dir")).selected_text(format!("{:?}", b.direction)).show_ui(ui, |ui| {
            for d in PieDirection::ALL {
                let blocked = fan_parent.is_some_and(|p| p.blocks_fan_out(d));
                if !blocked {
                    changed |= ui.selectable_value(&mut b.direction, d, format!("{d:?}")).changed();
                } else if b.direction == d {
                    let text = RichText::new(format!("{d:?} (faces the parent)")).color(Color32::LIGHT_RED);
                    let _ = ui.selectable_label(true, text);
                } else {
                    ui.add_enabled(false, egui::Button::selectable(false, format!("{d:?} (faces the parent)")));
                }
            }
        });
        changed |= ui.add(egui::TextEdit::singleline(&mut b.label).hint_text("label").desired_width(120.0)).changed();
        if ui.button("Remove").clicked() {
            remove = true;
        }
    });
    ui.horizontal(|ui| {
        if !b.is_cluster() {
            ui.label("mnemonic");
            changed |= mnemonic_edit(ui, &mut b.mnemonic, salt.with("mnemonic"));
        }
        changed |= ui.checkbox(&mut b.enabled, "enabled").changed();
        let current = kind_choice(b);
        egui::ComboBox::from_id_salt(salt.with("kind")).selected_text(current.name()).show_ui(ui, |ui| {
            for k in KindChoice::ALL {
                if ui.selectable_label(current == k, k.name()).clicked() && current != k {
                    convert_kind(b, k);
                    changed = true;
                }
            }
        });
    });
    let direction = b.direction;
    match &mut b.kind {
        PieButtonKind::SubPie { children, style } => {
            let shown = match style {
                None => "menu default",
                Some(SubPieStyle::Replace) => "Replace",
                Some(SubPieStyle::FanOut) => "Fan-out",
            };
            ui.horizontal(|ui| {
                ui.label("style");
                egui::ComboBox::from_id_salt(salt.with("style")).selected_text(shown).show_ui(ui, |ui| {
                    for (name, value) in
                        [("menu default", None), ("Replace", Some(SubPieStyle::Replace)), ("Fan-out", Some(SubPieStyle::FanOut))]
                    {
                        changed |= ui.selectable_value(style, value, name).changed();
                    }
                });
            });
            let is_fan = *style == Some(SubPieStyle::FanOut);
            let problems: Vec<ButtonProblem> = if is_fan {
                level_problems
                    .iter()
                    .filter_map(|p| match p {
                        ButtonProblem::InFanOut { parent, problem } if *parent == index => Some((**problem).clone()),
                        _ => None,
                    })
                    .collect()
            } else {
                problem_menu(auto).check_buttons(children)
            };
            let slots = direction.fan_out_slots();
            let candidates: &[PieDirection] = if is_fan { &slots } else { &PieDirection::ALL };
            ui.label("children");
            ui.indent(salt.with("children"), |ui| {
                let fan_parent = is_fan.then_some(direction);
                changed |= level_editor(ui, children, auto, &problems, candidates, fan_parent, salt.with("level"));
            });
        }
        PieButtonKind::Cluster(items) => {
            ui.label("items");
            ui.indent(salt.with("items"), |ui| {
                changed |= items_edit(ui, items, salt);
            });
        }
        PieButtonKind::Action | PieButtonKind::Back => {}
    }
    (changed, remove)
}

/// One level of buttons: a header per button, the add control and the problems text.
fn level_editor(
    ui: &mut Ui,
    buttons: &mut Vec<PieButton>,
    auto: bool,
    problems: &[ButtonProblem],
    candidates: &[PieDirection],
    fan_parent: Option<PieDirection>,
    salt: egui::Id,
) -> bool {
    let mut changed = false;
    let red: Vec<usize> = problems.iter().flat_map(|p| p.buttons()).collect();
    let mut remove = None;
    for (i, button) in buttons.iter_mut().enumerate() {
        let label = if button.label.is_empty() { "(no label)" } else { button.label.as_str() };
        let mut title = RichText::new(format!("{i} \u{b7} {label}"));
        if red.contains(&i) {
            title = title.color(Color32::LIGHT_RED);
        }
        let id = salt.with(i);
        let body = egui::CollapsingHeader::new(title).id_salt(id).default_open(false).show(ui, |ui| {
            button_body(ui, button, auto, problems, i, fan_parent, id)
        });
        if let Some((c, r)) = body.body_returned {
            changed |= c;
            if r {
                remove = Some(i);
            }
        }
    }
    if let Some(i) = remove {
        buttons.remove(i);
        changed = true;
    }
    ui.horizontal(|ui| match first_free_direction_in(buttons, candidates) {
        Some(d) => {
            if ui.button("Add button").clicked() {
                buttons.push(PieButton::new(d).with_label("New"));
                changed = true;
            }
            if fan_parent.is_none() {
                for (name, fan) in [("Add sub-pie", false), ("Add fan-out", true)] {
                    if ui.button(name).clicked() {
                        buttons.push(new_sub_pie(d, fan));
                        changed = true;
                    }
                }
            }
        }
        None => {
            ui.add_enabled(false, egui::Button::new("Add button"));
            if fan_parent.is_none() {
                ui.add_enabled(false, egui::Button::new("Add sub-pie"));
                ui.add_enabled(false, egui::Button::new("Add fan-out"));
            }
            ui.colored_label(Color32::ORANGE, "no free direction left");
        }
    });
    for p in problems.iter().filter(|p| !matches!(p, ButtonProblem::InFanOut { .. })) {
        ui.colored_label(Color32::LIGHT_RED, p.to_string());
    }
    changed
}

/// A tree editor over the real [`PieButton`] data: direction, label, mnemonic, enabled and kind
/// of every button, recursive sub-pie children and cluster items, add/remove controls. Buttons
/// involved in a [`PieMenu::check_buttons`] problem are drawn red and the problems are listed
/// under their level. `auto_mnemonics` should be the menu's `input.auto_mnemonics`. Returns
/// `true` if anything changed.
pub fn buttons_editor(ui: &mut Ui, buttons: &mut Vec<PieButton>, auto_mnemonics: bool) -> bool {
    let problems = problem_menu(auto_mnemonics).check_buttons(buttons);
    let salt = ui.id().with("pie_buttons_editor");
    level_editor(ui, buttons, auto_mnemonics, &problems, &PieDirection::ALL, None, salt)
}

// ---------------------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    #![allow(clippy::field_reassign_with_default)]
    use super::*;
    use crate::*;
    use egui::Context;

    /// Compiles the tokens as a `Vec<PieButton>` expression, checks it equals `$value` (Debug)
    /// and that `buttons_to_code($value)` is exactly the tokens (whitespace aside).
    macro_rules! assert_code_builds {
        ($value:expr, $($code:tt)+) => {{
            let built: Vec<PieButton> = $($code)+;
            assert_eq!(format!("{built:?}"), format!("{:?}", $value), "rebuilt buttons differ");
            let generated = buttons_to_code(&$value);
            let strip = |s: &str| s.chars().filter(|c| !c.is_whitespace()).collect::<String>();
            assert_eq!(strip(&generated), strip(stringify!($($code)+)), "generated text differs:\n{generated}");
        }};
    }

    /// Compiles `.configure(|s| { body })` on a new menu, checks the settings equal `$value`
    /// (Debug) and that `settings_to_code($value)` is exactly the tokens (whitespace aside).
    macro_rules! assert_configure_builds {
        ($value:expr, |$s:ident| { $($body:tt)* }) => {{
            let built = PieMenu::new().configure(|$s| { $($body)* }).settings;
            assert_eq!(format!("{built:?}"), format!("{:?}", $value), "rebuilt settings differ");
            let generated = settings_to_code(&$value);
            let strip = |s: &str| s.chars().filter(|c| !c.is_whitespace()).collect::<String>();
            let tokens = stringify!(.configure(|$s| { $($body)* }));
            assert_eq!(strip(&generated), strip(tokens), "generated text differs:\n{generated}");
        }};
    }

    fn lines(s: &PieMenuSettings) -> Vec<String> {
        settings_changes(s)
    }

    #[test]
    fn default_settings_give_no_code() {
        let d = PieMenuSettings::default();
        assert_eq!(settings_to_code(&d), "");
        assert!(settings_changes(&d).is_empty());
    }

    #[test]
    fn layout_group() {
        let mut s = PieMenuSettings::default();
        s.layout.radius = 120.0;
        s.layout.intercardinal_height = Some(0.25);
        assert_eq!(lines(&s), ["s.layout.radius = 120.0;", "s.layout.intercardinal_height = Some(0.25);"]);
        assert_eq!(settings_to_code(&s), ".configure(|s| {\n    s.layout.radius = 120.0;\n    s.layout.intercardinal_height = Some(0.25);\n})");
    }

    #[test]
    fn show_behavior_and_other_group() {
        let mut s = PieMenuSettings::default();
        s.show_behavior = ShowBehavior::Instant;
        s.block_background = false;
        s.mirror_horizontal = true;
        s.text_color = Some(Color32::WHITE);
        assert_eq!(
            lines(&s),
            [
                "s.show_behavior = ShowBehavior::Instant;",
                "s.block_background = false;",
                "s.mirror_horizontal = true;",
                "s.text_color = Some(egui::Color32::from_rgba_unmultiplied(255, 255, 255, 255));",
            ]
        );
        s.show_behavior = ShowBehavior::OnMovement { threshold: 25.0 };
        s.block_background = true;
        s.mirror_horizontal = false;
        s.text_color = None;
        assert_eq!(lines(&s), ["s.show_behavior = ShowBehavior::OnMovement { threshold: 25.0 };"]);
    }

    #[test]
    fn input_group() {
        let mut s = PieMenuSettings::default();
        s.input.auto_mnemonics = true;
        s.input.double_tap_window = Duration::from_millis(250);
        s.input.pinned_dismiss_inputs = vec![PieMenuDismissInput::Key(egui::Key::Escape), PieMenuDismissInput::PointerButton(egui::PointerButton::Secondary)];
        assert_eq!(
            lines(&s),
            [
                "s.input.auto_mnemonics = true;",
                "s.input.double_tap_window = Duration::from_millis(250);",
                "s.input.pinned_dismiss_inputs = vec![PieMenuDismissInput::Key(egui::Key::Escape), PieMenuDismissInput::PointerButton(egui::PointerButton::Secondary)];",
            ]
        );
    }

    #[test]
    fn sub_pies_group() {
        let mut s = PieMenuSettings::default();
        s.sub_pies.style = SubPieStyle::FanOut;
        s.sub_pies.hover_delay = Some(Duration::from_millis(300));
        s.sub_pies.fan_out.radius = Some(140.0);
        s.sub_pies.fan_out.opener_action = FanOpenerAction::Dismiss;
        assert_eq!(
            lines(&s),
            [
                "s.sub_pies.style = SubPieStyle::FanOut;",
                "s.sub_pies.hover_delay = Some(Duration::from_millis(300));",
                "s.sub_pies.fan_out.radius = Some(140.0);",
                "s.sub_pies.fan_out.opener_action = FanOpenerAction::Dismiss;",
            ]
        );
    }

    #[test]
    fn center_indicator_group() {
        let mut s = PieMenuSettings::default();
        s.center_indicator.background_radius = None;
        s.center_indicator.highlight_shape = HighlightShape { arc: true, slice: false, circle: true };
        s.center_indicator.highlight_stroke = Stroke::new(2.5, Color32::from_rgb(1, 2, 3));
        assert_eq!(
            lines(&s),
            [
                "s.center_indicator.background_radius = None;",
                "s.center_indicator.highlight_shape = HighlightShape { arc: true, slice: false, circle: true };",
                "s.center_indicator.highlight_stroke = egui::Stroke::new(2.5, egui::Color32::from_rgba_unmultiplied(1, 2, 3, 255));",
            ]
        );
    }

    #[test]
    fn title_group() {
        let mut s = PieMenuSettings::default();
        s.title.show = false;
        s.title.padding = Margin { left: 1, right: 2, top: 3, bottom: 4 };
        s.title.text_font = FontId::new(18.0, FontFamily::Monospace);
        assert_eq!(
            lines(&s),
            [
                "s.title.show = false;",
                "s.title.padding = egui::Margin { left: 1, right: 2, top: 3, bottom: 4 };",
                "s.title.text_font = egui::FontId::new(18.0, egui::FontFamily::Monospace);",
            ]
        );
        s.title = PieMenuTitleSettings::default();
        s.title.text_font = FontId::new(14.0, FontFamily::Name("Fancy".into()));
        assert_eq!(lines(&s), ["s.title.text_font = egui::FontId::new(14.0, egui::FontFamily::Name(\"Fancy\".into()));"]);
    }

    #[test]
    fn animations_group() {
        let mut s = PieMenuSettings::default();
        s.animations.reveal = RevealAnimation::SlideOut;
        s.animations.slide_fade = None;
        s.animations.sub_pie_hover_emphasis = HoverEmphasis::NONE;
        s.animations.hover_duration = Duration::from_millis(50);
        assert_eq!(
            lines(&s),
            [
                "s.animations.reveal = RevealAnimation::SlideOut;",
                "s.animations.slide_fade = None;",
                "s.animations.sub_pie_hover_emphasis = HoverEmphasis { shift: 0.0, scale: 1.0 };",
                "s.animations.hover_duration = Duration::from_millis(50);",
            ]
        );
    }

    #[test]
    fn number_hints_group() {
        let mut s = PieMenuSettings::default();
        s.number_hints.side = HintSide::Outside;
        s.number_hints.digits = HintDigits::Persian;
        s.number_hints.spacing = 8.0;
        assert_eq!(
            lines(&s),
            ["s.number_hints.spacing = 8.0;", "s.number_hints.side = HintSide::Outside;", "s.number_hints.digits = HintDigits::Persian;"]
        );
    }

    #[test]
    fn settings_compile_and_compare_every_group() {
        let mut s = PieMenuSettings::default();
        s.layout.radius = 120.0;
        s.layout.shape_factor = 0.5;
        s.layout.intercardinal_height = Some(0.75);
        s.layout.screen_margin = 12.0;
        s.layout.dead_zone_radius = 20.0;
        s.show_behavior = ShowBehavior::OnMovement { threshold: 25.0 };
        s.input.use_number_keys = false;
        s.input.use_mnemonic_keys = false;
        s.input.auto_mnemonics = true;
        s.input.double_tap_window = Duration::from_millis(250);
        s.input.select_on_primary_click = false;
        s.input.dismiss_inputs = vec![PieMenuDismissInput::Key(egui::Key::Escape), PieMenuDismissInput::PointerButton(egui::PointerButton::Middle)];
        s.input.pinned_dismiss_inputs = vec![PieMenuDismissInput::Key(egui::Key::Backspace)];
        s.center_indicator.background_radius = None;
        s.center_indicator.background_stroke = Stroke::new(2.0, Color32::from_rgb(10, 20, 30));
        s.center_indicator.background_fill_color = Color32::from_rgb(40, 50, 60);
        s.center_indicator.highlight_radius = Some(22.5);
        s.center_indicator.highlight_shape = HighlightShape { arc: true, slice: false, circle: true };
        s.center_indicator.highlight_stroke = Stroke::new(3.5, Color32::from_rgb(70, 80, 90));
        s.center_indicator.highlight_fill_color = Color32::from_rgb(100, 110, 120);
        s.center_indicator.highlight_angle = 0.5;
        s.center_indicator.highlight_circle_radius = 7.0;
        s.center_indicator.highlight_snapping = true;
        s.title.show = false;
        s.title.background_color = Color32::from_rgb(1, 2, 3);
        s.title.background_stroke = Stroke::new(1.0, Color32::from_rgb(4, 5, 6));
        s.title.padding = Margin { left: 1, right: 2, top: 3, bottom: 4 };
        s.title.text_color = Color32::from_rgb(7, 8, 9);
        s.title.text_font = FontId::new(18.0, FontFamily::Monospace);
        s.animations.reveal = RevealAnimation::SlideOut;
        s.animations.reveal_duration = Duration::from_millis(350);
        s.animations.slide_start_radius = 10.0;
        s.animations.slide_start_shape_factor = Some(-1.0);
        s.animations.slide_fade = None;
        s.animations.hover_emphasis = HoverEmphasis { shift: 2.0, scale: 1.25 };
        s.animations.sub_pie_hover_emphasis = HoverEmphasis::NONE;
        s.animations.hover_duration = Duration::from_millis(50);
        s.block_background = false;
        s.sub_pies.style = SubPieStyle::FanOut;
        s.sub_pies.center = SubPieCenter::Button;
        s.sub_pies.hover_delay = Some(Duration::from_millis(300));
        s.sub_pies.fan_out.delay = Duration::from_millis(150);
        s.sub_pies.fan_out.radius = Some(140.0);
        s.sub_pies.fan_out.opener_action = FanOpenerAction::Dismiss;
        s.mirror_horizontal = true;
        s.number_hints.color = Some(Color32::from_rgb(255, 255, 255));
        s.number_hints.spacing = 8.0;
        s.number_hints.side = HintSide::Outside;
        s.number_hints.digits = HintDigits::Persian;
        s.text_color = Some(Color32::from_rgb(200, 201, 202));

        assert_configure_builds!(s, |s| {
            s.layout.radius = 120.0;
            s.layout.shape_factor = 0.5;
            s.layout.intercardinal_height = Some(0.75);
            s.layout.screen_margin = 12.0;
            s.layout.dead_zone_radius = 20.0;
            s.show_behavior = ShowBehavior::OnMovement { threshold: 25.0 };
            s.input.use_number_keys = false;
            s.input.use_mnemonic_keys = false;
            s.input.auto_mnemonics = true;
            s.input.double_tap_window = Duration::from_millis(250);
            s.input.select_on_primary_click = false;
            s.input.dismiss_inputs = vec![PieMenuDismissInput::Key(egui::Key::Escape), PieMenuDismissInput::PointerButton(egui::PointerButton::Middle)];
            s.input.pinned_dismiss_inputs = vec![PieMenuDismissInput::Key(egui::Key::Backspace)];
            s.center_indicator.background_radius = None;
            s.center_indicator.background_stroke = egui::Stroke::new(2.0, egui::Color32::from_rgba_unmultiplied(10, 20, 30, 255));
            s.center_indicator.background_fill_color = egui::Color32::from_rgba_unmultiplied(40, 50, 60, 255);
            s.center_indicator.highlight_radius = Some(22.5);
            s.center_indicator.highlight_shape = HighlightShape { arc: true, slice: false, circle: true };
            s.center_indicator.highlight_stroke = egui::Stroke::new(3.5, egui::Color32::from_rgba_unmultiplied(70, 80, 90, 255));
            s.center_indicator.highlight_fill_color = egui::Color32::from_rgba_unmultiplied(100, 110, 120, 255);
            s.center_indicator.highlight_angle = 0.5;
            s.center_indicator.highlight_circle_radius = 7.0;
            s.center_indicator.highlight_snapping = true;
            s.title.show = false;
            s.title.background_color = egui::Color32::from_rgba_unmultiplied(1, 2, 3, 255);
            s.title.background_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(4, 5, 6, 255));
            s.title.padding = egui::Margin { left: 1, right: 2, top: 3, bottom: 4 };
            s.title.text_color = egui::Color32::from_rgba_unmultiplied(7, 8, 9, 255);
            s.title.text_font = egui::FontId::new(18.0, egui::FontFamily::Monospace);
            s.animations.reveal = RevealAnimation::SlideOut;
            s.animations.reveal_duration = Duration::from_millis(350);
            s.animations.slide_start_radius = 10.0;
            s.animations.slide_start_shape_factor = Some(-1.0);
            s.animations.slide_fade = None;
            s.animations.hover_emphasis = HoverEmphasis { shift: 2.0, scale: 1.25 };
            s.animations.sub_pie_hover_emphasis = HoverEmphasis { shift: 0.0, scale: 1.0 };
            s.animations.hover_duration = Duration::from_millis(50);
            s.block_background = false;
            s.sub_pies.style = SubPieStyle::FanOut;
            s.sub_pies.center = SubPieCenter::Button;
            s.sub_pies.hover_delay = Some(Duration::from_millis(300));
            s.sub_pies.fan_out.delay = Duration::from_millis(150);
            s.sub_pies.fan_out.radius = Some(140.0);
            s.sub_pies.fan_out.opener_action = FanOpenerAction::Dismiss;
            s.mirror_horizontal = true;
            s.number_hints.color = Some(egui::Color32::from_rgba_unmultiplied(255, 255, 255, 255));
            s.number_hints.spacing = 8.0;
            s.number_hints.side = HintSide::Outside;
            s.number_hints.digits = HintDigits::Persian;
            s.text_color = Some(egui::Color32::from_rgba_unmultiplied(200, 201, 202, 255));
        });
    }

    fn rich_tree() -> Vec<PieButton> {
        vec![
            PieButton::new(PieDirection::North).with_label("Copy").with_mnemonic('c').with_enabled(false),
            PieButton::new(PieDirection::East).with_label("Fan").with_fan_out(vec![
                PieButton::back(PieDirection::West),
                PieButton::new(PieDirection::NorthEast).with_label("A").without_mnemonic(),
                PieButton::new(PieDirection::East),
            ]),
            PieButton::new(PieDirection::South).with_label("More \"quoted\"").with_children(vec![
                PieButton::back(PieDirection::North).with_label("Up").with_mnemonic('u'),
                PieButton::new(PieDirection::South).with_label("Deep").with_children(vec![PieButton::new(PieDirection::West).with_label("Leaf")]),
            ]),
            PieButton::new(PieDirection::SouthEast).with_label("Rep").with_children(vec![PieButton::new(PieDirection::North).with_label("R1")]).with_sub_pie_style(Some(SubPieStyle::Replace)),
            PieButton::cluster(PieDirection::West).with_label("Format").with_items(vec![
                ClusterItem::new(7, "Bold").with_mnemonic('b'),
                ClusterItem::new(8, "Italic").without_mnemonic(),
                ClusterItem::new(9, "Strike").with_enabled(false),
                ClusterItem::new(10, "Under").with_mnemonic('u').with_enabled(false),
            ]),
            PieButton::back(PieDirection::NorthWest).without_mnemonic().with_enabled(false),
        ]
    }

    #[test]
    fn buttons_compile_and_compare() {
        let tree = rich_tree();
        assert_code_builds!(
            tree,
            vec![
                PieButton::new(PieDirection::North).with_label("Copy").with_mnemonic('c').with_enabled(false),
                PieButton::new(PieDirection::East).with_label("Fan").with_fan_out(vec![
                    PieButton::back(PieDirection::West),
                    PieButton::new(PieDirection::NorthEast).with_label("A").without_mnemonic(),
                    PieButton::new(PieDirection::East),
                ]),
                PieButton::new(PieDirection::South).with_label("More \"quoted\"").with_children(vec![
                    PieButton::back(PieDirection::North).with_label("Up").with_mnemonic('u'),
                    PieButton::new(PieDirection::South).with_label("Deep").with_children(vec![
                        PieButton::new(PieDirection::West).with_label("Leaf"),
                    ]),
                ]),
                PieButton::new(PieDirection::SouthEast).with_label("Rep").with_children(vec![
                    PieButton::new(PieDirection::North).with_label("R1"),
                ]).with_sub_pie_style(Some(SubPieStyle::Replace)),
                PieButton::cluster(PieDirection::West).with_label("Format").with_items(vec![
                    ClusterItem::new(7, "Bold").with_mnemonic('b'),
                    ClusterItem::new(8, "Italic").without_mnemonic(),
                    ClusterItem::new(9, "Strike").with_enabled(false),
                    ClusterItem::new(10, "Under").with_mnemonic('u').with_enabled(false),
                ]),
                PieButton::back(PieDirection::NorthWest).without_mnemonic().with_enabled(false),
            ]
        );
    }

    #[test]
    fn buttons_code_layout_and_empty() {
        assert_eq!(buttons_to_code(&[]), "vec![]");
        let b = vec![PieButton::new(PieDirection::East).with_label("Go").with_children(vec![PieButton::new(PieDirection::North)])];
        assert_eq!(
            buttons_to_code(&b),
            "vec![\n    PieButton::new(PieDirection::East).with_label(\"Go\").with_children(vec![\n        PieButton::new(PieDirection::North),\n    ]),\n]"
        );
    }

    #[test]
    fn menu_to_code_exact() {
        let buttons = vec![PieButton::new(PieDirection::North).with_label("Copy")];
        let mut s = PieMenuSettings::default();
        s.layout.radius = 120.0;
        let expected = "use egui_pie_menu::*;\n\
\n\
let buttons = vec![\n    PieButton::new(PieDirection::North).with_label(\"Copy\"),\n];\n\
\n\
let menu = PieMenu::new()\n    .with_title(\"Edit\")\n    .configure(|s| {\n        s.layout.radius = 120.0;\n    });\n\
\n\
// each frame (key_down: whether your trigger key is held):\n\
// match menu.show(ctx, &buttons, key_down, |ui, b| {\n\
//     ui.label(b.label_text());\n\
// }) {\n\
//     PieMenuResponse::Selected(idx) => { /* idx, b.path / menu.selected_path() */ }\n\
//     _ => {}\n\
// }\n";
        assert_eq!(menu_to_code(Some("Edit"), &s, &buttons), expected);
        // No settings change and no title: no configure, no Duration import.
        let plain = menu_to_code(None, &PieMenuSettings::default(), &buttons);
        assert!(plain.contains("let menu = PieMenu::new();"));
        assert!(!plain.contains("Duration"));
        // A duration setting adds the import.
        s.input.double_tap_window = Duration::from_millis(250);
        assert!(menu_to_code(None, &s, &buttons).contains("use std::time::Duration;\n"));
    }

    fn run_editor(f: impl FnOnce(&mut Ui) -> bool) -> bool {
        let ctx = Context::default();
        let mut changed = true;
        let mut f = Some(f);
        let mut out = ctx.run_ui(Default::default(), |ui| {
            if let Some(f) = f.take() {
                changed = f(ui);
            }
        });
        out.textures_delta.clear();
        changed
    }

    /// The editor draws without panicking and reports no change when nothing is touched.
    #[test]
    fn settings_editor_untouched() {
        let mut s = PieMenuSettings::default();
        assert!(!run_editor(|ui| settings_editor(ui, &mut s)));
        assert_eq!(settings_to_code(&s), "");
        let mut rich = PieMenuSettings::blender_like();
        rich.title.text_font = FontId::new(14.0, FontFamily::Name("Fancy".into()));
        rich.sub_pies.hover_delay = Some(Duration::from_millis(300));
        assert!(!run_editor(|ui| settings_editor(ui, &mut rich)));
    }

    #[test]
    fn buttons_editor_untouched() {
        let mut tree = rich_tree();
        let before = format!("{tree:?}");
        assert!(!run_editor(|ui| buttons_editor(ui, &mut tree, true)));
        assert_eq!(before, format!("{tree:?}"));
        let mut empty: Vec<PieButton> = Vec::new();
        assert!(!run_editor(|ui| buttons_editor(ui, &mut empty, false)));
        // Problems (two buttons in one direction) do not panic either.
        let mut clash = vec![PieButton::new(PieDirection::North), PieButton::new(PieDirection::North)];
        assert!(!run_editor(|ui| buttons_editor(ui, &mut clash, false)));
    }

    #[test]
    fn new_sub_pie_is_valid_in_both_modes() {
        let menu = PieMenu::new();
        for dir in PieDirection::ALL {
            for fan in [false, true] {
                let b = new_sub_pie(dir, fan);
                assert_eq!(b.direction, dir);
                assert_eq!(b.label, if fan { "Fan-out" } else { "Sub-pie" });
                let slots = dir.fan_out_slots();
                let kids = b.children();
                let dirs: Vec<_> = kids.iter().take(5).map(|c| c.direction).collect();
                assert_eq!(dirs, slots);
                for (n, c) in kids.iter().take(5).enumerate() {
                    assert_eq!(c.label, format!("Item {}", n + 1));
                }
                match &b.kind {
                    PieButtonKind::SubPie { style, .. } => {
                        assert_eq!(*style, if fan { Some(SubPieStyle::FanOut) } else { None })
                    }
                    _ => panic!("not a sub-pie"),
                }
                if fan {
                    assert_eq!(kids.len(), 5);
                } else {
                    assert_eq!(kids.len(), 6);
                    assert!(kids[5].is_back());
                    assert_eq!(kids[5].direction, dir.opposite());
                }
                assert_eq!(menu.check_buttons(&[b]), vec![], "{dir:?} fan={fan}");
            }
        }
    }

    #[test]
    fn new_sub_pie_code_builds() {
        let s = vec![new_sub_pie(PieDirection::East, false)];
        assert_code_builds!(
            s,
            vec![PieButton::new(PieDirection::East).with_label("Sub-pie").with_children(vec![
                PieButton::new(PieDirection::North).with_label("Item 1"),
                PieButton::new(PieDirection::NorthEast).with_label("Item 2"),
                PieButton::new(PieDirection::East).with_label("Item 3"),
                PieButton::new(PieDirection::SouthEast).with_label("Item 4"),
                PieButton::new(PieDirection::South).with_label("Item 5"),
                PieButton::back(PieDirection::West),
            ]),]
        );
        let f = vec![new_sub_pie(PieDirection::East, true)];
        assert_code_builds!(
            f,
            vec![PieButton::new(PieDirection::East).with_label("Fan-out").with_fan_out(vec![
                PieButton::new(PieDirection::North).with_label("Item 1"),
                PieButton::new(PieDirection::NorthEast).with_label("Item 2"),
                PieButton::new(PieDirection::East).with_label("Item 3"),
                PieButton::new(PieDirection::SouthEast).with_label("Item 4"),
                PieButton::new(PieDirection::South).with_label("Item 5"),
            ]),]
        );
    }

    #[test]
    fn buttons_editor_runs_with_new_sub_pies() {
        let mut tree = vec![new_sub_pie(PieDirection::North, false), new_sub_pie(PieDirection::East, true)];
        // A bad child (blocked direction) must not panic either.
        tree.push(PieButton::new(PieDirection::South).with_fan_out(vec![PieButton::new(PieDirection::North)]));
        assert!(!run_editor(|ui| buttons_editor(ui, &mut tree, true)));
    }

    #[test]
    fn code_view_runs() {
        assert!(run_editor(|ui| {
            code_view(ui, "let x = 1;");
            true
        }));
    }

    #[test]
    fn structure_helpers() {
        // first_free_direction
        let mut buttons: Vec<PieButton> = Vec::new();
        assert_eq!(first_free_direction(&buttons), Some(PieDirection::North));
        buttons.push(PieButton::new(PieDirection::North));
        buttons.push(PieButton::new(PieDirection::NorthEast));
        assert_eq!(first_free_direction(&buttons), Some(PieDirection::East));
        assert_eq!(first_free_direction_in(&buttons, &PieDirection::East.fan_out_slots()), Some(PieDirection::East));
        for d in PieDirection::ALL {
            if first_free_direction(&buttons).is_some() {
                buttons.push(PieButton::new(d));
            }
        }
        assert_eq!(first_free_direction(&buttons), None);

        // convert_kind
        let mut b = PieButton::new(PieDirection::West).with_label("Thing").with_mnemonic('t').with_enabled(false);
        convert_kind(&mut b, KindChoice::SubPie);
        assert_eq!(kind_choice(&b), KindChoice::SubPie);
        assert_eq!(b.children().len(), 1);
        assert_eq!((b.direction, b.label.as_str(), b.mnemonic, b.enabled), (PieDirection::West, "Thing", Mnemonic::Key('t'), false));
        convert_kind(&mut b, KindChoice::SubPie);
        assert_eq!(b.children().len(), 1, "same kind: untouched");
        convert_kind(&mut b, KindChoice::Cluster);
        assert_eq!(kind_choice(&b), KindChoice::Cluster);
        assert_eq!(b.cluster_items().len(), 1);
        assert_eq!(b.cluster_items()[0].number, 0);
        assert_eq!(b.mnemonic, Mnemonic::None);
        assert_eq!((b.label.as_str(), b.enabled), ("Thing", false));
        convert_kind(&mut b, KindChoice::Back);
        assert!(b.is_back());
        assert_eq!(b.label, "Thing");
        convert_kind(&mut b, KindChoice::Action);
        assert!(matches!(b.kind, PieButtonKind::Action));
    }
}
