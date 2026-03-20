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

/// Shape of the directional highlight drawn at the center indicator.
///
/// Variants can be combined: `ArcSlice` draws both an arc and a filled wedge,
/// `ArcSliceCircle` draws all three, etc.
#[derive(PartialEq, Clone, Copy, Debug)]
pub enum PieMenuHighlightShape {
    /// An arc drawn along the indicator radius.
    Arc,
    /// A filled wedge pointing in the hovered direction.
    Slice,
    /// A dot offset from the center in the hovered direction.
    Circle,
    /// Arc + filled wedge.
    ArcSlice,
    /// Arc + dot.
    ArcCircle,
    /// Arc + filled wedge + dot.
    ArcSliceCircle,
    /// Filled wedge + dot.
    SliceCircle,
    /// No highlight drawn.
    None,
}

/// Appearance of the center indicator (background circle + directional highlight).
pub struct PieMenuCenterIndicatorSettings {
    /// Radius of the static background circle. Set to [`f32::NAN`] to disable.
    pub background_radius: SmartFloat<f32>,
    /// Stroke of the background circle border.
    pub background_stroke: Stroke,
    /// Fill colour of the background circle.
    pub background_fill_color: Color32,

    /// Radius of the highlight arc/slice/circle. Set to [`f32::NAN`] to disable.
    pub highlight_radius: SmartFloat<f32>,
    /// Which highlight shape(s) to draw. See [`PieMenuHighlightShape`].
    pub highlight_shape: PieMenuHighlightShape,
    /// Stroke applied to arc and circle components of the highlight.
    pub highlight_stroke: Stroke,
    /// Fill colour applied to slice and circle components of the highlight.
    pub highlight_fill_color: Color32,
    /// Angular width of the arc/slice highlight in radians.
    pub highlight_angle: f32,
    /// Radius of the dot when the highlight shape includes a [`PieMenuHighlightShape::Circle`] component.
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
    /// Whether to show the title. The title string is passed to [`crate::PieMenu::show`].
    pub display: bool,
    /// Background fill of the label box. Use [`Color32::TRANSPARENT`] for no background.
    pub background_color: Color32,
    /// Border of the label box. Set `width` to `0` or `NAN` to disable.
    pub background_stroke: Stroke,
    /// Inner padding around the label text.
    pub padding: Padding,
    /// Text colour.
    pub text_color: Color32,
    /// Font used to render the title.
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

/// Uniform or per-side inner padding in logical pixels.
pub struct Padding {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

/// Input and dismissal behaviour.
pub struct PieMenuInput {
    /// Allow numpad keys 1–9 to activate buttons by direction.
    pub use_numpad_keys: bool,
    /// Allow single-character mnemonic keys to activate buttons.
    /// Matched key events are consumed so they don't reach other handlers.
    pub use_mnemonic_keys: bool,
    /// Maximum gap between two [`crate::PieMenuResponse::QuickTap`]s to produce a
    /// [`crate::PieMenuResponse::DoubleTap`].
    pub double_tap_window: Duration,
    /// Dismiss the menu when Numpad 5 is pressed.
    pub dismiss_on_numpad_5: bool,
    /// Dismiss the menu when the secondary mouse button is clicked.
    pub dismiss_on_secondary_mouse_click: bool,
    /// Dismiss the menu when Escape is pressed.
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
    /// Distance from the menu centre to the inner edge of each button, in logical pixels.
    pub layout_radius: f32,
    /// Controls the layout geometry: `0.0` = circle, `+1.0` = square, `-1.0` = diamond.
    ///
    /// Cardinal buttons (N/S/E/W) are unaffected; only the diagonal positions change.
    pub shape_factor: f32,
    /// Mouse must move at least this far from the open position before the menu
    /// is drawn. Releasing before this distance is reached counts as a [`crate::PieMenuResponse::QuickTap`].
    pub show_threshold: f32,
    /// Minimum gap between the outermost button edge and the screen border.
    /// The whole menu shifts to maintain this margin instead of clipping buttons.
    pub screen_margin: f32,
    /// Mouse must move at least this far from centre before a button is highlighted.
    pub mouse_trigger_threshold: f32,
    /// Keyboard and mouse input settings.
    pub input: PieMenuInput,
    /// Appearance of the center background circle and directional highlight.
    pub center_indicator: PieMenuCenterIndicatorSettings,
    /// Appearance of the title label shown above the center indicator.
    pub label: PieMenuLabelSettings,
    /// Highlight animation settings.
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
