use egui::{FontId, Stroke, Color32, Key};
use std::f32::consts::TAU;
use std::f32::NAN;
//use std::{array, default,option};
use std::time::Duration;
use crate::utils::common_utils::{BoundedVec, SmartFloat};
use std::collections::HashMap;
use std::fmt::Debug;
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
#[derive(Debug, Clone)]
pub enum PieMenuButtonContent {
    Label,
    Icon,
    Numpad,
}


#[derive(Debug, Clone)]
pub struct PieMenuButtonLayout {
    pub overrides: HashMap<PieDirection, BoundedVec<PieMenuButtonContent>>,
    pub default: BoundedVec<PieMenuButtonContent>,
}

impl PieMenuButtonLayout {
    /// Get the content for a given direction, using defaults if not overridden
    pub fn get_content(&self, direction: PieDirection) -> &[PieMenuButtonContent] {
        self.overrides.get(&direction).unwrap_or(&self.default).get()
    }

    pub fn set_override(&mut self, direction: PieDirection, content: BoundedVec<PieMenuButtonContent>) {
        self.overrides.insert(direction, content);
    }

    pub fn remove_override(&mut self, direction: PieDirection) {
        self.overrides.remove(&direction);
    }
}

impl Default for PieMenuButtonLayout {
    fn default() -> Self {
        // Default layout: Icon → Label → Numpad
        let default_layout = BoundedVec::new(
            vec![
                PieMenuButtonContent::Icon,
                PieMenuButtonContent::Label,
                PieMenuButtonContent::Numpad,
            ],
            3, // Max size enforced
        )
        .expect("Default layout should never be empty");

        Self {
            overrides: HashMap::new(),
            default: default_layout,
        }
    }
}

/// ### Highlight Shape
///
/// Defines the shape of the center ring highlight.
///
/// - `Arc` - A curved stroke highlighting part of the ring.
/// - `Slice` - A pie-slice-shaped highlight.
/// - `Circle` - A full circular highlight.
/// - `ArcSlice` - A combination of `Arc` and `Slice`.
/// - `ArcCircle` - A combination of `Arc` and `Circle`.
/// - `ArcSliceCircle` - A combination of `Arc`, `Slice`, and `Circle`.
/// - `SliceCircle` - A combination of `Slice` and `Circle`.
/// - `None` - No highlight.
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
/// ### Center Indicator Settings
///
/// Defines the appearance and behavior of the center ring indicator.
///
/// #### **Background Settings**
/// - `background`: Whether to show the background.
///     - `background_radius`: The radius of the indicator background.
///     - `background_stroke: The thickness of the indicator background stroke.
///     - `background_stroke: The color of the indicator background stroke.
///     - `background_fill_color`: The color of the indicator background fill.
///
/// #### **Highlight Settings**
/// - `highlight`: Whether to show the highlight.
///     - `highlight_shape`: The shape of the highlight. See [`PieMenuHighlightShape`].
///     - `highlight_radius`: The radius of the highlight.
///         - If `highlight_shape` is `Arc` or `Slice`, this sets the stroke radius.
///         - If `highlight_shape` is `Circle`, this sets the radius of the highlight circle.
///     - `highlight_stroke: The color of the highlight stroke.
pub struct PieMenuCenterIndicatorSettings {
    pub background_radius: SmartFloat<f32>,
    pub background_stroke: Stroke,
    pub background_fill_color: Color32,

    pub highlight_radius: SmartFloat<f32>,
    pub highlight_shape: PieMenuHighlightShape,
    pub highlight_stroke: Stroke,
    pub highlight_fill_color: Color32,
    pub highlight_gradient: bool,
    pub highlight_angle: f32,
    /// Radius of the dot/circle when the highlight shape includes a `Circle` component.
    pub highlight_circle_radius: f32,
}

impl Default for PieMenuCenterIndicatorSettings {
    fn default() -> Self {
        PieMenuCenterIndicatorSettings {
            background_radius: SmartFloat::new(15.0),
            background_stroke: Stroke::new(5.0, Color32::GRAY),
            background_fill_color: Color32::DARK_GRAY,

            highlight_radius: SmartFloat::new(15.0),
            highlight_shape: PieMenuHighlightShape::Slice,
            highlight_stroke: Stroke::new(5.0, Color32::WHITE),            
            highlight_fill_color: Color32::PURPLE,
            highlight_gradient: false,
            highlight_angle: TAU / 8.0, // 45 degrees
            highlight_circle_radius: 5.0,
        }
    }
}


/// ### Label Settings
///
/// Defines the appearance of text labels inside the menu.
///
/// - `background_color`: The background color of the label.
/// - `text_color`: The color of the label text.
/// - `text_font`: The font and size used for label text.
pub struct PieMenuLabelSettings {
    pub display: bool,
    pub background_color: Color32,
    pub background_stroke: Stroke,
    pub padding: Padding,
    pub text_color: Color32,
    pub text_font: FontId,
}

