//! The fields of [`PieMenuSettings`] as a list ([`SettingsField`]): their paths
//! (`layout.radius`), labels, sections ([`SettingsGroup`]), which apply to given settings,
//! and which can take a value from the program ([`SettingsField::is_bindable`]). The TOML
//! files, the bindings and the designer's editors all name settings this way.
use crate::settings::*;

/// One field of [`PieMenuSettings`] (nested fields count as one each: the paths are
/// `layout.radius`, `sub_pies.fan_out.delay`, ...). Every field of the settings is exactly one
/// variant of this enum, listed in [`SettingsField::ALL`] and in exactly one
/// [`SettingsGroup`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SettingsField {
    /// `show_behavior`
    ShowBehavior,
    /// `input.double_tap_window`
    DoubleTapWindow,
    /// `input.select_on_primary_click`
    SelectOnPrimaryClick,
    /// `input.dismiss_inputs`
    DismissInputs,
    /// `input.pinned_dismiss_inputs`
    PinnedDismissInputs,
    /// `block_background`
    BlockBackground,
    /// `input.use_number_keys`
    UseNumberKeys,
    /// `input.use_mnemonic_keys`
    UseMnemonicKeys,
    /// `input.auto_mnemonics`
    AutoMnemonics,
    /// `number_hints.color`
    HintColor,
    /// `number_hints.spacing`
    HintSpacing,
    /// `number_hints.side`
    HintSide,
    /// `number_hints.digits`
    HintDigits,
    /// `layout.radius`
    LayoutRadius,
    /// `layout.shape_factor`
    LayoutShapeFactor,
    /// `layout.intercardinal_height`
    LayoutIntercardinalHeight,
    /// `layout.dead_zone_radius`
    LayoutDeadZoneRadius,
    /// `layout.screen_margin`
    LayoutScreenMargin,
    /// `center_indicator.background_radius`
    CenterBackgroundRadius,
    /// `center_indicator.background_stroke`
    CenterBackgroundStroke,
    /// `center_indicator.background_fill_color`
    CenterBackgroundFillColor,
    /// `center_indicator.highlight_radius`
    CenterHighlightRadius,
    /// `center_indicator.highlight_shape`
    CenterHighlightShape,
    /// `center_indicator.highlight_stroke`
    CenterHighlightStroke,
    /// `center_indicator.highlight_fill_color`
    CenterHighlightFillColor,
    /// `center_indicator.highlight_angle`
    CenterHighlightAngle,
    /// `center_indicator.highlight_circle_radius`
    CenterHighlightCircleRadius,
    /// `center_indicator.highlight_snapping`
    CenterHighlightSnapping,
    /// `title.show`
    TitleShow,
    /// `title.background_color`
    TitleBackgroundColor,
    /// `title.background_stroke`
    TitleBackgroundStroke,
    /// `title.padding`
    TitlePadding,
    /// `title.text_color`
    TitleTextColor,
    /// `title.text_font`
    TitleTextFont,
    /// `text_color`
    TextColor,
    /// `animations.reveal`
    Reveal,
    /// `animations.reveal_duration`
    RevealDuration,
    /// `animations.slide_start_radius`
    SlideStartRadius,
    /// `animations.slide_start_shape_factor`
    SlideStartShapeFactor,
    /// `animations.slide_fade`
    SlideFade,
    /// `animations.hover_emphasis`
    HoverEmphasis,
    /// `animations.sub_pie_hover_emphasis`
    SubPieHoverEmphasis,
    /// `animations.hover_duration`
    HoverDuration,
    /// `sub_pies.style`
    SubPieStyle,
    /// `sub_pies.center`
    SubPieCenter,
    /// `sub_pies.hover_delay`
    SubPieHoverDelay,
    /// `sub_pies.fan_out.delay`
    FanOutDelay,
    /// `sub_pies.fan_out.radius`
    FanOutRadius,
    /// `sub_pies.fan_out.opener_action`
    FanOutOpenerAction,
    /// `mirror_horizontal`
    MirrorHorizontal,
}

