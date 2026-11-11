//! Buttons: the [`PieButton`] definition, the per-level sanity check, and the
//! [`ButtonView`] handed to the render closure.

use crate::mnemonic::{Mnemonic, append_mnemonic_text};
// Everything else at the crate root, for the doc links below.
#[allow(unused_imports)]
use crate::*;
use crate::settings::PieDirection;
use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId};

/// What a button does when it is selected.
#[derive(Clone, Debug, Default)]
pub enum PieButtonKind {
    /// An ordinary button: selecting it produces [`PieMenuResponse::Selected`].
    #[default]
    Action,
    /// Opens a sub-pie of `children`. `style` overrides
    /// [`PieMenuSettings::sub_pie_style`] for this button; `None` follows it.
    SubPie { children: Vec<PieButton>, style: Option<SubPieStyle> },
    /// A back button: inside a sub-pie it returns to the parent level (like Backspace);
    /// in a fan-out it closes the fan. At the root level, where there is nothing to go
    /// back to, it is selected like an [`PieButtonKind::Action`].
    Back,
}

/// A single button slot in the pie menu.
///
/// Defines *where* the button sits (`direction`), its keyboard mnemonic and the
/// label text used to generate one. All visual content is rendered by the caller
/// via the closure passed to [`PieMenu::show`].
#[derive(Clone, Debug)]
pub struct PieButton {
    /// Which of the eight directions this button occupies.
    pub direction: PieDirection,
    /// How this button gets its mnemonic key. Use [`PieMenu::mnemonics`] for the
    /// keys actually in effect (e.g. to underline them with [`mnemonic_text`]).
    pub mnemonic: Mnemonic,
    /// The text shown on the button, used to generate a mnemonic for
    /// [`Mnemonic::Auto`]. Rendering is still up to the closure.
    pub label: String,
    /// What selecting the button does: an action (default), opening a sub-pie, or going
    /// back. Usually set through [`PieButton::with_children`], [`PieButton::with_fan_out`]
    /// or [`PieButton::back`].
    pub kind: PieButtonKind,
    /// A disabled button is never highlighted or selected (mouse, number keys,
    /// mnemonic) and gets no generated mnemonic. It is still drawn by your closure;
    /// check [`ButtonView::enabled`] to grey it out.
    pub enabled: bool,
}

impl PieButton {
    /// Creates a button at `direction` with an empty label and [`Mnemonic::Auto`].
    pub fn new(direction: PieDirection) -> Self {
        Self {
            direction,
            mnemonic: Mnemonic::Auto,
            label: String::new(),
            kind: PieButtonKind::Action,
            enabled: true,
        }
    }

    /// Creates a back button at `direction`, labelled "Back": selecting it inside a
    /// sub-pie returns to the parent level. See [`PieButtonKind::Back`]. Put it opposite the
    /// button that opens the sub-pie ([`PieDirection::opposite`]) so that, with
    /// [`SubPieCenter::Button`], it points back at the parent.
    pub fn back(direction: PieDirection) -> Self {
        Self { kind: PieButtonKind::Back, label: "Back".to_string(), ..Self::new(direction) }
    }

    /// The buttons of this button's sub-pie; empty unless it is a [`PieButtonKind::SubPie`].
    pub fn children(&self) -> &[PieButton] {
        match &self.kind {
            PieButtonKind::SubPie { children, .. } => children,
            _ => &[],
        }
    }

    /// Whether this is a back button ([`PieButtonKind::Back`]).
    pub fn is_back(&self) -> bool {
        matches!(self.kind, PieButtonKind::Back)
    }

    /// Enables or disables this button. See [`PieButton::enabled`].
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Sets the label text used to generate a mnemonic.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Makes this button open a sub-pie containing `children` when selected, shown in the
    /// menu's [`PieMenuSettings::sub_pie_style`].
    ///
    /// Replace style: inside the sub-pie, Backspace or a click on the centre goes back one
    /// level. The render closure's [`ButtonView::path`] says where each button sits, and
    /// [`PieMenu::selected_path`] which leaf was chosen.
    pub fn with_children(mut self, children: Vec<PieButton>) -> Self {
        self.kind = PieButtonKind::SubPie { children, style: None };
        self
    }

    /// Makes this button open `children` as a fan-out ([`SubPieStyle::FanOut`]) whatever
    /// the menu's style is. The children use ordinary directions, but not the three facing
    /// the parent ([`PieDirection::fan_out_slots`] lists the five that work), and cannot
    /// have sub-pies of their own; [`PieMenu::check_buttons`] reports both.
    pub fn with_fan_out(mut self, children: Vec<PieButton>) -> Self {
        self.kind = PieButtonKind::SubPie { children, style: Some(SubPieStyle::FanOut) };
        self
    }

