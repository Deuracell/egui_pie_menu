//! Menus defined in TOML files, loaded at runtime (feature `toml`).
//!
//! A file holds any number of named menus. A button opens another menu by its name, so a
//! sub-pie can live in another file and one menu can be the sub-pie of several buttons.
//! [`MenuCatalog`] collects files: a menu defined in several of them (or twice in one) is
//! merged, its buttons added together and later values replacing earlier ones, so a plugin
//! file can add a button to the `edit` menu of the base file.
//!
//! ```toml
//! [menu.edit]
//! title = { var = "i18n.edit", default = "Edit" }
//!
//! [menu.edit.settings]
//! layout = { radius = 120.0 }
//! title = { text_color = "#ffcc00" }
//! animations = { reveal_duration = 150 }   # milliseconds
//! center_indicator = { highlight_fill_color = { var = "theme.accent", default = "#8000ff" } }
//!
//! [[menu.edit.button]]
//! dir = "N"
//! label = { var = "i18n.copy", default = "Copy" }
//! id = "edit.copy"
//! mnemonic = "c"
//!
//! [[menu.edit.button]]
//! dir = "E"
//! label = "Paste Special"
//! opens = "paste_special"      # another menu, in this file or another
//! style = "fan_out"            # optional: "replace" or "fan_out"
//!
//! [[menu.edit.button]]
//! dir = "S"
//! label = "Format"
//! items = [{ number = 1, label = "Bold", id = "fmt.bold" }, { number = 2, label = "Italic" }]
//!
//! [[menu.paste_special.button]]
//! dir = "N"
//! label = "Values only"
//! id = "paste.values"
//! ```
//!
//! A button is a plain button, a sub-pie (`opens`), a back button (`back = true`) or a
//! cluster (`items`). `dir` is `N`, `NE`, `E`, .. `NW` (or `North`, `NorthEast`, ..);
//! `mnemonic` is one character or `"none"` (left out: automatic); `id` names the button for
//! your code ([`MenuDefinition::ids`]): the action your program runs, and whose state
//! (enabled, visible) it sets with [`crate::Actions`] ([`MenuDefinition::apply_actions`]).
//! Files never say whether a button is enabled or visible.
//!
//! **Removing and changing.** `[menu.<name>] remove = ["id", ..]` takes buttons (with their
//! sub-pies) or cluster items out of the menu, and `[[menu.<name>.change]]` with an `id`
//! changes the fields it gives (`dir`, `label`, `mnemonic`, `opens`, `style`, `items`; a
//! cluster item's `label` and `mnemonic`), wherever they were defined. They apply when the
//! menu is built, after every file: changes in file order, then removals, then the check
//! that no two buttons share a direction. An id that is not there is noted
//! ([`MenuDefinition::notes`]), not an error.
//!
//! **Values from your program.** A label, an item label, the title, and the colours, strokes
//! and fonts of the settings ([`SettingsField::is_bindable`]) can each be a literal or a
//! reference to one of your app's [`Variables`] by name: `{ var = "i18n.copy", default =
//! "Copy" }`. [`MenuDefinition::apply_variables`] puts the variable's value in; `default` is
//! used until then, or when your app has no such variable (left out: the name itself for
//! text, the library default for a setting).
//!
//! `settings` uses the paths of [`PieMenuSettings`] (as in [`SettingsField::path`]); only
//! what differs from the default needs to be there. Colours are `"#rrggbb"` or
//! `"#rrggbbaa"`, durations are milliseconds, and `false` switches off an optional value
//! (`background_radius = false`). A menu's `title` and `settings` apply when it is built as a
//! menu of its own; where it opens as a sub-pie, only its buttons (and its `style`) count.
use crate::settings_fields::SettingsField;
use crate::values::{BindTarget, Binding, BindingMode, Bindings, VarValue, Variables};
use crate::{
    Actions, ButtonIds, ClusterItem, IdKey, Mnemonic, PieButton, PieButtonKind, PieDirection, PieMenu, PieMenuSettings, SubPieStyle,
};
use egui::Color32;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;
use toml::{Table, Value};

// ---------------------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------------------

/// Why a file could not be added or a menu not built: `source` is the file (or, for a
/// build, the menu) and `message` says what is wrong.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogError {
    pub source: String,
    pub message: String,
}

impl CatalogError {
    fn new(source: impl Into<String>, message: impl Into<String>) -> Self {
        Self { source: source.into(), message: message.into() }
    }
}

impl fmt::Display for CatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.source, self.message)
    }
}

impl std::error::Error for CatalogError {}

// ---------------------------------------------------------------------------------------
// The file format
// ---------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum StyleName {
    Replace,
    FanOut,
}

impl From<StyleName> for SubPieStyle {
    fn from(s: StyleName) -> Self {
        match s {
            StyleName::Replace => SubPieStyle::Replace,
            StyleName::FanOut => SubPieStyle::FanOut,
        }
    }
}

impl From<SubPieStyle> for StyleName {
    fn from(s: SubPieStyle) -> Self {
        match s {
            SubPieStyle::Replace => StyleName::Replace,
            SubPieStyle::FanOut => StyleName::FanOut,
        }
    }
}

/// A reference to one of your app's variables: `{ var = "i18n.copy", default = "Copy" }`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct VarRef<T> {
    var: String,
    #[serde(default = "none", skip_serializing_if = "Option::is_none")]
    default: Option<T>,
}

fn none<T>() -> Option<T> {
    None
}

/// Text: a literal (`"Copy"`) or a reference (`{ var = "i18n.copy", default = "Copy" }`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
enum TextDef {
    Literal(String),
    Var(VarRef<String>),
}

impl TextDef {
    /// `text`, as a reference to `var` if there is one.
    fn new(text: &str, var: Option<&str>) -> Self {
        match var {
            Some(v) => TextDef::Var(VarRef { var: v.to_string(), default: Some(text.to_string()) }),
            None => TextDef::Literal(text.to_string()),
        }
    }

    /// The text until a variable fills it: the literal, the default, else the name.
    fn text(&self) -> &str {
        match self {
            TextDef::Literal(t) => t,
            TextDef::Var(r) => r.default.as_deref().unwrap_or(&r.var),
        }
    }

    fn var(&self) -> Option<&str> {
        match self {
            TextDef::Literal(_) => None,
            TextDef::Var(r) => Some(&r.var),
        }
    }
}

impl Default for TextDef {
    fn default() -> Self {
        TextDef::Literal(String::new())
    }
}