impl SettingsField {
    /// Every field, each exactly once.
    pub const ALL: [SettingsField; 50] = [
        SettingsField::ShowBehavior,
        SettingsField::DoubleTapWindow,
        SettingsField::SelectOnPrimaryClick,
        SettingsField::DismissInputs,
        SettingsField::PinnedDismissInputs,
        SettingsField::BlockBackground,
        SettingsField::UseNumberKeys,
        SettingsField::UseMnemonicKeys,
        SettingsField::AutoMnemonics,
        SettingsField::HintColor,
        SettingsField::HintSpacing,
        SettingsField::HintSide,
        SettingsField::HintDigits,
        SettingsField::LayoutRadius,
        SettingsField::LayoutShapeFactor,
        SettingsField::LayoutIntercardinalHeight,
        SettingsField::LayoutDeadZoneRadius,
        SettingsField::LayoutScreenMargin,
        SettingsField::CenterBackgroundRadius,
        SettingsField::CenterBackgroundStroke,
        SettingsField::CenterBackgroundFillColor,
        SettingsField::CenterHighlightRadius,
        SettingsField::CenterHighlightShape,
        SettingsField::CenterHighlightStroke,
        SettingsField::CenterHighlightFillColor,
        SettingsField::CenterHighlightAngle,
        SettingsField::CenterHighlightCircleRadius,
        SettingsField::CenterHighlightSnapping,
        SettingsField::TitleShow,
        SettingsField::TitleBackgroundColor,
        SettingsField::TitleBackgroundStroke,
        SettingsField::TitlePadding,
        SettingsField::TitleTextColor,
        SettingsField::TitleTextFont,
        SettingsField::TextColor,
        SettingsField::Reveal,
        SettingsField::RevealDuration,
        SettingsField::SlideStartRadius,
        SettingsField::SlideStartShapeFactor,
        SettingsField::SlideFade,
        SettingsField::HoverEmphasis,
        SettingsField::SubPieHoverEmphasis,
        SettingsField::HoverDuration,
        SettingsField::SubPieStyle,
        SettingsField::SubPieCenter,
        SettingsField::SubPieHoverDelay,
        SettingsField::FanOutDelay,
        SettingsField::FanOutRadius,
        SettingsField::FanOutOpenerAction,
        SettingsField::MirrorHorizontal,
    ];

    /// The path of the field in the settings (`layout.radius`), as in the generated code.
    pub fn path(self) -> &'static str {
        use SettingsField as F;
        match self {
            F::ShowBehavior => "show_behavior",
            F::DoubleTapWindow => "input.double_tap_window",
            F::SelectOnPrimaryClick => "input.select_on_primary_click",
            F::DismissInputs => "input.dismiss_inputs",
            F::PinnedDismissInputs => "input.pinned_dismiss_inputs",
            F::BlockBackground => "block_background",
            F::UseNumberKeys => "input.use_number_keys",
            F::UseMnemonicKeys => "input.use_mnemonic_keys",
            F::AutoMnemonics => "input.auto_mnemonics",
            F::HintColor => "number_hints.color",
            F::HintSpacing => "number_hints.spacing",
            F::HintSide => "number_hints.side",
            F::HintDigits => "number_hints.digits",
            F::LayoutRadius => "layout.radius",
            F::LayoutShapeFactor => "layout.shape_factor",
            F::LayoutIntercardinalHeight => "layout.intercardinal_height",
            F::LayoutDeadZoneRadius => "layout.dead_zone_radius",
            F::LayoutScreenMargin => "layout.screen_margin",
            F::CenterBackgroundRadius => "center_indicator.background_radius",
            F::CenterBackgroundStroke => "center_indicator.background_stroke",
            F::CenterBackgroundFillColor => "center_indicator.background_fill_color",
            F::CenterHighlightRadius => "center_indicator.highlight_radius",
            F::CenterHighlightShape => "center_indicator.highlight_shape",
            F::CenterHighlightStroke => "center_indicator.highlight_stroke",
            F::CenterHighlightFillColor => "center_indicator.highlight_fill_color",
            F::CenterHighlightAngle => "center_indicator.highlight_angle",
            F::CenterHighlightCircleRadius => "center_indicator.highlight_circle_radius",
            F::CenterHighlightSnapping => "center_indicator.highlight_snapping",
            F::TitleShow => "title.show",
            F::TitleBackgroundColor => "title.background_color",
            F::TitleBackgroundStroke => "title.background_stroke",
            F::TitlePadding => "title.padding",
            F::TitleTextColor => "title.text_color",
            F::TitleTextFont => "title.text_font",
            F::TextColor => "text_color",
            F::Reveal => "animations.reveal",
            F::RevealDuration => "animations.reveal_duration",
            F::SlideStartRadius => "animations.slide_start_radius",
            F::SlideStartShapeFactor => "animations.slide_start_shape_factor",
            F::SlideFade => "animations.slide_fade",
            F::HoverEmphasis => "animations.hover_emphasis",
            F::SubPieHoverEmphasis => "animations.sub_pie_hover_emphasis",
            F::HoverDuration => "animations.hover_duration",
            F::SubPieStyle => "sub_pies.style",
            F::SubPieCenter => "sub_pies.center",
            F::SubPieHoverDelay => "sub_pies.hover_delay",
            F::FanOutDelay => "sub_pies.fan_out.delay",
            F::FanOutRadius => "sub_pies.fan_out.radius",
            F::FanOutOpenerAction => "sub_pies.fan_out.opener_action",
            F::MirrorHorizontal => "mirror_horizontal",
        }
    }

    /// The group the field is in.
    pub fn group(self) -> SettingsGroup {
        SettingsGroup::ALL.into_iter().find(|g| g.fields().contains(&self)).unwrap_or(SettingsGroup::OpeningClosing)
    }

    /// Whether `group_editor` shows the field for these settings: the double-tap window only
    /// applies to [`ShowBehavior::OnMovement`], the slide fields to
    /// [`RevealAnimation::SlideOut`], and the number hint fields while number keys are on.
    /// `field_editor` draws any field regardless.
    pub fn is_relevant(self, s: &PieMenuSettings) -> bool {
        use SettingsField as F;
        match self {
            F::DoubleTapWindow => matches!(s.show_behavior, ShowBehavior::OnMovement { .. }),
            F::SlideStartRadius | F::SlideStartShapeFactor | F::SlideFade => s.animations.reveal == RevealAnimation::SlideOut,
            F::HintColor | F::HintSpacing | F::HintSide | F::HintDigits => s.input.use_number_keys,
            _ => true,
        }
    }
}

