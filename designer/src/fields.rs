//! One editor widget per settings field ([`field_editor`]), the fields grouped into
//! sections ([`SettingsGroup`], [`group_editor`]) and the all-in-one [`settings_editor`].
//! The fields themselves ([`SettingsField`], [`SettingsGroup`], [`field_label`]) are the
//! core crate's (`egui_pie_menu::settings_fields`).
use crate::bindings::{BindTarget, Bindings};
#[allow(unused_imports)] // doc links
use crate::bindings::binding_details;
pub use egui_pie_menu::settings_fields::*;
use egui_pie_menu::settings::*;
use egui::{Color32, FontFamily, FontId, Margin, Stroke, Ui};
use egui_control_groups::{FieldGroup, GroupStyle, RadioGroup, ToggleGroup};
use std::ops::RangeInclusive;
use std::time::Duration;

// ---------------------------------------------------------------------------------------
// Widgets
// ---------------------------------------------------------------------------------------

// The layout every row follows, so that everything lines up: the name in a column on the
// left, the control filling the rest, the reset button at the right edge of the control and
// a slot after it for the bind toggle (or the demo tag). A row with more to show than fits
// the control (a list, a second value) continues on rows of the same layout below it.

/// Width of the slot after the control, for the bind toggle or the "demo" tag.
const TRAILING_W: f32 = 30.0;
/// Width kept for the reset button at the right edge of a control.
const RESET_W: f32 = 22.0;
/// The horizontal gap between the parts of a row (name, control, reset, trailing slot),
/// about as tight as the gap between two rows. Rows set it themselves; a caller that adds
/// to a row (the bind toggle, the "demo" tag) does so through [`row_line`].
pub const ROW_GAP: f32 = 4.0;

/// A horizontal line for one settings row and what follows it (bind toggle, tag), with the
/// row's gap between its parts.
pub fn row_line<R>(ui: &mut Ui, body: impl FnOnce(&mut Ui) -> R) -> R {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = ROW_GAP;
        body(ui)
    })
    .inner
}

/// The width a control may fill on the current row: what is left of the control column
/// after the reset button. Call it before adding the first widget of the control.
pub fn control_width(ui: &Ui) -> f32 {
    (ui.available_width() - RESET_W - ui.spacing().item_spacing.x).max(40.0)
}

/// The header of a sub-group of a section ([`SettingsField::sub_group`]): its name and a
/// rule to the right edge. The rows below it carry short names.
pub fn sub_group_header(ui: &mut Ui, name: &str) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(name).strong());
        let height = ui.spacing().interact_size.y;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width().max(0.0), height), egui::Sense::hover());
        ui.painter().hline(rect.x_range(), rect.center().y, ui.visuals().widgets.noninteractive.bg_stroke);
    });
}

/// The look of the joined groups in a row: compact cells, so that a few short words fit
/// the control column.
fn group_style(ui: &Ui) -> GroupStyle {
    GroupStyle::default().padding(egui::vec2(3.0, ui.spacing().button_padding.y))
}

/// `width` split into `n` equal parts with the item spacing between them.
fn part(ui: &Ui, width: f32, n: usize) -> f32 {
    ((width - ui.spacing().item_spacing.x * (n.saturating_sub(1)) as f32) / n as f32).max(20.0)
}

/// One row of the settings panels: `label` in a column of fixed width, then `body` in a cell
/// that fills the rest of the row, less a slot for the bind toggle or the "demo" tag that the
/// caller adds after it (on the same line: put both in one `ui.horizontal`). Every control
/// starts at the same x and every reset button ends at the same x. Returns what `body` returns.
pub fn setting_row(ui: &mut Ui, label: &str, hover: &str, body: impl FnOnce(&mut Ui) -> bool) -> bool {
    setting_row_with(
        ui,
        |ui| {
            ui.add(egui::Label::new(label).truncate()).on_hover_text(hover);
            false
        },
        body,
    )
}

/// [`setting_row`] with the label column drawn by `label` (for a checkbox that switches the
/// value on and off). Returns whether `label` or `body` reported a change.
pub fn setting_row_with(ui: &mut Ui, label: impl FnOnce(&mut Ui) -> bool, body: impl FnOnce(&mut Ui) -> bool) -> bool {
    let height = ui.spacing().interact_size.y;
    let gap = ROW_GAP;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        let total = ui.available_width();
        let label_w = (total * 0.36).clamp(90.0, 150.0);
        let cell_w = (total - label_w - TRAILING_W - 2.0 * gap).max(90.0);
        let mut changed = ui
            .allocate_ui_with_layout(egui::vec2(label_w, height), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.set_width(label_w);
                label(ui)
            })
            .inner;
        changed |= ui
            .allocate_ui_with_layout(egui::vec2(cell_w, height), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.set_width(cell_w);
                body(ui)
            })
            .inner;
        changed
    })
    .inner
}

/// A small "reset to default" button at the right edge of the control, enabled while `value`
/// differs from `default`. Puts `default` into `value` when clicked.
pub fn reset<T: PartialEq + Clone>(ui: &mut Ui, value: &mut T, default: &T) -> bool {
    let space = ui.available_width() - RESET_W;
    if space > 0.0 {
        ui.add_space(space);
    }
    // Exactly `RESET_W` wide, whatever the button would size itself to: a row that is wider
    // than it was given makes a resizable panel grow, frame after frame.
    let height = ui.spacing().interact_size.y;
    let clicked = ui
        .add_enabled_ui(*value != *default, |ui| {
            ui.add_sized([RESET_W, height], egui::Button::new("\u{21ba}").small()).on_hover_text("Reset to default").clicked()
        })
        .inner;
    if clicked {
        *value = default.clone();
    }
    clicked
}

