//! The binding widgets: the bind toggle and the row below a bound value (name, Build / Live,
//! the program's variables), the "Id" row, and the expression scanner of the generated code.
//! The bindings themselves ([`Bindings`], [`Variables`]) are the core crate's
//! (`egui_pie_menu::values`): a program uses them without the dev tool.
pub use egui_pie_menu::values::*;
use egui::{Color32, RichText, Ui};

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
    let options: Vec<(String, VarValue)> = bindings.variables().of_kind(kind).map(|(n, v)| (n.to_string(), v.clone())).collect();
    let other_kind = bindings
        .get(target)
        .and_then(Binding::active_expr)
        .and_then(|e| bindings.variables().get(e))
        .map(VarValue::kind)
        .filter(|k| *k != kind);
    let from_app = bindings.variable_for(target).is_some();
    let unknown = !bindings.variables().is_empty()
        && bindings.get(target).and_then(Binding::active_expr).is_some_and(|e| bindings.variables().get(e).is_none());
    let code = bindings
        .get(target)
        .and_then(Binding::active_expr)
        .map(|e| bindings.variables().code_for(e))
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
pub fn id_editor(ui: &mut Ui, ids: &mut egui_pie_menu::ButtonIds, key: &egui_pie_menu::IdKey, actions: &egui_pie_menu::Actions) -> bool {
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
    use egui_pie_menu::{PieMenuSettings, SettingsField};
    use egui::{FontId, Stroke};

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
        assert_eq!(bindable.len(), 10 + 19);
        assert!(SettingsField::LayoutRadius.is_bindable() && SettingsField::LayoutRadius.is_number());
        assert!(!SettingsField::ShowBehavior.is_bindable() && !SettingsField::TitleShow.is_bindable());
        assert!(SettingsField::SlideFade.is_duration() && SettingsField::SlideFade.is_optional_value());
        // Optional exactly where the settings' value is an Option (its code is Some(..) / None).
        let mut every_on = PieMenuSettings::default();
        every_on.number_hints.color = Some(Color32::WHITE);
        every_on.text_color = Some(Color32::WHITE);
        let entries = crate::settings_entries(&every_on);
        for f in bindable {
            let code = &entries.iter().find(|(p, _)| p == f.path()).expect("an entry per field").1;
            assert_eq!(f.is_optional_value(), code.starts_with("Some(") || code == "None", "{f:?}: {code}");
        }
    }
}
