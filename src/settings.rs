use egui::{Color32, FontId, Key, Margin, PointerButton, Stroke};

use std::f32::consts::TAU;
use std::time::Duration;

/// The eight directions a button can occupy in the pie menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    /// All eight directions, clockwise from North.
    pub const ALL: [PieDirection; 8] = [
        PieDirection::North,
        PieDirection::NorthEast,
        PieDirection::East,
        PieDirection::SouthEast,
        PieDirection::South,
        PieDirection::SouthWest,
        PieDirection::West,
        PieDirection::NorthWest,
    ];

    /// The direction straight across the centre (North↔South, NE↔SW, …).
    pub fn opposite(&self) -> PieDirection {
        match self {
            PieDirection::North => PieDirection::South,
            PieDirection::NorthEast => PieDirection::SouthWest,
            PieDirection::East => PieDirection::West,
            PieDirection::SouthEast => PieDirection::NorthWest,
            PieDirection::South => PieDirection::North,
            PieDirection::SouthWest => PieDirection::NorthEast,
            PieDirection::West => PieDirection::East,
            PieDirection::NorthWest => PieDirection::SouthEast,
        }
    }

    /// Position in [`PieDirection::ALL`] (clockwise from North).
    fn index(self) -> usize {
        Self::ALL.iter().position(|d| *d == self).unwrap_or(0)
    }

    /// The five directions a fan-out sub-pie opened by a button in this direction may
    /// use, counter-clockwise to clockwise around "straight ahead" (this direction): the
    /// other three face back at the parent pie and are blocked.
    ///
    /// ```
    /// use egui_pie_menu::PieDirection::*;
    /// assert_eq!(East.fan_out_slots(), [North, NorthEast, East, SouthEast, South]);
    /// ```
    pub fn fan_out_slots(self) -> [PieDirection; 5] {
        [6, 7, 0, 1, 2].map(|step| Self::ALL[(self.index() + step) % 8])
    }

    /// Whether `child` is one of the three directions a fan-out sub-pie opened by a
    /// button in this direction cannot use: straight back at the parent
    /// ([`Self::opposite`]) and its two neighbours.
    pub fn blocks_fan_out(self, child: PieDirection) -> bool {
        !self.fan_out_slots().contains(&child)
    }

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

/// Which components of the directional highlight are drawn at the center indicator.
/// Any combination works; none at all draws no highlight.
///
/// ```
/// use egui_pie_menu::HighlightShape;
/// let shape = HighlightShape { arc: true, ..HighlightShape::SLICE }; // arc + wedge
/// assert!(!shape.is_none());
/// ```
#[derive(PartialEq, Eq, Clone, Copy, Debug, Default)]
pub struct HighlightShape {
    /// An arc drawn along the indicator radius.
    pub arc: bool,
    /// A filled wedge pointing in the hovered direction.
    pub slice: bool,
    /// A dot offset from the center in the hovered direction.
    pub circle: bool,
}

impl HighlightShape {
    /// No highlight drawn.
    pub const NONE: Self = Self { arc: false, slice: false, circle: false };
    /// Only the arc.
    pub const ARC: Self = Self { arc: true, ..Self::NONE };
    /// Only the filled wedge (the default).
    pub const SLICE: Self = Self { slice: true, ..Self::NONE };
    /// Only the dot.
    pub const CIRCLE: Self = Self { circle: true, ..Self::NONE };

    /// Whether nothing is drawn.
    pub fn is_none(self) -> bool {
        self == Self::NONE
    }
}

/// Appearance of the center indicator (background circle + directional highlight).
#[derive(Clone, Debug)]
pub struct PieMenuCenterIndicatorSettings {
    /// Radius of the static background circle. `None` draws no circle.
    pub background_radius: Option<f32>,
    /// Stroke of the background circle border.
    pub background_stroke: Stroke,
    /// Fill color of the background circle.
    pub background_fill_color: Color32,

    /// Radius of the highlight arc/slice/circle. `None` draws no highlight.
    pub highlight_radius: Option<f32>,
    /// Which highlight component(s) to draw. See [`HighlightShape`].
    pub highlight_shape: HighlightShape,
    /// Stroke applied to arc and circle components of the highlight.
    pub highlight_stroke: Stroke,
    /// Fill color applied to slice and circle components of the highlight.
    pub highlight_fill_color: Color32,
    /// Angular width of the arc/slice highlight in radians.
    pub highlight_angle: f32,
    /// Radius of the dot when the highlight shape includes [`HighlightShape::circle`].
    pub highlight_circle_radius: f32,
    /// Snap the highlight to the hovered button's direction instead of following the mouse.
    pub highlight_snapping: bool,
}

impl Default for PieMenuCenterIndicatorSettings {
    fn default() -> Self {
        Self {
            background_radius: Some(15.0),
            background_stroke: Stroke::new(5.0, Color32::GRAY),
            background_fill_color: Color32::DARK_GRAY,

            highlight_radius: Some(15.0),
            highlight_shape: HighlightShape::SLICE,
            highlight_stroke: Stroke::new(5.0, Color32::WHITE),
            highlight_fill_color: Color32::PURPLE,
            highlight_angle: TAU / 8.0,
            highlight_circle_radius: 5.0,
            highlight_snapping: false,
        }
    }
}

