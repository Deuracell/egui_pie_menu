use egui::{Color32, Key, Pos2, Rect, Stroke, Ui, Vec2};
pub use egui::text::{LayoutJob, TextFormat};
pub use egui::FontId;
use std::f32::consts::{PI, TAU};
use std::time::Instant;

mod utils;
pub use utils::common_utils::{BoundedVec, SmartFloat};

pub mod settings;
pub use settings::*;

pub mod highlight_shapes;
pub use highlight_shapes::*;

/// A single button slot in the pie menu.
///
/// Only defines *where* the button sits (`direction`) and an optional keyboard
/// mnemonic. All visual content is rendered by the caller via the closure
/// passed to [`PieMenu::show`].
pub struct PieButton {
    pub direction: PieDirection,
    pub mnemonic: Option<char>,
}

impl PieButton {
    pub fn new(direction: PieDirection) -> Self {
        Self { direction, mnemonic: None }
    }

    pub fn with_mnemonic(mut self, c: char) -> Self {
        self.mnemonic = Some(c);
        self
    }
}

/// Outcome of a [`PieMenu::show`] call.
pub enum PieMenuResponse {
    /// A button was activated; value is its index in the slice passed to `show`.
    Selected(usize),
    /// The menu was dismissed without a selection.
    Dismissed,
    /// Key was tapped quickly without moving the mouse past the threshold.
    /// The caller decides what default action to perform.
    QuickTap,
    /// Two quick taps within [`PieMenuInput::double_tap_window`].
    /// The caller decides what alternate action to perform.
    DoubleTap,
    /// No action this frame.
    None,
}

/// Radial (pie) menu widget.
///
/// Owns interaction state but not button content. Construct once, then call
/// [`PieMenu::show`] every frame with the buttons for the current context.
pub struct PieMenu {
    pub id: egui::Id,
    open_time: Instant,
    selected_index: Option<usize>,
    pub position: Pos2,
    pub settings: PieMenuSettings,
    release_handled: bool,
    /// Whether the mouse has crossed `show_threshold` since `open()` was called.
    mouse_shown: bool,
    /// Timestamp of the last QuickTap, used for double-tap detection.
    last_quick_tap: Option<Instant>,
    /// Cached button sizes from the previous frame, used to centre each Area.
    button_sizes: Vec<Vec2>,
}

impl Default for PieMenu {
    fn default() -> Self {
        Self::new()
    }
}

impl PieMenu {
    pub fn new() -> Self {
        Self {
            id: egui::Id::new("pie_menu"),
            open_time: Instant::now(),
            selected_index: None,
            position: Pos2::ZERO,
            settings: PieMenuSettings::default(),
            release_handled: false,
            mouse_shown: false,
            last_quick_tap: None,
            button_sizes: Vec::new(),
        }
    }

    pub fn with_id(mut self, id: impl std::hash::Hash) -> Self {
        self.id = egui::Id::new(id);
        self
    }

    pub fn with_settings(mut self, settings: PieMenuSettings) -> Self {
        self.settings = settings;
        self
    }

    /// Call this when opening the menu. Resets interaction state and sets position.
    pub fn open(&mut self, pos: Pos2) {
        self.position = pos;
        self.open_time = Instant::now();
        self.release_handled = false;
        self.selected_index = None;
        self.mouse_shown = false;
    }

    /// Converts a [`PieDirection`] to a [`Vec2`] offset, applying the shape factor.
    ///
    /// - `0.0`  → circle: all buttons equidistant (L2, unit vector)
    /// - `+1.0` → square: diagonals pushed out to the corners (L∞ norm)
    /// - `-1.0` → diamond: diagonals pulled in toward the centre (L1 norm)
    fn direction_vec(dir: &PieDirection, shape_factor: f32) -> Vec2 {
        let fraction = match dir {
            PieDirection::North     => 0.75,
            PieDirection::NorthEast => 0.875,
            PieDirection::East      => 1.0,
            PieDirection::SouthEast => 0.125,
            PieDirection::South     => 0.25,
            PieDirection::SouthWest => 0.375,
            PieDirection::West      => 0.5,
            PieDirection::NorthWest => 0.625,
        };
        let angle = TAU * fraction;
        let circle = Vec2::new(angle.cos(), angle.sin());

        match dir {
            PieDirection::NorthEast
            | PieDirection::SouthEast
            | PieDirection::SouthWest
            | PieDirection::NorthWest => {
                if shape_factor > 0.0 {
                    let l_inf = circle / circle.x.abs().max(circle.y.abs());
                    circle * (1.0 - shape_factor) + l_inf * shape_factor
                } else if shape_factor < 0.0 {
                    let l1 = circle / (circle.x.abs() + circle.y.abs());
                    let t = -shape_factor;
                    circle * (1.0 - t) + l1 * t
                } else {
                    circle
                }
            }
            _ => circle,
        }
    }

