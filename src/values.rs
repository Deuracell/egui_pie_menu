//! Values a program supplies to its menus: [`Variables`] (the program's values, by name) and
//! [`Bindings`] (which parts of a menu take which variable). A menu loaded from TOML refers to
//! variables by name (`label = { var = "i18n.copy", default = "Copy" }`); the program
//! registers them and [`Bindings::apply`] puts their current values in. The designer
//! (`egui_pie_menu_designer`) edits bindings and generates Rust code from them.
use crate::PieButton;
use crate::button_ids::TreeEdit;
use crate::settings::PieMenuSettings;
use crate::settings_fields::SettingsField;
use egui::{Color32, FontId, Stroke};

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

/// The bindings of one menu: which parts take which variable. A menu loaded from TOML comes
/// with its own (the `catalog` module, feature `toml`); in the designer they are edited in the
/// inspector and the settings editors, and turned into Rust code.
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

    /// Follows a change to the tree's shape ([`crate::TreeEdit`]) so every binding of a
    /// button stays with it.
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
        (BindTarget::Enabled(p), VarValue::Bool(on)) => {
            crate::button_at_mut(buttons, p).is_some_and(|b| set_if_changed(&mut b.enabled, on))
        }
        (BindTarget::Visible(p), VarValue::Bool(on)) => {
            crate::button_at_mut(buttons, p).is_some_and(|b| set_if_changed(&mut b.visible, on))
        }
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
        (
            F::CenterHighlightCircleRadius,
            |s| Some(s.center_indicator.highlight_circle_radius),
            |s, n| s.center_indicator.highlight_circle_radius = n,
        ),
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
/// use egui_pie_menu::values::*;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ClusterItem, PieDirection};

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
}
