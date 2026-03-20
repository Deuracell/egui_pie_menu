use egui::{Color32, Key, Pos2, Rect, Stroke, Ui, Vec2};
use std::f32::consts::{PI, TAU};
use std::time::Instant;

mod utils;
pub use utils::common_utils::{BoundedVec, SmartFloat};

pub mod settings;
pub use settings::*;

pub mod highlight_shapes;
pub use highlight_shapes::*;

/// A single button passed to the pie menu each frame.
///
/// The pie menu owns rendering; the caller owns the button definitions and passes
/// them to [`PieMenu::show`] every frame.
pub struct PieButton {
    pub label: String,
    pub direction: PieDirection,
    pub color: Color32,
    pub mnemonic: Option<char>,
}

impl PieButton {
    pub fn new(direction: PieDirection, label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            direction,
            color: Color32::from_rgb(80, 80, 80),
            mnemonic: None,
        }
    }

    pub fn with_color(mut self, color: Color32) -> Self {
        self.color = color;
        self
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
    /// No action this frame.
    None,
}

/// Radial (pie) menu widget.
///
/// Owns interaction state but not button definitions. Construct once, then call
/// [`PieMenu::show`] every frame with the buttons relevant to the current context.
pub struct PieMenu {
    open_time: Instant,
    selected_index: Option<usize>,
    pub position: Pos2,
    pub settings: PieMenuSettings,
    release_handled: bool,
}

impl Default for PieMenu {
    fn default() -> Self {
        Self::new()
    }
}

impl PieMenu {
    pub fn new() -> Self {
        Self {
            open_time: Instant::now(),
            selected_index: None,
            position: Pos2::ZERO,
            settings: PieMenuSettings::default(),
            release_handled: false,
        }
    }

    pub fn with_position(mut self, pos: Pos2) -> Self {
        self.position = pos;
        self
    }

    pub fn with_settings(mut self, settings: PieMenuSettings) -> Self {
        self.settings = settings;
        self
    }

    /// Call this when the menu is opened to reset interaction state.
    /// Sets the position and restarts the internal timer used for key-hold detection.
    pub fn open(&mut self, pos: Pos2) {
        self.position = pos;
        self.open_time = Instant::now();
        self.release_handled = false;
        self.selected_index = None;
    }