    fn direction_numpad(dir: &PieDirection) -> Key {
        match dir {
            PieDirection::North     => Key::Num8,
            PieDirection::NorthEast => Key::Num9,
            PieDirection::East      => Key::Num6,
            PieDirection::SouthEast => Key::Num3,
            PieDirection::South     => Key::Num2,
            PieDirection::SouthWest => Key::Num1,
            PieDirection::West      => Key::Num4,
            PieDirection::NorthWest => Key::Num7,
        }
    }

    /// Show the pie menu for this frame.
    ///
    /// `render_button(ui, index, is_hovered)` is called once per button inside
    /// a floating [`egui::Area`] centred at that button's position. Use it to
    /// draw any egui content — labels, images, custom widgets, etc.
    ///
    /// `key_down` should be `true` while the hotkey that opened the menu is held.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        buttons: &[PieButton],
        current_mouse_pos: Option<Pos2>,
        key_down: bool,
        title: Option<&str>,
        mut render_button: impl FnMut(&mut Ui, usize, bool),
    ) -> PieMenuResponse {
        // Ensure cached size vec is long enough
        if self.button_sizes.len() < buttons.len() {
            self.button_sizes.resize(buttons.len(), Vec2::new(50.0, 24.0));
        }

        let center = self.position;

        // Update mouse_shown: once the mouse travels past show_threshold, latch it on.
        if !self.mouse_shown {
            if let Some(p) = current_mouse_pos {
                if (p - center).length() > self.settings.show_threshold {
                    self.mouse_shown = true;
                }
            }
        }

        // Before the menu is visible, handle key release as QuickTap/DoubleTap only.
        if !self.mouse_shown {
            if !key_down && !self.release_handled {
                self.release_handled = true;
                let is_double = self.last_quick_tap
                    .map(|t| t.elapsed() <= self.settings.input.double_tap_window)
                    .unwrap_or(false);
                if is_double {
                    self.last_quick_tap = None;
                    return PieMenuResponse::DoubleTap;
                } else {
                    self.last_quick_tap = Some(Instant::now());
                    return PieMenuResponse::QuickTap;
                }
            }
            return PieMenuResponse::None;
        }

        // Early exit checks
        if ctx.input(|i| {
            (self.settings.input.dismiss_on_numpad_5 && i.key_pressed(Key::Num5))
                || (self.settings.input.dismiss_on_escape_key && i.key_pressed(Key::Escape))
                || (self.settings.input.dismiss_on_secondary_mouse_click
                    && i.pointer.secondary_clicked())
        }) {
            return PieMenuResponse::Dismissed;
        }

        let shape_factor = self.settings.shape_factor;

        // Compute the bounding box of all buttons (relative to self.position) and
        // shift the whole menu so it stays within the screen rect with a margin.
        let screen_rect = ctx.content_rect();
        let margin = self.settings.screen_margin;
        let mut bb_min = Vec2::ZERO;
        let mut bb_max = Vec2::ZERO;
        for (idx, button) in buttons.iter().enumerate() {
            let dv = Self::direction_vec(&button.direction, shape_factor);
            let dn = dv.normalized();
            let sz = self.button_sizes[idx];
            let hw = sz.x / 2.0;
            let hh = sz.y / 2.0;
            let sx = if dn.x > 0.5 { 1.0 } else if dn.x < -0.5 { -1.0 } else { 0.0 };
            let sy = if dn.y > 0.5 { 1.0 } else if dn.y < -0.5 { -1.0 } else { 0.0 };
            let offset = dv * self.settings.layout_radius + Vec2::new(hw * sx, hh * sy);
            bb_min = bb_min.min(offset - sz / 2.0);
            bb_max = bb_max.max(offset + sz / 2.0);
        }
        let center = Pos2::new(
            center.x.clamp(
                screen_rect.min.x + margin - bb_min.x,
                screen_rect.max.x - margin - bb_max.x,
            ),
            center.y.clamp(
                screen_rect.min.y + margin - bb_min.y,
                screen_rect.max.y - margin - bb_max.y,
            ),
        );

        let painter = ctx.layer_painter(
            egui::LayerId::new(egui::Order::Tooltip, self.id),
        );

        // Center background circle
        if self.settings.center_indicator.background_radius.is_enabled() {
            painter.circle(
                center,
                self.settings.center_indicator.background_radius.get(),
                self.settings.center_indicator.background_fill_color,
                self.settings.center_indicator.background_stroke,
            );
        }

        // Numpad key selection
        if self.settings.input.use_numpad_keys {
            for (idx, button) in buttons.iter().enumerate() {
                if ctx.input(|i| i.key_pressed(Self::direction_numpad(&button.direction))) {
                    return PieMenuResponse::Selected(idx);
                }
            }
        }

        // Mnemonic key selection — consume matched events so other handlers never see them.
        if self.settings.input.use_mnemonic_keys {
            #[cfg(debug_assertions)]
            {
                let mut seen = std::collections::HashMap::new();
                for button in buttons {
                    if let Some(c) = button.mnemonic {
                        let key = c.to_ascii_lowercase();
                        let prev: &mut Vec<PieDirection> = seen.entry(key).or_default();
                        prev.push(button.direction.clone());
                        if prev.len() == 2 {
                            eprintln!(
                                "egui_pie_menu: duplicate mnemonic '{key}' on {:?}",
                                prev
                            );
                        }
                    }
                }
            }

            for (idx, button) in buttons.iter().enumerate() {
                if let Some(key) = button.mnemonic.and_then(char_to_key) {
                    let pressed = ctx.input_mut(|i| {
                        if i.key_pressed(key) {
                            i.consume_key(egui::Modifiers::NONE, key);
                            true
                        } else {
                            false
                        }
                    });
                    if pressed {
                        return PieMenuResponse::Selected(idx);
                    }
                }
            }
        }

        // Primary click selection
        if ctx.input(|i| i.pointer.primary_clicked()) {
            return match current_mouse_pos {
                Some(p) if (p - center).length() > self.settings.mouse_trigger_threshold => {
                    self.selected_index
                        .map_or(PieMenuResponse::None, PieMenuResponse::Selected)
                }
                Some(_) => PieMenuResponse::None,
                None => PieMenuResponse::Dismissed,
            };
        }

        // Update hover selection from mouse angle
        self.selected_index = current_mouse_pos.and_then(|mouse_pos| {
            let mouse_vec = mouse_pos - center;
            if mouse_vec.length() <= self.settings.mouse_trigger_threshold {
                return None;
            }
            let angle = mouse_vec.y.atan2(mouse_vec.x);
            buttons
                .iter()
                .enumerate()
                .map(|(idx, button)| {
                    let dir_vec = Self::direction_vec(&button.direction, shape_factor);
                    let button_angle = dir_vec.y.atan2(dir_vec.x);
                    let diff = ((angle - button_angle + PI) % TAU - PI).abs();
                    (idx, diff)
                })
                .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                .map(|(idx, _)| idx)
        });

        // Center highlight — progress driven by mouse distance from center.
        if self.settings.center_indicator.highlight_shape != PieMenuHighlightShape::None
            && self.settings.animations.center_highlight_show
        {
            let progress = current_mouse_pos
                .filter(|_| self.selected_index.is_some())
                .map(|p| {
                    let distance = (p - center).length();
                    let threshold = self.settings.mouse_trigger_threshold;
                    let max_dist = self.settings.layout_radius;
                    ((distance - threshold) / (max_dist - threshold)).clamp(0.0, 1.0)
                })
                .unwrap_or(0.0);

            let base_angle = if self.settings.animations.center_highlight_snapping {
                self.selected_index
                    .and_then(|idx| buttons.get(idx))
                    .map(|b| {
                        let v = Self::direction_vec(&b.direction, shape_factor);
                        v.y.atan2(v.x)
                    })
                    .unwrap_or(0.0)
            } else {
                current_mouse_pos
                    .map(|p| { let v = p - center; v.y.atan2(v.x) })
                    .unwrap_or(0.0)
            };

            let arc_angle = self.settings.center_indicator.highlight_angle * progress;
            let start_angle = base_angle - arc_angle / 2.0;
            let angle_range = start_angle..(start_angle + arc_angle);

            let stroke_color = self.settings.center_indicator.highlight_stroke.color.gamma_multiply(progress);
            let fill_color = self.settings.center_indicator.highlight_fill_color.gamma_multiply(progress);
            let colored_stroke = Stroke::new(self.settings.center_indicator.highlight_stroke.width, stroke_color);
            let highlight_radius = self.settings.center_indicator.highlight_radius.get();
            let shape = self.settings.center_indicator.highlight_shape;

            let needs_arc = matches!(shape,
                PieMenuHighlightShape::Arc | PieMenuHighlightShape::ArcSlice |
                PieMenuHighlightShape::ArcCircle | PieMenuHighlightShape::ArcSliceCircle);
            let needs_slice = matches!(shape,
                PieMenuHighlightShape::Slice | PieMenuHighlightShape::SliceCircle |
                PieMenuHighlightShape::ArcSlice | PieMenuHighlightShape::ArcSliceCircle);
            let needs_circle = matches!(shape,
                PieMenuHighlightShape::Circle | PieMenuHighlightShape::ArcCircle |
                PieMenuHighlightShape::SliceCircle | PieMenuHighlightShape::ArcSliceCircle);

            let arc_arg = needs_arc.then(|| ArcValues {
                angle_range: angle_range.clone(), center,
                radius: highlight_radius, resolution: 10.0, stroke: colored_stroke,
            });
            let slice_arg = needs_slice.then(|| {
                let inner_arc = matches!(shape, PieMenuHighlightShape::Slice | PieMenuHighlightShape::SliceCircle)
                    .then(|| ArcValues {
                        angle_range: angle_range.clone(), center,
                        radius: highlight_radius, resolution: 10.0, stroke: colored_stroke,
                    });
                SliceValues { arc_values: inner_arc, stroke: None, fill_color }
            });
            let circle_arg = needs_circle.then(|| CircleValues {
                offset_angle: base_angle, offset_radius: highlight_radius,
                offset_center: center,
                circle_radius: self.settings.center_indicator.highlight_circle_radius,
                stroke: colored_stroke, fill_color,
            });

            painter.highlight_shape(shape, arc_arg, slice_arg, circle_arg);
        }

        // Draw menu title above the center indicator
        if self.settings.label.display {
            if let Some(label) = title {
                let pad = &self.settings.label.padding;
                let galley = painter.layout_no_wrap(
                    label.to_string(),
                    self.settings.label.text_font.clone(),
                    self.settings.label.text_color,
                );
                let text_size = galley.size();
                let bg_w = text_size.x + pad.left + pad.right;
                let bg_h = text_size.y + pad.top + pad.bottom;
                let above_offset = self.settings.center_indicator.background_radius.get() + bg_h / 2.0 + 4.0;
                let label_center = center - Vec2::new(0.0, above_offset);
                let bg_rect = Rect::from_center_size(label_center, Vec2::new(bg_w, bg_h));

                if self.settings.label.background_color != Color32::TRANSPARENT {
                    painter.rect_filled(bg_rect, 3.0, self.settings.label.background_color);
                }
                if self.settings.label.background_stroke.width > 0.0
                    && self.settings.label.background_stroke.color != Color32::TRANSPARENT
                {
                    painter.rect_stroke(bg_rect, 3.0, self.settings.label.background_stroke, egui::StrokeKind::Outside);
                }
                painter.galley(bg_rect.min + Vec2::new(pad.left, pad.top), galley, self.settings.label.text_color);
            }
        }

        // Draw buttons via caller-provided closure inside floating Areas
        for (idx, button) in buttons.iter().enumerate() {
            let dir_vec = Self::direction_vec(&button.direction, shape_factor);
            let dir_norm = dir_vec.normalized();

            // Push the button outward so its inner face/corner sits on the layout circle.
            // Each axis is offset independently: X by ±hw, Y by ±hh, based on the sign of
            // the direction. This keeps NE/NW at the same Y and SE/SW at the same Y even
            // when button widths differ, and the inner corner still lands exactly on the circle.
            let cached_size = self.button_sizes[idx];
            let hw = cached_size.x / 2.0;
            let hh = cached_size.y / 2.0;
            // Use 0.5 as threshold: diagonal components are ~0.707, cardinal near-zero
            // components are floating-point noise (~1e-17) and must not contribute.
            let sx = if dir_norm.x > 0.5 { 1.0 } else if dir_norm.x < -0.5 { -1.0 } else { 0.0 };
            let sy = if dir_norm.y > 0.5 { 1.0 } else if dir_norm.y < -0.5 { -1.0 } else { 0.0 };

            let button_center = center + dir_vec * self.settings.layout_radius + Vec2::new(hw * sx, hh * sy);
            let is_hovered = self.selected_index == Some(idx);

            let area_pos = button_center - cached_size / 2.0;

            let response = egui::Area::new(self.id.with(idx))
                .order(egui::Order::Tooltip)
                .fixed_pos(area_pos)
                .show(ctx, |ui| render_button(ui, idx, is_hovered));

            self.button_sizes[idx] = response.response.rect.size();
        }

        // Key-hold release (mouse_shown is true here, so menu was visible)
        if !key_down && !self.release_handled {
            self.release_handled = true;
            if let Some(mouse_pos) = current_mouse_pos {
                if (mouse_pos - center).length() <= self.settings.mouse_trigger_threshold {
                    return PieMenuResponse::Dismissed;
                } else if let Some(idx) = self.selected_index {
                    return PieMenuResponse::Selected(idx);
                }
            }
        }

        PieMenuResponse::None
    }
}

