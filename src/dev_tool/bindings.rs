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
}

impl BindTarget {
    /// The path of the button this target belongs to (`None` for a field or the title).
    pub fn path(&self) -> Option<&[usize]> {
        match self {
            BindTarget::Label(p) | BindTarget::ItemLabel(p, _) => Some(p),
            BindTarget::Field(_) | BindTarget::Title => None,
        }
    }

    fn path_mut(&mut self) -> Option<&mut Vec<usize>> {
        match self {
            BindTarget::Label(p) | BindTarget::ItemLabel(p, _) => Some(p),
            BindTarget::Field(_) | BindTarget::Title => None,
        }
    }

    /// Whether the target is text (a label or the title) rather than a settings value.
    pub fn is_text(&self) -> bool {
        !matches!(self, BindTarget::Field(_))
    }

    /// The kind of value the target holds (what a [`Variables`] entry must be to fill it).
    pub fn kind(&self) -> VarKind {
        use SettingsField as F;
        match self {
            BindTarget::Field(F::CenterBackgroundStroke | F::CenterHighlightStroke | F::TitleBackgroundStroke) => VarKind::Stroke,
            BindTarget::Field(F::TitleTextFont) => VarKind::Font,
            BindTarget::Field(_) => VarKind::Color,
            BindTarget::Title | BindTarget::Label(_) | BindTarget::ItemLabel(..) => VarKind::Text,
        }
    }

