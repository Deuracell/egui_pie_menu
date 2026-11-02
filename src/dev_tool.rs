//! Interactive settings editor + code generator ("dev tool") for [`PieMenuSettings`].
use crate::settings::*;
use egui::{Color32, Context, Ui};
use std::time::Duration;

/// Draws the settings editor controls into `ui`. Returns `true` if anything changed.
pub fn settings_editor(ui: &mut Ui, s: &mut PieMenuSettings) -> bool {
    let mut changed = false;
    macro_rules! c { ($r:expr) => { changed |= $r.changed() }; }

    ui.heading("Layout");
    c!(ui.add(egui::Slider::new(&mut s.layout_radius, 20.0..=300.0).text("layout_radius")));
    c!(ui.add(egui::Slider::new(&mut s.shape_factor, -1.0..=1.0).text("shape_factor")));
    c!(ui.add(egui::Slider::new(&mut s.screen_margin, 0.0..=64.0).text("screen_margin")));
    c!(ui.add(egui::Slider::new(&mut s.mouse_trigger_threshold, 0.0..=60.0).text("mouse_trigger_threshold")));

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
    c!(ui.checkbox(&mut s.input.use_numpad_keys, "use_numpad_keys"));
    c!(ui.checkbox(&mut s.input.use_mnemonic_keys, "use_mnemonic_keys"));
    c!(ui.checkbox(&mut s.input.select_on_primary_click, "select_on_primary_click"));
    let mut ms = s.input.double_tap_window.as_millis() as u64;
    if ui.add(egui::Slider::new(&mut ms, 50..=1000).text("double_tap_window (ms)")).changed() {
        s.input.double_tap_window = Duration::from_millis(ms);
        changed = true;
    }

    ui.heading("Number hints");
    c!(ui.checkbox(&mut s.number_hints.show, "show"));
    c!(ui.add(egui::Slider::new(&mut s.number_hints.spacing, 0.0..=40.0).text("spacing")));
    c!(ui.color_edit_button_srgba(&mut s.number_hints.color));

    ui.heading("Animations");
    egui::ComboBox::from_label("reveal")
        .selected_text(format!("{:?}", s.animations.reveal))
        .show_ui(ui, |ui| {
            for r in [RevealAnimation::None, RevealAnimation::Fade, RevealAnimation::SlideOut] {
                changed |= ui.selectable_value(&mut s.animations.reveal, r, format!("{r:?}")).changed();
            }
        });
    let mut ms = s.animations.reveal_duration.as_millis() as u64;
    if ui.add(egui::Slider::new(&mut ms, 0..=1000).text("reveal_duration (ms)")).changed() {
        s.animations.reveal_duration = Duration::from_millis(ms);
        changed = true;
    }
    c!(ui.checkbox(&mut s.animations.center_highlight_show, "center_highlight_show"));
    c!(ui.checkbox(&mut s.animations.center_highlight_snapping, "center_highlight_snapping"));

    ui.heading("Center indicator");
    let mut bg = s.center_indicator.background_radius.get();
    let mut bg_on = bg == bg;
    if ui.checkbox(&mut bg_on, "background circle").changed() {
        bg = if bg_on { 15.0 } else { f32::NAN };
        changed = true;
    }
    if bg_on {
        c!(ui.add(egui::Slider::new(&mut bg, 1.0..=60.0).text("background_radius")));
    }
    s.center_indicator.background_radius = crate::SmartFloat::new(bg);
    c!(ui.color_edit_button_srgba(&mut s.center_indicator.background_fill_color));
    c!(ui.add(egui::Slider::new(&mut s.center_indicator.background_stroke.width, 0.0..=12.0).text("background stroke")));
    c!(ui.color_edit_button_srgba(&mut s.center_indicator.highlight_fill_color));
    c!(ui.add(egui::Slider::new(&mut s.center_indicator.highlight_angle, 0.1..=3.0).text("highlight_angle")));

    ui.heading("Title label");
    c!(ui.checkbox(&mut s.label.display, "display"));
    c!(ui.color_edit_button_srgba(&mut s.label.text_color));
    changed
}

fn color(c: Color32) -> String {
    format!("Color32::from_rgba_unmultiplied({}, {}, {}, {})", c.r(), c.g(), c.b(), c.a())
}

/// Generates Rust source that reproduces the editable parts of `s` as assignments
/// on a `menu` variable (`menu.settings.… = …;`).
pub fn settings_to_code(s: &PieMenuSettings) -> String {
    let bg = s.center_indicator.background_radius.get();
    let show = match s.show_behavior {
        ShowBehavior::Instant => "ShowBehavior::Instant".to_string(),
        ShowBehavior::OnMovement { threshold } => format!("ShowBehavior::OnMovement {{ threshold: {threshold:?} }}"),
    };
    let lines = [
        format!("menu.settings.layout_radius = {:?};", s.layout_radius),
        format!("menu.settings.mirror_horizontal = {};", s.mirror_horizontal),
        format!("menu.settings.block_background = {};", s.block_background),
        format!("menu.settings.shape_factor = {:?};", s.shape_factor),
        format!("menu.settings.screen_margin = {:?};", s.screen_margin),
        format!("menu.settings.mouse_trigger_threshold = {:?};", s.mouse_trigger_threshold),
        format!("menu.settings.show_behavior = {show};"),
        format!("menu.settings.input.use_numpad_keys = {};", s.input.use_numpad_keys),
        format!("menu.settings.input.use_mnemonic_keys = {};", s.input.use_mnemonic_keys),
        format!("menu.settings.input.select_on_primary_click = {};", s.input.select_on_primary_click),
        format!(
            "menu.settings.input.double_tap_window = Duration::from_millis({});",
            s.input.double_tap_window.as_millis()
        ),
        format!("menu.settings.number_hints.show = {};", s.number_hints.show),
        format!("menu.settings.number_hints.spacing = {:?};", s.number_hints.spacing),
        format!("menu.settings.number_hints.color = {};", color(s.number_hints.color)),
        format!("menu.settings.animations.reveal = RevealAnimation::{:?};", s.animations.reveal),
        format!(
            "menu.settings.animations.reveal_duration = Duration::from_millis({});",
            s.animations.reveal_duration.as_millis()
        ),
        format!("menu.settings.animations.center_highlight_show = {};", s.animations.center_highlight_show),
        format!("menu.settings.animations.center_highlight_snapping = {};", s.animations.center_highlight_snapping),
        format!(
            "menu.settings.center_indicator.background_radius = SmartFloat::new({});",
            if bg == bg { format!("{bg:?}") } else { "f32::NAN".to_string() }
        ),
        format!("menu.settings.center_indicator.background_fill_color = {};", color(s.center_indicator.background_fill_color)),
        format!("menu.settings.center_indicator.background_stroke.width = {:?};", s.center_indicator.background_stroke.width),
        format!("menu.settings.center_indicator.highlight_fill_color = {};", color(s.center_indicator.highlight_fill_color)),
        format!("menu.settings.center_indicator.highlight_angle = {:?};", s.center_indicator.highlight_angle),
        format!("menu.settings.label.display = {};", s.label.display),
        format!("menu.settings.label.text_color = {};", color(s.label.text_color)),
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
        assert!(code.contains("menu.settings.layout_radius = 100.0;"));
        assert!(code.contains("ShowBehavior::OnMovement { threshold: 10.0 }"));
        assert!(code.contains("RevealAnimation::None"));
    }
}
