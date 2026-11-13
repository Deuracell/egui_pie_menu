//! Bindings: values that come from your app (a Rust expression in the generated code, such
//! as `theme.accent` or `tr("edit.copy")`) instead of the literal the editor shows.
//!
//! The dev tool cannot evaluate Rust, so a bound value keeps its literal as the **preview
//! value**: the editor and the live preview go on using it, and only the generated code uses
//! the expression. Your app can do better by registering [`Variables`]: their names are
//! offered for the expression, and a value bound to one shows the variable's real value
//! ([`Bindings::apply`]).
use super::{SettingsField, TreeEdit};
use crate::PieButton;
use crate::settings::PieMenuSettings;
use egui::{Color32, FontId, RichText, Stroke, Ui};

/// What a binding replaces.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BindTarget {
    /// A settings field; only those where [`SettingsField::is_bindable`] holds (colours,
    /// strokes and fonts).
    Field(SettingsField),
    /// The menu title (`PieMenu::with_title`).
    Title,
    /// The label of the button at this path.
    Label(Vec<usize>),
    /// The label of the cluster item with this number, in the cluster at this path.
    ItemLabel(Vec<usize>, usize),
    /// Whether the button at this path is enabled.
    Enabled(Vec<usize>),
    /// Whether the cluster item with this number, in the cluster at this path, is enabled.
    ItemEnabled(Vec<usize>, usize),
    /// Whether the button at this path is shown ([`PieButton::visible`]).
    Visible(Vec<usize>),
}

impl BindTarget {
    /// The path of the button this target belongs to (`None` for a field or the title).
    pub fn path(&self) -> Option<&[usize]> {
        match self {
            BindTarget::Label(p)
            | BindTarget::ItemLabel(p, _)
            | BindTarget::Enabled(p)
            | BindTarget::ItemEnabled(p, _)
            | BindTarget::Visible(p) => Some(p),
            BindTarget::Field(_) | BindTarget::Title => None,
        }
    }

    fn path_mut(&mut self) -> Option<&mut Vec<usize>> {
        match self {
            BindTarget::Label(p)
            | BindTarget::ItemLabel(p, _)
            | BindTarget::Enabled(p)
            | BindTarget::ItemEnabled(p, _)
            | BindTarget::Visible(p) => Some(p),
            BindTarget::Field(_) | BindTarget::Title => None,
        }
    }

    /// The cluster item number, for a target that belongs to a cluster item.
    pub fn item(&self) -> Option<usize> {
        match self {
            BindTarget::ItemLabel(_, n) | BindTarget::ItemEnabled(_, n) => Some(*n),
            _ => None,
        }
    }

    fn item_mut(&mut self) -> Option<&mut usize> {
        match self {
            BindTarget::ItemLabel(_, n) | BindTarget::ItemEnabled(_, n) => Some(n),
            _ => None,
        }
    }

    /// Whether the target is text (a label or the title) rather than a settings value.
    pub fn is_text(&self) -> bool {
        self.kind() == VarKind::Text
    }

    /// The kind of value the target holds (what a [`Variables`] entry must be to fill it).
    pub fn kind(&self) -> VarKind {
        use SettingsField as F;
        match self {
            BindTarget::Field(F::CenterBackgroundStroke | F::CenterHighlightStroke | F::TitleBackgroundStroke) => VarKind::Stroke,
            BindTarget::Field(F::TitleTextFont) => VarKind::Font,
            BindTarget::Field(f) if f.is_number() => VarKind::Number,
            BindTarget::Field(_) => VarKind::Color,
            BindTarget::Title | BindTarget::Label(_) | BindTarget::ItemLabel(..) => VarKind::Text,
            BindTarget::Enabled(_) | BindTarget::ItemEnabled(..) | BindTarget::Visible(_) => VarKind::Bool,
        }
    }

    /// An example expression, shown as the hint of an empty expression.
    pub fn example(&self) -> &'static str {
        use SettingsField as F;
        match self {
            BindTarget::Field(F::TitleTextFont) => "theme.title_font",
            BindTarget::Field(f) if f.is_duration() => "prefs.animation_ms",
            BindTarget::Field(f) if f.is_number() => "prefs.pie_radius",
            BindTarget::Field(F::CenterBackgroundStroke | F::CenterHighlightStroke | F::TitleBackgroundStroke) => {
                "egui::Stroke::new(2.0, theme.accent)"
            }
            BindTarget::Field(_) => "theme.accent",
            BindTarget::Title => "i18n.menu.edit",
            BindTarget::Label(_) | BindTarget::ItemLabel(..) => "i18n.edit.copy",
            BindTarget::Enabled(_) | BindTarget::ItemEnabled(..) => "app.can_undo",
            BindTarget::Visible(_) => "plugins.share",
        }
    }
}

/// When the generated code reads a bound value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BindingMode {
    /// Once, where the menu is built: the expression takes the literal's place
    /// (`s.title.text_color = theme.accent;`, `.with_label(tr("edit.copy"))`). For values that
    /// are fixed for the menu's life, such as the theme at startup or the language.
    #[default]
    Build,
    /// Every frame: the menu is built with the preview value, and an "each frame, before
    /// `show`" block assigns the expression (`menu.settings.title.text_color = theme.accent;`).
    /// For values that change while the app runs (a theme toggle, "Undo (3)").
    Live,
}

/// One binding: the expression for the generated code and when it is read.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct Binding {
    /// Rust source of the value. For an optional settings field (such as `text_color`) it is
    /// the inner value: the code wraps it in `Some(..)`. For a label or the title: built, any
    /// `Into<String>`; live, a `String` (it is assigned). Empty: the binding is not used (the
    /// code keeps the literal).
    pub expr: String,
    pub mode: BindingMode,
}

impl Binding {
    pub fn new(expr: impl Into<String>, mode: BindingMode) -> Self {
        Self { expr: expr.into(), mode }
    }

    /// The expression, trimmed, if it is not empty.
    pub fn active_expr(&self) -> Option<&str> {
        let e = self.expr.trim();
        (!e.is_empty()).then_some(e)
    }
}

/// The bindings of one menu, owned by you like the [`super::Selection`]. Edit them with
/// [`super::field_editor_bound`], [`super::button_inspector_bound`] (which keeps them with
/// their buttons when the tree changes) or [`binding_toggle`] / [`binding_details`], and
/// generate code with [`super::menu_to_code_bound`].
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct Bindings {
    entries: Vec<(BindTarget, Binding)>,
    /// The value a target had before [`Bindings::apply`] first replaced it with a variable's:
    /// the fallback that files keep. Not saved.
    #[cfg_attr(feature = "serde", serde(skip))]
    fallbacks: Vec<(BindTarget, Option<VarValue>)>,
    /// What your app registered this frame ([`Bindings::set_variables`]); not saved.
    #[cfg_attr(feature = "serde", serde(skip))]
    variables: Variables,
}