/// What a field's name shows on hover: its full name and its path in the settings.
fn field_hover(field: SettingsField) -> String {
    format!("{}\n{}", field_label(field), field.path())
}

/// A labelled row for a field: its short name (the full name and its path on hover), then
/// the widgets of `body`.
fn row(ui: &mut Ui, field: SettingsField, body: impl FnOnce(&mut Ui) -> bool) -> bool {
    setting_row(ui, field_short_label(field), &field_hover(field), body)
}

/// A row below a field's own, for what did not fit its control: `name` (may be empty) in the
/// name column, `body` in the control column.
fn continued_row(ui: &mut Ui, name: &str, hover: &str, body: impl FnOnce(&mut Ui) -> bool) -> bool {
    setting_row_with(
        ui,
        |ui| {
            if !name.is_empty() {
                ui.add(egui::Label::new(egui::RichText::new(name).weak()).truncate()).on_hover_text(hover);
            }
            false
        },
        body,
    )
}

/// A drag value for a float, `width` wide, with a unit suffix. Existing out-of-range values
/// are not touched.
fn drag_w(ui: &mut Ui, v: &mut f32, range: RangeInclusive<f32>, speed: f64, suffix: &str, width: f32) -> bool {
    let height = ui.spacing().interact_size.y;
    ui.add_sized(
        [width, height],
        egui::DragValue::new(v).speed(speed).range(range).suffix(suffix).clamp_existing_to_range(false),
    )
    .changed()
}

/// A drag value for a float filling the control, with a unit suffix.
fn drag_f32(ui: &mut Ui, v: &mut f32, range: RangeInclusive<f32>, speed: f64, suffix: &str) -> bool {
    let w = control_width(ui);
    drag_w(ui, v, range, speed, suffix, w)
}

/// A drag value in milliseconds for a [`Duration`], filling the control.
fn drag_ms(ui: &mut Ui, d: &mut Duration, range: RangeInclusive<u64>) -> bool {
    let mut ms = d.as_millis() as u64;
    let height = ui.spacing().interact_size.y;
    let w = control_width(ui);
    let changed = ui
        .add_sized(
            [w, height],
            egui::DragValue::new(&mut ms).range(range).suffix(" ms").speed(5.0).clamp_existing_to_range(false),
        )
        .changed();
    if changed {
        *d = Duration::from_millis(ms);
    }
    changed
}

/// A colour button `width` wide.
fn color_w(ui: &mut Ui, c: &mut Color32, width: f32) -> bool {
    ui.scope(|ui| {
        ui.spacing_mut().interact_size.x = width;
        ui.color_edit_button_srgba(c).changed()
    })
    .inner
}

/// A checkbox row for a boolean field: the box is in the control column, like the other
/// controls, and the name is in the label column.
fn bool_row(ui: &mut Ui, field: SettingsField, value: &mut bool, default: bool) -> bool {
    row(ui, field, |ui| {
        let mut c = ui.add(egui::Checkbox::without_text(value)).on_hover_text(field.path()).changed();
        c |= reset(ui, value, &default);
        c
    })
}

/// A combo box over the named `options` of an enum, filling the control.
fn enum_row<T: Copy + PartialEq>(ui: &mut Ui, field: SettingsField, value: &mut T, default: T, options: &[(T, &str)]) -> bool {
    row(ui, field, |ui| {
        let mut changed = false;
        let w = control_width(ui);
        let shown = options.iter().find(|(o, _)| o == value).map_or("?", |(_, name)| *name);
        egui::ComboBox::from_id_salt(field).width(w).truncate().selected_text(shown).show_ui(ui, |ui| {
            for (o, name) in options {
                changed |= ui.selectable_value(value, *o, *name).changed();
            }
        });
        changed | reset(ui, value, &default)
    })
}

/// The named `options` of an enum side by side as one joined group, filling the control: for
/// two or three options with short names.
fn radio_row<T: Copy + PartialEq>(ui: &mut Ui, field: SettingsField, value: &mut T, default: T, options: &[(T, &str)]) -> bool {
    row(ui, field, |ui| radio(ui, field, value, options) | reset(ui, value, &default))
}

/// The named `options` of a value as one joined group, `control_width` wide (the control of
/// [`radio_row`]). Returns whether the value changed.
pub fn radio<T: Copy + PartialEq>(ui: &mut Ui, salt: impl std::hash::Hash + std::fmt::Debug, value: &mut T, options: &[(T, &str)]) -> bool {
    let before = *value;
    let w = control_width(ui);
    let style = group_style(ui);
    ui.add(RadioGroup::new(salt, value).options(options.iter().copied()).width(w).style(style));
    *value != before
}

/// A float row.
fn f32_row(
    ui: &mut Ui,
    field: SettingsField,
    v: &mut f32,
    default: f32,
    range: RangeInclusive<f32>,
    speed: f64,
    suffix: &str,
) -> bool {
    row(ui, field, |ui| drag_f32(ui, v, range, speed, suffix) | reset(ui, v, &default))
}

/// A duration row in milliseconds.
fn ms_row(ui: &mut Ui, field: SettingsField, d: &mut Duration, default: Duration, range: RangeInclusive<u64>) -> bool {
    row(ui, field, |ui| drag_ms(ui, d, range) | reset(ui, d, &default))
}