/// A human-readable name for the field, for labels and search ("Ring radius").
pub fn field_label(field: SettingsField) -> &'static str {
    use SettingsField as F;
    match field {
        F::ShowBehavior => "Show menu",
        F::DoubleTapWindow => "Double-tap window",
        F::SelectOnPrimaryClick => "Select on primary click",
        F::DismissInputs => "Dismiss inputs",
        F::PinnedDismissInputs => "Dismiss inputs (pinned menu)",
        F::BlockBackground => "Block the background",
        F::UseNumberKeys => "Number keys",
        F::UseMnemonicKeys => "Mnemonic keys",
        F::AutoMnemonics => "Automatic mnemonics",
        F::HintColor => "Number hint colour",
        F::HintSpacing => "Number hint spacing",
        F::HintSide => "Number hint side",
        F::HintDigits => "Number hint digits",
        F::LayoutRadius => "Ring radius",
        F::LayoutShapeFactor => "Ring shape",
        F::LayoutIntercardinalHeight => "Diagonal button height",
        F::LayoutDeadZoneRadius => "Dead zone radius",
        F::LayoutScreenMargin => "Screen margin",
        F::CenterBackgroundRadius => "Centre circle radius",
        F::CenterBackgroundStroke => "Centre circle border",
        F::CenterBackgroundFillColor => "Centre circle fill",
        F::CenterHighlightRadius => "Highlight radius",
        F::CenterHighlightShape => "Highlight shape",
        F::CenterHighlightStroke => "Highlight stroke",
        F::CenterHighlightFillColor => "Highlight fill",
        F::CenterHighlightAngle => "Highlight angle",
        F::CenterHighlightCircleRadius => "Highlight dot radius",
        F::CenterHighlightSnapping => "Highlight snaps to button",
        F::TitleShow => "Show title",
        F::TitleBackgroundColor => "Title background",
        F::TitleBackgroundStroke => "Title border",
        F::TitlePadding => "Title padding",
        F::TitleTextColor => "Title text colour",
        F::TitleTextFont => "Title font",
        F::TextColor => "Button text colour",
        F::Reveal => "Reveal animation",
        F::RevealDuration => "Reveal duration",
        F::SlideStartRadius => "Slide start radius",
        F::SlideStartShapeFactor => "Slide start shape",
        F::SlideFade => "Slide fade",
        F::HoverEmphasis => "Hover emphasis (root pie)",
        F::SubPieHoverEmphasis => "Hover emphasis (sub-pies, fans)",
        F::HoverDuration => "Hover easing duration",
        F::SubPieStyle => "Sub-pie style",
        F::SubPieCenter => "Sub-pie centre",
        F::SubPieHoverDelay => "Sub-pie hover delay",
        F::FanOutDelay => "Fan-out delay",
        F::FanOutRadius => "Fan-out radius",
        F::FanOutOpenerAction => "Fan-out opener action",
        F::MirrorHorizontal => "Mirror horizontally (right-to-left)",
    }
}