/// Equal when they bind the same targets the same way (in any order) and hold the same
/// variables.
impl PartialEq for Bindings {
    fn eq(&self, other: &Self) -> bool {
        self.entries.len() == other.entries.len()
            && self.entries.iter().all(|(t, b)| other.get(t) == Some(b))
            && self.variables == other.variables
    }
}

impl Bindings {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn get(&self, target: &BindTarget) -> Option<&Binding> {
        self.entries.iter().find(|(t, _)| t == target).map(|(_, b)| b)
    }

    pub fn get_mut(&mut self, target: &BindTarget) -> Option<&mut Binding> {
        self.entries.iter_mut().find(|(t, _)| t == target).map(|(_, b)| b)
    }

    /// Binds `target` (replacing a binding it had).
    pub fn set(&mut self, target: BindTarget, binding: Binding) {
        match self.get_mut(&target) {
            Some(b) => *b = binding,
            None => self.entries.push((target, binding)),
        }
    }

    pub fn remove(&mut self, target: &BindTarget) -> Option<Binding> {
        let i = self.entries.iter().position(|(t, _)| t == target)?;
        self.fallbacks.retain(|(t, _)| t != target);
        Some(self.entries.remove(i).1)
    }

    /// The value `target` had before [`Bindings::apply`] first replaced it with your app's:
    /// the fallback to write to a file. `None` if nothing was replaced (the current value is
    /// the fallback); `Some(None)` if it had none (an optional colour that was off).
    pub fn fallback(&self, target: &BindTarget) -> Option<Option<&VarValue>> {
        self.fallbacks.iter().find(|(t, _)| t == target).map(|(_, v)| v.as_ref())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&BindTarget, &Binding)> {
        self.entries.iter().map(|(t, b)| (t, b))
    }

    /// What generated code writes for `target` bound in `mode`: the binding's name or
    /// expression, translated through [`Variables::code`] when it names a variable.
    pub fn code_expr(&self, target: &BindTarget, mode: BindingMode) -> Option<&str> {
        self.expr(target, mode).map(|e| self.variables.code_for(e))
    }

    /// Keeps the bindings `keep` accepts (e.g. `|t, _| t.path().is_none()` drops every label
    /// binding when the tree is replaced).
    pub fn retain(&mut self, mut keep: impl FnMut(&BindTarget, &Binding) -> bool) {
        self.entries.retain(|(t, b)| keep(t, b));
        self.drop_orphan_fallbacks();
    }

    fn drop_orphan_fallbacks(&mut self) {
        let entries = &self.entries;
        self.fallbacks.retain(|(t, _)| entries.iter().any(|(e, _)| e == t));
    }

    /// The expression of `target` if it is bound in `mode` with a non-empty expression.
    pub fn expr(&self, target: &BindTarget, mode: BindingMode) -> Option<&str> {
        self.get(target).filter(|b| b.mode == mode).and_then(Binding::active_expr)
    }

    /// Follows a change to the tree's shape (as reported in [`super::InspectorOutput::edits`])
    /// so every label binding stays with its button. [`super::button_inspector_bound`] does
    /// this itself.
    pub fn follow(&mut self, edit: &TreeEdit) {
        let keep = |target: &mut BindTarget| {
            let item = target.item().is_some();
            target.path_mut().is_none_or(|key| crate::button_ids::follow_key(key, item, edit))
        };
        self.entries.retain_mut(|(t, _)| keep(t));
        self.fallbacks.retain_mut(|(t, _)| keep(t));
    }

    /// The cluster item `from` of the cluster at `path` now has the number `to`.
    pub fn renumber_item(&mut self, path: &[usize], from: usize, to: usize) {
        let targets = self.entries.iter_mut().map(|(t, _)| t).chain(self.fallbacks.iter_mut().map(|(t, _)| t));
        for t in targets {
            if t.path() == Some(path)
                && let Some(n) = t.item_mut()
                && *n == from
            {
                *n = to;
            }
        }
    }

    /// Hands the editors this frame's [`Variables`] of your app: the expression row offers
    /// their names, and a target bound to one is shown with its value ([`Bindings::apply`]).
    pub fn set_variables(&mut self, variables: Variables) {
        self.variables = variables;
    }

    pub fn variables(&self) -> &Variables {
        &self.variables
    }

    /// The registered variable `target` is bound to: its expression is a variable's name and
    /// the variable is of the target's [`BindTarget::kind`].
    pub fn variable_for(&self, target: &BindTarget) -> Option<&VarValue> {
        let name = self.get(target)?.active_expr()?;
        self.variables.get(name).filter(|v| v.kind() == target.kind())
    }

    /// Writes the value of every variable-bound target into `settings` and `buttons`, so the
    /// editors and the preview show your app's real values (call it each frame, after
    /// [`Bindings::set_variables`] and before drawing). The value a target had before is kept
    /// as its [`Bindings::fallback`]. The title is done by [`Bindings::apply_title`]. Returns
    /// `true` if anything changed.
    pub fn apply(&mut self, settings: &mut PieMenuSettings, buttons: &mut [PieButton]) -> bool {
        let mut changed = false;
        // The title is not in `settings` or `buttons`: apply_title does it.
        let targets: Vec<BindTarget> = self.entries.iter().map(|(t, _)| t.clone()).filter(|t| *t != BindTarget::Title).collect();
        for target in targets {
            let Some(value) = self.variable_for(&target).cloned() else { continue };
            let before = current_value(settings, buttons, &target);
            if before.as_ref() == Some(&value) {
                continue;
            }
            if self.fallback(&target).is_none() {
                self.fallbacks.push((target.clone(), before));
            }
            changed |= write_value(settings, buttons, &target, &value);
        }
        changed
    }

    /// [`Bindings::apply`] for the title (`""`: none): sets it to the variable's text if the
    /// title is bound to one. Returns `true` if it changed.
    pub fn apply_title(&mut self, title: &mut String) -> bool {
        let Some(VarValue::Text(t)) = self.variable_for(&BindTarget::Title).cloned() else { return false };
        if *title == t {
            return false;
        }
        if self.fallback(&BindTarget::Title).is_none() {
            self.fallbacks.push((BindTarget::Title, Some(VarValue::Text(title.clone()))));
        }
        *title = t;
        true
    }

    /// Forgets the bindings of buttons and cluster items that are not in `buttons`.
    pub fn prune(&mut self, buttons: &[PieButton]) {
        self.entries.retain(|(t, _)| match (t.path(), t.item()) {
            (Some(p), None) => crate::button_at(buttons, p).is_some(),
            (Some(p), Some(n)) => crate::button_at(buttons, p).is_some_and(|b| b.cluster_items().iter().any(|i| i.number == n)),
            (None, _) => true,
        });
        self.drop_orphan_fallbacks();
    }
}

fn set_if_changed<T: PartialEq + Clone>(slot: &mut T, value: &T) -> bool {
    if slot == value {
        return false;
    }
    *slot = value.clone();
    true
}