/// A binding to the variable `name` (bindings from files have no Build / Live mode).
fn var_binding(name: &str) -> Binding {
    Binding::new(name, BindingMode::Build)
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ItemDef {
    number: usize,
    #[serde(default)]
    label: TextDef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mnemonic: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ButtonDef {
    dir: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    label: Option<TextDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mnemonic: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    opens: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    style: Option<StyleName>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    back: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    items: Option<Vec<ItemDef>>,
}

/// `[[menu.<name>.change]]`: changes the fields it gives of the button (or cluster item) with
/// this `id`, wherever it was defined.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ChangeDef {
    id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    label: Option<TextDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mnemonic: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    opens: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    style: Option<StyleName>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    items: Option<Vec<ItemDef>>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MenuDef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    title: Option<TextDef>,
    /// How the menu opens where a button opens it as a sub-pie (the button's own `style`
    /// wins); left out: the menu-wide setting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    style: Option<StyleName>,
    #[serde(default, skip_serializing_if = "Table::is_empty")]
    settings: Table,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    button: Vec<ButtonDef>,
    /// Ids of buttons or cluster items to take out of the menu, wherever they were defined.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    remove: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    change: Vec<ChangeDef>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileDef {
    #[serde(default)]
    menu: BTreeMap<String, MenuDef>,
}

/// A table key: bare where TOML allows, quoted otherwise.
fn key(k: &str) -> String {
    if !k.is_empty() && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        k.to_string()
    } else {
        Value::String(k.to_string()).to_string()
    }
}

/// `key = value` lines for the entries of `t`, values inline; the entries `section` picks
/// (by their path below `prefix`) are left out and returned to be written as sections.
fn lines<'a>(out: &mut String, t: &'a Table, prefix: &str, section: &dyn Fn(&str) -> bool) -> Vec<(&'a str, &'a Table)> {
    let mut later = Vec::new();
    for (k, v) in t {
        match v {
            Value::Table(sub) if section(&join(prefix, k)) => later.push((k.as_str(), sub)),
            // Several tables (cluster items, dismiss inputs): one per line.
            Value::Array(a) if a.len() > 1 && a.iter().all(Value::is_table) => {
                out.push_str(&format!("{} = [\n", key(k)));
                for x in a {
                    out.push_str(&format!("    {x},\n"));
                }
                out.push_str("]\n");
            }
            _ => out.push_str(&format!("{} = {v}\n", key(k))),
        }
    }
    later
}