/// Build a [`LayoutJob`] that renders `text` with the first case-insensitive
/// occurrence of `mnemonic` underlined — suitable for use as a pie menu button
/// label when you want to surface keyboard shortcuts visually.
///
/// The rest of the text uses `format` unchanged. The underline colour is
/// derived from `format.color`.  If `mnemonic` does not appear in `text`
/// the whole string is rendered with `format` and no underline.
///
/// # Example
/// ```ignore
/// ui.label(mnemonic_text("Copy", 'c', TextFormat {
///     color: Color32::WHITE,
///     font_id: FontId::default(),
///     ..Default::default()
/// }));
/// ```
pub fn mnemonic_text(text: &str, mnemonic: char, format: TextFormat) -> LayoutJob {
    let mnemonic_lower = mnemonic.to_ascii_lowercase();
    let split = text.char_indices().find(|(_, c)| c.to_ascii_lowercase() == mnemonic_lower);

    let mut job = LayoutJob::default();

    match split {
        None => {
            job.append(text, 0.0, format);
        }
        Some((byte_idx, c)) => {
            let before = &text[..byte_idx];
            let after  = &text[byte_idx + c.len_utf8()..];
            let ch_str = &text[byte_idx..byte_idx + c.len_utf8()];

            let underline_format = TextFormat {
                underline: Stroke::new(1.0, format.color),
                ..format.clone()
            };

            if !before.is_empty() { job.append(before, 0.0, format.clone()); }
            job.append(ch_str, 0.0, underline_format);
            if !after.is_empty()  { job.append(after,  0.0, format); }
        }
    }

    job
}

fn char_to_key(c: char) -> Option<Key> {
    match c.to_ascii_lowercase() {
        'a' => Some(Key::A), 'b' => Some(Key::B), 'c' => Some(Key::C),
        'd' => Some(Key::D), 'e' => Some(Key::E), 'f' => Some(Key::F),
        'g' => Some(Key::G), 'h' => Some(Key::H), 'i' => Some(Key::I),
        'j' => Some(Key::J), 'k' => Some(Key::K), 'l' => Some(Key::L),
        'm' => Some(Key::M), 'n' => Some(Key::N), 'o' => Some(Key::O),
        'p' => Some(Key::P), 'q' => Some(Key::Q), 'r' => Some(Key::R),
        's' => Some(Key::S), 't' => Some(Key::T), 'u' => Some(Key::U),
        'v' => Some(Key::V), 'w' => Some(Key::W), 'x' => Some(Key::X),
        'y' => Some(Key::Y), 'z' => Some(Key::Z),
        _ => None,
    }
}