/// Appearance of the optional title shown above the center indicator.
/// The text itself is [`crate::PieMenu::title`].
#[derive(Clone, Debug)]
pub struct PieMenuTitleSettings {
    /// Whether to show the title.
    pub show: bool,
    /// Background fill of the title box. Use [`Color32::TRANSPARENT`] for no background.
    pub background_color: Color32,
    /// Border of the title box. Set `width` to `0` to disable (the default).
    pub background_stroke: Stroke,
    /// Inner padding around the title text.
    pub padding: Margin,
    /// Text color.
    pub text_color: Color32,
    /// Font used to render the title.
    pub text_font: FontId,
}

impl Default for PieMenuTitleSettings {
    fn default() -> Self {
        Self {
            show: true,
            background_color: Color32::TRANSPARENT,
            background_stroke: Stroke::new(0.0, Color32::GRAY),
            padding: Margin::same(5),
            text_color: Color32::LIGHT_GRAY,
            text_font: FontId::default(),
        }
    }
}

/// An input event that dismisses the pie menu.
#[derive(Clone, Debug, PartialEq)]
pub enum PieMenuDismissInput {
    /// A keyboard key press.
    Key(Key),
    /// A mouse button click.
    PointerButton(PointerButton),
}

/// How a button's sub-pie is shown. The two styles are not interchangeable designs: pick
/// one for the menu with [`PieMenuSettings::sub_pie_style`]; a single button can override
/// it ([`crate::PieButton::with_fan_out`], [`crate::PieButton::with_sub_pie_style`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SubPieStyle {
    /// The sub-pie replaces its parent: one level is on screen at a time, to any depth.
    /// See [`SubPieCenter`], [`crate::PieButton::back`] and
    /// [`PieMenuSettings::sub_pie_hover_delay`].
    #[default]
    Replace,
    /// The sub-pie fans out around the button that opens it while the parent stays
    /// visible (greyed out). It holds up to five buttons: the three directions facing
    /// the parent are blocked ([`PieDirection::fan_out_slots`]), and its buttons cannot
    /// have sub-pies of their own. It opens when the pointer reaches its button
    /// ([`PieMenuSettings::fan_out_delay`]) and closes when the pointer returns to the
    /// parent. While open it has the keyboard: number keys and mnemonics reach only it.
    FanOut,
}

/// Where a sub-pie is centred when it opens (and where its parent reappears on going back).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SubPieCenter {
    /// At the pointer (default), so every direction is equally far away. Going back
    /// re-centres the parent on the pointer too. A pinned menu stays where it is.
    #[default]
    Pointer,
    /// At the same centre as its parent: the sub-pie replaces it in place.
    Parent,
    /// At the button that opened it, so the menu grows outward. Going back returns to the
    /// parent's own centre; a [`crate::PieButton::back`] button in the opposite direction
    /// then sits between the two centres.
    Button,
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
    pub use_number_keys: bool,
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
    /// The dismiss inputs of a pinned menu ([`crate::PieMenu::pin`]), used instead of
    /// [`Self::dismiss_inputs`]. Default: only Escape, so stray clicks and digits
    /// elsewhere in the app never close it.
    pub pinned_dismiss_inputs: Vec<PieMenuDismissInput>,
}

impl Default for PieMenuInput {
    fn default() -> Self {
        Self {
            use_number_keys: true,
            use_mnemonic_keys: true,
            auto_mnemonics: false,
            double_tap_window: Duration::from_millis(400),
            select_on_primary_click: true,
            dismiss_inputs: vec![
                PieMenuDismissInput::Key(Key::Escape),
                PieMenuDismissInput::Key(Key::Num5),
                PieMenuDismissInput::PointerButton(PointerButton::Secondary),
            ],
            pinned_dismiss_inputs: vec![PieMenuDismissInput::Key(Key::Escape)],
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

/// Small digit hints (the number key that selects each button), drawn by your render
/// closure inside the button from [`crate::ButtonView::number_hint`]. Only produced when
/// [`PieMenuInput::use_number_keys`] is on.
#[derive(Clone, Debug)]
pub struct PieMenuNumberHints {
    /// Force the digit colour, whether hovered or not (Blender-style). `None` (default)
    /// uses egui's widget text colours like the labels: `widgets.inactive` normally,
    /// `widgets.hovered` on the hovered button. Independent of
    /// [`PieMenuSettings::text_color`]. Disabled buttons are faded by egui either way.
    pub color: Option<Color32>,
    /// Suggested gap between the label and the digit, in logical pixels (for your closure).
    pub spacing: f32,
    /// Which side of the label the digit goes on.
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

/// Side of the label on which a number hint is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HintSide {
    /// After the label in reading order (default): right of it for left-to-right text,
    /// left of it for right-to-left text.
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
            color: None,
            spacing: 4.0,
            side: HintSide::Trailing,
            digits: HintDigits::Western,
        }
    }
}

/// Reveal animation settings.
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
}