/// Writes the menus as TOML: a `[menu.name]` section for the title and style (when there
/// is one), the settings by group, the bindings, then one `[[menu.name.button]]` per button
/// with its values on single lines (cluster items, strokes and fonts inline).
fn write_menus(menus: &[(String, MenuDef)]) -> String {
    let mut out = String::new();
    let mut section = |out: &mut String, header: String| {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&header);
        out.push('\n');
    };
    for (name, def) in menus {
        let base = format!("menu.{}", key(name));
        let empty = def.settings.is_empty() && def.button.is_empty();
        if def.title.is_some() || def.style.is_some() || empty {
            section(&mut out, format!("[{base}]"));
            if let Some(t) = &def.title {
                out.push_str(&format!("title = {}\n", Value::try_from(t).expect("title")));
            }
            if let Some(st) = def.style {
                out.push_str(&format!("style = {}\n", Value::try_from(st).expect("style")));
            }
        }
        if !def.settings.is_empty() {
            // The groups as sections, nested groups (sub_pies.fan_out) as sections of their own.
            fn write(out: &mut String, t: &Table, prefix: &str, header: &str, section: &mut dyn FnMut(&mut String, String)) {
                let mut body = String::new();
                let later = lines(&mut body, t, prefix, &|p| GROUPS.contains(&p));
                if !body.is_empty() {
                    section(out, format!("[{header}]"));
                    out.push_str(&body);
                }
                for (k, sub) in later {
                    write(out, sub, &join(prefix, k), &format!("{header}.{}", key(k)), section);
                }
            }
            write(&mut out, &def.settings, "", &format!("{base}.settings"), &mut section);
        }
        for b in &def.button {
            section(&mut out, format!("[[{base}.button]]"));
            if let Ok(Value::Table(t)) = Value::try_from(b) {
                lines(&mut out, &t, "", &|_| false);
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------------------
// Directions and mnemonics
// ---------------------------------------------------------------------------------------

const DIRECTION_NAMES: [(&str, PieDirection); 8] = [
    ("N", PieDirection::North),
    ("NE", PieDirection::NorthEast),
    ("E", PieDirection::East),
    ("SE", PieDirection::SouthEast),
    ("S", PieDirection::South),
    ("SW", PieDirection::SouthWest),
    ("W", PieDirection::West),
    ("NW", PieDirection::NorthWest),
];

fn parse_direction(text: &str) -> Option<PieDirection> {
    let t = text.trim();
    DIRECTION_NAMES.iter().find(|(short, d)| short.eq_ignore_ascii_case(t) || format!("{d:?}").eq_ignore_ascii_case(t)).map(|(_, d)| *d)
}

fn direction_name(d: PieDirection) -> &'static str {
    DIRECTION_NAMES.iter().find(|(_, x)| *x == d).map_or("N", |(n, _)| n)
}

fn parse_mnemonic(text: &str) -> Result<Mnemonic, String> {
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => Ok(Mnemonic::Key(c)),
        _ if text.eq_ignore_ascii_case("none") => Ok(Mnemonic::None),
        _ => Err(format!("mnemonic `{text}` is neither one character nor \"none\"")),
    }
}

fn mnemonic_name(m: Mnemonic) -> Option<String> {
    match m {
        Mnemonic::Auto => None,
        Mnemonic::Key(c) => Some(c.to_string()),
        Mnemonic::None => Some("none".to_string()),
    }
}

// ---------------------------------------------------------------------------------------
// Settings: the readable form (hex colours, milliseconds, `false` for off)
// ---------------------------------------------------------------------------------------

/// The settings groups (tables whose keys are merged and diffed one by one).
const GROUPS: [&str; 8] = ["layout", "input", "center_indicator", "title", "animations", "sub_pies", "sub_pies.fan_out", "number_hints"];

const DURATIONS: [&str; 6] = [
    "input.double_tap_window",
    "animations.reveal_duration",
    "animations.slide_fade",
    "animations.hover_duration",
    "sub_pies.hover_delay",
    "sub_pies.fan_out.delay",
];

/// The optional settings: `false` in a file means `None`.
const OPTIONALS: [&str; 9] = [
    "center_indicator.background_radius",
    "center_indicator.highlight_radius",
    "number_hints.color",
    "animations.slide_start_shape_factor",
    "animations.slide_fade",
    "layout.intercardinal_height",
    "sub_pies.fan_out.radius",
    "sub_pies.hover_delay",
    "text_color",
];

fn set_none(s: &mut PieMenuSettings, path: &str) {
    match path {
        "center_indicator.background_radius" => s.center_indicator.background_radius = None,
        "center_indicator.highlight_radius" => s.center_indicator.highlight_radius = None,
        "number_hints.color" => s.number_hints.color = None,
        "animations.slide_start_shape_factor" => s.animations.slide_start_shape_factor = None,
        "animations.slide_fade" => s.animations.slide_fade = None,
        "layout.intercardinal_height" => s.layout.intercardinal_height = None,
        "sub_pies.fan_out.radius" => s.sub_pies.fan_out.radius = None,
        "sub_pies.hover_delay" => s.sub_pies.hover_delay = None,
        "text_color" => s.text_color = None,
        _ => {}
    }
}

fn join(prefix: &str, key: &str) -> String {
    if prefix.is_empty() { key.to_string() } else { format!("{prefix}.{key}") }
}

fn hex(c: Color32) -> String {
    let [r, g, b, a] = c.to_srgba_unmultiplied();
    if a == 255 { format!("#{r:02x}{g:02x}{b:02x}") } else { format!("#{r:02x}{g:02x}{b:02x}{a:02x}") }
}

fn parse_hex(text: &str) -> Option<Color32> {
    let h = text.strip_prefix('#')?;
    let byte = |i: usize| h.get(i..i + 2).and_then(|x| u8::from_str_radix(x, 16).ok());
    match h.len() {
        6 => Some(Color32::from_rgb(byte(0)?, byte(2)?, byte(4)?)),
        8 => Some(Color32::from_rgba_unmultiplied(byte(0)?, byte(2)?, byte(4)?, byte(6)?)),
        _ => None,
    }
}

/// A serialized `Color32` (its premultiplied bytes).
fn as_color(v: &Value) -> Option<Color32> {
    let a = v.as_array().filter(|a| a.len() == 4)?;
    let bytes: Vec<u8> = a.iter().map(|x| x.as_integer().and_then(|i| u8::try_from(i).ok())).collect::<Option<_>>()?;
    Some(Color32::from_rgba_premultiplied(bytes[0], bytes[1], bytes[2], bytes[3]))
}

/// The serde form of a settings value, made readable.
fn readable(v: Value, path: &str) -> Value {
    if DURATIONS.contains(&path)
        && let Some(t) = v.as_table()
    {
        let secs = t.get("secs").and_then(Value::as_integer).unwrap_or(0);
        let nanos = t.get("nanos").and_then(Value::as_integer).unwrap_or(0);
        return if nanos % 1_000_000 == 0 {
            Value::Integer(secs * 1000 + nanos / 1_000_000)
        } else {
            Value::Float(secs as f64 * 1000.0 + nanos as f64 / 1e6)
        };
    }
    if let Some(c) = as_color(&v) {
        return Value::String(hex(c));
    }
    match v {
        // The settings' floats are f32: write the shortest decimal that reads back the same.
        Value::Float(x) => Value::Float(format!("{}", x as f32).parse().unwrap_or(x)),
        Value::Table(t) => Value::Table(t.into_iter().map(|(k, x)| (k.clone(), readable(x, &join(path, &k)))).collect()),
        Value::Array(a) => Value::Array(a.into_iter().map(|x| readable(x, path)).collect()),
        other => other,
    }
}

/// The readable form turned back into the serde form; `nones` collects the optional
/// settings switched off with `false`.
fn unreadable(v: Value, path: &str, nones: &mut Vec<String>) -> Result<Option<Value>, String> {
    Ok(Some(match v {
        Value::Boolean(false) if OPTIONALS.contains(&path) => {
            nones.push(path.to_string());
            return Ok(None);
        }
        Value::Integer(ms) if DURATIONS.contains(&path) => {
            if ms < 0 {
                return Err(format!("`{path}` is negative"));
            }
            duration_value(ms as f64)
        }
        Value::Float(ms) if DURATIONS.contains(&path) => {
            if ms < 0.0 {
                return Err(format!("`{path}` is negative"));
            }
            duration_value(ms)
        }
        Value::String(s) if s.starts_with('#') || colour_paths().iter().any(|p| p == path) => {
            let c = parse_hex(&s).ok_or_else(|| format!("`{path}`: `{s}` is not a colour (#rrggbb or #rrggbbaa)"))?;
            Value::Array(c.to_array().iter().map(|b| Value::Integer(i64::from(*b))).collect())
        }
        Value::Table(t) => {
            let mut out = Table::new();
            for (k, x) in t {
                if let Some(x) = unreadable(x, &join(path, &k), nones)? {
                    out.insert(k, x);
                }
            }
            Value::Table(out)
        }
        Value::Array(a) => {
            Value::Array(a.into_iter().map(|x| unreadable(x, path, nones)).collect::<Result<Vec<_>, _>>()?.into_iter().flatten().collect())
        }
        other => other,
    }))
}

/// The paths of the colours in the settings (strokes' included), in their readable form.
fn colour_paths() -> Vec<String> {
    fn walk(t: &Table, prefix: &str, out: &mut Vec<String>) {
        for (k, v) in t {
            let path = join(prefix, k);
            match v {
                Value::String(s) if s.starts_with('#') => out.push(path),
                Value::Table(sub) => walk(sub, &path, out),
                _ => {}
            }
        }
    }
    let mut out = vec!["number_hints.color".to_string(), "text_color".to_string()]; // None by default
    walk(&readable_settings(&PieMenuSettings::default()), "", &mut out);
    out
}

fn duration_value(ms: f64) -> Value {
    let nanos_total = (ms * 1e6).round() as i64;
    let mut t = Table::new();
    t.insert("secs".into(), Value::Integer(nanos_total / 1_000_000_000));
    t.insert("nanos".into(), Value::Integer(nanos_total % 1_000_000_000));
    Value::Table(t)
}

fn readable_settings(s: &PieMenuSettings) -> Table {
    match readable(Value::try_from(s).expect("settings serialize"), "") {
        Value::Table(t) => t,
        _ => Table::new(),
    }
}

/// Every key of `given` must be a settings group or field.
fn check_settings_keys(given: &Table, prefix: &str) -> Result<(), String> {
    for (k, v) in given {
        let path = join(prefix, k);
        if GROUPS.contains(&path.as_str()) {
            let t = v.as_table().ok_or_else(|| format!("settings `{path}` should be a table"))?;
            check_settings_keys(t, &path)?;
        } else if !SettingsField::ALL.iter().any(|f| f.path() == path) {
            return Err(format!("unknown setting `{path}`"));
        }
    }
    Ok(())
}

/// Merges `new` into `base`: groups key by key, anything else replaced.
fn merge_settings(base: &mut Table, new: &Table, prefix: &str) {
    for (k, v) in new {
        let path = join(prefix, k);
        match (base.get_mut(k), v) {
            (Some(Value::Table(b)), Value::Table(n)) if GROUPS.contains(&path.as_str()) => merge_settings(b, n, &path),
            _ => {
                base.insert(k.clone(), v.clone());
            }
        }
    }
}

/// The readable settings table: what differs from the default (`false` for an optional
/// setting that is off but on by default).
pub(crate) fn settings_to_table(s: &PieMenuSettings) -> Table {
    fn diff(cur: &Table, def: &Table, prefix: &str) -> Table {
        let mut out = Table::new();
        for (k, v) in cur {
            let path = join(prefix, k);
            match (def.get(k), v) {
                (Some(Value::Table(d)), Value::Table(c)) if GROUPS.contains(&path.as_str()) => {
                    let sub = diff(c, d, &path);
                    if !sub.is_empty() {
                        out.insert(k.clone(), Value::Table(sub));
                    }
                }
                (Some(d), v) if d == v => {}
                _ => {
                    out.insert(k.clone(), v.clone());
                }
            }
        }
        for k in def.keys().filter(|k| !cur.contains_key(*k)) {
            if OPTIONALS.contains(&join(prefix, k).as_str()) {
                out.insert(k.clone(), Value::Boolean(false));
            }
        }
        out
    }
    diff(&readable_settings(s), &readable_settings(&PieMenuSettings::default()), "")
}

/// Takes the references (`{ var = .., default = .. }`) out of a settings table: each is
/// replaced by its default (or left out) and returned with its field.
fn take_refs(t: &mut Table, prefix: &str, refs: &mut Vec<(SettingsField, String)>) -> Result<(), String> {
    let keys: Vec<String> = t.keys().cloned().collect();
    for k in keys {
        let path = join(prefix, &k);
        if GROUPS.contains(&path.as_str()) {
            if let Some(Value::Table(sub)) = t.get_mut(&k) {
                take_refs(sub, &path, refs)?;
            }
            continue;
        }
        let Some(Value::Table(v)) = t.get(&k) else { continue };
        if !v.contains_key("var") {
            continue;
        }
        let field = SettingsField::ALL.into_iter().find(|f| f.path() == path).ok_or_else(|| format!("unknown setting `{path}`"))?;
        if !field.is_bindable() {
            return Err(format!("`{path}` cannot come from a variable (colours, strokes, fonts, numbers and durations can)"));
        }
        let r: VarRef<Value> = Value::Table(v.clone()).try_into().map_err(|e: toml::de::Error| format!("`{path}`: {}", e.message()))?;
        refs.push((field, r.var));
        match r.default {
            Some(d) => {
                t.insert(k, d);
            }
            None => {
                t.remove(&k);
            }
        }
    }
    Ok(())
}

/// The settings a readable table describes (the default for everything left out), and the
/// fields that refer to a variable (with its name).
pub(crate) fn settings_from_table(t: &Table) -> Result<(PieMenuSettings, Vec<(SettingsField, String)>), String> {
    check_settings_keys(t, "")?;
    let mut t = t.clone();
    let mut refs = Vec::new();
    take_refs(&mut t, "", &mut refs)?;
    let mut nones = Vec::new();
    let Some(Value::Table(serde_form)) = unreadable(Value::Table(t), "", &mut nones)? else {
        return Ok((PieMenuSettings::default(), refs));
    };
    let mut s: PieMenuSettings = Value::Table(serde_form).try_into().map_err(|e: toml::de::Error| format!("settings: {}", e.message()))?;
    for path in nones {
        set_none(&mut s, &path);
    }
    Ok((s, refs))
}

/// The value at a dotted `path` of a table.
fn table_get<'a>(t: &'a Table, path: &str) -> Option<&'a Value> {
    let (head, rest) = path.split_once('.').map_or((path, None), |(h, r)| (h, Some(r)));
    match rest {
        None => t.get(head),
        Some(rest) => table_get(t.get(head)?.as_table()?, rest),
    }
}