/// The value `target` has now (`None` for the title, an optional colour that is off, or a
/// button that is not there).
fn current_value(s: &PieMenuSettings, buttons: &[PieButton], target: &BindTarget) -> Option<VarValue> {
    use SettingsField as F;
    let item = |p: &[usize], n: usize| crate::button_at(buttons, p).and_then(|b| b.cluster_items().iter().find(|i| i.number == n));
    Some(match target {
        BindTarget::Field(f) => match f {
            F::HintColor => VarValue::Color(s.number_hints.color?),
            F::TextColor => VarValue::Color(s.text_color?),
            F::CenterBackgroundFillColor => VarValue::Color(s.center_indicator.background_fill_color),
            F::CenterHighlightFillColor => VarValue::Color(s.center_indicator.highlight_fill_color),
            F::TitleBackgroundColor => VarValue::Color(s.title.background_color),
            F::TitleTextColor => VarValue::Color(s.title.text_color),
            F::CenterBackgroundStroke => VarValue::Stroke(s.center_indicator.background_stroke),
            F::CenterHighlightStroke => VarValue::Stroke(s.center_indicator.highlight_stroke),
            F::TitleBackgroundStroke => VarValue::Stroke(s.title.background_stroke),
            F::TitleTextFont => VarValue::Font(s.title.text_font.clone()),
            f => VarValue::Number(number_field(s, *f)?),
        },
        BindTarget::Title => return None,
        BindTarget::Label(p) => VarValue::Text(crate::button_at(buttons, p)?.label.clone()),
        BindTarget::ItemLabel(p, n) => VarValue::Text(item(p, *n)?.label.clone()),
        BindTarget::Enabled(p) => VarValue::Bool(crate::button_at(buttons, p)?.enabled),
        BindTarget::Visible(p) => VarValue::Bool(crate::button_at(buttons, p)?.visible),
        BindTarget::ItemEnabled(p, n) => VarValue::Bool(item(p, *n)?.enabled),
    })
}

/// Writes `value` into `target` if the kinds match. Returns whether it changed.
fn write_value(settings: &mut PieMenuSettings, buttons: &mut [PieButton], target: &BindTarget, value: &VarValue) -> bool {
    match (target, value) {
        (BindTarget::Field(f), v) => set_field(settings, *f, v),
        (BindTarget::Label(p), VarValue::Text(t)) => crate::button_at_mut(buttons, p).is_some_and(|b| set_if_changed(&mut b.label, t)),
        (BindTarget::Enabled(p), VarValue::Bool(on)) => crate::button_at_mut(buttons, p).is_some_and(|b| set_if_changed(&mut b.enabled, on)),
        (BindTarget::Visible(p), VarValue::Bool(on)) => crate::button_at_mut(buttons, p).is_some_and(|b| set_if_changed(&mut b.visible, on)),
        (BindTarget::ItemLabel(p, n), VarValue::Text(t)) => {
            crate::button_at_mut(buttons, p).and_then(|b| b.cluster_item_mut(*n)).is_some_and(|i| set_if_changed(&mut i.label, t))
        }
        (BindTarget::ItemEnabled(p, n), VarValue::Bool(on)) => {
            crate::button_at_mut(buttons, p).and_then(|b| b.cluster_item_mut(*n)).is_some_and(|i| set_if_changed(&mut i.enabled, on))
        }
        _ => false,
    }
}

/// Writes `value` into a bindable `field` if the kinds match. Returns whether it changed.
fn set_field(s: &mut PieMenuSettings, field: SettingsField, value: &VarValue) -> bool {
    use SettingsField as F;
    match (field, value) {
        (F::HintColor, VarValue::Color(c)) => set_if_changed(&mut s.number_hints.color, &Some(*c)),
        (F::TextColor, VarValue::Color(c)) => set_if_changed(&mut s.text_color, &Some(*c)),
        (F::CenterBackgroundFillColor, VarValue::Color(c)) => set_if_changed(&mut s.center_indicator.background_fill_color, c),
        (F::CenterHighlightFillColor, VarValue::Color(c)) => set_if_changed(&mut s.center_indicator.highlight_fill_color, c),
        (F::TitleBackgroundColor, VarValue::Color(c)) => set_if_changed(&mut s.title.background_color, c),
        (F::TitleTextColor, VarValue::Color(c)) => set_if_changed(&mut s.title.text_color, c),
        (F::CenterBackgroundStroke, VarValue::Stroke(st)) => set_if_changed(&mut s.center_indicator.background_stroke, st),
        (F::CenterHighlightStroke, VarValue::Stroke(st)) => set_if_changed(&mut s.center_indicator.highlight_stroke, st),
        (F::TitleBackgroundStroke, VarValue::Stroke(st)) => set_if_changed(&mut s.title.background_stroke, st),
        (F::TitleTextFont, VarValue::Font(f)) => set_if_changed(&mut s.title.text_font, f),
        (f, VarValue::Number(n)) => set_number_field(s, f, *n),
        _ => false,
    }
}

/// The number settings: (field, getter, setter), the durations in milliseconds and the
/// optional ones `None` when off.
type NumberSlot = (SettingsField, fn(&PieMenuSettings) -> Option<f32>, fn(&mut PieMenuSettings, f32));

fn ms(d: std::time::Duration) -> f32 {
    d.as_secs_f32() * 1000.0
}

fn from_ms(n: f32) -> std::time::Duration {
    std::time::Duration::from_secs_f32(n.max(0.0) / 1000.0)
}

const NUMBER_SLOTS: [NumberSlot; 19] = {
    use SettingsField as F;
    [
        (F::HintSpacing, |s| Some(s.number_hints.spacing), |s, n| s.number_hints.spacing = n),
        (F::LayoutRadius, |s| Some(s.layout.radius), |s, n| s.layout.radius = n),
        (F::LayoutShapeFactor, |s| Some(s.layout.shape_factor), |s, n| s.layout.shape_factor = n),
        (F::LayoutIntercardinalHeight, |s| s.layout.intercardinal_height, |s, n| s.layout.intercardinal_height = Some(n)),
        (F::LayoutDeadZoneRadius, |s| Some(s.layout.dead_zone_radius), |s, n| s.layout.dead_zone_radius = n),
        (F::LayoutScreenMargin, |s| Some(s.layout.screen_margin), |s, n| s.layout.screen_margin = n),
        (F::CenterBackgroundRadius, |s| s.center_indicator.background_radius, |s, n| s.center_indicator.background_radius = Some(n)),
        (F::CenterHighlightRadius, |s| s.center_indicator.highlight_radius, |s, n| s.center_indicator.highlight_radius = Some(n)),
        (F::CenterHighlightAngle, |s| Some(s.center_indicator.highlight_angle), |s, n| s.center_indicator.highlight_angle = n),
        (F::CenterHighlightCircleRadius, |s| Some(s.center_indicator.highlight_circle_radius), |s, n| {
            s.center_indicator.highlight_circle_radius = n
        }),
        (F::SlideStartRadius, |s| Some(s.animations.slide_start_radius), |s, n| s.animations.slide_start_radius = n),
        (F::SlideStartShapeFactor, |s| s.animations.slide_start_shape_factor, |s, n| s.animations.slide_start_shape_factor = Some(n)),
        (F::FanOutRadius, |s| s.sub_pies.fan_out.radius, |s, n| s.sub_pies.fan_out.radius = Some(n)),
        (F::DoubleTapWindow, |s| Some(ms(s.input.double_tap_window)), |s, n| s.input.double_tap_window = from_ms(n)),
        (F::RevealDuration, |s| Some(ms(s.animations.reveal_duration)), |s, n| s.animations.reveal_duration = from_ms(n)),
        (F::HoverDuration, |s| Some(ms(s.animations.hover_duration)), |s, n| s.animations.hover_duration = from_ms(n)),
        (F::FanOutDelay, |s| Some(ms(s.sub_pies.fan_out.delay)), |s, n| s.sub_pies.fan_out.delay = from_ms(n)),
        (F::SlideFade, |s| s.animations.slide_fade.map(ms), |s, n| s.animations.slide_fade = Some(from_ms(n))),
        (F::SubPieHoverDelay, |s| s.sub_pies.hover_delay.map(ms), |s, n| s.sub_pies.hover_delay = Some(from_ms(n))),
    ]
};