    /// Overrides the menu's [`PieMenuSettings::sub_pie_style`] for this button's sub-pie
    /// (`None`: follow the menu again). Does nothing on a button without one.
    pub fn with_sub_pie_style(mut self, style: Option<SubPieStyle>) -> Self {
        if let PieButtonKind::SubPie { style: own, .. } = &mut self.kind {
            *own = style;
        }
        self
    }

    /// Sets an explicit mnemonic key for this button. Case-insensitive.
    pub fn with_mnemonic(mut self, c: char) -> Self {
        self.mnemonic = Mnemonic::Key(c);
        self
    }

    /// This button never gets a mnemonic, even when auto mnemonics are on.
    pub fn without_mnemonic(mut self) -> Self {
        self.mnemonic = Mnemonic::None;
        self
    }
}

/// A problem with the buttons of one pie level, from [`PieMenu::check_buttons`].
///
/// Indices refer to the slice that was checked; inside [`ButtonProblem::InFanOut`] they
/// refer to that fan-out's children.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ButtonProblem {
    /// Two buttons occupy the same direction.
    SharedDirection { first: usize, second: usize, direction: PieDirection },
    /// Two enabled buttons have the same mnemonic key.
    DuplicateMnemonic { first: usize, second: usize, key: char },
    /// A fan-out child sits in one of the three directions facing the parent pie
    /// (see [`PieDirection::fan_out_slots`]).
    BlockedDirection { button: usize, direction: PieDirection },
    /// A fan-out child has a sub-pie of its own: a fan-out holds plain buttons only.
    NestedInFanOut { button: usize },
    /// A problem among the fan-out children of button `parent`.
    InFanOut { parent: usize, problem: Box<ButtonProblem> },
}

impl ButtonProblem {
    /// The indices of the buttons involved, in the slice that was checked (for
    /// [`ButtonProblem::InFanOut`]: the fan-out's button).
    pub fn buttons(&self) -> Vec<usize> {
        match self {
            ButtonProblem::SharedDirection { first, second, .. }
            | ButtonProblem::DuplicateMnemonic { first, second, .. } => vec![*first, *second],
            ButtonProblem::BlockedDirection { button, .. } | ButtonProblem::NestedInFanOut { button } => vec![*button],
            ButtonProblem::InFanOut { parent, .. } => vec![*parent],
        }
    }
}

impl std::fmt::Display for ButtonProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ButtonProblem::SharedDirection { first, second, direction } => {
                write!(f, "buttons {first} and {second} are both in direction {direction:?}")
            }
            ButtonProblem::DuplicateMnemonic { first, second, key } => {
                write!(f, "buttons {first} and {second} share the mnemonic '{key}'")
            }
            ButtonProblem::BlockedDirection { button, direction } => {
                write!(f, "button {button} is in direction {direction:?}, which faces the parent pie")
            }
            ButtonProblem::NestedInFanOut { button } => {
                write!(f, "button {button} has a sub-pie of its own, which a fan-out cannot hold")
            }
            ButtonProblem::InFanOut { parent, problem } => {
                write!(f, "in the fan-out of button {parent}: {problem}")
            }
        }
    }
}

/// Problems with the children of a fan-out opened by a button in direction `parent`:
/// the ordinary level problems, children in a blocked direction, and nesting.
pub(crate) fn fan_out_problems(
    parent: PieDirection,
    children: &[PieButton],
    mnemonics: &[Option<char>],
) -> Vec<ButtonProblem> {
    let mut problems = button_problems(children, mnemonics);
    for (i, child) in children.iter().enumerate() {
        if parent.blocks_fan_out(child.direction) {
            problems.push(ButtonProblem::BlockedDirection { button: i, direction: child.direction });
        }
        if matches!(child.kind, PieButtonKind::SubPie { .. }) {
            problems.push(ButtonProblem::NestedInFanOut { button: i });
        }
    }
    problems
}