/// Sets the value at a dotted `path` of a table, making the tables on the way.
fn table_set(t: &mut Table, path: &str, value: Value) {
    match path.split_once('.') {
        None => {
            t.insert(path.to_string(), value);
        }
        Some((head, rest)) => {
            let sub = t.entry(head.to_string()).or_insert_with(|| Value::Table(Table::new()));
            if let Value::Table(sub) = sub {
                table_set(sub, rest, value);
            }
        }
    }
}

// ---------------------------------------------------------------------------------------
// The catalog
// ---------------------------------------------------------------------------------------

/// One menu as merged from every definition of its name.
#[derive(Clone, Debug, Default)]
struct Merged {
    title: Option<TextDef>,
    style: Option<StyleName>,
    settings: Table,
    /// The buttons with the file each came from.
    buttons: Vec<(ButtonDef, String)>,
    /// The changes, in file order, with their file.
    changes: Vec<(ChangeDef, String)>,
    /// The ids to remove, with their file.
    removes: Vec<(String, String)>,
}

/// Menus collected from TOML files (see the [module documentation](self) for the format).
/// Add files with [`MenuCatalog::add_toml`] / [`MenuCatalog::add_file`], then
/// [`MenuCatalog::build`] a menu by name.
///
/// ```
/// use egui_pie_menu::catalog::MenuCatalog;
/// let mut catalog = MenuCatalog::new();
/// catalog.add_toml("base.toml", r#"
///     [[menu.edit.button]]
///     dir = "N"
///     label = "Copy"
///     id = "edit.copy"
/// "#).unwrap();
/// // A plugin adds a button to the same menu.
/// catalog.add_toml("plugin.toml", r#"
///     [[menu.edit.button]]
///     dir = "S"
///     label = "Translate"
///     id = "plugin.translate"
/// "#).unwrap();
/// let edit = catalog.build("edit").unwrap();
/// assert_eq!(edit.buttons.len(), 2);
/// assert_eq!(edit.ids.button(&[1]), Some("plugin.translate"));
/// ```
#[derive(Clone, Debug, Default)]
pub struct MenuCatalog {
    menus: Vec<(String, Merged)>,
}