    /// Converts a [`PieDirection`] to a [`Vec2`] offset, applying squarification
    /// to diagonal directions.
    ///
    /// - `0.0`  → circle: all buttons equidistant (L2, unit vector)
    /// - `+1.0` → square: diagonals pushed out to the corners (L∞ norm)
    /// - `-1.0` → diamond: diagonals pulled in toward the centre (L1 norm)
    fn direction_vec(dir: &PieDirection, squarification: f32) -> Vec2 {
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
        let circle = Vec2::new(angle.cos(), angle.sin()); // L2, magnitude = 1

        match dir {
            PieDirection::NorthEast
            | PieDirection::SouthEast
            | PieDirection::SouthWest
            | PieDirection::NorthWest => {
                if squarification > 0.0 {
                    // Lerp toward L∞: divide by max component → (1, 1) for NE
                    let l_inf = circle / circle.x.abs().max(circle.y.abs());
                    circle * (1.0 - squarification) + l_inf * squarification
                } else if squarification < 0.0 {
                    // Lerp toward L1: divide by sum of components → (0.5, 0.5) for NE
                    let l1 = circle / (circle.x.abs() + circle.y.abs());
                    let t = -squarification;
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

    fn button_width(painter: &egui::Painter, label: &str) -> f32 {
        let side_margin = 5.0;
        let galley = painter.layout_no_wrap(
            label.to_string(),
            egui::FontId::default(),
            Color32::WHITE,
        );
        galley.size().x + side_margin * 2.0
    }

    fn draw_button_label(painter: &egui::Painter, pos: Pos2, label: &str, mnemonic: Option<char>) {
        let galley = painter.layout_no_wrap(
            label.to_string(),
            egui::FontId::default(),
            Color32::WHITE,
        );

        let origin = pos - galley.size() / 2.0;
        painter.galley(origin, galley.clone(), Color32::WHITE);

        if let Some(m) = mnemonic {
            'outer: for row in &galley.rows {
                for glyph in &row.glyphs {
                    if glyph.chr == m {
                        let x0 = origin.x + glyph.pos.x;
                        let y  = origin.y + row.rect.max.y + 1.0;
                        painter.line_segment(
                            [Pos2::new(x0, y), Pos2::new(x0 + glyph.size().x, y)],
                            Stroke::new(1.0, Color32::WHITE),
                        );
                        break 'outer;
                    }
                }
            }
        }
    }

    /// Show the pie menu for this frame.
    ///
    /// Pass the buttons relevant for the current context; the menu does not store them.
    /// `key_down` should be `true` while the hotkey that opened the menu is held.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        buttons: &[PieButton],
        current_mouse_pos: Option<Pos2>,
        key_down: bool,
        title: Option<&str>,
    ) -> PieMenuResponse {
        // Early exit checks
        if ui.input(|i| {
            (self.settings.input.dismiss_on_numpad_5 && i.key_pressed(Key::Num5))
                || (self.settings.input.dismiss_on_escape_key && i.key_pressed(Key::Escape))
                || (self.settings.input.dismiss_on_secondary_mouse_click
                    && i.pointer.secondary_clicked())
        }) {
            return PieMenuResponse::Dismissed;
        }

        let center = self.position;
        let squarification = self.settings.layout_squarification;

        // Top-level painter: draws above all panels, unclipped.
        let painter = ui.ctx().layer_painter(
            egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("pie_menu")),
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
                if ui.input(|i| i.key_pressed(Self::direction_numpad(&button.direction))) {
                    return PieMenuResponse::Selected(idx);
                }
            }
        }

        // Mnemonic key selection
        if self.settings.input.use_mnemonic_keys {
            for (idx, button) in buttons.iter().enumerate() {
                if let Some(key) = button.mnemonic.and_then(char_to_key) {
                    if ui.input(|i| i.key_pressed(key)) {
                        return PieMenuResponse::Selected(idx);
                    }
                }
            }
        }

        // Primary click selection
        if ui.input(|i| i.pointer.primary_clicked()) {
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
                    let dir_vec = Self::direction_vec(&button.direction, squarification);
                    let button_angle = dir_vec.y.atan2(dir_vec.x);
                    let diff = ((angle - button_angle + PI) % TAU - PI).abs();
                    (idx, diff)
                })
                .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                .map(|(idx, _)| idx)
        });

        // Center highlight — progress driven by mouse distance from center.
        // 0.0 at the threshold edge, 1.0 at layout_radius (where buttons sit).
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
                        let v = Self::direction_vec(&b.direction, squarification);
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

            let stroke_color = self
                .settings
                .center_indicator
                .highlight_stroke
                .color
                .gamma_multiply(progress);
            let fill_color = self
                .settings
                .center_indicator
                .highlight_fill_color
                .gamma_multiply(progress);
            let colored_stroke =
                Stroke::new(self.settings.center_indicator.highlight_stroke.width, stroke_color);
            let highlight_radius = self.settings.center_indicator.highlight_radius.get();

            let shape = self.settings.center_indicator.highlight_shape;

            let needs_arc = matches!(
                shape,
                PieMenuHighlightShape::Arc
                    | PieMenuHighlightShape::ArcSlice
                    | PieMenuHighlightShape::ArcCircle
                    | PieMenuHighlightShape::ArcSliceCircle
            );
            let needs_slice = matches!(
                shape,
                PieMenuHighlightShape::Slice
                    | PieMenuHighlightShape::SliceCircle
                    | PieMenuHighlightShape::ArcSlice
                    | PieMenuHighlightShape::ArcSliceCircle
            );
            let needs_circle = matches!(
                shape,
                PieMenuHighlightShape::Circle
                    | PieMenuHighlightShape::ArcCircle
                    | PieMenuHighlightShape::SliceCircle
                    | PieMenuHighlightShape::ArcSliceCircle
            );

            let arc_arg = needs_arc.then(|| ArcValues {
                angle_range: angle_range.clone(),
                center,
                radius: highlight_radius,
                resolution: 10.0,
                stroke: colored_stroke,
            });

            let slice_arg = needs_slice.then(|| {
                // For ArcSlice/ArcSliceCircle the arc is drawn separately, so no arc inside slice.
                let inner_arc = matches!(
                    shape,
                    PieMenuHighlightShape::Slice | PieMenuHighlightShape::SliceCircle
                )
                .then(|| ArcValues {
                    angle_range: angle_range.clone(),
                    center,
                    radius: highlight_radius,
                    resolution: 10.0,
                    stroke: colored_stroke,
                });
                SliceValues {
                    arc_values: inner_arc,
                    stroke: None,
                    fill_color,
                }
            });

            let circle_arg = needs_circle.then(|| CircleValues {
                offset_angle: base_angle,
                offset_radius: highlight_radius,
                offset_center: center,
                circle_radius: self.settings.center_indicator.highlight_circle_radius,
                stroke: colored_stroke,
                fill_color,
            });

            painter.highlight_shape(shape, arc_arg, slice_arg, circle_arg);
        }

        // Draw menu title above the center indicator
        if self.settings.label.display {
            if let Some(label) = title {
                let pad = &self.settings.label.padding;
                let font = self.settings.label.text_font.clone();

                // Measure text to size the background rect
                let galley = painter.layout_no_wrap(
                    label.to_string(),
                    font.clone(),
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

                let text_pos = bg_rect.min + Vec2::new(pad.left, pad.top);
                painter.galley(text_pos, galley, self.settings.label.text_color);
            }
        }

        // Draw buttons
        for (idx, button) in buttons.iter().enumerate() {
            let dir_vec = Self::direction_vec(&button.direction, squarification);
            let button_pos = center + dir_vec * self.settings.layout_radius;
            let button_rect = Rect::from_center_size(
                button_pos,
                Vec2::new(Self::button_width(&painter, &button.label), 30.0),
            );

            let color = if self.selected_index == Some(idx) {
                button.color.linear_multiply(0.5)
            } else {
                button.color
            };

            painter.rect_filled(button_rect, 5.0, color);
            Self::draw_button_label(&painter, button_pos, &button.label, button.mnemonic);
        }

        // Key-hold release: select hovered button or dismiss if center was held too long
        if !key_down && !self.release_handled {
            if let Some(mouse_pos) = current_mouse_pos {
                if (mouse_pos - center).length() <= self.settings.mouse_trigger_threshold {
                    if self.open_time.elapsed() > self.settings.input.key_timeout {
                        return PieMenuResponse::Dismissed;
                    }
                } else if let Some(idx) = self.selected_index {
                    return PieMenuResponse::Selected(idx);
                }
            }
            self.release_handled = true;
        }

        PieMenuResponse::None
    }
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