impl Default for PieMenuAnimations {
    fn default() -> Self {
        Self {
            reveal: RevealAnimation::Fade,
            reveal_duration: Duration::from_millis(200),
            slide_start_radius: 0.0,
            slide_start_shape_factor: None,
            slide_fade: Some(Duration::from_millis(200)),
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
    /// Height of the diagonal (intercardinal: NE, SE, SW, NW) buttons, as a fraction of a
    /// North/South button's centre height above/below the East–West row: `0.5` puts them
    /// halfway between N/S and E/W, `1.0` level with N/S; low values bottom out with the
    /// inner corner on the E–W axis. The buttons slide *along* the layout shape (circle,
    /// square or diamond per [`Self::shape_factor`]), keeping their inner corner on it, so
    /// lower diagonals also move outward. Hover picking and the centre highlight follow.
    /// `None` (default): diagonals sit at 45° on the shape.
    pub intercardinal_height: Option<f32>,
    /// Controls when the menu becomes visible and whether tap responses are possible.
    /// See [`ShowBehavior`].
    pub show_behavior: ShowBehavior,
    /// Minimum gap between the outermost button edge and the screen border.
    /// The whole menu shifts to maintain this margin instead of clipping buttons.
    pub screen_margin: f32,
    /// Radius of the dead zone around the centre: the mouse must be farther out than this
    /// before a button is highlighted. Releasing or clicking inside it selects nothing.
    pub dead_zone_radius: f32,
    /// Keyboard and mouse input settings.
    pub input: PieMenuInput,
    /// Appearance of the center background circle and directional highlight.
    pub center_indicator: PieMenuCenterIndicatorSettings,
    /// Appearance of the title shown above the center indicator.
    pub title: PieMenuTitleSettings,
    /// Reveal animation settings.
    pub animations: PieMenuAnimations,
    /// While the menu is visible, swallow pointer input meant for widgets behind it
    /// (a full-screen blocker). A pinned menu ([`crate::PieMenu::pin`]) never blocks,
    /// whatever this is set to. Default `true`.
    pub block_background: bool,
    /// Where a sub-pie is centred when it opens. See [`SubPieCenter`].
    pub sub_pie_center: SubPieCenter,
    /// [`SubPieStyle::Replace`]:
    /// follow a button that leads to another pie once the pointer has rested on it this
    /// long, without a click, key or release: a sub-pie button opens its sub-pie, and a
    /// [`crate::PieButton::back`] button inside a sub-pie returns to the parent. The button
    /// the pointer already points at when a level opens is ignored until the pointer has
    /// left it once, so a resting pointer never bounces between levels.
    /// `None` (default): off.
    pub sub_pie_hover_delay: Option<Duration>,
    /// How sub-pies are shown, for buttons that don't say themselves. See [`SubPieStyle`].
    pub sub_pie_style: SubPieStyle,
    /// [`SubPieStyle::FanOut`]: how long the pointer must rest on the opening button
    /// before its fan opens. Default: zero (opens as the pointer reaches the button).
    pub fan_out_delay: Duration,
    /// [`SubPieStyle::FanOut`]: distance from the opening button's centre to the inner
    /// edge of the fan's buttons. `None` (default): the same as [`Self::layout_radius`].
    pub fan_out_radius: Option<f32>,
    /// Mirror the layout horizontally (East↔West, diagonals follow), as used by some
    /// right-to-left UIs. Number keys and number hints follow the *displayed* positions,
    /// and [`HintSide::Trailing`] then means "left of the label". Off by default.
    pub mirror_horizontal: bool,
    /// Optional number-key hints next to the buttons.
    pub number_hints: PieMenuNumberHints,
    /// Force the label colour of [`crate::ButtonView::label_text`], whether hovered or
    /// not (Blender-style: `Some(Color32::WHITE)`). `None` (default) uses egui's widget
    /// text colours: `widgets.inactive` normally, `widgets.hovered` on the hovered button,
    /// so the app's theme applies. Disabled buttons are faded by egui either way.
    /// Number hints have their own [`PieMenuNumberHints::color`].
    pub text_color: Option<Color32>,
}

impl Default for PieMenuSettings {
    fn default() -> Self {
        Self {
            layout_radius: 100.0,
            shape_factor: 0.0,
            intercardinal_height: None,
            show_behavior: ShowBehavior::OnMovement { threshold: 10.0 },
            screen_margin: 8.0,
            dead_zone_radius: 12.0,
            input: PieMenuInput::default(),
            center_indicator: PieMenuCenterIndicatorSettings::default(),
            title: PieMenuTitleSettings::default(),
            animations: PieMenuAnimations::default(),
            block_background: true,
            sub_pie_center: SubPieCenter::Pointer,
            sub_pie_hover_delay: None,
            sub_pie_style: SubPieStyle::Replace,
            fan_out_delay: Duration::ZERO,
            fan_out_radius: None,
            mirror_horizontal: false,
            number_hints: PieMenuNumberHints::default(),
            text_color: None,
        }
    }
}