impl MenuCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    /// The names of the menus, in the order they were first defined.
    pub fn menu_names(&self) -> impl Iterator<Item = &str> {
        self.menus.iter().map(|(n, _)| n.as_str())
    }

    pub fn contains(&self, name: &str) -> bool {
        self.menus.iter().any(|(n, _)| n == name)
    }

    fn get(&self, name: &str) -> Option<&Merged> {
        self.menus.iter().find(|(n, _)| n == name).map(|(_, m)| m)
    }

    /// Reads a TOML file and adds its menus (see [`MenuCatalog::add_toml`]).
    pub fn add_file(&mut self, path: impl AsRef<Path>) -> Result<(), CatalogError> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path).map_err(|e| CatalogError::new(path.display().to_string(), e.to_string()))?;
        self.add_toml(&path.display().to_string(), &text)
    }

    /// Adds the menus of a TOML text; `source` names it in errors. A menu that is already
    /// defined is merged: its buttons, changes and removals are added, and its title, style
    /// and settings (key by key) replace the earlier ones where given. On an error (bad TOML,
    /// an unknown key or setting, ..) nothing of the text is added. Whether the buttons fit
    /// together (two in one direction) is checked by [`MenuCatalog::build`], after every
    /// change and removal, so the order of the files does not matter for those.
    pub fn add_toml(&mut self, source: &str, text: &str) -> Result<(), CatalogError> {
        let file: FileDef = toml::from_str(text).map_err(|e| CatalogError::new(source, e.to_string().trim_end()))?;
        let mut next = self.menus.clone();
        for (name, def) in file.menu {
            let err = |m: String| CatalogError::new(source, format!("menu `{name}`: {m}"));
            settings_from_table(&def.settings).map_err(err)?;
            for b in &def.button {
                check_button(b).map_err(err)?;
            }
            for c in &def.change {
                check_change(c).map_err(err)?;
            }
            let i = match next.iter().position(|(n, _)| *n == name) {
                Some(i) => i,
                None => {
                    next.push((name.clone(), Merged::default()));
                    next.len() - 1
                }
            };
            let merged = &mut next[i].1;
            if def.title.is_some() {
                merged.title = def.title;
            }
            if def.style.is_some() {
                merged.style = def.style;
            }
            merge_settings(&mut merged.settings, &def.settings, "");
            merged.buttons.extend(def.button.into_iter().map(|b| (b, source.to_string())));
            merged.changes.extend(def.change.into_iter().map(|c| (c, source.to_string())));
            merged.removes.extend(def.remove.into_iter().map(|id| (id, source.to_string())));
        }
        self.menus = next;
        Ok(())
    }

    /// Builds the menu `name`: its buttons (with the menus they open as sub-pies, from
    /// whichever files define them), settings, title, ids and bindings. Each menu's buttons
    /// are those of every file, then changed (`change`, in file order) and then thinned out
    /// (`remove`). Fails if two buttons then share a direction (the error names their files),
    /// a button opens a menu nobody defines, menus open each other in a circle, or the tree
    /// has another problem ([`crate::PieMenu::check_buttons`], e.g. a mnemonic used twice).
    /// A `change` or `remove` whose id is not there is no error: it is noted in
    /// [`MenuDefinition::notes`] (the plugin it was for may not be installed).
    pub fn build(&self, name: &str) -> Result<MenuDefinition, CatalogError> {
        let menu = self.get(name).ok_or_else(|| CatalogError::new(name, "no file defines this menu"))?;
        let mut ids = ButtonIds::new();
        let mut bindings = Bindings::new();
        let mut notes = Vec::new();
        let buttons = self.level(name, &mut vec![name.to_string()], &mut Vec::new(), &mut ids, &mut bindings, &mut notes)?;
        let (settings, refs) = settings_from_table(&menu.settings).map_err(|m| CatalogError::new(name, m))?;
        for (field, var) in refs {
            bindings.set(BindTarget::Field(field), var_binding(&var));
        }
        if let Some(var) = menu.title.as_ref().and_then(TextDef::var) {
            bindings.set(BindTarget::Title, var_binding(var));
        }
        let problems = PieMenu::new().with_settings(settings.clone()).check_tree(&buttons);
        if !problems.is_empty() {
            let lines: Vec<String> = problems.iter().map(|(path, p)| format!("{}: {p}", labels_along(&buttons, path))).collect();
            return Err(CatalogError::new(name, lines.join("; ")));
        }
        let title = menu.title.as_ref().map(|t| t.text().to_string());
        Ok(MenuDefinition { name: name.to_string(), title, settings, buttons, ids, bindings, notes, unhandled: Vec::new() })
    }

    fn level(
        &self,
        name: &str,
        stack: &mut Vec<String>,
        path: &mut Vec<usize>,
        ids: &mut ButtonIds,
        bindings: &mut Bindings,
        notes: &mut Vec<ReferenceProblem>,
    ) -> Result<Vec<PieButton>, CatalogError> {
        let defs = self.effective_buttons(name, notes)?;
        let mut out = Vec::new();
        for (n, (def, source)) in defs.iter().enumerate() {
            path.push(n);
            let err = |m: String| CatalogError::new(source.clone(), format!("menu `{name}`, button {}: {m}", button_name(def)));
            let dir = parse_direction(&def.dir).ok_or_else(|| err(format!("unknown direction `{}`", def.dir)))?;
            let mut b = if let Some(target) = &def.opens {
                let opened = self.get(target).ok_or_else(|| err(format!("opens `{target}`, which no file defines")))?;
                if stack.contains(target) {
                    return Err(err(format!("menus open each other: {} \u{2192} {target}", stack.join(" \u{2192} "))));
                }
                stack.push(target.clone());
                let children = self.level(target, stack, path, ids, bindings, notes)?;
                stack.pop();
                PieButton::new(dir).with_children(children).with_sub_pie_style(def.style.or(opened.style).map(Into::into))
            } else if def.back {
                PieButton::back(dir)
            } else if let Some(items) = &def.items {
                let mut list = Vec::new();
                for item in items {
                    let mut it = ClusterItem::new(item.number, item.label.text());
                    if let Some(m) = &item.mnemonic {
                        it.mnemonic = parse_mnemonic(m).map_err(err)?;
                    }
                    if let Some(id) = &item.id {
                        ids.set(IdKey { path: path.clone(), item: Some(item.number) }, id.clone());
                    }
                    if let Some(var) = item.label.var() {
                        bindings.set(BindTarget::ItemLabel(path.clone(), item.number), var_binding(var));
                    }
                    list.push(it);
                }
                PieButton::cluster(dir).with_items(list)
            } else {
                PieButton::new(dir)
            };
            if let Some(label) = &def.label {
                b.label = label.text().to_string();
                if let Some(var) = label.var() {
                    bindings.set(BindTarget::Label(path.clone()), var_binding(var));
                }
            }
            if let Some(m) = &def.mnemonic {
                b.mnemonic = parse_mnemonic(m).map_err(err)?;
            }
            if let Some(id) = &def.id {
                ids.set(IdKey { path: path.clone(), item: None }, id.clone());
            }
            out.push(b);
            path.pop();
        }
        Ok(out)
    }
}