/// The sections `settings_editor` is divided into. Every [`SettingsField`] is in exactly one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SettingsGroup {
    /// When the menu appears and what closes it.
    OpeningClosing,
    /// Number keys, mnemonics and number hints.
    KeysAndHints,
    /// Geometry of the ring.
    Ring,
    /// The centre circle and the directional highlight.
    CenterIndicator,
    /// The title above the centre.
    Title,
    /// How button labels look (their text colour).
    ButtonLook,
    /// Reveal and hover animations.
    Animation,
    /// Sub-pies and fan-outs.
    SubPies,
    /// Right-to-left layout.
    Language,
}

impl SettingsGroup {
    /// Every group, in the order of `settings_editor`.
    pub const ALL: [SettingsGroup; 9] = [
        SettingsGroup::OpeningClosing,
        SettingsGroup::KeysAndHints,
        SettingsGroup::Ring,
        SettingsGroup::CenterIndicator,
        SettingsGroup::Title,
        SettingsGroup::ButtonLook,
        SettingsGroup::Animation,
        SettingsGroup::SubPies,
        SettingsGroup::Language,
    ];

    /// The header text of the section.
    pub fn title(self) -> &'static str {
        match self {
            SettingsGroup::OpeningClosing => "Opening and closing",
            SettingsGroup::KeysAndHints => "Keys and hints",
            SettingsGroup::Ring => "Ring",
            SettingsGroup::CenterIndicator => "Centre indicator",
            SettingsGroup::Title => "Title",
            SettingsGroup::ButtonLook => "Button look",
            SettingsGroup::Animation => "Animation",
            SettingsGroup::SubPies => "Sub-pies",
            SettingsGroup::Language => "Language",
        }
    }

    /// All fields of the section, whether or not they currently apply.
    pub fn fields(self) -> &'static [SettingsField] {
        use SettingsField as F;
        match self {
            SettingsGroup::OpeningClosing => &[
                F::ShowBehavior,
                F::DoubleTapWindow,
                F::SelectOnPrimaryClick,
                F::DismissInputs,
                F::PinnedDismissInputs,
                F::BlockBackground,
            ],
            SettingsGroup::KeysAndHints => {
                &[F::UseNumberKeys, F::UseMnemonicKeys, F::AutoMnemonics, F::HintColor, F::HintSpacing, F::HintSide, F::HintDigits]
            }
            SettingsGroup::Ring => {
                &[F::LayoutRadius, F::LayoutShapeFactor, F::LayoutIntercardinalHeight, F::LayoutDeadZoneRadius, F::LayoutScreenMargin]
            }
            SettingsGroup::CenterIndicator => &[
                F::CenterBackgroundRadius,
                F::CenterBackgroundStroke,
                F::CenterBackgroundFillColor,
                F::CenterHighlightRadius,
                F::CenterHighlightShape,
                F::CenterHighlightStroke,
                F::CenterHighlightFillColor,
                F::CenterHighlightAngle,
                F::CenterHighlightCircleRadius,
                F::CenterHighlightSnapping,
            ],
            SettingsGroup::Title => {
                &[F::TitleShow, F::TitleBackgroundColor, F::TitleBackgroundStroke, F::TitlePadding, F::TitleTextColor, F::TitleTextFont]
            }
            SettingsGroup::ButtonLook => &[F::TextColor],
            SettingsGroup::Animation => &[
                F::Reveal,
                F::RevealDuration,
                F::SlideStartRadius,
                F::SlideStartShapeFactor,
                F::SlideFade,
                F::HoverEmphasis,
                F::SubPieHoverEmphasis,
                F::HoverDuration,
            ],
            SettingsGroup::SubPies => {
                &[F::SubPieStyle, F::SubPieCenter, F::SubPieHoverDelay, F::FanOutDelay, F::FanOutRadius, F::FanOutOpenerAction]
            }
            SettingsGroup::Language => &[F::MirrorHorizontal],
        }
    }

    /// The fields of the section that apply to `s` ([`SettingsField::is_relevant`]).
    pub fn relevant_fields(self, s: &PieMenuSettings) -> Vec<SettingsField> {
        self.fields().iter().copied().filter(|f| f.is_relevant(s)).collect()
    }
}