fn number_field(s: &PieMenuSettings, field: SettingsField) -> Option<f32> {
    NUMBER_SLOTS.iter().find(|(f, ..)| *f == field).and_then(|(_, get, _)| get(s))
}

fn set_number_field(s: &mut PieMenuSettings, field: SettingsField, n: f32) -> bool {
    let Some((_, get, set)) = NUMBER_SLOTS.iter().find(|(f, ..)| *f == field) else { return false };
    if get(s) == Some(n) {
        return false;
    }
    let before = format!("{s:?}");
    set(s, n);
    // A duration rounds to whole nanoseconds: "changed" only if it really did.
    format!("{s:?}") != before
}

// ---------------------------------------------------------------------------------------
// Variables
// ---------------------------------------------------------------------------------------

/// The kinds of value a binding can hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VarKind {
    Color,
    Stroke,
    Font,
    Text,
    /// True or false (whether a button is enabled).
    Bool,
    /// A number: a size, factor or angle (radians) as in the settings, a duration in
    /// milliseconds.
    Number,
}

impl VarKind {
    pub fn name(self) -> &'static str {
        match self {
            VarKind::Color => "a colour",
            VarKind::Stroke => "a stroke",
            VarKind::Font => "a font",
            VarKind::Text => "text",
            VarKind::Bool => "true or false",
            VarKind::Number => "a number",
        }
    }
}

/// The current value of one of your app's variables.
#[derive(Clone, Debug, PartialEq)]
pub enum VarValue {
    Color(Color32),
    Stroke(Stroke),
    Font(FontId),
    Text(String),
    Bool(bool),
    Number(f32),
}

impl VarValue {
    pub fn kind(&self) -> VarKind {
        match self {
            VarValue::Color(_) => VarKind::Color,
            VarValue::Stroke(_) => VarKind::Stroke,
            VarValue::Font(_) => VarKind::Font,
            VarValue::Text(_) => VarKind::Text,
            VarValue::Bool(_) => VarKind::Bool,
            VarValue::Number(_) => VarKind::Number,
        }
    }
}

/// Values of your app the dev tool can bind to, each under the Rust expression that reads it
/// in your code. Build it each frame from your real state and hand it over with
/// [`Bindings::set_variables`]:
///
/// ```
/// use egui_pie_menu::dev_tool::*;
/// # let accent = egui::Color32::LIGHT_BLUE;
/// # fn tr(k: &str) -> String { k.to_uppercase() }
/// let mut vars = Variables::new();
/// vars.color("theme.accent", accent).text("tr(\"edit.copy\")", tr("edit.copy"));
/// let mut bindings = Bindings::new();
/// bindings.set_variables(vars);
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Variables {
    entries: Vec<(String, VarValue)>,
    /// The Rust expression of a name, for generated code (by name).
    code: Vec<(String, String)>,
    /// The entry registered last (what [`Variables::code`] applies to).
    last: Option<usize>,
}

impl Variables {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Registers (or replaces) the variable `name`: a plain key such as `theme.accent` or
    /// `i18n.edit.copy`, which TOML files and bindings refer to.
    pub fn set(&mut self, name: impl Into<String>, value: VarValue) -> &mut Self {
        let name = name.into();
        let i = match self.entries.iter().position(|(n, _)| *n == name) {
            Some(i) => {
                self.entries[i].1 = value;
                i
            }
            None => {
                self.entries.push((name, value));
                self.entries.len() - 1
            }
        };
        self.last = Some(i);
        self
    }

    pub fn color(&mut self, name: impl Into<String>, value: Color32) -> &mut Self {
        self.set(name, VarValue::Color(value))
    }

    pub fn stroke(&mut self, name: impl Into<String>, value: Stroke) -> &mut Self {
        self.set(name, VarValue::Stroke(value))
    }

    pub fn font(&mut self, name: impl Into<String>, value: FontId) -> &mut Self {
        self.set(name, VarValue::Font(value))
    }

    pub fn text(&mut self, name: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.set(name, VarValue::Text(value.into()))
    }

    /// A number, such as the user's preferred ring radius or animation time (in
    /// milliseconds for a duration).
    pub fn number(&mut self, name: impl Into<String>, value: f32) -> &mut Self {
        self.set(name, VarValue::Number(value))
    }

    /// A true-or-false variable, such as whether undo is possible (for `enabled`).
    pub fn flag(&mut self, name: impl Into<String>, value: bool) -> &mut Self {
        self.set(name, VarValue::Bool(value))
    }

    /// The Rust expression that reads the variable registered last, for the generated code
    /// (`.text("i18n.edit.copy", tr("edit.copy")).code("tr(\"edit.copy\")")`). Without it the
    /// generated code uses the name itself, which suits names like `theme.accent`.
    pub fn code(&mut self, expr: impl Into<String>) -> &mut Self {
        if let Some(name) = self.last.and_then(|i| self.entries.get(i)).map(|(n, _)| n.clone()) {
            self.code.retain(|(n, _)| *n != name);
            self.code.push((name, expr.into()));
        }
        self
    }

    pub fn get(&self, name: &str) -> Option<&VarValue> {
        self.entries.iter().find(|(n, _)| n == name).map(|(_, v)| v)
    }

    /// What the generated code writes for `name`: its [`Variables::code`], else `name`.
    pub fn code_for<'a>(&'a self, name: &'a str) -> &'a str {
        self.code.iter().find(|(n, _)| n == name).map_or(name, |(_, c)| c.as_str())
    }

    /// All variables, in the order they were registered.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &VarValue)> {
        self.entries.iter().map(|(n, v)| (n.as_str(), v))
    }

    /// The variables of one kind, in the order they were registered.
    pub fn of_kind(&self, kind: VarKind) -> impl Iterator<Item = (&str, &VarValue)> {
        self.iter().filter(move |(_, v)| v.kind() == kind)
    }
}