impl MenuCatalog {
    /// The buttons of the menu `name` after its changes and removals, each with the file it
    /// came from; changes and removals that found nothing are added to `notes`.
    fn effective_buttons(&self, name: &str, notes: &mut Vec<ReferenceProblem>) -> Result<Vec<(ButtonDef, String)>, CatalogError> {
        let menu = self.get(name).ok_or_else(|| CatalogError::new(name, "no file defines this menu"))?;
        let mut buttons = menu.buttons.clone();
        for (c, source) in &menu.changes {
            let err = |m: String| CatalogError::new(source.clone(), format!("menu `{name}`, change of `{}`: {m}", c.id));
            if let Some((b, _)) = buttons.iter_mut().find(|(b, _)| b.id.as_deref() == Some(c.id.as_str())) {
                if let Some(d) = &c.dir {
                    b.dir = d.clone();
                }
                if let Some(l) = &c.label {
                    b.label = Some(l.clone());
                }
                if let Some(m) = &c.mnemonic {
                    b.mnemonic = Some(m.clone());
                }
                if let Some(o) = &c.opens {
                    // Now a sub-pie: no longer a back button or a cluster.
                    b.opens = Some(o.clone());
                    b.back = false;
                    b.items = None;
                }
                if let Some(st) = c.style {
                    b.style = Some(st);
                }
                if let Some(items) = &c.items {
                    b.items = Some(items.clone());
                    b.opens = None;
                    b.style = None;
                    b.back = false;
                    b.mnemonic = None;
                }
                check_button(b).map_err(err)?;
                continue;
            }
            let item = buttons.iter_mut().flat_map(|(b, _)| b.items.iter_mut().flatten()).find(|i| i.id.as_deref() == Some(c.id.as_str()));
            match item {
                Some(item) => {
                    if c.dir.is_some() || c.opens.is_some() || c.style.is_some() || c.items.is_some() {
                        return Err(err("a cluster item can only change its `label` and `mnemonic`".to_string()));
                    }
                    if let Some(l) = &c.label {
                        item.label = l.clone();
                    }
                    if let Some(m) = &c.mnemonic {
                        item.mnemonic = Some(m.clone());
                    }
                }
                None => notes.push(ReferenceProblem {
                    place: format!("`change` in menu `{name}` (from {source})"),
                    name: c.id.clone(),
                    issue: ReferenceIssue::NothingToChange,
                }),
            }
        }
        for (id, source) in &menu.removes {
            let before = buttons.len() + buttons.iter().map(|(b, _)| b.items.as_ref().map_or(0, Vec::len)).sum::<usize>();
            buttons.retain(|(b, _)| b.id.as_deref() != Some(id.as_str()));
            for (b, _) in &mut buttons {
                if let Some(items) = &mut b.items {
                    items.retain(|i| i.id.as_deref() != Some(id.as_str()));
                }
            }
            let after = buttons.len() + buttons.iter().map(|(b, _)| b.items.as_ref().map_or(0, Vec::len)).sum::<usize>();
            if after == before {
                notes.push(ReferenceProblem {
                    place: format!("`remove` in menu `{name}` (from {source})"),
                    name: id.clone(),
                    issue: ReferenceIssue::NothingToRemove,
                });
            }
        }
        for (i, (a, from_a)) in buttons.iter().enumerate() {
            let dir = parse_direction(&a.dir);
            if let Some((b, from_b)) = buttons[i + 1..].iter().find(|(b, _)| parse_direction(&b.dir) == dir) {
                return Err(CatalogError::new(
                    from_b.clone(),
                    format!(
                        "menu `{name}`: direction {} is taken twice: by {} (from {from_a}) and {} (from {from_b})",
                        a.dir,
                        button_name(a),
                        button_name(b)
                    ),
                ));
            }
        }
        Ok(buttons)
    }
}

/// The rules of one change: its id, and the values it gives.
fn check_change(c: &ChangeDef) -> Result<(), String> {
    if c.id.trim().is_empty() {
        return Err("a `change` needs the `id` of what it changes".to_string());
    }
    if let Some(d) = &c.dir {
        parse_direction(d).ok_or_else(|| format!("change of `{}`: unknown direction `{d}` (N, NE, E, SE, S, SW, W, NW)", c.id))?;
    }
    if let Some(m) = &c.mnemonic {
        parse_mnemonic(m).map_err(|e| format!("change of `{}`: {e}", c.id))?;
    }
    if c.items.is_some() && (c.opens.is_some() || c.mnemonic.is_some()) {
        return Err(format!("change of `{}`: `items` excludes `opens` and `mnemonic`", c.id));
    }
    Ok(())
}

/// `"Copy" (N)` for errors.
fn button_name(b: &ButtonDef) -> String {
    match b.label.as_ref().map(TextDef::text) {
        Some(l) if !l.is_empty() => format!("\"{l}\" ({})", b.dir),
        _ => format!("at {}", b.dir),
    }
}

fn labels_along(buttons: &[PieButton], path: &[usize]) -> String {
    if path.is_empty() {
        return "the menu".to_string();
    }
    (1..=path.len())
        .map(|n| crate::button_at(buttons, &path[..n]).map_or("?".to_string(), |b| format!("\"{}\"", b.label)))
        .collect::<Vec<_>>()
        .join(" \u{203a} ")
}

/// The shape rules of one button definition.
fn check_button(b: &ButtonDef) -> Result<(), String> {
    let name = button_name(b);
    parse_direction(&b.dir).ok_or_else(|| format!("button {name}: unknown direction `{}` (N, NE, E, SE, S, SW, W, NW)", b.dir))?;
    let kinds = usize::from(b.opens.is_some()) + usize::from(b.back) + usize::from(b.items.is_some());
    if kinds > 1 {
        return Err(format!("button {name}: `opens`, `back` and `items` exclude each other"));
    }
    if b.style.is_some() && b.opens.is_none() {
        return Err(format!("button {name}: `style` needs `opens`"));
    }
    if b.items.is_some() && b.mnemonic.is_some() {
        return Err(format!("button {name}: a cluster has no mnemonic of its own (give its items one)"));
    }
    if let Some(m) = &b.mnemonic {
        parse_mnemonic(m).map_err(|e| format!("button {name}: {e}"))?;
    }
    for item in b.items.iter().flatten() {
        if let Some(m) = &item.mnemonic {
            parse_mnemonic(m).map_err(|e| format!("button {name}, item {}: {e}", item.number))?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------
// A built menu
// ---------------------------------------------------------------------------------------

/// What is wrong with a reference ([`MenuDefinition::check_references`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReferenceIssue {
    /// No variable of the program has the name.
    UnknownVariable,
    /// The variable is of another kind than the place needs (a font for a colour).
    WrongKind { found: crate::values::VarKind, needed: crate::values::VarKind },
    /// The program does not handle the id.
    UnknownAction,
    /// A `remove` named an id the menu does not have.
    NothingToRemove,
    /// A `change` named an id the menu does not have.
    NothingToChange,
}

/// A reference the program does not answer: where it is (`place`, e.g. `the label of
/// "Edit › Copy"`), the name or id, and what is wrong.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReferenceProblem {
    pub place: String,
    pub name: String,
    pub issue: ReferenceIssue,
}

impl fmt::Display for ReferenceProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.issue {
            ReferenceIssue::UnknownVariable => write!(f, "{}: `{}` is not a variable of the program", self.place, self.name),
            ReferenceIssue::WrongKind { found, needed } => {
                write!(f, "{}: `{}` is {} of the program; this needs {}", self.place, self.name, found.name(), needed.name())
            }
            ReferenceIssue::UnknownAction => write!(f, "{}: `{}` is not an action of the program", self.place, self.name),
            ReferenceIssue::NothingToRemove => write!(f, "{}: `{}` is not in the menu (nothing removed)", self.place, self.name),
            ReferenceIssue::NothingToChange => write!(f, "{}: `{}` is not in the menu (nothing changed)", self.place, self.name),
        }
    }
}

