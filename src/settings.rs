use egui::{FontId, Stroke, Color32};
use std::f32::consts::TAU;
use std::f32::NAN;
use std::time::Duration;
use crate::utils::common_utils::SmartFloat;

/// The eight directions a button can occupy in the pie menu.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PieDirection {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
}

/// Shape of the center ring highlight.
#[derive(PartialEq, Clone, Copy, Debug)]
pub enum PieMenuHighlightShape {
    Arc,
    Slice,
    Circle,
    ArcSlice,
    ArcCircle,
    ArcSliceCircle,
    SliceCircle,
    None,
}

/// Appearance of the center indicator (background circle + directional highlight).
pub struct PieMenuCenterIndicatorSettings {
    /// Radius of the static background circle. Set to `NAN` to disable.
    pub background_radius: SmartFloat<f32>,
    pub background_stroke: Stroke,
    pub background_fill_color: Color32,

    /// Radius of the highlight arc/slice/circle.
    pub highlight_radius: SmartFloat<f32>,
    pub highlight_shape: PieMenuHighlightShape,
    pub highlight_stroke: Stroke,
    pub highlight_fill_color: Color32,
    /// Angular width of the arc/slice highlight in radians.
    pub highlight_angle: f32,
    /// Radius of the dot when the highlight shape includes a `Circle` component.
    pub highlight_circle_radius: f32,
}

impl Default for PieMenuCenterIndicatorSettings {
    fn default() -> Self {
        Self {
            background_radius: SmartFloat::new(15.0),
            background_stroke: Stroke::new(5.0, Color32::GRAY),
            background_fill_color: Color32::DARK_GRAY,

            highlight_radius: SmartFloat::new(15.0),
            highlight_shape: PieMenuHighlightShape::Slice,
            highlight_stroke: Stroke::new(5.0, Color32::WHITE),
            highlight_fill_color: Color32::PURPLE,
            highlight_angle: TAU / 8.0,
            highlight_circle_radius: 5.0,
        }
    }
}

/// Appearance of the optional title label shown above the center indicator.
pub struct PieMenuLabelSettings {
    /// Whether to show the title at all.
    pub display: bool,
    pub background_color: Color32,
    pub background_stroke: Stroke,
    pub padding: Padding,
    pub text_color: Color32,
    pub text_font: FontId,
}

impl Default for PieMenuLabelSettings {
    fn default() -> Self {
        Self {
            display: true,
            background_color: Color32::TRANSPARENT,
            background_stroke: Stroke::new(NAN, Color32::GRAY),
            padding: Padding { top: 5.0, right: 5.0, bottom: 5.0, left: 5.0 },
            text_color: Color32::LIGHT_GRAY,
            text_font: FontId::default(),
        }
    }
}

pub struct Padding {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

/// Input / dismissal behaviour.
pub struct PieMenuInput {
    pub use_numpad_keys: bool,
    pub use_mnemonic_keys: bool,
    /// Maximum gap between two quick taps to count as a double tap.
    pub double_tap_window: Duration,
    pub dismiss_on_numpad_5: bool,
    pub dismiss_on_secondary_mouse_click: bool,
    pub dismiss_on_escape_key: bool,
}

impl Default for PieMenuInput {
    fn default() -> Self {
        Self {
            use_numpad_keys: true,
            use_mnemonic_keys: true,
            double_tap_window: Duration::from_millis(400),
            dismiss_on_numpad_5: true,
            dismiss_on_secondary_mouse_click: true,
            dismiss_on_escape_key: true,
        }
    }
}

/// Animation / highlight behaviour.
pub struct PieMenuAnimations {
    /// Snap the highlight to the nearest button direction instead of following the mouse.
    pub center_highlight_snapping: bool,
    /// Whether to draw the center highlight at all.
    pub center_highlight_show: bool,
}

impl Default for PieMenuAnimations {
    fn default() -> Self {
        Self {
            center_highlight_snapping: false,
            center_highlight_show: true,
        }
    }
}

/// Top-level settings for a [`crate::PieMenu`].
pub struct PieMenuSettings {
    /// Distance from the centre at which buttons are placed.
    pub layout_radius: f32,
    /// Shape of the button layout: `0` = circle, `+1` = square, `-1` = diamond.
    pub shape_factor: f32,
    /// Mouse must move at least this far from the open position before the menu
    /// is drawn. Releasing before this distance is reached counts as a [`crate::PieMenuResponse::QuickTap`].
    pub show_threshold: f32,
    /// Minimum gap between the outermost button edge and the screen border.
    /// The whole menu shifts to maintain this margin instead of clipping buttons.
    pub screen_margin: f32,
    /// Mouse must move at least this far from centre before a button is highlighted.
    pub mouse_trigger_threshold: f32,
    pub input: PieMenuInput,
    pub center_indicator: PieMenuCenterIndicatorSettings,
    pub label: PieMenuLabelSettings,
    pub animations: PieMenuAnimations,
}

impl Default for PieMenuSettings {
    fn default() -> Self {
        Self {
            layout_radius: 100.0,
            shape_factor: 0.0,
            show_threshold: 10.0,
            screen_margin: 8.0,
            mouse_trigger_threshold: 12.0,
            input: PieMenuInput::default(),
            center_indicator: PieMenuCenterIndicatorSettings::default(),
            label: PieMenuLabelSettings::default(),
            animations: PieMenuAnimations::default(),
        }
    }
}