impl SettingsField {
    /// Whether the field can be bound to an expression: the colours, strokes, fonts, numbers
    /// and durations.
    pub fn is_bindable(self) -> bool {
        use SettingsField as F;
        self.is_number()
            || matches!(
            self,
            F::HintColor
                | F::CenterBackgroundStroke
                | F::CenterBackgroundFillColor
                | F::CenterHighlightStroke
                | F::CenterHighlightFillColor
                | F::TitleBackgroundColor
                | F::TitleBackgroundStroke
                | F::TitleTextColor
                | F::TitleTextFont
                | F::TextColor
        )
    }

    /// Whether the field is an `Option` (a binding's expression gives the inner value).
    pub(crate) fn is_optional_value(self) -> bool {
        use SettingsField as F;
        matches!(
            self,
            F::HintColor
                | F::TextColor
                | F::LayoutIntercardinalHeight
                | F::CenterBackgroundRadius
                | F::CenterHighlightRadius
                | F::SlideStartShapeFactor
                | F::FanOutRadius
                | F::SlideFade
                | F::SubPieHoverDelay
        )
    }

    /// Whether the field is a number or a duration (bound to a [`VarKind::Number`]).
    pub fn is_number(self) -> bool {
        NUMBER_SLOTS.iter().any(|(f, ..)| *f == self)
    }

    /// Whether the field is a duration (a number variable gives milliseconds).
    pub fn is_duration(self) -> bool {
        use SettingsField as F;
        matches!(self, F::DoubleTapWindow | F::RevealDuration | F::HoverDuration | F::FanOutDelay | F::SlideFade | F::SubPieHoverDelay)
    }
}

// ---------------------------------------------------------------------------------------
// Expressions
// ---------------------------------------------------------------------------------------

/// Walks `expr`, calling `f(index, char, depth)` for every char outside string and char literals.
fn scan(expr: &str, mut f: impl FnMut(usize, char, usize)) {
    let chars: Vec<(usize, char)> = expr.char_indices().collect();
    let mut depth = 0usize;
    let mut i = 0;
    while i < chars.len() {
        let (at, c) = chars[i];
        match c {
            '"' => {
                i += 1;
                while i < chars.len() && chars[i].1 != '"' {
                    if chars[i].1 == '\\' {
                        i += 1;
                    }
                    i += 1;
                }
            }
            '\'' if i + 2 < chars.len() && (chars[i + 2].1 == '\'' || chars[i + 1].1 == '\\') => {
                i += 1;
                while i < chars.len() && chars[i].1 != '\'' {
                    if chars[i].1 == '\\' {
                        i += 1;
                    }
                    i += 1;
                }
            }
            '(' | '[' | '{' => {
                f(at, c, depth);
                depth += 1;
            }
            ')' | ']' | '}' => {
                depth = depth.saturating_sub(1);
                f(at, c, depth);
            }
            _ => f(at, c, depth),
        }
        i += 1;
    }
}

const KEYWORDS: [&str; 16] =
    ["as", "else", "false", "fn", "if", "in", "let", "match", "move", "mut", "ref", "return", "self", "super", "crate", "true"];

/// The names an expression expects in scope: identifiers that start a path or a field access
/// (`theme` in `theme.accent`, `tr` in `tr("x")`, `n` in `format!("{}", n)`), leaving out
/// macros, paths through modules (`egui::Color32::RED`), fields, methods, keywords and
/// capitalised names (types, constants, `Some`).
pub fn free_identifiers(expr: &str) -> Vec<String> {
    let mut code = String::with_capacity(expr.len());
    // Blank out literals so their contents are not taken for names.
    let mut last = 0;
    scan(expr, |at, c, _| {
        while last < at {
            code.push(' ');
            last += 1;
        }
        code.push(c);
        last = at + c.len_utf8();
    });
    let chars: Vec<char> = code.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let before = chars[..start].iter().rev().find(|c| !c.is_whitespace()).copied();
            let after: String = chars[i..].iter().filter(|c| !c.is_whitespace()).take(2).collect();
            let prefixed = matches!(before, Some('.')) || (start >= 2 && chars[start - 2..start] == [':', ':']);
            let is_macro_or_path = after.starts_with('!') || after.starts_with("::");
            let lower = word.starts_with(|c: char| c.is_lowercase() || c == '_');
            if !prefixed && !is_macro_or_path && lower && !KEYWORDS.contains(&word.as_str()) && !out.contains(&word) {
                out.push(word);
            }
        } else if c.is_ascii_digit() {
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '.') {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    out
}

// ---------------------------------------------------------------------------------------
// Widgets
// ---------------------------------------------------------------------------------------

/// The bind toggle's glyph (chains; in egui's default emoji font).
const LINK: char = '\u{26d3}';

fn last_expr_id(ui: &Ui, target: &BindTarget) -> egui::Id {
    egui::Id::new(("pie_binding_last", target)).with(ui.id())
}

/// The bind toggle for `target`: a small link button, highlighted while bound. Unbinding
/// remembers the expression (in egui's temporary memory) and binding again restores it.
/// Returns `true` if the binding changed.
pub fn binding_toggle(ui: &mut Ui, bindings: &mut Bindings, target: &BindTarget) -> bool {
    let bound = bindings.get(target).is_some();
    let hover = if bound {
        "Bound: the value comes from your app (the name below). Click to unbind."
    } else {
        "Bind to a value from your app, by name (such as theme.accent): TOML files keep the name, generated code reads it."
    };
    if !ui.selectable_label(bound, LINK.to_string()).on_hover_text(hover).clicked() {
        return false;
    }
    let id = last_expr_id(ui, target);
    if let Some(b) = bindings.remove(target) {
        ui.data_mut(|d| d.insert_temp(id, b));
    } else {
        let b = ui.data(|d| d.get_temp::<Binding>(id)).unwrap_or_default();
        bindings.set(target.clone(), b);
    }
    true
}

