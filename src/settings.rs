use crate::utils::common_utils::SmartFloat;
use egui::{Color32, FontId, Key, PointerButton, Stroke};

use std::f32::consts::TAU;
use std::time::Duration;

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

impl PieDirection {
    /// The direction reflected across the vertical axis (East↔West, NE↔NW, SE↔SW).
    pub fn mirrored(&self) -> PieDirection {
        match self {
            PieDirection::North => PieDirection::North,
            PieDirection::South => PieDirection::South,
            PieDirection::East => PieDirection::West,
            PieDirection::West => PieDirection::East,
            PieDirection::NorthEast => PieDirection::NorthWest,
            PieDirection::NorthWest => PieDirection::NorthEast,
            PieDirection::SouthEast => PieDirection::SouthWest,
            PieDirection::SouthWest => PieDirection::SouthEast,
        }
    }
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
#[derive(Clone, Debug)]
pub struct PieMenuCenterIndicatorSettings {
    /// Radius of the static background circle. Set to [`f32::NAN`] to disable.
    pub background_radius: SmartFloat<f32>,
    /// Stroke of the background circle border.
    pub background_stroke: Stroke,
    /// Fill color of the background circle.
    pub background_fill_color: Color32,

    /// Radius of the highlight arc/slice/circle. Set to [`f32::NAN`] to disable.
    pub highlight_radius: SmartFloat<f32>,
    /// Which highlight shape(s) to draw. See [`PieMenuHighlightShape`].
    pub highlight_shape: PieMenuHighlightShape,
    /// Stroke applied to arc and circle components of the highlight.
    pub highlight_stroke: Stroke,
    /// Fill color applied to slice and circle components of the highlight.
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
#[derive(Clone, Debug)]
pub struct PieMenuLabelSettings {
    /// Whether to show the title. The title string is passed to [`crate::PieMenu::show`].
    pub display: bool,
    /// Background fill of the label box. Use [`Color32::TRANSPARENT`] for no background.
    pub background_color: Color32,
    /// Border of the label box. Set `width` to `0` or `NAN` to disable.
    pub background_stroke: Stroke,
    /// Inner padding around the label text.
    pub padding: Padding,
    /// Text color.
    pub text_color: Color32,
    /// Font used to render the title.
    pub text_font: FontId,
}

impl Default for PieMenuLabelSettings {
    fn default() -> Self {
        Self {
            display: true,
            background_color: Color32::TRANSPARENT,
            background_stroke: Stroke::new(f32::NAN, Color32::GRAY),
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

/// Uniform or per-side inner padding in logical pixels.
#[derive(Clone, Debug)]
pub struct Padding {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

/// An input event that dismisses the pie menu.
#[derive(Clone, Debug, PartialEq)]
pub enum PieMenuDismissInput {
    /// A keyboard key press.
    Key(Key),
    /// A mouse button click.
    PointerButton(PointerButton),
}

/// Controls when the pie menu becomes visible after the trigger is held.
#[derive(Clone, Debug, PartialEq)]
pub enum ShowBehavior {
    /// Show the menu immediately on key hold.
    ///
    /// [`crate::PieMenuResponse::QuickTap`] and [`crate::PieMenuResponse::DoubleTap`]
    /// are never returned in this mode.
    Instant,
    /// Wait until the mouse moves at least `threshold` pixels from the open position.
    ///
    /// Releasing before crossing the threshold returns
    /// [`crate::PieMenuResponse::QuickTap`] or [`crate::PieMenuResponse::DoubleTap`].
    OnMovement { threshold: f32 },
}

/// Input and dismissal behaviour.
#[derive(Clone, Debug)]
pub struct PieMenuInput {
    /// Allow number keys 1–9 (numpad or number row; egui does not distinguish them) to activate buttons by direction.
    pub use_numpad_keys: bool,
    /// Allow single-character mnemonic keys to activate buttons.
    /// Matched key events are consumed so they don't reach other handlers.
    pub use_mnemonic_keys: bool,
    /// Generate mnemonics from button labels for buttons left at
    /// [`crate::Mnemonic::Auto`]. Explicit keys ([`crate::Mnemonic::Key`]) are kept and
    /// reserved; [`crate::Mnemonic::None`] buttons never get one. Off by default.
    pub auto_mnemonics: bool,
    /// Maximum gap between two [`crate::PieMenuResponse::QuickTap`]s to produce a
    /// [`crate::PieMenuResponse::DoubleTap`].
    pub double_tap_window: Duration,
    /// Select the hovered button when the primary mouse button (LMB) is clicked.
    ///
    /// Set to `false` for Blender-style interaction, where selection is triggered
    /// by releasing the key that opened the menu rather than by a separate click.
    /// When `false`, a click on the centre of a sub-pie does not go back either
    /// (Backspace still does), so the menu ignores primary clicks entirely.
    pub select_on_primary_click: bool,
    /// Inputs that dismiss the menu without selecting a button.
    ///
    /// Any [`PieMenuDismissInput::Key`] or [`PieMenuDismissInput::PointerButton`]
    /// in this list will close the menu when triggered. Clear the vec to disable
    /// all dismiss inputs.
    pub dismiss_inputs: Vec<PieMenuDismissInput>,
}

impl Default for PieMenuInput {
    fn default() -> Self {
        Self {
            use_numpad_keys: true,
            use_mnemonic_keys: true,
            auto_mnemonics: false,
            double_tap_window: Duration::from_millis(400),
            select_on_primary_click: true,
            dismiss_inputs: vec![
                PieMenuDismissInput::Key(Key::Escape),
                PieMenuDismissInput::Key(Key::Num5),
                PieMenuDismissInput::PointerButton(PointerButton::Secondary),
            ],
        }
    }
}

/// How the buttons appear when the menu is revealed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RevealAnimation {
    /// No animation: buttons appear instantly.
    None,
    /// Buttons fade in linearly over [`PieMenuAnimations::reveal_duration`]. The default;
    /// with the default 200 ms it matches egui's own area fade.
    Fade,
    /// Buttons slide out to their positions over [`PieMenuAnimations::reveal_duration`].
    /// Where they start is set by [`PieMenuAnimations::slide_start_radius`] and
    /// [`PieMenuAnimations::slide_start_shape_factor`]; the fade while sliding by
    /// [`PieMenuAnimations::slide_fade`].
    SlideOut,
}

/// Small digit hints (the number key that selects each button) drawn beside buttons.
#[derive(Clone, Debug)]
pub struct PieMenuNumberHints {
    /// Have the menu paint the digit *beside* each button. Only has an effect when
    /// [`PieMenuInput::use_numpad_keys`] is on. To put the digit *inside* your button
    /// instead (so the button grows to fit it), leave this off and use
    /// [`crate::PieMenu::number_hint`] in your render closure. `side`, `spacing` and
    /// `digits` apply to both; `font` only to the painted digit. The colour is applied to
    /// the painted digit and passed on in [`crate::NumberHint::color`] for digits you draw.
    pub show: bool,
    /// Digit colour. `None` (default) uses egui's weak text colour: a dimmed version of
    /// the default label colour that follows the light/dark theme. On a hovered button the
    /// digit gets more contrast automatically (brighter in a dark theme, darker in a light one).
    pub color: Option<Color32>,
    /// Digit font.
    pub font: FontId,
    /// Gap between the button's edge and the digit, in logical pixels.
    pub spacing: f32,
    /// Which side of the button the digit is drawn on.
    pub side: HintSide,
    /// Which digits to draw (Western, Arabic-Indic or Persian).
    pub digits: HintDigits,
}

/// Digit shapes used for number hints. The keys pressed are the same in every case;
/// only the drawn digit changes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HintDigits {
    /// 0123456789 (default).
    #[default]
    Western,
    /// ٠١٢٣٤٥٦٧٨٩, used with Arabic.
    ArabicIndic,
    /// ۰۱۲۳۴۵۶۷۸۹ (Extended Arabic-Indic), used with Persian and Urdu.
    Persian,
}

/// Side of a button on which a number hint is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HintSide {
    /// After the label in reading order (default): right of the button for left-to-right
    /// text, left of it for right-to-left text.
    Trailing,
    /// The side facing away from the centre: right for eastern buttons, left for western
    /// ones, right for North/South. Based on the displayed position, so it follows a
    /// mirrored layout.
    Outside,
    /// Always on the left.
    Left,
    /// Always on the right.
    Right,
}

impl Default for PieMenuNumberHints {
    fn default() -> Self {
        Self {
            show: false,
            color: None,
            font: FontId::proportional(11.0),
            spacing: 4.0,
            side: HintSide::Trailing,
            digits: HintDigits::Western,
        }
    }
}

/// Animation / highlight behaviour.
#[derive(Clone, Debug)]
pub struct PieMenuAnimations {
    /// Animation used when the buttons are revealed.
    pub reveal: RevealAnimation,
    /// Duration of the [`RevealAnimation::Fade`] and [`RevealAnimation::SlideOut`] animations.
    pub reveal_duration: Duration,
    /// [`RevealAnimation::SlideOut`]: layout radius (px) the buttons start from.
    /// `0.0` starts them at the centre; values above
    /// [`PieMenuSettings::layout_radius`] make them slide inward.
    pub slide_start_radius: f32,
    /// [`RevealAnimation::SlideOut`]: shape factor the layout starts with, blended to
    /// [`PieMenuSettings::shape_factor`] during the slide (e.g. `-1.0` slides a diamond
    /// out into whatever the final shape is). `None` keeps the final shape throughout.
    pub slide_start_shape_factor: Option<f32>,
    /// [`RevealAnimation::SlideOut`]: fade the buttons in over this duration while they
    /// slide (it may be shorter or longer than the slide). `None` keeps them fully
    /// opaque from the start.
    pub slide_fade: Option<Duration>,
    /// Snap the highlight to the nearest button direction instead of following the mouse.
    pub center_highlight_snapping: bool,
    /// Whether to draw the center highlight at all.
    pub center_highlight_show: bool,
}

impl Default for PieMenuAnimations {
    fn default() -> Self {
        Self {
            reveal: RevealAnimation::Fade,
            reveal_duration: Duration::from_millis(200),
            slide_start_radius: 0.0,
            slide_start_shape_factor: None,
            slide_fade: Some(Duration::from_millis(200)),
            center_highlight_snapping: false,
            center_highlight_show: true,
        }
    }
}

/// Top-level settings for a [`crate::PieMenu`].
#[derive(Clone, Debug)]
pub struct PieMenuSettings {
    /// Distance from the menu centre to the inner edge of each button, in logical pixels.
    pub layout_radius: f32,
    /// Controls the layout geometry: `0.0` = circle, `+1.0` = square, `-1.0` = diamond.
    ///
    /// Cardinal buttons (N/S/E/W) are unaffected; only the diagonal positions change.
    pub shape_factor: f32,
    /// Controls when the menu becomes visible and whether tap responses are possible.
    /// See [`ShowBehavior`].
    pub show_behavior: ShowBehavior,
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
    /// While the menu is visible, swallow pointer input meant for widgets behind it
    /// (a full-screen blocker). Set to `false` for a pinned/persistent menu so the
    /// rest of the UI stays usable. Default `true`.
    pub block_background: bool,
    /// Mirror the layout horizontally (East↔West, diagonals follow), as used by some
    /// right-to-left UIs. Number keys and number hints follow the *displayed* positions,
    /// and [`HintSide::Trailing`] then means "left of the label". Off by default.
    pub mirror_horizontal: bool,
    /// Optional number-key hints next to the buttons.
    pub number_hints: PieMenuNumberHints,
}

impl Default for PieMenuSettings {
    fn default() -> Self {
        Self {
            layout_radius: 100.0,
            shape_factor: 0.0,
            show_behavior: ShowBehavior::OnMovement { threshold: 10.0 },
            screen_margin: 8.0,
            mouse_trigger_threshold: 12.0,
            input: PieMenuInput::default(),
            center_indicator: PieMenuCenterIndicatorSettings::default(),
            label: PieMenuLabelSettings::default(),
            animations: PieMenuAnimations::default(),
            block_background: true,
            mirror_horizontal: false,
            number_hints: PieMenuNumberHints::default(),
        }
    }
}