/// Problems with the buttons of one pie level: several buttons in one direction, or
/// the same mnemonic on several enabled buttons. Buttons in different (sub-)pie levels
/// are never compared with each other, so they may reuse directions and keys.
pub(crate) fn button_problems(buttons: &[PieButton], mnemonics: &[Option<char>]) -> Vec<ButtonProblem> {
    let mut problems = Vec::new();
    let mut dirs: Vec<(usize, PieDirection)> = Vec::new();
    for (i, b) in buttons.iter().enumerate() {
        if let Some((first, _)) = dirs.iter().find(|(_, d)| *d == b.direction) {
            problems.push(ButtonProblem::SharedDirection {
                first: *first,
                second: i,
                direction: b.direction,
            });
        } else {
            dirs.push((i, b.direction));
        }
    }
    let mut keys: Vec<(char, usize)> = Vec::new();
    for (i, (b, m)) in buttons.iter().zip(mnemonics).enumerate() {
        let (true, Some(c)) = (b.enabled, *m) else { continue };
        if let Some((_, first)) = keys.iter().find(|(k, _)| *k == c) {
            problems.push(ButtonProblem::DuplicateMnemonic { first: *first, second: i, key: c });
        } else {
            keys.push((c, i));
        }
    }
    problems
}

/// What the menu knows about one button, handed to your render closure by
/// [`PieMenu::show`]. Everything is resolved for the level currently shown, so you never
/// have to precompute mnemonics or hints.
pub struct ButtonView<'a> {
    /// Index of the button within its level (what `Selected(idx)` returns): the level
    /// being shown, or an open fan-out's children.
    pub idx: usize,
    /// The full route to this button from the root level, its own index last: `[idx]` at
    /// the root, `[parent, idx]` in that button's sub-pie or fan-out, and so on. Tells a
    /// parent's buttons from an open fan-out's, which are on screen together.
    pub path: &'a [usize],
    /// Whether the pointer currently highlights this button.
    pub hovered: bool,
    /// Whether the button can be selected (`false`: grey it out).
    pub enabled: bool,
    /// The button itself (label, direction, …).
    pub button: &'a PieButton,
    /// The mnemonic key in effect for this button (lower-case), if any.
    pub mnemonic: Option<char>,
    /// A number-key hint to draw inside your own button. `None` when number keys are off.
    pub number_hint: Option<NumberHint>,
    /// Right-to-left: [`PieMenuSettings::mirror_horizontal`] is on. Lay out your own widgets
    /// in reverse (e.g. a checkbox's box right of its text); [`Self::label_text`] already
    /// puts its prefix (icon) on the right.
    pub rtl: bool,
    pub(crate) text_color: Color32,
    pub(crate) body_font: FontId,
    pub(crate) opens_fan: bool,
}

impl ButtonView<'_> {
    /// Whether selecting this button opens a sub-pie (replace style or fan-out) instead of
    /// being an action of its own. Lets your closure draw such "folder" buttons differently.
    pub fn opens_sub_pie(&self) -> bool {
        !self.button.children().is_empty()
    }

    /// Whether this button opens its sub-pie as a fan-out ([`SubPieStyle::FanOut`], from
    /// its own style or the menu's) rather than replacing the level. Implies
    /// [`Self::opens_sub_pie`].
    pub fn opens_fan_out(&self) -> bool {
        self.opens_fan
    }

    /// The text colour in effect for this button: what [`Self::label_text`] uses (egui's
    /// widget text colour for its state, or [`PieMenuSettings::text_color`] when set;
    /// greyed out when disabled). For drawing extras of your own, such as a marker at the
    /// button's far end, to match the label.
    pub fn text_color(&self) -> Color32 {
        self.text_color
    }

    /// The button's label with its mnemonic (if any) underlined, ready for
    /// `ui.label(..)` or `egui::Button::new(..)`. The colour is egui's widget text colour
    /// for the button's state (`inactive` / `hovered`), or [`PieMenuSettings::text_color`]
    /// when set; the font is egui's body font. Override with [`LabelText::color`] /
    /// [`LabelText::font`].
    pub fn label_text(&self) -> LabelText {
        LabelText {
            prefix: String::new(),
            text: self.button.label.clone(),
            mnemonic: self.mnemonic,
            color: self.text_color,
            font: self.body_font.clone(),
            rtl: self.rtl,
        }
    }
}

/// A button label with its mnemonic underlined; see [`ButtonView::label_text`].
/// Converts into [`egui::WidgetText`], so pass it to `ui.label(..)` directly.
#[derive(Clone, Debug)]
pub struct LabelText {
    prefix: String,
    text: String,
    mnemonic: Option<char>,
    color: Color32,
    font: FontId,
    rtl: bool,
}

impl LabelText {
    /// Text (and underline) colour.
    pub fn color(mut self, color: Color32) -> Self {
        self.color = color;
        self
    }

    /// Font.
    pub fn font(mut self, font: FontId) -> Self {
        self.font = font;
        self
    }