/// A menu built from a [`MenuCatalog`] (or made in the dev tool): everything to show it, and
/// to write it back to TOML with [`MenuDefinition::to_toml`].
///
/// ```no_run
/// # use egui_pie_menu::{*, catalog::*, values::Variables};
/// # fn f(ctx: &egui::Context, catalog: &MenuCatalog, key_down: bool, vars: Variables) {
/// let mut edit = catalog.build("edit").unwrap();
/// let mut menu = edit.to_pie_menu();
/// // each frame:
/// edit.apply_variables(vars, &mut menu);
/// if let PieMenuResponse::Selected(_) = menu.show(ctx, &edit.buttons, key_down, |ui, b| { ui.label(b.label_text()); }) {
///     match edit.ids.selected(&edit.buttons, menu.selected_path()) {
///         Some("edit.copy") => { /* copy */ }
///         _ => {}
///     }
/// }
/// # }
/// ```
#[derive(Clone, Debug, Default)]
pub struct MenuDefinition {
    /// The menu's name in the files.
    pub name: String,
    pub title: Option<String>,
    pub settings: PieMenuSettings,
    pub buttons: Vec<PieButton>,
    /// The `id`s of the buttons and cluster items.
    pub ids: ButtonIds,
    /// The values that refer to your app's variables (`{ var = .. }`): resolved by
    /// [`MenuDefinition::apply_variables`].
    pub bindings: Bindings,
    /// What building noted: a `change` or `remove` whose id was not in the menu. Reported
    /// by [`MenuDefinition::check_references`] as well.
    pub notes: Vec<ReferenceProblem>,
    /// The buttons and cluster items whose id the program did not register, as of the last
    /// [`MenuDefinition::apply_actions`] ([`MenuDefinition::is_unhandled`]).
    pub unhandled: Vec<IdKey>,
}

impl MenuDefinition {
    /// A [`PieMenu`] with this definition's settings and title.
    pub fn to_pie_menu(&self) -> PieMenu {
        let mut menu = PieMenu::new();
        menu.settings = self.settings.clone();
        menu.title = self.title.clone();
        menu
    }

    /// Resolves the bindings with your app's current values: every bound label goes into
    /// [`MenuDefinition::buttons`], every bound setting and the title into `menu`. Call it
    /// when the values change (or every frame). A binding whose name is not among `vars`
    /// keeps the value from the file. Returns `true` if anything changed.
    pub fn apply_variables(&mut self, vars: Variables, menu: &mut PieMenu) -> bool {
        self.bindings.set_variables(vars);
        let mut changed = self.bindings.apply(&mut menu.settings, &mut self.buttons);
        let mut title = menu.title.clone().unwrap_or_default();
        if self.bindings.apply_title(&mut title) {
            menu.title = Some(title);
            changed = true;
        }
        changed
    }

    /// Puts your program's state into the buttons: each button or cluster item whose id is
    /// one of the `actions` takes its [`crate::ActionState`] (enabled, visible); one whose id is not
    /// is disabled (unless [`Actions::keep_unregistered`]) and remembered as unhandled
    /// ([`MenuDefinition::is_unhandled`]). Buttons without an id are left alone. Call it when
    /// the state changes, or every frame.
    ///
    /// Returns the ids that became unhandled in this call (each once, not every frame): the
    /// place to log them, if you want to.
    pub fn apply_actions(&mut self, actions: &Actions) -> Vec<String> {
        let now = actions.apply(&self.ids, &mut self.buttons);
        let mut fresh: Vec<String> = Vec::new();
        for key in now.iter().filter(|k| !self.unhandled.contains(k)) {
            let id = match key.item {
                Some(n) => self.ids.item(&key.path, n),
                None => self.ids.button(&key.path),
            };
            if let Some(id) = id
                && !fresh.iter().any(|f| f == id)
            {
                fresh.push(id.to_string());
            }
        }
        self.unhandled = now;
        fresh
    }

    /// Whether the button at `path` has an id the program does not register (as of the last
    /// [`MenuDefinition::apply_actions`]): for marking it in your render closure, e.g. in
    /// [`crate::unhandled_color`].
    pub fn is_unhandled(&self, path: &[usize]) -> bool {
        self.unhandled.iter().any(|k| k.path == path && k.item.is_none())
    }

    /// [`MenuDefinition::is_unhandled`] for the item `number` of the cluster at `path`.
    pub fn is_item_unhandled(&self, path: &[usize], number: usize) -> bool {
        self.unhandled.iter().any(|k| k.path == path && k.item == Some(number))
    }

    /// The references your program does not answer: what building noted (a `change` or
    /// `remove` that found nothing), names that are not among `vars` (or are of another kind
    /// than the place needs) and ids that are not among the `actions` your program handles.
    /// Empty when every reference is known. Run it after loading, e.g. to log a typo in a
    /// plugin's file.
    ///
    /// ```
    /// use egui_pie_menu::{Actions, catalog::MenuCatalog, values::Variables};
    /// let mut catalog = MenuCatalog::new();
    /// catalog.add_toml("m.toml", r#"
    ///     [[menu.m.button]]
    ///     dir = "N"
    ///     label = { var = "i18n.cpy", default = "Copy" }
    ///     id = "edit.cpy"
    /// "#).unwrap();
    /// let mut vars = Variables::new();
    /// vars.text("i18n.copy", "Copy");
    /// let mut actions = Actions::new();
    /// actions.add("edit.copy");
    /// let problems = catalog.build("m").unwrap().check_references(&vars, &actions);
    /// assert_eq!(problems.len(), 2);
    /// assert_eq!(problems[0].to_string(), "the label of \"Copy\": `i18n.cpy` is not a variable of the program");
    /// assert_eq!(problems[1].to_string(), "the id of \"Copy\": `edit.cpy` is not an action of the program");
    /// ```
    pub fn check_references(&self, vars: &Variables, actions: &Actions) -> Vec<ReferenceProblem> {
        let mut out = self.notes.clone();
        // The title, the settings (in their order), then the buttons in tree order.
        let mut targets: Vec<&BindTarget> = self.bindings.iter().map(|(t, _)| t).collect();
        targets.sort_by_key(|t| {
            let field = match t {
                BindTarget::Field(f) => SettingsField::ALL.iter().position(|x| x == f).unwrap_or(0) + 1,
                _ => 0,
            };
            let rank = match t {
                BindTarget::Enabled(_) | BindTarget::ItemEnabled(..) => 1,
                BindTarget::Visible(_) => 2,
                _ => 0,
            };
            (t.path().is_some(), field, t.path().map(<[usize]>::to_vec), t.item(), rank)
        });
        for target in targets {
            let Some(name) = self.var(target) else { continue };
            let issue = match vars.get(name) {
                None => ReferenceIssue::UnknownVariable,
                Some(v) if v.kind() != target.kind() => ReferenceIssue::WrongKind { found: v.kind(), needed: target.kind() },
                Some(_) => continue,
            };
            out.push(ReferenceProblem { place: self.place(target), name: name.to_string(), issue });
        }
        let mut ids: Vec<(&crate::IdKey, &str)> = self.ids.iter().collect();
        ids.sort_by_key(|(k, _)| (k.path.clone(), k.item));
        for (key, id) in ids {
            if !actions.contains(id) {
                let place = match key.item {
                    Some(n) => format!("the id of item {n} of {}", labels_along(&self.buttons, &key.path)),
                    None => format!("the id of {}", labels_along(&self.buttons, &key.path)),
                };
                out.push(ReferenceProblem { place, name: id.to_string(), issue: ReferenceIssue::UnknownAction });
            }
        }
        out
    }