impl Default for PieMenuLabelSettings {
    fn default() -> Self {
        PieMenuLabelSettings {
            display: true,
            background_color: Color32::TRANSPARENT,
            background_stroke: Stroke::new(NAN, Color32::GRAY),
            padding: Padding {
                top: 5.0,
                right: 5.0,
                bottom: 5.0,
                left: 5.0,
            },
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

/// ### Button Settings
///
/// Defines the visual properties of the menu buttons.
///
/// - `true_background_color`: The background color when a button is selected.
/// - `false_background_color`: The background color when a button is not selected.
/// - `true_text_color`: The text color when a button is selected.
/// - `false_text_color`: The text color when a button is not selected.
/// - `text_font`: The font and size used for button text.
pub struct PieMenuButtonSettings {
    pub background_corner_radius: SmartFloat<f32>,
    pub background_stroke: Stroke,
    pub padding: Padding,
    pub element_spacing: f32,
    pub content_layout: PieMenuButtonLayout,
    pub true_background_color: Color32,
    pub false_background_color: Color32,
    pub true_text_color: Color32,
    pub false_text_color: Color32,
    pub text_font: FontId,
}

impl Default for PieMenuButtonSettings {
    fn default() -> Self {
        Self {
            background_corner_radius: SmartFloat::new(10.0),
            background_stroke: Stroke::new(1.0, Color32::BLACK),
            padding: Padding {
                top: 5.0,
                right: 5.0,
                bottom: 5.0,
                left: 5.0,
            },
            element_spacing: 0.5,
            content_layout: PieMenuButtonLayout::default(),
            true_background_color: Color32::from_rgb(100, 100, 100),
            false_background_color: Color32::from_rgb(200, 200, 200),
            true_text_color: Color32::WHITE,
            false_text_color: Color32::BLACK,
            text_font: FontId::default(),
        }
    }
}


pub struct PieMenuInput {
    pub use_numpad_keys: bool,
    pub use_mnemonic_keys: bool,
    pub dismiss_after_shortcut_hold: bool,
    pub key_timeout: Duration,
    pub dismiss_on_numpad_5: bool,
    pub dismiss_on_secondary_mouse_click: bool,
    pub dismiss_on_center_click: bool,
    pub dismiss_on_escape_key: bool,
    pub dismiss_custom_keys: Vec<Key>,
    
}

impl Default for PieMenuInput {
    fn default() -> Self {
        Self {
            use_numpad_keys: true,
            use_mnemonic_keys: true,
            dismiss_after_shortcut_hold: true,
            key_timeout: Duration::from_millis(100),
            dismiss_on_numpad_5: true,
            dismiss_on_secondary_mouse_click: true,
            dismiss_on_center_click: true,
            dismiss_on_escape_key: true,
            dismiss_custom_keys: Vec::new(),
        }
    }
    
}



/// ### Pie Menu Animations
///
/// Controls specific animation aspects of the pie menu.
/// - `open_animation`: If `true`, the menu opens with an animation.
/// - `open_duration`: The time the opening animation takes.
/// 
/// - `center_highlight_snapping`: If `true`, the center highlight snaps to the nearest cardinal angle.
/// - `center_highlight_show`: If `true`, the center highlight is shown.
///     - `center_highlight_duration`: The duration of the center highlight animations.
///         - `center_highlight_grow`: If `true`, the center highlight grows from zero to full size.
///         - `center_highlight_alpha`: Fade in/out the center highlight.
pub struct PieMenuAnimations {
    pub open_animation: bool,
    pub open_duration: Duration,
    
    pub center_highlight_snapping: bool,
    pub center_highlight_show: bool,
    pub center_highlight_duration: Duration,
    pub center_highlight_grow: bool,
    pub center_highlight_alpha: bool,
}

impl Default for PieMenuAnimations {
    fn default() -> Self {
        Self {
            open_animation: true,
            open_duration: Duration::from_millis(6),

            center_highlight_snapping: false,
            center_highlight_show: true,
            center_highlight_duration: Duration::from_millis(5000),
            center_highlight_grow: true,
            center_highlight_alpha: true,
        }
    }    
}



/// ### Pie Menu Settings
///
/// This struct defines the primary settings for a pie menu, including:
/// - General behavior settings.
/// - Visual settings for labels, buttons, and the center indicator.
/// - Animation settings for interactions.
pub struct PieMenuSettings {

    pub layout_radius: f32,
    pub layout_squarification: f32,
    pub mouse_trigger_threshold: f32,
    pub input: PieMenuInput,
    pub center_indicator: PieMenuCenterIndicatorSettings,
    pub label: PieMenuLabelSettings,
    pub button: PieMenuButtonSettings,
    pub animations: PieMenuAnimations,

}

impl Default for PieMenuSettings {
    fn default() -> Self {
        Self {
            layout_radius: 100.0,
            layout_squarification: 0.0,
            mouse_trigger_threshold: 12.0,
            input: PieMenuInput::default(),
            center_indicator: PieMenuCenterIndicatorSettings::default(),
            label: PieMenuLabelSettings::default(),
            button: PieMenuButtonSettings::default(),
            animations: PieMenuAnimations::default(),
        }
    }
}