    /// Text placed before the label in reading order, never underlined (e.g. an icon:
    /// `"🗑  "`). Right-to-left (see [`Self::rtl`]) it goes on the right of the label, with
    /// its trailing spacing moved to face the label.
    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// Right-to-left: put the prefix on the right. Set from [`ButtonView::rtl`] by
    /// [`ButtonView::label_text`]; override it here. egui lays text out left to right (no
    /// bidi), so this only moves the prefix; keep each label single-script.
    pub fn rtl(mut self, rtl: bool) -> Self {
        self.rtl = rtl;
        self
    }

    fn to_job(&self) -> LayoutJob {
        let format = TextFormat { color: self.color, font_id: self.font.clone(), ..Default::default() };
        let mut job = LayoutJob::default();
        if !self.rtl && !self.prefix.is_empty() {
            job.append(&self.prefix, 0.0, format.clone());
        }
        match self.mnemonic {
            Some(m) => append_mnemonic_text(&mut job, &self.text, m, format.clone()),
            None => job.append(&self.text, 0.0, format.clone()),
        }
        if self.rtl && !self.prefix.is_empty() {
            // "🗑  " → "  🗑": the spacing that separated icon and label stays between them.
            let icon = self.prefix.trim_end();
            job.append(&self.prefix[icon.len()..], 0.0, format.clone());
            job.append(icon, 0.0, format);
        }
        job
    }
}

impl From<LabelText> for egui::WidgetText {
    fn from(l: LabelText) -> Self {
        l.to_job().into()
    }
}

/// A number-key hint to draw inside your own button; see [`ButtonView::number_hint`]
/// (or [`PieMenu::number_hint`] outside the render closure).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NumberHint {
    /// The digit, already converted to the [`PieMenuNumberHints::digits`] style.
    pub text: String,
    /// The digit's colour: [`PieMenuNumberHints::color`] when set, else egui's widget text
    /// colour for the button's state (`inactive` / `hovered`).
    pub color: Color32,
    /// Whether the digit goes after (right of) the label rather than before it,
    /// resolved from [`PieMenuNumberHints::side`].
    pub on_right: bool,
}

/// Button text colour: the forced colour if any, else egui's widget text colour for the
/// state (`hovered` = the pie's highlighted button). Disabled buttons get egui's
/// `Visuals::gray_out` of that colour: the text carries an explicit colour, which egui's
/// disabled `Ui` does not fade by itself.
pub(crate) fn button_text_color(forced: Option<Color32>, visuals: &egui::Visuals, hovered: bool, enabled: bool) -> Color32 {
    let state = if hovered && enabled { &visuals.widgets.hovered } else { &visuals.widgets.inactive };
    let color = forced.unwrap_or(state.fg_stroke.color);
    if enabled { color } else { visuals.gray_out(color) }
}

#[cfg(test)]
mod tests {
    #![allow(unused_imports)]
    use super::*;
    use crate::{button::*, highlight_shapes::*, mnemonic::*, *};
    use egui::{Color32, FontId, Key, Pos2, Rect, Stroke, Vec2};

    mod button_view_tests {
        use super::*;

        fn label(prefix: &str, mnemonic: Option<char>) -> LabelText {
            LabelText {
                prefix: prefix.to_string(),
                text: "Copy".to_string(),
                mnemonic,
                color: Color32::WHITE,
                font: FontId::default(),
                rtl: false,
            }
        }

        #[test]
        fn rtl_puts_the_prefix_on_the_right() {
            let job = label("🗑  ", Some('p')).rtl(true).to_job();
            assert_eq!(job.text, "Copy  🗑");
            let underlined: Vec<_> = job.sections.iter().filter(|s| s.format.underline != Stroke::NONE).collect();
            assert_eq!(underlined.len(), 1, "still only the mnemonic is underlined");
            assert_eq!(label("", None).rtl(true).to_job().text, "Copy");
        }

        #[test]
        fn plain_label_is_one_section() {
            assert_eq!(label("", None).to_job().sections.len(), 1);
        }

        #[test]
        fn mnemonic_is_underlined_and_prefix_is_not() {
            let job = label("X ", Some('p')).to_job();
            assert_eq!(job.text, "X Copy");
            // egui merges equal adjacent formats: "X Co" | "p" (underlined) | "y"
            assert_eq!(job.sections.len(), 3);
            assert!(job.sections[1].format.underline != Stroke::NONE);
            let underlined: Vec<_> = job.sections.iter().filter(|s| s.format.underline != Stroke::NONE).collect();
            assert_eq!(underlined.len(), 1);
        }

        #[test]
        fn overrides_apply() {
            let l = label("", None).color(Color32::RED).font(FontId::monospace(20.0));
            let job = l.to_job();
            assert_eq!(job.sections[0].format.color, Color32::RED);
            assert_eq!(job.sections[0].format.font_id, FontId::monospace(20.0));
        }
    }
}