    /// An example expression, shown as the hint of an empty expression.
    pub fn example(&self) -> &'static str {
        use SettingsField as F;
        match self {
            BindTarget::Field(F::TitleTextFont) => "theme.title_font",
            BindTarget::Field(F::CenterBackgroundStroke | F::CenterHighlightStroke | F::TitleBackgroundStroke) => {
                "egui::Stroke::new(2.0, theme.accent)"
            }
            BindTarget::Field(_) => "theme.accent",
            BindTarget::Title => "tr(\"menu.edit\")",
            BindTarget::Label(_) | BindTarget::ItemLabel(..) => "tr(\"edit.copy\")",
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
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct Bindings {
    entries: Vec<(BindTarget, Binding)>,
    /// What your app registered this frame ([`Bindings::set_variables`]); not saved.
    #[cfg_attr(feature = "serde", serde(skip))]
    variables: Variables,
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
        Some(self.entries.remove(i).1)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&BindTarget, &Binding)> {
        self.entries.iter().map(|(t, b)| (t, b))
    }

    /// The expression of `target` if it is bound in `mode` with a non-empty expression.
    pub fn expr(&self, target: &BindTarget, mode: BindingMode) -> Option<&str> {
        self.get(target).filter(|b| b.mode == mode).and_then(Binding::active_expr)
    }

    /// Follows a change to the tree's shape (as reported in [`super::InspectorOutput::edits`])
    /// so every label binding stays with its button. [`super::button_inspector_bound`] does
    /// this itself.
    pub fn follow(&mut self, edit: &TreeEdit) {
        match edit {
            TreeEdit::Removed(path) => {
                let Some((last, parent)) = path.split_last() else {
                    return;
                };
                let depth = parent.len();
                self.entries.retain_mut(|(target, _)| {
                    let Some(key) = target.path_mut() else {
                        return true;
                    };
                    if key.starts_with(path) {
                        return false;
                    }
                    if key.len() > depth && key.starts_with(parent) && key[depth] > *last {
                        key[depth] -= 1;
                    }
                    true
                });
            }
            TreeEdit::Dropped(path) => self.entries.retain(|(t, _)| match t {
                BindTarget::Label(key) => !(key.len() > path.len() && key.starts_with(path)),
                // The cluster's items went (or it became a cluster): their bindings go too.
                BindTarget::ItemLabel(key, _) => !key.starts_with(path),
                _ => true,
            }),
            TreeEdit::Added(path) => self.entries.retain(|(t, _)| t.path().is_none_or(|key| !key.starts_with(path))),
        }
    }

    /// The cluster item `from` of the cluster at `path` now has the number `to`.
    pub fn renumber_item(&mut self, path: &[usize], from: usize, to: usize) {
        for (t, _) in &mut self.entries {
            if let BindTarget::ItemLabel(p, n) = t
                && p == path
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
    /// [`Bindings::set_variables`] and before drawing). The title is not touched: read it with
    /// [`Bindings::variable_for`]`(&BindTarget::Title)`. Returns `true` if anything changed.
    pub fn apply(&self, settings: &mut PieMenuSettings, buttons: &mut [PieButton]) -> bool {
        let mut changed = false;
        for (target, _) in &self.entries {
            let Some(value) = self.variable_for(target) else { continue };
            changed |= match (target, value) {
                (BindTarget::Field(f), v) => set_field(settings, *f, v),
                (BindTarget::Label(p), VarValue::Text(t)) => crate::button_at_mut(buttons, p).is_some_and(|b| set_if_changed(&mut b.label, t)),
                (BindTarget::ItemLabel(p, n), VarValue::Text(t)) => crate::button_at_mut(buttons, p)
                    .and_then(|b| b.cluster_item_mut(*n))
                    .is_some_and(|i| set_if_changed(&mut i.label, t)),
                _ => false,
            };
        }
        changed
    }

    /// Forgets the bindings of buttons and cluster items that are not in `buttons`.
    pub fn prune(&mut self, buttons: &[PieButton]) {
        self.entries.retain(|(t, _)| match t {
            BindTarget::Label(p) => crate::button_at(buttons, p).is_some(),
            BindTarget::ItemLabel(p, n) => crate::button_at(buttons, p).is_some_and(|b| b.cluster_items().iter().any(|i| i.number == *n)),
            _ => true,
        });
    }
}

fn set_if_changed<T: PartialEq + Clone>(slot: &mut T, value: &T) -> bool {
    if slot == value {
        return false;
    }
    *slot = value.clone();
    true
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
        _ => false,
    }
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
}

impl VarKind {
    pub fn name(self) -> &'static str {
        match self {
            VarKind::Color => "a colour",
            VarKind::Stroke => "a stroke",
            VarKind::Font => "a font",
            VarKind::Text => "text",
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
}

impl VarValue {
    pub fn kind(&self) -> VarKind {
        match self {
            VarValue::Color(_) => VarKind::Color,
            VarValue::Stroke(_) => VarKind::Stroke,
            VarValue::Font(_) => VarKind::Font,
            VarValue::Text(_) => VarKind::Text,
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

    /// Registers (or replaces) `name`, the expression that reads the value in your code.
    pub fn set(&mut self, name: impl Into<String>, value: VarValue) -> &mut Self {
        let name = name.into();
        match self.entries.iter_mut().find(|(n, _)| *n == name) {
            Some((_, v)) => *v = value,
            None => self.entries.push((name, value)),
        }
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

    pub fn get(&self, name: &str) -> Option<&VarValue> {
        self.entries.iter().find(|(n, _)| n == name).map(|(_, v)| v)
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
    /// Whether the field can be bound to an expression: the colours, strokes and fonts.
    pub fn is_bindable(self) -> bool {
        use SettingsField as F;
        matches!(
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
        matches!(self, SettingsField::HintColor | SettingsField::TextColor)
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
        "Bound: the generated code uses the expression below. Click to unbind."
    } else {
        "Bind to a value from your app: the generated code uses an expression (such as theme.accent) instead of this value."
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
    let Some(binding) = bindings.get_mut(target) else {
        return false;
    };
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.add_space(ui.spacing().indent);
        ui.label(RichText::new("code:").weak())
            .on_hover_text("The Rust expression the generated code uses. The value above stays as the preview value.");
        for (mode, name, hover) in [
            (BindingMode::Build, "Build", "Read once, where the menu is built (theme at startup, language)."),
            (BindingMode::Live, "Live", "Read every frame, in a block that runs before show (theme toggle, \"Undo (3)\")."),
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
        (Some("No expression yet: the code uses the value."), true)
    } else if let Some(k) = other_kind {
        mismatch = format!("That is {} of your app; this needs {}.", k.name(), kind.name());
        (Some(mismatch.as_str()), true)
    } else if binding.mode == BindingMode::Live && matches!(target, BindTarget::Label(_) | BindTarget::ItemLabel(..)) {
        (Some("A live label can change its automatic mnemonic key as the text changes."), true)
    } else if from_app {
        (Some("From your app: the value above follows it."), false)
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
                VarKind::Text => unreachable!(),
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
    fn bindable_fields() {
        let bindable: Vec<SettingsField> = SettingsField::ALL.into_iter().filter(|f| f.is_bindable()).collect();
        assert_eq!(bindable.len(), 10);
        assert!(!SettingsField::LayoutRadius.is_bindable());
        assert!(bindable.iter().all(|f| !f.is_optional_value() || f.path().ends_with("color")));
    }
}