    /// "the label of "Edit › Copy"" for a binding target.
    fn place(&self, target: &BindTarget) -> String {
        let along = |p: &[usize]| labels_along(&self.buttons, p);
        match target {
            BindTarget::Field(f) => format!("the setting `{}`", f.path()),
            BindTarget::Title => "the title".to_string(),
            BindTarget::Label(p) => format!("the label of {}", along(p)),
            BindTarget::Enabled(p) => format!("`enabled` of {}", along(p)),
            BindTarget::Visible(p) => format!("`visible` of {}", along(p)),
            BindTarget::ItemLabel(p, n) => format!("the label of item {n} of {}", along(p)),
            BindTarget::ItemEnabled(p, n) => format!("`enabled` of item {n} of {}", along(p)),
        }
    }

    /// The definition as a TOML file: this menu first, then one menu per sub-pie, named
    /// after its button's label (`paste_special`). Settings are written where they differ
    /// from the default. [`MenuCatalog::add_toml`] reads it back to the same definition.
    /// Whether buttons are enabled or visible is the program's state, not the file's
    /// ([`MenuDefinition::apply_actions`]), and is not written.
    pub fn to_toml(&self) -> String {
        let mut menus: Vec<(String, MenuDef)> = vec![(self.name.clone(), MenuDef::default())];
        let mut taken: Vec<String> = vec![self.name.clone()];
        let buttons = self.export_level(&self.buttons, &mut Vec::new(), &mut menus, &mut taken);
        let root = &mut menus[0].1;
        let title_var = self.var(&BindTarget::Title);
        root.title = match (&self.title, title_var) {
            (None, None) => None,
            (t, var) => Some(TextDef::new(&self.fallback_text(&BindTarget::Title, t.as_deref().unwrap_or_default()), var)),
        };
        let mut settings = settings_to_table(&self.settings);
        let current = readable_settings(&self.settings);
        for field in SettingsField::ALL {
            if let Some(var) = self.var(&BindTarget::Field(field)) {
                let mut r = Table::new();
                r.insert("var".into(), Value::String(var.to_string()));
                let default = match self.bindings.fallback(&BindTarget::Field(field)) {
                    Some(fallback) => fallback.and_then(readable_var_value),
                    None => table_get(&current, field.path()).cloned(),
                };
                if let Some(d) = default {
                    r.insert("default".into(), d);
                }
                table_set(&mut settings, field.path(), Value::Table(r));
            }
        }
        root.settings = settings;
        root.button = buttons;
        write_menus(&menus)
    }

    /// The text to write for `target`: what it had before your app's variable replaced it,
    /// else `current`.
    fn fallback_text(&self, target: &BindTarget, current: &str) -> String {
        match self.bindings.fallback(target) {
            Some(Some(VarValue::Text(t))) => t.clone(),
            _ => current.to_string(),
        }
    }

    /// The variable `target` refers to (a binding's name; its Build / Live mode is for
    /// generated code and is not written).
    fn var(&self, target: &BindTarget) -> Option<&str> {
        self.bindings.get(target).and_then(Binding::active_expr)
    }

    fn export_level(
        &self,
        level: &[PieButton],
        path: &mut Vec<usize>,
        menus: &mut Vec<(String, MenuDef)>,
        taken: &mut Vec<String>,
    ) -> Vec<ButtonDef> {
        let mut out = Vec::new();
        for (n, b) in level.iter().enumerate() {
            path.push(n);
            let default_label = if b.is_back() { "Back" } else { "" };
            let label_target = BindTarget::Label(path.clone());
            let label_var = self.var(&label_target);
            let label = self.fallback_text(&label_target, &b.label);
            let mut def = ButtonDef {
                dir: direction_name(b.direction).to_string(),
                label: (label != default_label || label_var.is_some()).then(|| TextDef::new(&label, label_var)),
                id: self.ids.button(path).map(str::to_string),
                mnemonic: if b.is_cluster() { None } else { mnemonic_name(b.mnemonic) },
                back: b.is_back(),
                ..ButtonDef::default()
            };
            match &b.kind {
                PieButtonKind::SubPie { children, style } => {
                    let name = unique_name(&b.label, path, taken);
                    let slot = menus.len();
                    menus.push((name.clone(), MenuDef::default()));
                    let kids = self.export_level(children, path, menus, taken);
                    menus[slot].1.button = kids;
                    def.opens = Some(name);
                    def.style = style.map(Into::into);
                }
                PieButtonKind::Cluster(items) => {
                    def.items = Some(
                        items
                            .iter()
                            .map(|i| ItemDef {
                                number: i.number,
                                label: {
                                    let t = BindTarget::ItemLabel(path.clone(), i.number);
                                    TextDef::new(&self.fallback_text(&t, &i.label), self.var(&t))
                                },
                                id: self.ids.item(path, i.number).map(str::to_string),
                                mnemonic: mnemonic_name(i.mnemonic),
                            })
                            .collect(),
                    );
                }
                PieButtonKind::Action | PieButtonKind::Back => {}
            }
            out.push(def);
            path.pop();
        }
        out
    }
}

/// A settings value of a variable in its readable form (`"#rrggbb"`, a stroke table, ..).
fn readable_var_value(v: &VarValue) -> Option<Value> {
    Some(match v {
        VarValue::Color(c) => Value::String(hex(*c)),
        VarValue::Stroke(st) => readable(Value::try_from(st).ok()?, ""),
        VarValue::Font(f) => readable(Value::try_from(f).ok()?, ""),
        VarValue::Number(n) => Value::Float(format!("{n}").parse().unwrap_or(f64::from(*n))),
        VarValue::Text(_) | VarValue::Bool(_) => return None,
    })
}

/// A menu name from a label (`Paste Special` → `paste_special`), unique among `taken`.
fn unique_name(label: &str, path: &[usize], taken: &mut Vec<String>) -> String {
    let mut base: String = label
        .chars()
        .flat_map(|c| if c.is_alphanumeric() { c.to_lowercase().collect::<Vec<_>>() } else { vec!['_'] })
        .collect::<String>()
        .split('_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("_");
    if base.is_empty() {
        base = format!("sub_{}", path.iter().map(|p| p.to_string()).collect::<Vec<_>>().join("_"));
    }
    let mut name = base.clone();
    let mut n = 2;
    while taken.contains(&name) {
        name = format!("{base}_{n}");
        n += 1;
    }
    taken.push(name.clone());
    name
}

#[cfg(test)]
mod tests;