/// A colour row.
fn color_row(ui: &mut Ui, field: SettingsField, c: &mut Color32, default: Color32) -> bool {
    row(ui, field, |ui| {
        let w = control_width(ui);
        color_w(ui, c, w) | reset(ui, c, &default)
    })
}

/// Switches `value` on (to the remembered value, else `default_on`) or off, remembering the
/// value it had.
fn switch_option<T: Copy>(value: &mut Option<T>, on: bool, remembered: &mut Option<T>, default_on: T) {
    if on {
        *value = Some(remembered.unwrap_or(default_on));
    } else {
        if let Some(v) = *value {
            *remembered = Some(v);
        }
        *value = None;
    }
}

/// An optional value: a checkbox switches it on (with the value it had when it was last on in
/// this editor, remembered in egui's temporary memory, else `default_on`) or off; `edit` draws
/// the value while it is on.
fn opt_row<T: Copy + PartialEq + Send + Sync + 'static>(
    ui: &mut Ui,
    field: SettingsField,
    value: &mut Option<T>,
    default: Option<T>,
    default_on: T,
    edit: impl FnOnce(&mut Ui, &mut T) -> bool,
) -> bool {
    let memory = ui.id().with(("pie_last_value", field));
    let mut on = value.is_some();
    let mut toggled = false;
    let mut changed = setting_row_with(
        ui,
        |ui| {
            // The box, then the name truncated to the column (a checkbox's own text would not
            // truncate and would make the row wider than it was given).
            let boxed = ui.add(egui::Checkbox::without_text(&mut on)).on_hover_text(field_hover(field));
            let name = ui
                .add(egui::Label::new(field_short_label(field)).truncate().sense(egui::Sense::click()))
                .on_hover_text(field_hover(field));
            if name.clicked() {
                on = !on;
            }
            toggled = boxed.changed() || name.clicked();
            toggled
        },
        |ui| {
            let mut changed = false;
            match value {
                Some(v) => changed |= edit(ui, v),
                None => {
                    // Off: the value is not editable (it comes back as it was when it is switched on).
                    let w = control_width(ui);
                    let h = ui.spacing().interact_size.y;
                    ui.add_sized([w, h], egui::Label::new(egui::RichText::new("off").weak()));
                }
            }
            changed | reset(ui, value, &default)
        },
    );
    if toggled {
        let mut remembered = ui.data(|d| d.get_temp::<T>(memory));
        switch_option(value, on, &mut remembered, default_on);
        if let Some(v) = remembered {
            ui.data_mut(|d| d.insert_temp(memory, v));
        }
        changed = true;
    }
    changed
}

#[allow(clippy::too_many_arguments)]
fn opt_f32_row(
    ui: &mut Ui,
    field: SettingsField,
    value: &mut Option<f32>,
    default: Option<f32>,
    default_on: f32,
    range: RangeInclusive<f32>,
    speed: f64,
    suffix: &str,
) -> bool {
    opt_row(ui, field, value, default, default_on, |ui, v| drag_f32(ui, v, range, speed, suffix))
}

fn stroke_row(ui: &mut Ui, field: SettingsField, stroke: &mut Stroke, default: Stroke) -> bool {
    row(ui, field, |ui| {
        let w = part(ui, control_width(ui), 2);
        let mut changed = drag_w(ui, &mut stroke.width, 0.0..=64.0, 0.1, " px", w);
        changed |= color_w(ui, &mut stroke.color, w);
        changed | reset(ui, stroke, &default)
    })
}

fn margin_row(ui: &mut Ui, field: SettingsField, m: &mut Margin, default: Margin) -> bool {
    row(ui, field, |ui| {
        let before = *m;
        let w = control_width(ui);
        let style = group_style(ui);
        // One joined group: click a side to type its value, drag across several to set them
        // together.
        ui.add(
            FieldGroup::new(field)
                .labeled("L", &mut m.left)
                .labeled("R", &mut m.right)
                .labeled("T", &mut m.top)
                .labeled("B", &mut m.bottom)
                .width(w)
                .style(style),
        )
        .on_hover_text("Left, right, top and bottom padding, in px");
        (*m != before) | reset(ui, m, &default)
    })
}

/// Size and family (proportional / monospace) of a font. A named family is kept as it is.
fn font_row(ui: &mut Ui, field: SettingsField, f: &mut FontId, default: FontId) -> bool {
    row(ui, field, |ui| {
        let full = control_width(ui);
        let size_w = part(ui, full, 3);
        let mut changed = drag_w(ui, &mut f.size, 4.0..=96.0, 0.2, " pt", size_w);
        let shown = match &f.family {
            FontFamily::Proportional => "Proportional".to_string(),
            FontFamily::Monospace => "Monospace".to_string(),
            FontFamily::Name(n) => format!("\"{}\"", &**n),
        };
        // What is left of the control once the size is placed.
        let combo_w = control_width(ui);
        // In a cell of its own: a truncating combo box takes all the width that is left,
        // which here includes the reset button's.
        let height = ui.spacing().interact_size.y;
        ui.allocate_ui_with_layout(egui::vec2(combo_w, height), egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.set_max_width(combo_w);
            egui::ComboBox::from_id_salt(field).width(combo_w).truncate().selected_text(shown).show_ui(ui, |ui| {
                for (name, fam) in [("Proportional", FontFamily::Proportional), ("Monospace", FontFamily::Monospace)] {
                    changed |= ui.selectable_value(&mut f.family, fam, name).changed();
                }
            });
        });
        changed | reset(ui, f, &default)
    })
}