/// The details of a bound `target`, below its row: Build / Live and the expression, then, if
/// your app registered [`Variables`] of the target's kind, a picker of their names, and a
/// note where one is due. Nothing if `target` is not bound. Returns `true` if the binding
/// changed.
pub fn binding_details(ui: &mut Ui, bindings: &mut Bindings, target: &BindTarget) -> bool {
    let kind = target.kind();
    let options: Vec<(String, VarValue)> = bindings.variables.of_kind(kind).map(|(n, v)| (n.to_string(), v.clone())).collect();
    let other_kind = bindings
        .get(target)
        .and_then(Binding::active_expr)
        .and_then(|e| bindings.variables.get(e))
        .map(VarValue::kind)
        .filter(|k| *k != kind);
    let from_app = bindings.variable_for(target).is_some();
    let unknown = !bindings.variables.is_empty()
        && bindings.get(target).and_then(Binding::active_expr).is_some_and(|e| bindings.variables.get(e).is_none());
    let code = bindings
        .get(target)
        .and_then(Binding::active_expr)
        .map(|e| bindings.variables.code_for(e))
        .filter(|c| bindings.get(target).and_then(Binding::active_expr) != Some(*c))
        .map(str::to_string);
    let Some(binding) = bindings.get_mut(target) else {
        return false;
    };
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.add_space(ui.spacing().indent);
        ui.label(RichText::new("from:").weak()).on_hover_text(
            "A variable your app registers, by name (TOML files refer to it). Without one, generated code can still use any Rust expression typed here. The value above is the fallback.",
        );
        for (mode, name, hover) in [
            (BindingMode::Build, "Build", "Generated code reads it once, where the menu is built (theme at startup, language). TOML files have no modes: a loaded menu takes your app's values whenever you apply them."),
            (BindingMode::Live, "Live", "Generated code reads it every frame, in a block that runs before show (theme toggle, \"Undo (3)\"). TOML files have no modes."),
        ] {
            if ui.selectable_label(binding.mode == mode, name).on_hover_text(hover).clicked() && binding.mode != mode {
                binding.mode = mode;
                changed = true;
            }
        }
        // The rest of the line.
        let width = ui.available_width().max(80.0);
        let edit = egui::TextEdit::singleline(&mut binding.expr).code_editor().hint_text(target.example());
        changed |= ui.add_sized([width, ui.spacing().interact_size.y], edit).changed();
    });
    if !options.is_empty() {
        ui.horizontal(|ui| {
            ui.add_space(ui.spacing().indent);
            ui.label(RichText::new("app value:").weak()).on_hover_text("A value your app registered: pick one to bind to it.");
            let current = options.iter().find(|(n, _)| n == binding.expr.trim()).map(|(n, _)| n.clone());
            egui::ComboBox::from_id_salt(("pie_binding_vars", target))
                .selected_text(current.clone().unwrap_or_else(|| "pick\u{2026}".to_string()))
                .show_ui(ui, |ui| {
                    for (name, value) in &options {
                        ui.horizontal(|ui| {
                            var_swatch(ui, value);
                            if ui.selectable_label(current.as_ref() == Some(name), name).clicked() && current.as_ref() != Some(name) {
                                binding.expr = name.clone();
                                changed = true;
                            }
                        });
                    }
                });
        });
    }
    let mismatch;
    let (note, warn) = if binding.active_expr().is_none() {
        (Some("No name yet: the value above is used."), true)
    } else if let Some(k) = other_kind {
        mismatch = format!("That is {} of your app; this needs {}.", k.name(), kind.name());
        (Some(mismatch.as_str()), true)
    } else if binding.mode == BindingMode::Live && matches!(target, BindTarget::Label(_) | BindTarget::ItemLabel(..)) {
        (Some("A live label can change its automatic mnemonic key as the text changes."), true)
    } else if unknown {
        (Some("Not a variable of your app: a TOML file keeps the value above; generated code uses this as Rust."), true)
    } else if from_app {
        mismatch = match &code {
            Some(c) => format!("From your app: the value above follows it. In generated code: {c}"),
            None => "From your app: the value above follows it.".to_string(),
        };
        (Some(mismatch.as_str()), false)
    } else {
        (None, false)
    };
    if let Some(note) = note {
        ui.horizontal_wrapped(|ui| {
            ui.add_space(ui.spacing().indent);
            let text = RichText::new(note).small();
            ui.label(if warn { text.color(Color32::from_rgb(220, 160, 60)) } else { text.weak() });
        });
    }
    changed
}

/// An "Id" row for the button or cluster item `key`: a text field and, when your program
/// registers the `actions` it handles, a picker of them and a warning for an id that is not
/// one. Returns `true` if the id changed.
pub fn id_editor(ui: &mut Ui, ids: &mut crate::ButtonIds, key: &crate::IdKey, actions: &crate::Actions) -> bool {
    let mut id = match key.item {
        Some(n) => ids.item(&key.path, n),
        None => ids.button(&key.path),
    }
    .unwrap_or_default()
    .to_string();
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label("Id").on_hover_text("Names this for your code: the action your program runs when it is selected.");
        changed |= ui.add(egui::TextEdit::singleline(&mut id).hint_text("edit.copy").desired_width(140.0)).changed();
        if !actions.is_empty() {
            egui::ComboBox::from_id_salt(("pie_id_actions", key)).selected_text("\u{2026}").width(24.0).show_ui(ui, |ui| {
                for a in actions.ids() {
                    if ui.selectable_label(a == id, a).clicked() {
                        id = a.to_string();
                        changed = true;
                    }
                }
            });
        }
    });
    if !id.is_empty() && !actions.is_empty() && !actions.contains(&id) {
        ui.horizontal_wrapped(|ui| {
            ui.add_space(ui.spacing().indent);
            ui.label(RichText::new("Not an action of your program.").small().color(Color32::from_rgb(220, 160, 60)));
        });
    }
    if changed {
        ids.set(key.clone(), id);
    }
    changed
}

/// A small sample of a variable's value for the picker: a colour swatch, a stroke's colour.
fn var_swatch(ui: &mut Ui, value: &VarValue) {
    let size = egui::vec2(ui.spacing().interact_size.y * 0.6, ui.spacing().interact_size.y * 0.6);
    match value {
        VarValue::Color(c) => {
            egui::color_picker::show_color(ui, *c, size);
        }
        VarValue::Stroke(st) => {
            egui::color_picker::show_color(ui, st.color, size);
        }
        VarValue::Bool(on) => {
            ui.label(RichText::new(if *on { "on" } else { "off" }).small().weak());
        }
        VarValue::Number(n) => {
            ui.label(RichText::new(format!("{n}")).small().weak());
        }
        VarValue::Font(_) | VarValue::Text(_) => {}
    }
}

