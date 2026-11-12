//! Interactive settings editor + code generator ("dev tool") for [`PieMenuSettings`].
use crate::settings::*;
use egui::{Color32, Context, Ui};
use std::ops::RangeInclusive;
use std::time::Duration;

/// A checkbox that switches an optional value on (with `default`) or off, and a slider for it
/// while it is on. Returns `true` if anything changed.
fn optional_slider(ui: &mut Ui, value: &mut Option<f32>, default: f32, range: RangeInclusive<f32>, label: &str) -> bool {
    let mut changed = false;
    let mut on = value.is_some();
    if ui.checkbox(&mut on, label).changed() {
        *value = on.then_some(default);
        changed = true;
    }
    if let Some(v) = value {
        changed |= ui.add(egui::Slider::new(v, range)).changed();
    }
    changed
}

/// A checkbox that switches an optional colour on (with `default`) or off, and a colour button
/// while it is on. Returns `true` if anything changed.
fn optional_color(ui: &mut Ui, value: &mut Option<Color32>, default: Color32, label: &str) -> bool {
    let mut changed = false;
    let mut on = value.is_some();
    if ui.checkbox(&mut on, label).changed() {
        *value = on.then_some(default);
        changed = true;
    }
    if let Some(c) = value {
        changed |= ui.color_edit_button_srgba(c).changed();
    }
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

/// Draws the settings editor controls into `ui`. Returns `true` if anything changed.
pub fn settings_editor(ui: &mut Ui, s: &mut PieMenuSettings) -> bool {
    let mut changed = false;
    macro_rules! c { ($r:expr) => { changed |= $r.changed() }; }

    ui.heading("Layout");
    c!(ui.add(egui::Slider::new(&mut s.layout.radius, 20.0..=300.0).text("layout.radius")));
    c!(ui.add(egui::Slider::new(&mut s.layout.shape_factor, -1.0..=1.0).text("layout.shape_factor")));
    changed |= optional_slider(ui, &mut s.layout.intercardinal_height, 0.5, 0.0..=1.0, "layout.intercardinal_height");
    c!(ui.add(egui::Slider::new(&mut s.layout.screen_margin, 0.0..=64.0).text("layout.screen_margin")));
    c!(ui.add(egui::Slider::new(&mut s.layout.dead_zone_radius, 0.0..=60.0).text("layout.dead_zone_radius")));

    c!(ui.checkbox(&mut s.mirror_horizontal, "mirror_horizontal (RTL layout)"));

    c!(ui.checkbox(&mut s.block_background, "block_background"));

    ui.heading("Show behaviour");
    let mut instant = matches!(s.show_behavior, ShowBehavior::Instant);
    if ui.checkbox(&mut instant, "Instant").changed() {
        s.show_behavior = if instant { ShowBehavior::Instant } else { ShowBehavior::OnMovement { threshold: 10.0 } };
        changed = true;
    }
    if let ShowBehavior::OnMovement { threshold } = &mut s.show_behavior {
        c!(ui.add(egui::Slider::new(threshold, 0.0..=50.0).text("threshold")));
    }

    ui.heading("Input");
    c!(ui.checkbox(&mut s.input.use_number_keys, "use_number_keys"));
    c!(ui.checkbox(&mut s.input.use_mnemonic_keys, "use_mnemonic_keys"));
    c!(ui.checkbox(&mut s.input.auto_mnemonics, "auto_mnemonics"));
    c!(ui.checkbox(&mut s.input.select_on_primary_click, "select_on_primary_click"));
    changed |= duration_slider(ui, &mut s.input.double_tap_window, 50..=1000, "double_tap_window (ms)");

    ui.heading("Sub-pies");
    egui::ComboBox::from_label("sub_pies.style")
        .selected_text(format!("{:?}", s.sub_pies.style))
        .show_ui(ui, |ui| {
            for v in [SubPieStyle::Replace, SubPieStyle::FanOut] {
                changed |= ui.selectable_value(&mut s.sub_pies.style, v, format!("{v:?}")).changed();
            }
        });
    egui::ComboBox::from_label("sub_pies.center")
        .selected_text(format!("{:?}", s.sub_pies.center))
        .show_ui(ui, |ui| {
            for v in [SubPieCenter::Pointer, SubPieCenter::Parent, SubPieCenter::Button] {
                changed |= ui.selectable_value(&mut s.sub_pies.center, v, format!("{v:?}")).changed();
            }
        });
    let mut hover = s.sub_pies.hover_delay.is_some();
    if ui.checkbox(&mut hover, "sub_pies.hover_delay").changed() {
        s.sub_pies.hover_delay = hover.then_some(Duration::from_millis(400));
        changed = true;
    }
    if let Some(d) = &mut s.sub_pies.hover_delay {
        changed |= duration_slider(ui, d, 0..=2000, "hover_delay (ms)");
    }
    changed |= duration_slider(ui, &mut s.sub_pies.fan_out.delay, 0..=2000, "fan_out.delay (ms)");
    changed |= optional_slider(ui, &mut s.sub_pies.fan_out.radius, 100.0, 20.0..=300.0, "fan_out.radius");
    egui::ComboBox::from_label("fan_out.opener_action")
        .selected_text(format!("{:?}", s.sub_pies.fan_out.opener_action))
        .show_ui(ui, |ui| {
            for v in [FanOpenerAction::BreakOut, FanOpenerAction::Dismiss] {
                changed |= ui.selectable_value(&mut s.sub_pies.fan_out.opener_action, v, format!("{v:?}")).changed();
            }
        });

    ui.heading("Number hints");
    c!(ui.add(egui::Slider::new(&mut s.number_hints.spacing, 0.0..=40.0).text("spacing")));
    changed |= optional_color(ui, &mut s.number_hints.color, Color32::WHITE, "color");

    ui.heading("Animations");
    egui::ComboBox::from_label("reveal")
        .selected_text(format!("{:?}", s.animations.reveal))
        .show_ui(ui, |ui| {
            for r in [RevealAnimation::None, RevealAnimation::Fade, RevealAnimation::SlideOut] {
                changed |= ui.selectable_value(&mut s.animations.reveal, r, format!("{r:?}")).changed();
            }
        });
    changed |= duration_slider(ui, &mut s.animations.reveal_duration, 0..=1000, "reveal_duration (ms)");
    changed |= duration_slider(ui, &mut s.animations.hover_duration, 0..=500, "hover_duration (ms)");
    c!(ui.add(egui::Slider::new(&mut s.animations.hover_emphasis.shift, 0.0..=30.0).text("hover_emphasis.shift")));
    c!(ui.add(egui::Slider::new(&mut s.animations.hover_emphasis.scale, 1.0..=1.5).text("hover_emphasis.scale")));
    c!(ui.add(egui::Slider::new(&mut s.animations.sub_pie_hover_emphasis.shift, 0.0..=30.0).text("sub_pie_hover_emphasis.shift")));
    c!(ui.add(egui::Slider::new(&mut s.animations.sub_pie_hover_emphasis.scale, 1.0..=1.5).text("sub_pie_hover_emphasis.scale")));

    ui.heading("Center indicator");
    changed |= optional_slider(ui, &mut s.center_indicator.background_radius, 15.0, 1.0..=60.0, "background_radius");
    c!(ui.color_edit_button_srgba(&mut s.center_indicator.background_fill_color));
    c!(ui.add(egui::Slider::new(&mut s.center_indicator.background_stroke.width, 0.0..=12.0).text("background stroke")));
    c!(ui.checkbox(&mut s.center_indicator.highlight_shape.arc, "highlight arc"));
    c!(ui.checkbox(&mut s.center_indicator.highlight_shape.slice, "highlight slice"));
    c!(ui.checkbox(&mut s.center_indicator.highlight_shape.circle, "highlight circle"));
    c!(ui.checkbox(&mut s.center_indicator.highlight_snapping, "highlight_snapping"));
    c!(ui.color_edit_button_srgba(&mut s.center_indicator.highlight_fill_color));
    c!(ui.add(egui::Slider::new(&mut s.center_indicator.highlight_angle, 0.1..=3.0).text("highlight_angle")));

    ui.heading("Title");
    c!(ui.checkbox(&mut s.title.show, "show"));
    c!(ui.color_edit_button_srgba(&mut s.title.text_color));
    changed
}

fn color(c: Color32) -> String {
    format!("Color32::from_rgba_unmultiplied({}, {}, {}, {})", c.r(), c.g(), c.b(), c.a())
}

fn opt_f32(v: Option<f32>) -> String {
    match v {
        Some(x) => format!("Some({x:?})"),
        None => "None".to_string(),
    }
}

fn millis(d: Duration) -> String {
    format!("Duration::from_millis({})", d.as_millis())
}

/// Generates Rust source that reproduces the editable parts of `s` as assignments
/// on a `menu` variable (`menu.settings.… = …;`).
pub fn settings_to_code(s: &PieMenuSettings) -> String {
    let show = match s.show_behavior {
        ShowBehavior::Instant => "ShowBehavior::Instant".to_string(),
        ShowBehavior::OnMovement { threshold } => format!("ShowBehavior::OnMovement {{ threshold: {threshold:?} }}"),
    };
    let hover_color = match s.number_hints.color {
        Some(c) => format!("Some({})", color(c)),
        None => "None".to_string(),
    };
    let hover_delay = match s.sub_pies.hover_delay {
        Some(d) => format!("Some({})", millis(d)),
        None => "None".to_string(),
    };
    let shape = s.center_indicator.highlight_shape;
    let emphasis = |e: HoverEmphasis| format!("HoverEmphasis {{ shift: {:?}, scale: {:?} }}", e.shift, e.scale);
    let lines = [
        format!("menu.settings.layout.radius = {:?};", s.layout.radius),
        format!("menu.settings.layout.shape_factor = {:?};", s.layout.shape_factor),
        format!("menu.settings.layout.intercardinal_height = {};", opt_f32(s.layout.intercardinal_height)),
        format!("menu.settings.layout.screen_margin = {:?};", s.layout.screen_margin),
        format!("menu.settings.layout.dead_zone_radius = {:?};", s.layout.dead_zone_radius),
        format!("menu.settings.mirror_horizontal = {};", s.mirror_horizontal),
        format!("menu.settings.block_background = {};", s.block_background),
        format!("menu.settings.show_behavior = {show};"),
        format!("menu.settings.input.use_number_keys = {};", s.input.use_number_keys),
        format!("menu.settings.input.use_mnemonic_keys = {};", s.input.use_mnemonic_keys),
        format!("menu.settings.input.auto_mnemonics = {};", s.input.auto_mnemonics),
        format!("menu.settings.input.select_on_primary_click = {};", s.input.select_on_primary_click),
        format!("menu.settings.input.double_tap_window = {};", millis(s.input.double_tap_window)),
        format!("menu.settings.sub_pies.style = SubPieStyle::{:?};", s.sub_pies.style),
        format!("menu.settings.sub_pies.center = SubPieCenter::{:?};", s.sub_pies.center),
        format!("menu.settings.sub_pies.hover_delay = {hover_delay};"),
        format!("menu.settings.sub_pies.fan_out.delay = {};", millis(s.sub_pies.fan_out.delay)),
        format!("menu.settings.sub_pies.fan_out.radius = {};", opt_f32(s.sub_pies.fan_out.radius)),
        format!("menu.settings.sub_pies.fan_out.opener_action = FanOpenerAction::{:?};", s.sub_pies.fan_out.opener_action),
        format!("menu.settings.number_hints.spacing = {:?};", s.number_hints.spacing),
        format!("menu.settings.number_hints.color = {hover_color};"),
        format!("menu.settings.animations.reveal = RevealAnimation::{:?};", s.animations.reveal),
        format!("menu.settings.animations.reveal_duration = {};", millis(s.animations.reveal_duration)),
        format!("menu.settings.animations.hover_duration = {};", millis(s.animations.hover_duration)),
        format!("menu.settings.animations.hover_emphasis = {};", emphasis(s.animations.hover_emphasis)),
        format!("menu.settings.animations.sub_pie_hover_emphasis = {};", emphasis(s.animations.sub_pie_hover_emphasis)),
        format!("menu.settings.center_indicator.background_radius = {};", opt_f32(s.center_indicator.background_radius)),
        format!("menu.settings.center_indicator.background_fill_color = {};", color(s.center_indicator.background_fill_color)),
        format!("menu.settings.center_indicator.background_stroke.width = {:?};", s.center_indicator.background_stroke.width),
        format!(
            "menu.settings.center_indicator.highlight_shape = HighlightShape {{ arc: {}, slice: {}, circle: {} }};",
            shape.arc, shape.slice, shape.circle
        ),
        format!("menu.settings.center_indicator.highlight_snapping = {};", s.center_indicator.highlight_snapping),
        format!("menu.settings.center_indicator.highlight_fill_color = {};", color(s.center_indicator.highlight_fill_color)),
        format!("menu.settings.center_indicator.highlight_angle = {:?};", s.center_indicator.highlight_angle),
        format!("menu.settings.title.show = {};", s.title.show),
        format!("menu.settings.title.text_color = {};", color(s.title.text_color)),
    ];
    lines.join("\n")
}

/// Opens a floating "Pie menu dev tool" window: edit `settings` live, then copy the
/// generated code into your project. `open` controls the window's visibility.
pub fn show_dev_tool(ctx: &Context, open: &mut bool, settings: &mut PieMenuSettings) {
    egui::Window::new("Pie menu dev tool").open(open).show(ctx, |ui| {
        egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
            settings_editor(ui, settings);
        });
        ui.separator();
        let mut code = settings_to_code(settings);
        ui.horizontal(|ui| {
            ui.label("Generated code");
            if ui.button("Copy").clicked() {
                ui.ctx().copy_text(code.clone());
            }
        });
        egui::ScrollArea::vertical().id_salt("code").max_height(160.0).show(ui, |ui| {
            ui.add(egui::TextEdit::multiline(&mut code).code_editor().desired_width(f32::INFINITY));
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_contains_assignments() {
        let code = settings_to_code(&PieMenuSettings::default());
        assert!(code.contains("menu.settings.layout.radius = 100.0;"));
        assert!(code.contains("menu.settings.layout.intercardinal_height = None;"));
        assert!(code.contains("ShowBehavior::OnMovement { threshold: 10.0 }"));
        assert!(code.contains("RevealAnimation::Fade"));
        assert!(code.contains("menu.settings.sub_pies.style = SubPieStyle::Replace;"));
        assert!(code.contains("menu.settings.sub_pies.fan_out.opener_action = FanOpenerAction::BreakOut;"));
        assert!(code.contains("HighlightShape { arc: false, slice: true, circle: false }"));
        assert!(code.contains("menu.settings.center_indicator.background_radius = Some(15.0);"));
    }

    /// Every changed setting shows up in the generated code, so the code reproduces the settings.
    #[test]
    fn code_follows_changes() {
        let mut s = PieMenuSettings::default();
        s.layout.radius = 123.5;
        s.layout.intercardinal_height = Some(0.25);
        s.sub_pies.style = SubPieStyle::FanOut;
        s.sub_pies.hover_delay = Some(Duration::from_millis(250));
        s.sub_pies.fan_out.radius = Some(140.0);
        s.center_indicator.background_radius = None;
        s.center_indicator.highlight_shape = HighlightShape { arc: true, slice: false, circle: true };
        s.animations.sub_pie_hover_emphasis = HoverEmphasis::NONE;
        let code = settings_to_code(&s);
        assert!(code.contains("menu.settings.layout.radius = 123.5;"));
        assert!(code.contains("menu.settings.layout.intercardinal_height = Some(0.25);"));
        assert!(code.contains("menu.settings.sub_pies.style = SubPieStyle::FanOut;"));
        assert!(code.contains("menu.settings.sub_pies.hover_delay = Some(Duration::from_millis(250));"));
        assert!(code.contains("menu.settings.sub_pies.fan_out.radius = Some(140.0);"));
        assert!(code.contains("menu.settings.center_indicator.background_radius = None;"));
        assert!(code.contains("HighlightShape { arc: true, slice: false, circle: true }"));
        assert!(code.contains("sub_pie_hover_emphasis = HoverEmphasis { shift: 0.0, scale: 1.0 };"));
    }

    /// The editor draws without panicking and reports no change when nothing is touched.
    #[test]
    fn editor_runs_and_reports_no_change_untouched() {
        let ctx = Context::default();
        let mut s = PieMenuSettings::default();
        let mut changed = true;
        let mut out = ctx.run_ui(Default::default(), |ui| {
            changed = settings_editor(ui, &mut s);
        });
        out.textures_delta.clear();
        assert!(!changed);
    }
}