fn emphasis_row(ui: &mut Ui, field: SettingsField, e: &mut HoverEmphasis, default: HoverEmphasis) -> bool {
    row(ui, field, |ui| {
        let w = part(ui, control_width(ui), 2);
        let mut changed = drag_w(ui, &mut e.shift, 0.0..=30.0, 0.1, " px", w);
        changed |= drag_w(ui, &mut e.scale, 1.0..=1.5, 0.005, " \u{d7}", w);
        changed | reset(ui, e, &default)
    })
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

fn dismiss_name(d: &PieMenuDismissInput) -> String {
    match d {
        PieMenuDismissInput::Key(k) => format!("{k:?} key"),
        PieMenuDismissInput::PointerButton(b) => format!("{b:?} click"),
    }
}

/// A dismiss-input list: the "add" combo and the reset on the field's own row, then one row
/// per entry in the control column, each with a remove button where the reset is.
fn dismiss_row(ui: &mut Ui, field: SettingsField, list: &mut Vec<PieMenuDismissInput>, default: &[PieMenuDismissInput]) -> bool {
    let mut changed = row(ui, field, |ui| {
        let mut added = false;
        let w = control_width(ui);
        egui::ComboBox::from_id_salt(field).width(w).truncate().selected_text("Add\u{2026}").show_ui(ui, |ui| {
            let keys = DISMISS_KEYS.into_iter().map(PieMenuDismissInput::Key);
            let buttons = DISMISS_POINTER.into_iter().map(PieMenuDismissInput::PointerButton);
            for input in keys.chain(buttons) {
                // One of each: an input already in the list is not offered again.
                if !list.contains(&input) && ui.selectable_label(false, dismiss_name(&input)).clicked() {
                    list.push(input);
                    added = true;
                }
            }
        });
        added | reset(ui, list, &default.to_vec())
    });
    let mut remove = None;
    for (i, d) in list.iter().enumerate() {
        continued_row(ui, "", "", |ui| {
            let w = control_width(ui);
            let height = ui.spacing().interact_size.y;
            // The entry: a read-only cell as wide as the other controls.
            let (rect, _) = ui.allocate_exact_size(egui::vec2(w, height), egui::Sense::hover());
            let visuals = ui.visuals();
            ui.painter().rect_filled(rect, visuals.widgets.inactive.corner_radius, visuals.extreme_bg_color);
            ui.painter().text(
                rect.left_center() + egui::vec2(ui.spacing().button_padding.x, 0.0),
                egui::Align2::LEFT_CENTER,
                dismiss_name(d),
                egui::TextStyle::Body.resolve(ui.style()),
                visuals.text_color(),
            );
            if ui.add_sized([RESET_W, height], egui::Button::new("\u{2716}").small()).on_hover_text("Remove").clicked() {
                remove = Some(i);
            }
            false
        });
    }
    if let Some(i) = remove {
        list.remove(i);
        changed = true;
    }
    changed
}

// ---------------------------------------------------------------------------------------
// The editors
// ---------------------------------------------------------------------------------------

/// The editor widget of one settings field: a labelled row with a drag value (units: px, ms,
/// degrees, pt), checkbox, combo box or colour button, and a reset-to-default button.
/// An `Option` field is a checkbox that switches it on and off and remembers its last value in
/// egui's temporary memory. Draws the field whether or not it applies right now
/// ([`SettingsField::is_relevant`]). Returns `true` if the field changed.
pub fn field_editor(ui: &mut Ui, s: &mut PieMenuSettings, field: SettingsField) -> bool {
    use SettingsField as F;
    let d = PieMenuSettings::default();
    match field {
        F::ShowBehavior => {
            let memory = ui.id().with(("pie_last_threshold", field));
            let mut changed = row(ui, field, |ui| {
                let was_instant = matches!(s.show_behavior, ShowBehavior::Instant);
                let mut instant = was_instant;
                radio(ui, field, &mut instant, &[(true, "Instant"), (false, "On move")]);
                if instant != was_instant {
                    // The threshold comes back as it was when "On move" is chosen again.
                    if let ShowBehavior::OnMovement { threshold } = s.show_behavior {
                        ui.data_mut(|d| d.insert_temp(memory, threshold));
                    }
                    s.show_behavior = if instant {
                        ShowBehavior::Instant
                    } else {
                        ShowBehavior::OnMovement { threshold: ui.data(|d| d.get_temp::<f32>(memory)).unwrap_or(10.0) }
                    };
                }
                (instant != was_instant) | reset(ui, &mut s.show_behavior, &d.show_behavior)
            });
            if let ShowBehavior::OnMovement { threshold } = &mut s.show_behavior {
                changed |= continued_row(ui, "Threshold", "How far the pointer moves before the menu shows", |ui| {
                    drag_f32(ui, threshold, 0.0..=50.0, 0.2, " px")
                });
            }
            changed
        }
        F::DoubleTapWindow => {
            ms_row(ui, field, &mut s.input.double_tap_window, d.input.double_tap_window, 50..=1000)
        }
        F::SelectOnPrimaryClick => {
            bool_row(ui, field, &mut s.input.select_on_primary_click, d.input.select_on_primary_click)
        }
        F::DismissInputs => dismiss_row(ui, field, &mut s.input.dismiss_inputs, &d.input.dismiss_inputs),
        F::PinnedDismissInputs => {
            dismiss_row(ui, field, &mut s.input.pinned_dismiss_inputs, &d.input.pinned_dismiss_inputs)
        }
        F::BlockBackground => bool_row(ui, field, &mut s.block_background, d.block_background),
        F::UseNumberKeys => bool_row(ui, field, &mut s.input.use_number_keys, d.input.use_number_keys),
        F::UseMnemonicKeys => bool_row(ui, field, &mut s.input.use_mnemonic_keys, d.input.use_mnemonic_keys),
        F::AutoMnemonics => bool_row(ui, field, &mut s.input.auto_mnemonics, d.input.auto_mnemonics),
        F::HintColor => {
            let n = &mut s.number_hints;
            opt_row(ui, field, &mut n.color, d.number_hints.color, Color32::WHITE, |ui, c| {
                let w = control_width(ui);
                color_w(ui, c, w)
            })
        }
        F::HintSpacing => f32_row(ui, field, &mut s.number_hints.spacing, d.number_hints.spacing, 0.0..=40.0, 0.1, " px"),
        F::HintSide => enum_row(
            ui,
            field,
            &mut s.number_hints.side,
            d.number_hints.side,
            &[(HintSide::Trailing, "Trailing"), (HintSide::Outside, "Outside"), (HintSide::Left, "Left"), (HintSide::Right, "Right")],
        ),
        F::HintDigits => enum_row(
            ui,
            field,
            &mut s.number_hints.digits,
            d.number_hints.digits,
            &[(HintDigits::Western, "Western"), (HintDigits::ArabicIndic, "Arabic-Indic"), (HintDigits::Persian, "Persian")],
        ),
        F::LayoutRadius => f32_row(ui, field, &mut s.layout.radius, d.layout.radius, 20.0..=300.0, 0.5, " px"),
        F::LayoutShapeFactor => {
            f32_row(ui, field, &mut s.layout.shape_factor, d.layout.shape_factor, -1.0..=1.0, 0.01, "")
        }
        F::LayoutIntercardinalHeight => opt_f32_row(
            ui,
            field,
            &mut s.layout.intercardinal_height,
            d.layout.intercardinal_height,
            0.5,
            0.0..=1.0,
            0.01,
            "",
        ),
        F::LayoutDeadZoneRadius => {
            f32_row(ui, field, &mut s.layout.dead_zone_radius, d.layout.dead_zone_radius, 0.0..=60.0, 0.2, " px")
        }
        F::LayoutScreenMargin => {
            f32_row(ui, field, &mut s.layout.screen_margin, d.layout.screen_margin, 0.0..=64.0, 0.2, " px")
        }
        F::CenterBackgroundRadius => opt_f32_row(
            ui,
            field,
            &mut s.center_indicator.background_radius,
            d.center_indicator.background_radius,
            15.0,
            1.0..=60.0,
            0.2,
            " px",
        ),
        F::CenterBackgroundStroke => stroke_row(
            ui,
            field,
            &mut s.center_indicator.background_stroke,
            d.center_indicator.background_stroke,
        ),
        F::CenterBackgroundFillColor => color_row(
            ui,
            field,
            &mut s.center_indicator.background_fill_color,
            d.center_indicator.background_fill_color,
        ),
        F::CenterHighlightRadius => opt_f32_row(
            ui,
            field,
            &mut s.center_indicator.highlight_radius,
            d.center_indicator.highlight_radius,
            15.0,
            1.0..=60.0,
            0.2,
            " px",
        ),
        F::CenterHighlightShape => {
            // The four parts as one joined group in the control: press one and drag across
            // the others to switch several.
            let shape = &mut s.center_indicator.highlight_shape;
            row(ui, field, |ui| {
                let before = *shape;
                let w = control_width(ui);
                let style = group_style(ui);
                ui.add(
                    ToggleGroup::new(field)
                        .cell("arc", &mut shape.arc)
                        .cell("slice", &mut shape.slice)
                        .cell("dot", &mut shape.circle)
                        .cell("tooth", &mut shape.tooth)
                        .width(w)
                        .style(style),
                );
                (*shape != before) | reset(ui, shape, &d.center_indicator.highlight_shape)
            })
        }
        F::CenterHighlightStroke => stroke_row(
            ui,
            field,
            &mut s.center_indicator.highlight_stroke,
            d.center_indicator.highlight_stroke,
        ),
        F::CenterHighlightFillColor => color_row(
            ui,
            field,
            &mut s.center_indicator.highlight_fill_color,
            d.center_indicator.highlight_fill_color,
        ),
        F::CenterHighlightAngle => row(ui, field, |ui| {
            // Radians in the settings, degrees on screen; written back only when edited.
            let angle = &mut s.center_indicator.highlight_angle;
            let mut degrees = angle.to_degrees();
            let changed = drag_f32(ui, &mut degrees, 5.0..=180.0, 0.5, "\u{b0}");
            if changed {
                *angle = degrees.to_radians();
            }
            changed | reset(ui, angle, &d.center_indicator.highlight_angle)
        }),
        F::CenterHighlightCircleRadius => f32_row(
            ui,
            field,
            &mut s.center_indicator.highlight_circle_radius,
            d.center_indicator.highlight_circle_radius,
            1.0..=30.0,
            0.1,
            " px",
        ),
        F::CenterHighlightSnapping => {
            bool_row(ui, field, &mut s.center_indicator.highlight_snapping, d.center_indicator.highlight_snapping)
        }
        F::CenterToothStartLength => f32_row(ui, field, &mut s.center_indicator.tooth.start_length, d.center_indicator.tooth.start_length, 0.0..=100.0, 0.5, " px"),
        F::CenterToothLength => f32_row(ui, field, &mut s.center_indicator.tooth.length, d.center_indicator.tooth.length, 5.0..=100.0, 0.5, " px"),
        F::CenterToothStartBend => f32_row(ui, field, &mut s.center_indicator.tooth.start_bend, d.center_indicator.tooth.start_bend, 0.0..=1.0, 0.01, ""),
        F::CenterToothBend => f32_row(ui, field, &mut s.center_indicator.tooth.bend, d.center_indicator.tooth.bend, 0.0..=1.0, 0.01, ""),
        F::CenterToothStartTipRadius => f32_row(ui, field, &mut s.center_indicator.tooth.start_tip_radius, d.center_indicator.tooth.start_tip_radius, 0.0..=40.0, 0.2, " px"),
        F::CenterToothTipRadius => f32_row(ui, field, &mut s.center_indicator.tooth.tip_radius, d.center_indicator.tooth.tip_radius, 0.0..=20.0, 0.1, " px"),
        F::CenterToothStartRoot => row(ui, field, |ui| {
            // Radians in the settings, degrees on screen; written back only when edited.
            let angle = &mut s.center_indicator.tooth.start_root;
            let mut degrees = angle.to_degrees();
            let w = control_width(ui);
            let changed = drag_w(ui, &mut degrees, 0.0..=120.0, 0.5, "\u{b0}", w);
            if changed {
                *angle = degrees.to_radians();
            }
            changed | reset(ui, angle, &d.center_indicator.tooth.start_root)
        }),
        F::CenterToothRoot => row(ui, field, |ui| {
            // Radians in the settings, degrees on screen; written back only when edited.
            let angle = &mut s.center_indicator.tooth.root;
            let mut degrees = angle.to_degrees();
            let w = control_width(ui);
            let changed = drag_w(ui, &mut degrees, 0.0..=120.0, 0.5, "\u{b0}", w);
            if changed {
                *angle = degrees.to_radians();
            }
            changed | reset(ui, angle, &d.center_indicator.tooth.root)
        }),
        F::CenterToothRootTaper => bool_row(ui, field, &mut s.center_indicator.tooth.root_taper, d.center_indicator.tooth.root_taper),
        F::CenterToothLengthEasing => f32_row(ui, field, &mut s.center_indicator.tooth.length_easing, d.center_indicator.tooth.length_easing, 0.5..=3.0, 0.02, ""),
        F::CenterToothSharpenEasing => f32_row(ui, field, &mut s.center_indicator.tooth.sharpen_easing, d.center_indicator.tooth.sharpen_easing, 1.0..=4.0, 0.02, ""),
        F::CenterToothOutlineBase => bool_row(ui, field, &mut s.center_indicator.tooth.outline_base, d.center_indicator.tooth.outline_base),
        F::TitleShow => bool_row(ui, field, &mut s.title.show, d.title.show),
        F::TitleBackgroundColor => color_row(ui, field, &mut s.title.background_color, d.title.background_color),
        F::TitleBackgroundStroke => stroke_row(ui, field, &mut s.title.background_stroke, d.title.background_stroke),
        F::TitlePadding => margin_row(ui, field, &mut s.title.padding, d.title.padding),
        F::TitleTextColor => color_row(ui, field, &mut s.title.text_color, d.title.text_color),
        F::TitleTextFont => font_row(ui, field, &mut s.title.text_font, d.title.text_font.clone()),
        F::TextColor => {
            opt_row(ui, field, &mut s.text_color, d.text_color, Color32::WHITE, |ui, c| {
                let w = control_width(ui);
                color_w(ui, c, w)
            })
        }
        F::Reveal => radio_row(
            ui,
            field,
            &mut s.animations.reveal,
            d.animations.reveal,
            &[(RevealAnimation::None, "None"), (RevealAnimation::Fade, "Fade"), (RevealAnimation::SlideOut, "Slide")],
        ),
        F::RevealDuration => {
            ms_row(ui, field, &mut s.animations.reveal_duration, d.animations.reveal_duration, 0..=1000)
        }
        F::SlideStartRadius => {
            f32_row(ui, field, &mut s.animations.slide_start_radius, d.animations.slide_start_radius, 0.0..=300.0, 0.5, " px")
        }
        F::SlideStartShapeFactor => opt_f32_row(
            ui,
            field,
            &mut s.animations.slide_start_shape_factor,
            d.animations.slide_start_shape_factor,
            -1.0,
            -1.0..=1.0,
            0.01,
            "",
        ),
        F::SlideFade => opt_row(
            ui,
            field,
            &mut s.animations.slide_fade,
            d.animations.slide_fade,
            Duration::from_millis(200),
            |ui, v| drag_ms(ui, v, 0..=1000),
        ),
        F::HoverEmphasis => emphasis_row(ui, field, &mut s.animations.hover_emphasis, d.animations.hover_emphasis),
        F::SubPieHoverEmphasis => {
            emphasis_row(ui, field, &mut s.animations.sub_pie_hover_emphasis, d.animations.sub_pie_hover_emphasis)
        }
        F::HoverDuration => ms_row(ui, field, &mut s.animations.hover_duration, d.animations.hover_duration, 0..=500),
        F::SubPieStyle => radio_row(
            ui,
            field,
            &mut s.sub_pies.style,
            d.sub_pies.style,
            &[(SubPieStyle::Replace, "Replace"), (SubPieStyle::FanOut, "Fan-out")],
        ),
        F::SubPieCenter => radio_row(
            ui,
            field,
            &mut s.sub_pies.center,
            d.sub_pies.center,
            &[(SubPieCenter::Pointer, "Pointer"), (SubPieCenter::Parent, "Parent"), (SubPieCenter::Button, "Button")],
        ),
        F::SubPieHoverDelay => opt_row(
            ui,
            field,
            &mut s.sub_pies.hover_delay,
            d.sub_pies.hover_delay,
            Duration::from_millis(400),
            |ui, v| drag_ms(ui, v, 0..=2000),
        ),
        F::FanOutDelay => ms_row(ui, field, &mut s.sub_pies.fan_out.delay, d.sub_pies.fan_out.delay, 0..=2000),
        F::FanOutRadius => opt_f32_row(
            ui,
            field,
            &mut s.sub_pies.fan_out.radius,
            d.sub_pies.fan_out.radius,
            100.0,
            20.0..=300.0,
            0.5,
            " px",
        ),
        F::FanOutOpenerAction => radio_row(
            ui,
            field,
            &mut s.sub_pies.fan_out.opener_action,
            d.sub_pies.fan_out.opener_action,
            &[(FanOpenerAction::BreakOut, "Break out"), (FanOpenerAction::Dismiss, "Dismiss")],
        ),
        F::MirrorHorizontal => bool_row(ui, field, &mut s.mirror_horizontal, d.mirror_horizontal),
    }
}

/// The fields of one section that apply to `s` ([`SettingsGroup::relevant_fields`]), each drawn
/// by [`field_editor`]. Returns `true` if anything changed.
pub fn group_editor(ui: &mut Ui, s: &mut PieMenuSettings, group: SettingsGroup) -> bool {
    group_editor_with(ui, s, group, field_editor)
}

/// The fields of a section that apply, under their sub-group headers, each drawn by `editor`.
fn group_editor_with(
    ui: &mut Ui,
    s: &mut PieMenuSettings,
    group: SettingsGroup,
    mut editor: impl FnMut(&mut Ui, &mut PieMenuSettings, SettingsField) -> bool,
) -> bool {
    let mut changed = false;
    let mut sub_group = None;
    for field in group.relevant_fields(s) {
        if field.sub_group() != sub_group {
            sub_group = field.sub_group();
            if let Some(name) = sub_group {
                sub_group_header(ui, name);
            }
        }
        changed |= editor(ui, s, field);
    }
    changed
}

/// [`field_editor`] with a bind toggle in front of a bindable field
/// ([`SettingsField::is_bindable`]) and, while it is bound, the expression and Build / Live on a
/// row below ([`binding_details`]). The value stays editable: it is the preview value. Other
/// fields are drawn as by [`field_editor`]. Returns `true` if the field or its binding changed.
pub fn field_editor_bound(ui: &mut Ui, s: &mut PieMenuSettings, field: SettingsField, bindings: &mut Bindings) -> bool {
    if !field.is_bindable() {
        return field_editor(ui, s, field);
    }
    crate::bindings::bound_row(ui, bindings, &BindTarget::Field(field), |ui| field_editor(ui, s, field))
}

/// [`group_editor`] drawing its fields with [`field_editor_bound`].
pub fn group_editor_bound(ui: &mut Ui, s: &mut PieMenuSettings, group: SettingsGroup, bindings: &mut Bindings) -> bool {
    group_editor_with(ui, s, group, |ui, s, field| field_editor_bound(ui, s, field, bindings))
}

/// Controls for every field of [`PieMenuSettings`], one collapsing header per
/// [`SettingsGroup`]. Returns `true` if anything changed.
pub fn settings_editor(ui: &mut Ui, s: &mut PieMenuSettings) -> bool {
    let mut changed = false;
    for group in SettingsGroup::ALL {
        let open = group == SettingsGroup::Ring;
        changed |= egui::CollapsingHeader::new(group.title())
            .default_open(open)
            .show(ui, |ui| group_editor(ui, s, group))
            .body_returned
            .unwrap_or(false);
    }
    changed
}

#[cfg(test)]
mod tests {
    #![allow(clippy::field_reassign_with_default)]
    use crate::settings_entries;
    use super::*;
    use egui::Context;
    use std::collections::HashSet;

    fn run(f: impl FnOnce(&mut Ui) -> bool) -> bool {
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

    /// Settings with every `Option` on, odd values, and the non-default branch of each enum.
    fn rich() -> PieMenuSettings {
        let mut s = PieMenuSettings::blender_like();
        s.show_behavior = ShowBehavior::Instant;
        s.layout.intercardinal_height = Some(0.3);
        s.center_indicator.background_radius = None;
        s.animations.reveal = RevealAnimation::SlideOut;
        s.animations.slide_start_shape_factor = Some(-0.5);
        s.animations.slide_fade = None;
        s.sub_pies.hover_delay = Some(Duration::from_millis(333));
        s.sub_pies.fan_out.radius = Some(77.0);
        s.sub_pies.style = SubPieStyle::FanOut;
        s.title.text_font = FontId::new(14.0, FontFamily::Name("Fancy".into()));
        s.input.dismiss_inputs.clear();
        s.mirror_horizontal = true;
        s
    }

    /// #28: every section lists its plain fields first and then each sub-group's together,
    /// and a short name is unique among the rows that share its headers.
    #[test]
    fn sub_groups_are_contiguous_and_short_names_unique_under_their_headers() {
        for group in SettingsGroup::ALL {
            let fields = group.fields();
            let mut seen: Vec<Option<&str>> = Vec::new();
            for f in fields {
                let sub = f.sub_group();
                if seen.last() != Some(&sub) {
                    assert!(!seen.contains(&sub), "{group:?}: sub-group {sub:?} is split up at {f:?}");
                    assert!(sub.is_some() || seen.is_empty(), "{group:?}: {f:?} has no sub-group but follows one");
                    seen.push(sub);
                }
            }
            let mut names = HashSet::new();
            for f in fields {
                assert!(!field_short_label(*f).is_empty());
                assert!(names.insert((f.sub_group(), field_short_label(*f))), "{group:?}: two rows named {:?}", field_short_label(*f));
            }
        }
        assert_eq!(SettingsField::HintColor.sub_group(), Some("Number hints"));
        assert_eq!(field_short_label(SettingsField::HintColor), "Colour");
        assert_eq!(field_label(SettingsField::HintColor), "Number hint colour", "the full name is kept for hovers and notes");
        assert_eq!(SettingsField::LayoutRadius.sub_group(), None);
    }

    #[test]
    fn every_settings_path_is_one_field_in_exactly_one_group() {
        let paths: Vec<String> = settings_entries(&PieMenuSettings::default()).into_iter().map(|(p, _)| p).collect();
        assert_eq!(paths.len(), SettingsField::ALL.len());
        let field_paths: HashSet<&str> = SettingsField::ALL.iter().map(|f| f.path()).collect();
        assert_eq!(field_paths.len(), SettingsField::ALL.len(), "paths are unique");
        let entry_paths: HashSet<&str> = paths.iter().map(|p| p.as_str()).collect();
        assert_eq!(field_paths, entry_paths);

        let mut in_groups: Vec<SettingsField> = SettingsGroup::ALL.iter().flat_map(|g| g.fields().iter().copied()).collect();
        assert_eq!(in_groups.len(), SettingsField::ALL.len(), "no field twice");
        in_groups.sort_by_key(|f| f.path());
        let mut all = SettingsField::ALL.to_vec();
        all.sort_by_key(|f| f.path());
        assert_eq!(in_groups, all);
        for f in SettingsField::ALL {
            assert!(f.group().fields().contains(&f));
            assert!(!field_label(f).is_empty());
        }
        let labels: HashSet<&str> = SettingsField::ALL.iter().map(|f| field_label(*f)).collect();
        assert_eq!(labels.len(), SettingsField::ALL.len(), "labels are unique");
    }

    #[test]
    fn relevance_hides_inapplicable_fields() {
        let mut s = PieMenuSettings::default();
        let shown = |s: &PieMenuSettings, g: SettingsGroup| g.relevant_fields(s);
        assert!(shown(&s, SettingsGroup::OpeningClosing).contains(&SettingsField::DoubleTapWindow));
        s.show_behavior = ShowBehavior::Instant;
        assert!(!shown(&s, SettingsGroup::OpeningClosing).contains(&SettingsField::DoubleTapWindow));
        assert!(!shown(&s, SettingsGroup::Animation).contains(&SettingsField::SlideFade));
        s.animations.reveal = RevealAnimation::SlideOut;
        for f in [SettingsField::SlideStartRadius, SettingsField::SlideStartShapeFactor, SettingsField::SlideFade] {
            assert!(shown(&s, SettingsGroup::Animation).contains(&f));
        }
        assert!(shown(&s, SettingsGroup::KeysAndHints).contains(&SettingsField::HintSide));
        s.input.use_number_keys = false;
        let keys = shown(&s, SettingsGroup::KeysAndHints);
        assert_eq!(keys, [SettingsField::UseNumberKeys, SettingsField::UseMnemonicKeys, SettingsField::AutoMnemonics]);
    }

    #[test]
    fn untouched_editors_change_nothing() {
        for start in [PieMenuSettings::default(), rich()] {
            for field in SettingsField::ALL {
                let mut s = start.clone();
                assert!(!run(|ui| field_editor(ui, &mut s, field)), "{field:?} reported a change");
                assert_eq!(format!("{s:?}"), format!("{start:?}"), "{field:?} changed the settings");
            }
            for group in SettingsGroup::ALL {
                let mut s = start.clone();
                assert!(!run(|ui| group_editor(ui, &mut s, group)), "{group:?}");
                assert_eq!(format!("{s:?}"), format!("{start:?}"));
            }
            let mut s = start.clone();
            assert!(!run(|ui| settings_editor(ui, &mut s)));
            assert_eq!(format!("{s:?}"), format!("{start:?}"));
        }
    }

    #[test]
    fn optional_field_remembers_its_last_value() {
        let mut value = Some(0.8f32);
        let mut remembered = None;
        switch_option(&mut value, false, &mut remembered, 0.5);
        assert_eq!((value, remembered), (None, Some(0.8)));
        switch_option(&mut value, true, &mut remembered, 0.5);
        assert_eq!(value, Some(0.8));
        // Never on before: the default.
        let (mut value, mut remembered) = (None, None);
        switch_option(&mut value, true, &mut remembered, 0.5);
        assert_eq!(value, Some(0.5));
        // Switching off a value that is already off keeps what was remembered.
        let (mut value, mut remembered) = (None, Some(0.2f32));
        switch_option(&mut value, false, &mut remembered, 0.5);
        assert_eq!(remembered, Some(0.2));
    }

    #[test]
    fn reset_restores_the_default() {
        let mut v = 5.0f32;
        let mut clicked = false;
        let _ = run(|ui| {
            // Not clicked in a headless run: enabled, no change.
            clicked = reset(ui, &mut v, &1.0);
            false
        });
        assert!(!clicked);
        assert_eq!(v, 5.0);
    }
}