/// `editor` followed by [`binding_toggle`] on one row (so the labels stay aligned with
/// unbound rows), then [`binding_details`]. While the target follows one of your app's
/// [`Variables`], the editor is shown disabled (its value comes from [`Bindings::apply`]).
/// Returns whether the editor or the binding changed.
pub(crate) fn bound_row(ui: &mut Ui, bindings: &mut Bindings, target: &BindTarget, editor: impl FnOnce(&mut Ui) -> bool) -> bool {
    let mut changed = false;
    let from_app = bindings.variable_for(target).is_some();
    ui.horizontal(|ui| {
        changed |= ui.add_enabled_ui(!from_app, editor).inner;
        changed |= binding_toggle(ui, bindings, target);
    });
    changed | binding_details(ui, bindings, target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ClusterItem, PieDirection};

    #[test]
    fn free_identifiers_are_the_roots() {
        assert_eq!(free_identifiers("theme.accent"), ["theme"]);
        assert_eq!(free_identifiers("tr(\"edit.copy\")"), ["tr"]);
        assert_eq!(free_identifiers("format!(\"Undo ({count})\", history.len())"), ["history"]);
        assert_eq!(free_identifiers("egui::Stroke::new(2.0, theme.accent)"), ["theme"]);
        assert_eq!(free_identifiers("egui::Color32::RED"), Vec::<String>::new());
        assert_eq!(free_identifiers("Some(self.x)"), Vec::<String>::new());
        assert_eq!(free_identifiers("if dark { a } else { b }"), ["dark", "a", "b"]);
        assert_eq!(free_identifiers("'x'.to_string() + suffix"), ["suffix"]);
    }

    fn label(p: &[usize]) -> BindTarget {
        BindTarget::Label(p.to_vec())
    }

    #[test]
    fn bindings_follow_the_tree() {
        let mut b = Bindings::new();
        for p in [&[0][..], &[1], &[1, 0], &[2], &[2, 3]] {
            b.set(label(p), Binding::new(format!("x{p:?}"), BindingMode::Build));
        }
        b.set(BindTarget::ItemLabel(vec![2], 1), Binding::default());
        b.set(BindTarget::Field(SettingsField::TextColor), Binding::default());
        b.follow(&TreeEdit::Removed(vec![1]));
        assert!(b.get(&label(&[1, 0])).is_none(), "the removed button's subtree goes");
        assert_eq!(b.get(&label(&[1])).map(|x| x.expr.as_str()), Some("x[2]"), "later siblings move down");
        assert_eq!(b.get(&label(&[1, 3])).map(|x| x.expr.as_str()), Some("x[2, 3]"));
        assert!(b.get(&BindTarget::ItemLabel(vec![1], 1)).is_some());
        assert!(b.get(&BindTarget::Field(SettingsField::TextColor)).is_some(), "fields are not paths");
        b.follow(&TreeEdit::Dropped(vec![1]));
        assert!(b.get(&label(&[1, 3])).is_none());
        assert!(b.get(&label(&[1])).is_some(), "the converted button keeps its own label binding");
        assert!(b.get(&BindTarget::ItemLabel(vec![1], 1)).is_none());
        b.follow(&TreeEdit::Added(vec![0]));
        assert!(b.get(&label(&[0])).is_none(), "a stale binding at an appended path is dropped");
    }

    #[test]
    fn renumber_and_prune() {
        let buttons = vec![PieButton::cluster(PieDirection::North).with_items(vec![ClusterItem::new(1, "a"), ClusterItem::new(4, "b")])];
        let mut b = Bindings::new();
        b.set(BindTarget::ItemLabel(vec![0], 2), Binding::default());
        b.set(label(&[0]), Binding::default());
        b.set(label(&[5]), Binding::default());
        b.renumber_item(&[0], 2, 4);
        b.prune(&buttons);
        assert_eq!(b.len(), 2);
        assert!(b.get(&BindTarget::ItemLabel(vec![0], 4)).is_some());
        assert!(b.get(&label(&[5])).is_none());
    }

    #[test]
    fn active_and_mode_filtered_expressions() {
        let mut b = Bindings::new();
        let t = BindTarget::Title;
        b.set(t.clone(), Binding::new("  ", BindingMode::Build));
        assert_eq!(b.expr(&t, BindingMode::Build), None, "blank is unused");
        b.set(t.clone(), Binding::new(" tr(\"t\") ", BindingMode::Live));
        assert_eq!(b.expr(&t, BindingMode::Live), Some("tr(\"t\")"));
        assert_eq!(b.expr(&t, BindingMode::Build), None);
    }

    #[test]
    fn the_glyphs_are_in_the_default_fonts() {
        let ctx = egui::Context::default();
        let font = egui::FontId::proportional(14.0);
        let mut found = false;
        let mut out = ctx.run_ui(Default::default(), |ui| {
            found = ui.ctx().fonts_mut(|f| f.has_glyph(&font, LINK));
        });
        out.textures_delta.clear();
        assert!(found, "the bind toggle glyph is missing from the default fonts");
    }

    fn app_vars() -> Variables {
        let mut v = Variables::new();
        v.color("theme.accent", Color32::from_rgb(1, 2, 3))
            .stroke("theme.outline", Stroke::new(3.0, Color32::RED))
            .font("theme.title_font", FontId::monospace(22.0))
            .text("tr(\"copy\")", "Kopieren")
            .text("tr(\"bold\")", "Fett");
        v
    }

    #[test]
    fn variables_register_and_replace() {
        let mut v = app_vars();
        assert_eq!(v.len(), 5);
        v.color("theme.accent", Color32::WHITE);
        assert_eq!(v.len(), 5, "same name replaces");
        assert_eq!(v.get("theme.accent"), Some(&VarValue::Color(Color32::WHITE)));
        let texts: Vec<&str> = v.of_kind(VarKind::Text).map(|(n, _)| n).collect();
        assert_eq!(texts, ["tr(\"copy\")", "tr(\"bold\")"]);
        assert_eq!(BindTarget::Field(SettingsField::TitleBackgroundStroke).kind(), VarKind::Stroke);
        assert_eq!(BindTarget::Field(SettingsField::TextColor).kind(), VarKind::Color);
        assert_eq!(BindTarget::Field(SettingsField::TitleTextFont).kind(), VarKind::Font);
        assert_eq!(BindTarget::Title.kind(), VarKind::Text);
        // Every bindable field takes a value of its kind through apply's writer.
        for f in SettingsField::ALL.into_iter().filter(|f| f.is_bindable()) {
            let value = match BindTarget::Field(f).kind() {
                VarKind::Color => VarValue::Color(Color32::from_rgb(9, 9, 9)),
                VarKind::Stroke => VarValue::Stroke(Stroke::new(9.0, Color32::BLUE)),
                VarKind::Font => VarValue::Font(FontId::monospace(9.0)),
                VarKind::Number => VarValue::Number(9.0),
                VarKind::Text | VarKind::Bool => unreachable!(),
            };
            let mut s = PieMenuSettings::default();
            assert!(set_field(&mut s, f, &value), "{f:?}");
            assert!(!set_field(&mut s, f, &value), "{f:?}: unchanged the second time");
            assert!(!set_field(&mut s, f, &VarValue::Text("x".into())), "{f:?}: wrong kind is ignored");
        }
    }

    #[test]
    fn apply_writes_variable_values() {
        let mut buttons = vec![
            PieButton::new(PieDirection::North).with_label("Copy"),
            PieButton::cluster(PieDirection::South).with_items(vec![ClusterItem::new(1, "Bold")]),
        ];
        let mut settings = PieMenuSettings::default();
        let mut b = Bindings::new();
        b.set(BindTarget::Field(SettingsField::TextColor), Binding::new("theme.accent", BindingMode::Build));
        b.set(BindTarget::Field(SettingsField::CenterHighlightStroke), Binding::new("theme.outline", BindingMode::Live));
        b.set(BindTarget::Field(SettingsField::TitleTextFont), Binding::new("theme.title_font", BindingMode::Build));
        b.set(BindTarget::Label(vec![0]), Binding::new(" tr(\"copy\") ", BindingMode::Build));
        b.set(BindTarget::ItemLabel(vec![1], 1), Binding::new("tr(\"bold\")", BindingMode::Live));
        // Wrong kind and unknown name: left alone.
        b.set(BindTarget::Field(SettingsField::TitleTextColor), Binding::new("theme.title_font", BindingMode::Build));
        b.set(BindTarget::Title, Binding::new("tr(\"nope\")", BindingMode::Build));
        assert!(!b.apply(&mut settings, &mut buttons), "no variables yet");
        b.set_variables(app_vars());
        assert!(b.apply(&mut settings, &mut buttons));
        assert_eq!(settings.text_color, Some(Color32::from_rgb(1, 2, 3)));
        assert_eq!(settings.center_indicator.highlight_stroke, Stroke::new(3.0, Color32::RED));
        assert_eq!(settings.title.text_font, FontId::monospace(22.0));
        assert_eq!(settings.title.text_color, PieMenuSettings::default().title.text_color);
        assert_eq!(buttons[0].label, "Kopieren");
        assert_eq!(buttons[1].cluster_items()[0].label, "Fett");
        assert!(!b.apply(&mut settings, &mut buttons), "nothing left to change");
        assert!(b.variable_for(&BindTarget::Title).is_none());
        assert!(b.variable_for(&BindTarget::Field(SettingsField::TitleTextColor)).is_none(), "a font is not a colour");
        // A clone keeps the variables (only serialization leaves them out).
        assert_eq!(b.clone().variables().len(), 5);
    }

    #[test]
    fn bound_rows_draw_with_variables() {
        let mut b = Bindings::new();
        b.set_variables(app_vars());
        let targets = [
            BindTarget::Field(SettingsField::TextColor),
            BindTarget::Field(SettingsField::CenterHighlightStroke),
            BindTarget::Field(SettingsField::TitleTextColor),
            BindTarget::Label(vec![0]),
        ];
        b.set(targets[0].clone(), Binding::new("theme.accent", BindingMode::Build));
        b.set(targets[1].clone(), Binding::new("", BindingMode::Build));
        b.set(targets[2].clone(), Binding::new("theme.title_font", BindingMode::Build));
        b.set(targets[3].clone(), Binding::new("tr(\"copy\")", BindingMode::Live));
        let before = b.clone();
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(Default::default(), |ui| {
            for t in &targets {
                let mut editor_enabled = None;
                let changed = bound_row(ui, &mut b, t, |ui| {
                    editor_enabled = Some(ui.is_enabled());
                    false
                });
                assert!(!changed);
                let follows = matches!(t, BindTarget::Field(SettingsField::TextColor) | BindTarget::Label(_));
                assert_eq!(editor_enabled, Some(!follows), "{t:?}: the editor is disabled while it follows a variable");
            }
        });
        out.textures_delta.clear();
        assert_eq!(b, before);
    }

    #[test]
    fn apply_keeps_the_fallback() {
        let mut buttons = vec![
            PieButton::new(PieDirection::North).with_label("Undo"),
            PieButton::cluster(PieDirection::South).with_items(vec![ClusterItem::new(1, "B")]),
        ];
        let mut settings = PieMenuSettings::default();
        let mut title = "Edit".to_string();
        let mut b = Bindings::new();
        b.set(BindTarget::Label(vec![0]), Binding::new("i18n.undo", BindingMode::Build));
        b.set(BindTarget::Enabled(vec![0]), Binding::new("app.can_undo", BindingMode::Build));
        b.set(BindTarget::ItemEnabled(vec![1], 1), Binding::new("app.bold", BindingMode::Build));
        b.set(BindTarget::Field(SettingsField::TitleTextColor), Binding::new("theme.text", BindingMode::Build));
        b.set(BindTarget::Title, Binding::new("i18n.edit", BindingMode::Build));
        let mut vars = Variables::new();
        vars.text("i18n.undo", "Отменить").flag("app.can_undo", false).flag("app.bold", false).color("theme.text", Color32::RED);
        vars.text("i18n.edit", "Правка");
        b.set_variables(vars.clone());
        assert!(b.apply(&mut settings, &mut buttons));
        assert!(b.apply_title(&mut title));
        assert_eq!((buttons[0].label.as_str(), buttons[0].enabled, title.as_str()), ("Отменить", false, "Правка"));
        assert!(!buttons[1].cluster_items()[0].enabled);
        assert_eq!(settings.title.text_color, Color32::RED);
        // The values before stay as fallbacks, through later changes of the variables.
        vars.text("i18n.undo", "Rückgängig");
        b.set_variables(vars);
        assert!(b.apply(&mut settings, &mut buttons));
        assert_eq!(buttons[0].label, "Rückgängig");
        assert_eq!(b.fallback(&BindTarget::Label(vec![0])), Some(Some(&VarValue::Text("Undo".into()))));
        assert_eq!(b.fallback(&BindTarget::Enabled(vec![0])), Some(Some(&VarValue::Bool(true))));
        assert_eq!(b.fallback(&BindTarget::Title), Some(Some(&VarValue::Text("Edit".into()))));
        let default_colour = VarValue::Color(PieMenuSettings::default().title.text_color);
        assert_eq!(b.fallback(&BindTarget::Field(SettingsField::TitleTextColor)), Some(Some(&default_colour)));
        // An optional colour that was off: written, with "off" as its fallback.
        b.set(BindTarget::Field(SettingsField::TextColor), Binding::new("theme.text", BindingMode::Build));
        assert!(b.apply(&mut settings, &mut buttons));
        assert_eq!(settings.text_color, Some(Color32::RED));
        assert_eq!(b.fallback(&BindTarget::Field(SettingsField::TextColor)), Some(None));
        // They follow the tree, and go with their binding.
        b.follow(&TreeEdit::Removed(vec![0]));
        assert_eq!(b.fallback(&BindTarget::ItemEnabled(vec![0], 1)), Some(Some(&VarValue::Bool(true))));
        b.renumber_item(&[0], 1, 7);
        assert!(b.get(&BindTarget::ItemEnabled(vec![0], 7)).is_some());
        assert!(b.fallback(&BindTarget::ItemEnabled(vec![0], 7)).is_some());
        b.remove(&BindTarget::Title);
        assert!(b.fallback(&BindTarget::Title).is_none());
        assert_eq!(BindTarget::Enabled(vec![0]).kind(), VarKind::Bool);
    }

    #[test]
    fn bindable_fields() {
        let bindable: Vec<SettingsField> = SettingsField::ALL.into_iter().filter(|f| f.is_bindable()).collect();
        assert_eq!(bindable.len(), 10 + 19);
        assert!(SettingsField::LayoutRadius.is_bindable() && SettingsField::LayoutRadius.is_number());
        assert!(!SettingsField::ShowBehavior.is_bindable() && !SettingsField::TitleShow.is_bindable());
        assert!(SettingsField::SlideFade.is_duration() && SettingsField::SlideFade.is_optional_value());
        // Optional exactly where the settings' value is an Option (its code is Some(..) / None).
        let mut every_on = PieMenuSettings::default();
        every_on.number_hints.color = Some(Color32::WHITE);
        every_on.text_color = Some(Color32::WHITE);
        let entries = super::super::settings_entries(&every_on);
        for f in bindable {
            let code = &entries.iter().find(|(p, _)| p == f.path()).expect("an entry per field").1;
            assert_eq!(f.is_optional_value(), code.starts_with("Some(") || code == "None", "{f:?}: {code}");
        }
    }
}
