//! The button tree ([`button_tree`]) and the inspector of the selected node
//! ([`button_inspector`]), and what to preview for a selection ([`preview_target`]).
use super::bindings::{BindTarget, Bindings, bound_row, id_editor};
use crate::{Actions, ButtonIds, IdKey};
use super::fields::*;
use super::legality::*;
use super::{KindChoice, kind_choice};
use crate::settings::*;
use crate::{ButtonProblem, Mnemonic, PieButton, PieButtonKind};
use egui::collapsing_header::CollapsingState;
use egui::{Color32, RichText, Ui};
use std::collections::HashMap;

// ---------------------------------------------------------------------------------------
// Selection
// ---------------------------------------------------------------------------------------

/// The selected node of the tree: the menu itself (the default), a button, or an item of a
/// cluster.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    /// Indices from the root level down to the button (each into its level); empty: the menu.
    pub path: Vec<usize>,
    /// The declared item of the cluster at `path`, if one is selected.
    pub item: Option<usize>,
}

impl Selection {
    /// The menu itself.
    pub fn root() -> Self {
        Self::default()
    }

    /// The button at `path`.
    pub fn button(path: impl Into<Vec<usize>>) -> Self {
        Self { path: path.into(), item: None }
    }

    /// Item `item` of the cluster at `path`.
    pub fn item(path: impl Into<Vec<usize>>, item: usize) -> Self {
        Self { path: path.into(), item: Some(item) }
    }

    /// The same selection, cut back to what exists in `buttons` (a removed button selects its
    /// nearest surviving ancestor).
    pub fn clamped(&self, buttons: &[PieButton]) -> Selection {
        let mut path = Vec::new();
        let mut level = buttons;
        for &i in &self.path {
            let Some(b) = level.get(i) else { break };
            path.push(i);
            level = b.children();
        }
        let item = (path.len() == self.path.len())
            .then_some(self.item)
            .flatten()
            .filter(|k| button_at(buttons, &path).is_some_and(|b| *k < b.cluster_items().len()));
        Selection { path, item }
    }
}

/// What a [`Selection`] is, for deciding which menu-wide settings concern it
/// ([`relevant_fields`]) and what to preview ([`preview_target`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionKind {
    /// The menu itself.
    Root,
    /// A plain button of the root ring (or a back button there, which acts like one).
    Action,
    /// A back button in a replace-style sub-pie.
    Back,
    /// A button that opens a replace-style sub-pie.
    ReplaceSubPie,
    /// A button that opens a fan-out.
    FanOut,
    /// A plain button in a replace-style sub-pie.
    ReplaceChild,
    /// Any button inside a fan-out.
    FanChild,
    /// A cluster button.
    Cluster,
    /// A declared item of a cluster.
    ClusterItem,
}

/// The [`SelectionKind`] of `selection`. `settings` says whether a sub-pie without a style of
/// its own is a fan-out (the menu-wide [`PieMenuSubPieSettings::style`]). A selection that does
/// not exist counts as the menu.
pub fn selection_kind(buttons: &[PieButton], settings: &PieMenuSettings, selection: &Selection) -> SelectionKind {
    let Some((_, parent)) = selection.path.split_last() else { return SelectionKind::Root };
    let Some(button) = button_at(buttons, &selection.path) else { return SelectionKind::Root };
    if selection.item.is_some() && button.is_cluster() {
        return SelectionKind::ClusterItem;
    }
    let level = level_kind(buttons, settings, parent);
    if matches!(level, LevelKind::Fan(_)) {
        return SelectionKind::FanChild;
    }
    match &button.kind {
        PieButtonKind::SubPie { .. } => match effective_style(button, settings) {
            Some(SubPieStyle::FanOut) => SelectionKind::FanOut,
            _ => SelectionKind::ReplaceSubPie,
        },
        PieButtonKind::Cluster(_) => SelectionKind::Cluster,
        PieButtonKind::Back if level == LevelKind::Replace => SelectionKind::Back,
        PieButtonKind::Action if level == LevelKind::Replace => SelectionKind::ReplaceChild,
        PieButtonKind::Action | PieButtonKind::Back => SelectionKind::Action,
    }
}

/// The menu-wide settings that affect a node of this kind, for the inspector's "Menu-wide
/// settings that affect this" section. The number hint fields are only listed while number
/// keys are on.
///
/// - `Root`: the ring, centre indicator and title groups, root hover emphasis and duration,
///   automatic mnemonics.
/// - `Action`: text colour, automatic mnemonics, number keys (and hints), root hover
///   emphasis, hover duration.
/// - `Back`, `ReplaceChild`: the same with the sub-pie hover emphasis; `Back` adds the sub-pie
///   centre and hover delay.
/// - `ReplaceSubPie`: like an action, plus both emphases, sub-pie style, centre, hover delay and
///   whether the title shows.
/// - `FanOut`: like an action, plus both emphases, sub-pie style, fan-out delay, radius and
///   opener action.
/// - `FanChild`: sub-pie hover emphasis, fan-out radius, text colour, number hints.
/// - `Cluster`: automatic mnemonics, mnemonic keys, select on primary click.
/// - `ClusterItem`: automatic mnemonics, mnemonic keys.
pub fn relevant_fields(settings: &PieMenuSettings, kind: SelectionKind) -> Vec<SettingsField> {
    use SettingsField as F;
    let hints = |v: &mut Vec<SettingsField>| {
        if settings.input.use_number_keys {
            v.extend([F::HintColor, F::HintSpacing, F::HintSide, F::HintDigits]);
        }
    };
    let action = |emphasis: &[SettingsField]| {
        let mut v = vec![F::TextColor, F::AutoMnemonics, F::UseNumberKeys];
        hints(&mut v);
        v.extend_from_slice(emphasis);
        v.push(F::HoverDuration);
        v
    };
    let mut v = match kind {
        SelectionKind::Root => {
            let mut v: Vec<SettingsField> = [SettingsGroup::Ring, SettingsGroup::CenterIndicator, SettingsGroup::Title]
                .into_iter()
                .flat_map(|g| g.fields().iter().copied())
                .collect();
            v.extend([F::HoverEmphasis, F::HoverDuration, F::AutoMnemonics]);
            v
        }
        SelectionKind::Action => action(&[F::HoverEmphasis]),
        SelectionKind::Back => {
            let mut v = action(&[F::SubPieHoverEmphasis]);
            v.extend([F::SubPieCenter, F::SubPieHoverDelay]);
            v
        }
        SelectionKind::ReplaceSubPie => {
            let mut v = action(&[F::HoverEmphasis, F::SubPieHoverEmphasis]);
            v.extend([F::SubPieStyle, F::SubPieCenter, F::SubPieHoverDelay, F::TitleShow]);
            v
        }
        SelectionKind::FanOut => {
            let mut v = action(&[F::HoverEmphasis, F::SubPieHoverEmphasis]);
            v.extend([F::SubPieStyle, F::FanOutDelay, F::FanOutRadius, F::FanOutOpenerAction]);
            v
        }
        SelectionKind::ReplaceChild => action(&[F::SubPieHoverEmphasis]),
        SelectionKind::FanChild => {
            let mut v = vec![F::SubPieHoverEmphasis, F::FanOutRadius, F::TextColor];
            hints(&mut v);
            v
        }
        SelectionKind::Cluster => vec![F::AutoMnemonics, F::UseMnemonicKeys, F::SelectOnPrimaryClick],
        SelectionKind::ClusterItem => vec![F::AutoMnemonics, F::UseMnemonicKeys],
    };
    let mut seen = Vec::new();
    v.retain(|f| {
        let fresh = !seen.contains(f);
        seen.push(*f);
        fresh
    });
    v
}

// ---------------------------------------------------------------------------------------
// Preview
// ---------------------------------------------------------------------------------------

/// What a live preview of a [`Selection`] should show: which level to display, which fan to
/// hold open, what to highlight. Feed it to the menu (descend to `level_path`,
/// [`crate::PieMenu::set_open_fan_out`] with `fan_trigger`) and your render closure.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PreviewTarget {
    /// The sub-pie button whose children form the ring to show (empty: the root ring).
    pub level_path: Vec<usize>,
    /// The button of that ring whose fan-out is open, if any.
    pub fan_trigger: Option<usize>,
    /// The button to highlight: in the ring, or (with `fan_trigger`) among the fan's children.
    pub highlight: Option<usize>,
    /// The item of the highlighted cluster to highlight.
    pub item: Option<usize>,
}

/// What to preview for `selection`: the menu and plain buttons, clusters and their items show
/// the ring that holds them (highlighted); a replace-style sub-pie shows its own children; a
/// fan-out shows its parent ring with the fan open; a fan button shows that fan with the button
/// highlighted.
pub fn preview_target(buttons: &[PieButton], settings: &PieMenuSettings, selection: &Selection) -> PreviewTarget {
    let selection = selection.clamped(buttons);
    let kind = selection_kind(buttons, settings, &selection);
    let path = &selection.path;
    let n = path.len();
    match kind {
        SelectionKind::Root => PreviewTarget::default(),
        SelectionKind::ReplaceSubPie => PreviewTarget { level_path: path.clone(), ..Default::default() },
        SelectionKind::FanOut => PreviewTarget {
            level_path: path[..n - 1].to_vec(),
            fan_trigger: Some(path[n - 1]),
            ..Default::default()
        },
        SelectionKind::FanChild => PreviewTarget {
            level_path: path[..n - 2].to_vec(),
            fan_trigger: Some(path[n - 2]),
            highlight: Some(path[n - 1]),
            item: None,
        },
        SelectionKind::Action
        | SelectionKind::Back
        | SelectionKind::ReplaceChild
        | SelectionKind::Cluster
        | SelectionKind::ClusterItem => PreviewTarget {
            level_path: path[..n - 1].to_vec(),
            fan_trigger: None,
            highlight: Some(path[n - 1]),
            item: selection.item,
        },
    }
}

// ---------------------------------------------------------------------------------------
// The tree
// ---------------------------------------------------------------------------------------

fn dir_short(d: PieDirection) -> &'static str {
    match d {
        PieDirection::North => "N",
        PieDirection::NorthEast => "NE",
        PieDirection::East => "E",
        PieDirection::SouthEast => "SE",
        PieDirection::South => "S",
        PieDirection::SouthWest => "SW",
        PieDirection::West => "W",
        PieDirection::NorthWest => "NW",
    }
}

/// "NE · Paste Special ▸": direction, label and a glyph for what it opens.
fn node_text(button: &PieButton, settings: &PieMenuSettings) -> String {
    let label = if button.label.is_empty() { "(no label)" } else { button.label.as_str() };
    let glyph = match &button.kind {
        PieButtonKind::SubPie { .. } => match effective_style(button, settings) {
            Some(SubPieStyle::FanOut) => " \u{bb}",
            _ => " \u{25b8}",
        },
        PieButtonKind::Cluster(_) => " \u{2261}",
        PieButtonKind::Back => " \u{25c2}",
        PieButtonKind::Action => "",
    };
    let hidden = if button.visible { "" } else { " (hidden)" };
    format!("{} \u{b7} {label}{glyph}{hidden}", dir_short(button.direction))
}

fn node_title(text: String, bad: bool) -> RichText {
    if bad { RichText::new(format!("\u{26a0} {text}")).color(Color32::LIGHT_RED) } else { RichText::new(text) }
}

struct TreeCtx<'a> {
    settings: &'a PieMenuSettings,
    /// Only the buttons this accepts, and the ones above them, are shown (`None`: all).
    filter: Option<&'a dyn Fn(&PieButton) -> bool>,
    bad: Vec<Vec<usize>>,
    /// The selection changed since the last frame: open the way to it.
    reveal: bool,
    id: egui::Id,
}

impl TreeCtx<'_> {
    /// Whether the node at `path` or something below it has a problem.
    fn has_problem(&self, path: &[usize]) -> bool {
        self.bad.iter().any(|p| p.starts_with(path))
    }
}

/// Whether `button` or something below it passes `filter`.
fn subtree_matches(filter: &dyn Fn(&PieButton) -> bool, button: &PieButton) -> bool {
    filter(button) || button.children().iter().any(|c| subtree_matches(filter, c))
}

fn tree_node(ui: &mut Ui, ctx: &TreeCtx, button: &PieButton, path: &[usize], selection: &mut Selection) {
    if let Some(filter) = ctx.filter
        && !subtree_matches(filter, button)
    {
        return;
    }
    let title = node_title(node_text(button, ctx.settings), ctx.has_problem(path));
    let selected = selection.path == path && selection.item.is_none();
    let items = button.cluster_items();
    let children = button.children();
    if children.is_empty() && items.is_empty() {
        ui.horizontal(|ui| {
            ui.add_space(ui.spacing().icon_width + ui.spacing().icon_spacing);
            if ui.selectable_label(selected, title).clicked() {
                *selection = Selection::button(path);
            }
        });
        return;
    }
    let id = ctx.id.with(("node", path));
    let mut state = CollapsingState::load_with_default_open(ui.ctx(), id, false);
    if ctx.reveal && selection.path.len() > path.len() && selection.path.starts_with(path) {
        state.set_open(true);
    }
    // A search opens the way to every match below.
    if let Some(filter) = ctx.filter
        && children.iter().any(|c| subtree_matches(filter, c))
    {
        state.set_open(true);
    }
    state
        .show_header(ui, |ui| {
            if ui.selectable_label(selected, title).clicked() {
                *selection = Selection::button(path);
            }
        })
        .body(|ui| {
            for (i, child) in children.iter().enumerate() {
                let mut p = path.to_vec();
                p.push(i);
                tree_node(ui, ctx, child, &p, selection);
            }
            for (k, item) in items.iter().enumerate() {
                let on = selection.path == path && selection.item == Some(k);
                if ui.selectable_label(on, format!("{} \u{b7} {}", item.number, item.label)).clicked() {
                    *selection = Selection::item(path, k);
                }
            }
        });
}

/// A collapsible tree of the buttons: the menu at the top, buttons as "NE · Paste Special ▸"
/// (direction, label, a glyph for sub-pie `▸`, fan-out `»`, cluster `≡` and back `◂`), their
/// children below, a cluster's items as leaves. A red warning sign marks a button whose subtree
/// has a [`crate::PieMenu::check_buttons`] problem. Clicking selects; the selection changes in
/// `selection`, and the way to a newly selected node opens.
///
/// `settings` decides what a sub-pie without a style of its own is (the menu-wide style) and
/// supplies `input.auto_mnemonics` for the problem check.
pub fn button_tree(ui: &mut Ui, settings: &PieMenuSettings, buttons: &[PieButton], selection: &mut Selection) {
    tree(ui, settings, buttons, selection, None, &[]);
}

/// [`button_tree`] (with an optional search `filter`, as in [`button_tree_filtered`]) that
/// also marks the buttons at `marked` (and the nodes above them) with a warning sign, like a
/// button problem: e.g. the buttons whose id the program does not handle
/// ([`crate::Actions::unregistered`]; a cluster item's mark goes on its cluster).
pub fn button_tree_marked(
    ui: &mut Ui,
    settings: &PieMenuSettings,
    buttons: &[PieButton],
    selection: &mut Selection,
    filter: Option<&dyn Fn(&PieButton) -> bool>,
    marked: &[Vec<usize>],
) {
    tree(ui, settings, buttons, selection, filter, marked);
}

/// [`button_tree`] showing only the buttons `filter` accepts and the buttons above them (a
/// search); the way to every match is kept open. The menu node and the selection are unaffected.
pub fn button_tree_filtered(
    ui: &mut Ui,
    settings: &PieMenuSettings,
    buttons: &[PieButton],
    selection: &mut Selection,
    filter: &dyn Fn(&PieButton) -> bool,
) {
    tree(ui, settings, buttons, selection, Some(filter), &[]);
}

fn tree(
    ui: &mut Ui,
    settings: &PieMenuSettings,
    buttons: &[PieButton],
    selection: &mut Selection,
    filter: Option<&dyn Fn(&PieButton) -> bool>,
    marked: &[Vec<usize>],
) {
    *selection = selection.clamped(buttons);
    let id = ui.id().with("pie_button_tree");
    let last_id = id.with("last_selection");
    let last: Option<Selection> = ui.data(|d| d.get_temp(last_id));
    let reveal = last.as_ref() != Some(&*selection);
    let problems = tree_problems(settings, buttons);
    let mut bad = problem_button_paths(&problems);
    bad.extend(marked.iter().cloned());
    let ctx = TreeCtx { settings, filter, bad, reveal, id };
    let root_title = node_title("Menu".to_string(), !problems.is_empty() || !marked.is_empty());
    if ui.selectable_label(*selection == Selection::root(), root_title).clicked() {
        *selection = Selection::root();
    }
    ui.indent(id, |ui| {
        for (i, button) in buttons.iter().enumerate() {
            tree_node(ui, &ctx, button, &[i], selection);
        }
    });
    let now = selection.clone();
    ui.data_mut(|d| d.insert_temp(last_id, now));
}

// ---------------------------------------------------------------------------------------
// The inspector
// ---------------------------------------------------------------------------------------

pub use crate::TreeEdit;

/// What [`button_inspector`] did this frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InspectorOutput {
    /// The buttons changed (anything below, including the path of the selection).
    pub changed: bool,
    /// A menu-wide setting changed (through the "Menu-wide settings that affect this" section).
    pub settings_changed: bool,
    /// Changes to the tree's shape, in the order they happened.
    pub edits: Vec<TreeEdit>,
    /// A message for the user that arose this frame (the inspector shows it as well until
    /// dismissed), e.g. which sub-pies a menu-wide fan-out style left replace-style.
    pub notice: Option<String>,
}

/// Where converting a button stashed its children / items, by path.
#[derive(Clone, Debug, Default)]
struct StashMap(HashMap<Vec<usize>, Stash>);

impl StashMap {
    /// A button was removed: forget it and everything below, shift later siblings.
    fn removed(&mut self, path: &[usize]) {
        let Some((last, parent)) = path.split_last() else { return };
        let depth = path.len() - 1;
        let old = std::mem::take(&mut self.0);
        for (mut key, stash) in old {
            if key.starts_with(path) {
                continue;
            }
            if key.len() > depth && key.starts_with(parent) && key[depth] > *last {
                key[depth] -= 1;
            }
            self.0.insert(key, stash);
        }
    }

    /// The children below `path` are gone: forget what was stashed for them.
    fn dropped_below(&mut self, path: &[usize]) {
        self.0.retain(|key, _| !(key.len() > path.len() && key.starts_with(path)));
    }
}

/// Something the user asked for while the inspector was drawn.
enum Intent {
    Select(Selection),
    Label(String),
    Direction(PieDirection),
    Kind(KindChoice),
    Style(Option<SubPieStyle>),
    Enabled(bool),
    Visible(bool),
    Mnemonic(MnemonicChange),
    Remove,
    Add(Vec<usize>, AddKind),
    AddItem,
    RemoveItem(usize),
    ItemLabel(usize, String),
    ItemNumber(usize, usize),
    ItemEnabled(usize, bool),
    ItemMnemonic(usize, MnemonicChange),
}

struct Applied<'a> {
    buttons: &'a mut Vec<PieButton>,
    settings: &'a PieMenuSettings,
    selection: &'a mut Selection,
    stashes: &'a mut StashMap,
    out: &'a mut InspectorOutput,
}

impl Applied<'_> {
    fn apply(&mut self, intent: Intent) -> Result<(), String> {
        let path = self.selection.path.clone();
        let item_of = |this: &Self| this.selection.item;
        match intent {
            Intent::Select(s) => *self.selection = s,
            Intent::Label(text) => {
                button_at_mut(self.buttons, &path).ok_or("no such button")?.label = text;
                self.out.changed = true;
            }
            Intent::Direction(d) => {
                set_direction(self.buttons, self.settings, &path, d)?;
                self.out.changed = true;
            }
            Intent::Kind(k) => {
                let stash = self.stashes.0.entry(path.clone()).or_default();
                let button = button_at_mut(self.buttons, &path).ok_or("no such button")?;
                if convert_kind_in(button, k, self.settings, stash) {
                    self.out.edits.push(TreeEdit::Dropped(path.clone()));
                    self.stashes.dropped_below(&path);
                }
                self.selection.item = None;
                self.out.changed = true;
            }
            Intent::Style(style) => {
                let removed = set_sub_pie_style(self.buttons, self.settings, &path, style)?;
                for i in removed.iter().rev() {
                    let mut child = path.clone();
                    child.push(*i);
                    self.stashes.removed(&child);
                    self.out.edits.push(TreeEdit::Removed(child));
                }
                self.out.changed = true;
            }
            Intent::Enabled(on) => {
                set_enabled(self.buttons, &path, item_of(self), on)?;
                self.out.changed = true;
            }
            Intent::Visible(on) => {
                button_at_mut(self.buttons, &path).ok_or("no such button")?.visible = on;
                self.out.changed = true;
            }
            Intent::Mnemonic(change) => {
                set_mnemonic(self.buttons, self.settings, &path, None, change)?;
                self.out.changed = true;
            }
            Intent::Remove => {
                if !remove_button(self.buttons, &path) {
                    return Err("cannot remove this button".to_string());
                }
                self.stashes.removed(&path);
                self.out.edits.push(TreeEdit::Removed(path.clone()));
                *self.selection = Selection::button(&path[..path.len() - 1]);
                self.out.changed = true;
            }
            Intent::Add(parent, kind) => {
                let at = add_button(self.buttons, self.settings, &parent, kind).ok_or("cannot add that here")?;
                let mut new = parent;
                new.push(at);
                self.out.edits.push(TreeEdit::Added(new.clone()));
                *self.selection = Selection::button(new);
                self.out.changed = true;
            }
            Intent::AddItem => {
                let at = add_item(self.buttons, &path).ok_or("not a cluster")?;
                self.selection.item = Some(at);
                self.out.changed = true;
            }
            Intent::RemoveItem(k) => {
                if !remove_item(self.buttons, &path, k) {
                    return Err("a cluster needs at least one item".to_string());
                }
                self.selection.item = match self.selection.item {
                    Some(s) if s == k => None,
                    Some(s) if s > k => Some(s - 1),
                    other => other,
                };
                self.out.changed = true;
            }
            Intent::ItemLabel(k, text) => {
                match &mut button_at_mut(self.buttons, &path).ok_or("no such button")?.kind {
                    PieButtonKind::Cluster(items) => items.get_mut(k).ok_or("no such item")?.label = text,
                    _ => return Err("not a cluster".to_string()),
                }
                self.out.changed = true;
            }
            Intent::ItemNumber(k, n) => {
                match &mut button_at_mut(self.buttons, &path).ok_or("no such button")?.kind {
                    PieButtonKind::Cluster(items) => set_item_number(items, k, n)?,
                    _ => return Err("not a cluster".to_string()),
                }
                self.out.changed = true;
            }
            Intent::ItemEnabled(k, on) => {
                set_enabled(self.buttons, &path, Some(k), on)?;
                self.out.changed = true;
            }
            Intent::ItemMnemonic(k, change) => {
                set_mnemonic(self.buttons, self.settings, &path, Some(k), change)?;
                self.out.changed = true;
            }
        }
        Ok(())
    }
}

fn red(ui: &mut Ui, text: impl Into<String>) {
    ui.colored_label(Color32::LIGHT_RED, text.into());
}

/// A collapsing section that stays as the user left it across selections.
fn section<R>(ui: &mut Ui, title: &str, open: bool, body: impl FnOnce(&mut Ui) -> R) -> Option<R> {
    egui::CollapsingHeader::new(title).id_salt(("pie_inspector_section", title)).default_open(open).show(ui, body).body_returned
}

/// "Menu › Copy › More": the labels along the path.
fn breadcrumb(buttons: &[PieButton], selection: &Selection) -> String {
    let mut parts = vec!["Menu".to_string()];
    let mut level = buttons;
    for &i in &selection.path {
        let Some(b) = level.get(i) else { break };
        parts.push(if b.label.is_empty() { dir_short(b.direction).to_string() } else { b.label.clone() });
        level = b.children();
    }
    if let (Some(k), Some(b)) = (selection.item, button_at(buttons, &selection.path))
        && let Some(item) = b.cluster_items().get(k)
    {
        parts.push(item.label.clone());
    }
    parts.join(" \u{203a} ")
}

/// The mnemonic mode combo and key field. Returns the change asked for.
fn mnemonic_controls(ui: &mut Ui, salt: &'static str, m: Mnemonic) -> Option<MnemonicChange> {
    let mut out = None;
    let name = match m {
        Mnemonic::Auto => "Auto",
        Mnemonic::Key(_) => "Key",
        Mnemonic::None => "None",
    };
    egui::ComboBox::from_id_salt(salt).selected_text(name).width(70.0).show_ui(ui, |ui| {
        if ui.selectable_label(m == Mnemonic::Auto, "Auto").clicked() && m != Mnemonic::Auto {
            out = Some(MnemonicChange::Auto);
        }
        if ui.selectable_label(matches!(m, Mnemonic::Key(_)), "Key").clicked() && !matches!(m, Mnemonic::Key(_)) {
            out = Some(MnemonicChange::KeyFromLabel);
        }
        if ui.selectable_label(m == Mnemonic::None, "None").clicked() && m != Mnemonic::None {
            out = Some(MnemonicChange::None);
        }
    });
    if let Mnemonic::Key(c) = m {
        let mut text = c.to_string();
        if ui.add(egui::TextEdit::singleline(&mut text).desired_width(24.0)).changed() {
            // Typing appends: the newest character wins; an emptied field keeps the old key.
            if let Some(n) = text.chars().last().filter(|n| *n != c) {
                out = Some(MnemonicChange::Key(n));
            }
        }
    }
    out
}

/// The children of the level at `parent_path` as select links, the "Add" buttons, and the
/// level's problems.
fn level_contents(
    ui: &mut Ui,
    settings: &PieMenuSettings,
    buttons: &[PieButton],
    parent_path: &[usize],
    intents: &mut Vec<Intent>,
) {
    let level = level_at(buttons, parent_path).unwrap_or(&[]);
    let kind = level_kind(buttons, settings, parent_path);
    ui.label(match kind {
        LevelKind::Fan(_) => format!("{} of 5 fan-out buttons", level.len()),
        _ => format!("{} of 8 buttons", level.len()),
    });
    for (i, child) in level.iter().enumerate() {
        let mut p = parent_path.to_vec();
        p.push(i);
        if ui.link(node_text(child, settings)).clicked() {
            intents.push(Intent::Select(Selection::button(p)));
        }
    }
    let addable = addable_kinds(buttons, settings, parent_path);
    ui.horizontal_wrapped(|ui| {
        for kind in &addable {
            if ui.button(format!("Add {}", kind.name().to_lowercase())).clicked() {
                intents.push(Intent::Add(parent_path.to_vec(), *kind));
            }
        }
    });
    if addable.is_empty() {
        ui.colored_label(Color32::ORANGE, "no free direction left");
    }
    for (level_path, problem) in tree_problems(settings, buttons) {
        if level_path == parent_path && !matches!(problem, ButtonProblem::InFanOut { .. }) {
            red(ui, problem.to_string());
        }
    }
}

/// Draws Identity, Keys and Contents of the selected button; returns what was asked for.
fn button_sections(
    ui: &mut Ui,
    settings: &PieMenuSettings,
    buttons: &[PieButton],
    selection: &Selection,
    bindings: Option<&mut Bindings>,
    ids: Option<(&mut ButtonIds, &Actions)>,
) -> Vec<Intent> {
    let mut intents = Vec::new();
    let path = &selection.path;
    let Some(button) = button_at(buttons, path) else { return intents };
    let parent = &path[..path.len() - 1];
    let menu = problem_menu_for(settings);

    section(ui, "Identity", true, |ui| {
        if let Some((ids, actions)) = ids {
            id_editor(ui, ids, &IdKey { path: path.clone(), item: None }, actions);
        }
        let label_row = |ui: &mut Ui| {
            ui.label("Label");
            let mut label = button.label.clone();
            if ui.add(egui::TextEdit::singleline(&mut label).desired_width(160.0)).changed() {
                intents.push(Intent::Label(label));
            }
            false
        };
        match bindings {
            Some(b) => {
                bound_row(ui, b, &BindTarget::Label(path.clone()), label_row);
            }
            None => {
                ui.horizontal(label_row);
            }
        }
        ui.horizontal(|ui| {
            ui.label("Direction");
            egui::ComboBox::from_id_salt("pie_inspector_direction")
                .selected_text(format!("{:?}", button.direction))
                .show_ui(ui, |ui| {
                    for d in PieDirection::ALL {
                        match direction_block_reason(buttons, settings, path, d) {
                            None => {
                                if ui.selectable_label(d == button.direction, format!("{d:?}")).clicked() && d != button.direction {
                                    intents.push(Intent::Direction(d));
                                }
                            }
                            Some(reason) => {
                                ui.add_enabled(false, egui::Button::selectable(false, format!("{d:?} ({reason})")));
                            }
                        }
                    }
                });
        });
        ui.horizontal(|ui| {
            ui.label("Kind");
            let current = kind_choice(button);
            egui::ComboBox::from_id_salt("pie_inspector_kind").selected_text(current.name()).show_ui(ui, |ui| {
                for k in allowed_kinds(buttons, settings, path) {
                    if ui.selectable_label(current == k, k.name()).clicked() && current != k {
                        intents.push(Intent::Kind(k));
                    }
                }
            });
        });
        if let PieButtonKind::SubPie { style, .. } = &button.kind {
            ui.horizontal(|ui| {
                ui.label("Opens as");
                let menu_default = match settings.sub_pies.style {
                    SubPieStyle::Replace => "Menu default (replace)",
                    SubPieStyle::FanOut => "Menu default (fan-out)",
                };
                let name = |s: Option<SubPieStyle>| match s {
                    None => menu_default,
                    Some(SubPieStyle::Replace) => "Replace the level",
                    Some(SubPieStyle::FanOut) => "Fan-out",
                };
                egui::ComboBox::from_id_salt("pie_inspector_style").selected_text(name(*style)).show_ui(ui, |ui| {
                    for option in [None, Some(SubPieStyle::Replace), Some(SubPieStyle::FanOut)] {
                        match style_availability(button, settings, option) {
                            Ok(()) => {
                                if ui.selectable_label(*style == option, name(option)).clicked() && *style != option {
                                    intents.push(Intent::Style(option));
                                }
                            }
                            Err(reason) => {
                                ui.add_enabled(false, egui::Button::selectable(false, name(option)))
                                    .on_disabled_hover_text(reason);
                            }
                        }
                    }
                });
            });
        }
        let enabled_row = |ui: &mut Ui| {
            let mut enabled = button.enabled;
            if ui.checkbox(&mut enabled, "enabled").changed() {
                intents.push(Intent::Enabled(enabled));
            }
            false
        };
        // Whether a button is enabled or visible is the program's state (by its id, through
        // `Actions`): these checkboxes are for trying it out, not bound to anything.
        ui.horizontal(enabled_row);
        let visible_row = |ui: &mut Ui| {
            let mut visible = button.visible;
            if ui
                .checkbox(&mut visible, "visible")
                .on_hover_text("A hidden button is not drawn and cannot be picked; it keeps its place and direction.")
                .changed()
            {
                intents.push(Intent::Visible(visible));
            }
            false
        };
        ui.horizontal(visible_row);
        if remove_block_reason(buttons, path).is_none() && ui.button("Remove").clicked() {
            intents.push(Intent::Remove);
        }
    });

    section(ui, "Keys", true, |ui| {
        if button.is_cluster() {
            ui.label("A cluster has no key of its own: its items have (select one in the tree).");
            return;
        }
        ui.horizontal(|ui| {
            ui.label("Mnemonic");
            if let Some(change) = mnemonic_controls(ui, "pie_inspector_mnemonic", button.mnemonic) {
                intents.push(Intent::Mnemonic(change));
            }
        });
        let level = level_at(buttons, parent).unwrap_or(&[]);
        let index = path[path.len() - 1];
        let in_effect = menu.mnemonics(level).get(index).copied().flatten();
        ui.label(match in_effect {
            Some(c) => format!("Key in effect: {}", c.to_uppercase()),
            None => "Key in effect: none".to_string(),
        });
        ui.label(match menu.number_hint(button) {
            Some(hint) => format!("Number key: {}", hint.text),
            None => "Number key: off (number keys are disabled)".to_string(),
        });
    });

    section(ui, "Contents", true, |ui| match &button.kind {
        PieButtonKind::SubPie { .. } => level_contents(ui, settings, buttons, path, &mut intents),
        PieButtonKind::Cluster(items) => {
            for (k, item) in items.iter().enumerate() {
                if ui.link(format!("{} \u{b7} {}", item.number, item.label)).clicked() {
                    intents.push(Intent::Select(Selection::item(path.clone(), k)));
                }
            }
            if ui.button("Add item").clicked() {
                intents.push(Intent::AddItem);
            }
        }
        PieButtonKind::Action | PieButtonKind::Back => {
            ui.label("A plain button: selecting it returns its index.");
        }
    });
    intents
}

/// Draws the editor of the selected cluster item; returns what was asked for.
fn item_sections(
    ui: &mut Ui,
    settings: &PieMenuSettings,
    buttons: &[PieButton],
    selection: &Selection,
    bindings: Option<&mut Bindings>,
    ids: Option<(&mut ButtonIds, &Actions)>,
) -> Vec<Intent> {
    let mut intents = Vec::new();
    let Some(cluster) = button_at(buttons, &selection.path) else { return intents };
    let Some(k) = selection.item else { return intents };
    let Some(item) = cluster.cluster_items().get(k) else { return intents };
    let menu = problem_menu_for(settings);
    if ui.link("\u{2191} the cluster").clicked() {
        intents.push(Intent::Select(Selection::button(selection.path.clone())));
    }
    section(ui, "Identity", true, |ui| {
        if let Some((ids, actions)) = ids {
            id_editor(ui, ids, &IdKey { path: selection.path.clone(), item: Some(item.number) }, actions);
        }
        ui.horizontal(|ui| {
            ui.label("Number");
            let mut number = item.number;
            if ui.add(egui::DragValue::new(&mut number)).changed() {
                intents.push(Intent::ItemNumber(k, number));
            }
        });
        let label_row = |ui: &mut Ui| {
            ui.label("Label");
            let mut label = item.label.clone();
            if ui.add(egui::TextEdit::singleline(&mut label).desired_width(120.0)).changed() {
                intents.push(Intent::ItemLabel(k, label));
            }
            false
        };
        match bindings {
            Some(b) => {
                bound_row(ui, b, &BindTarget::ItemLabel(selection.path.clone(), item.number), label_row);
            }
            None => {
                ui.horizontal(label_row);
            }
        }
        let enabled_row = |ui: &mut Ui| {
            let mut enabled = item.enabled;
            if ui.checkbox(&mut enabled, "enabled").changed() {
                intents.push(Intent::ItemEnabled(k, enabled));
            }
            false
        };
        ui.horizontal(enabled_row);
        if item_remove_block_reason(cluster.cluster_items().len()).is_none() && ui.button("Remove item").clicked() {
            intents.push(Intent::RemoveItem(k));
        }
    });
    section(ui, "Keys", true, |ui| {
        ui.horizontal(|ui| {
            ui.label("Mnemonic");
            if let Some(change) = mnemonic_controls(ui, "pie_inspector_item_mnemonic", item.mnemonic) {
                intents.push(Intent::ItemMnemonic(k, change));
            }
        });
        let parent = &selection.path[..selection.path.len() - 1];
        let level = level_at(buttons, parent).unwrap_or(&[]);
        let index = selection.path[selection.path.len() - 1];
        let in_effect = menu.cluster_mnemonics(level).get(index).and_then(|v| v.get(k)).copied().flatten();
        ui.label(match in_effect {
            Some(c) => format!("Key in effect: {}", c.to_uppercase()),
            None => "Key in effect: none".to_string(),
        });
    });
    intents
}

/// The inspector of the selected node: Identity (label, direction, kind, how a sub-pie opens,
/// enabled, remove), Keys (mnemonic and the keys in effect), Contents (the children with links
/// that select them and the "Add" buttons, or a cluster's items), then your own controls
/// (`extras`, called with the selected button and its path; not for the menu or a cluster item),
/// then "Menu-wide settings that affect this" ([`relevant_fields`], drawn with [`field_editor`]).
/// The menu itself shows its children and the settings of the ring, centre indicator and title.
///
/// Every offer is legal: directions that are taken or face a fan-out's parent, kinds a level
/// cannot hold and "Opens as" choices that do not fit are disabled with a reason or left out,
/// "Remove" is hidden where it would leave an empty menu, sub-pie or cluster, and a mnemonic
/// key or item number that another holds is refused (the reason is shown). Turning a button
/// into another kind keeps what it dropped aside, and converting back restores it.
///
/// `settings` is edited by the menu-wide section only; setting the sub-pie style to a fan-out
/// there pins the sub-pies that cannot fan out to replace style (see the notice).
pub fn button_inspector(
    ui: &mut Ui,
    settings: &mut PieMenuSettings,
    buttons: &mut Vec<PieButton>,
    selection: &mut Selection,
    extras: impl FnMut(&mut Ui, &[usize], &mut PieButton),
) -> InspectorOutput {
    inspect(ui, settings, buttons, selection, None, None, extras, true)
}

/// [`button_inspector`] that also edits what ties the menu to your program: [`Bindings`] (a
/// bind toggle on the label of a button or cluster item, with the name and Build / Live below
/// while bound, and the menu-wide settings drawn with [`field_editor_bound`]) and the
/// [`ButtonIds`] of buttons and cluster items (an "Id" row, [`id_editor`], offering the
/// `actions` your program handles; whether they are enabled and visible is theirs). Bindings and ids follow the
/// tree: a removed button's go with it, later siblings' move down, a renumbered item's
/// follow it.
#[allow(clippy::too_many_arguments)]
pub fn button_inspector_bound(
    ui: &mut Ui,
    settings: &mut PieMenuSettings,
    buttons: &mut Vec<PieButton>,
    selection: &mut Selection,
    bindings: &mut Bindings,
    ids: &mut ButtonIds,
    actions: &Actions,
    extras: impl FnMut(&mut Ui, &[usize], &mut PieButton),
) -> InspectorOutput {
    inspect(ui, settings, buttons, selection, Some(bindings), Some((ids, actions)), extras, true)
}

#[allow(clippy::too_many_arguments)]
fn inspect(
    ui: &mut Ui,
    settings: &mut PieMenuSettings,
    buttons: &mut Vec<PieButton>,
    selection: &mut Selection,
    mut bindings: Option<&mut Bindings>,
    mut ids: Option<(&mut ButtonIds, &Actions)>,
    mut extras: impl FnMut(&mut Ui, &[usize], &mut PieButton),
    menu_wide: bool,
) -> InspectorOutput {
    let mut out = InspectorOutput::default();
    *selection = selection.clamped(buttons);

    let stash_id = ui.id().with("pie_inspector_stashes");
    let error_id = ui.id().with("pie_inspector_error");
    let notice_id = ui.id().with("pie_inspector_notice");
    let mut stashes: StashMap = ui.data(|d| d.get_temp(stash_id)).unwrap_or_default();
    let mut error: Option<(Selection, String)> = ui.data(|d| d.get_temp(error_id));
    let mut notice: Option<String> = ui.data(|d| d.get_temp(notice_id));
    if error.as_ref().is_some_and(|(s, _)| s != &*selection) {
        error = None;
    }

    ui.strong(breadcrumb(buttons, selection));
    if let Some(text) = notice.clone() {
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(Color32::ORANGE, text);
            if ui.small_button("dismiss").clicked() {
                notice = None;
            }
        });
    }
    if let Some((_, text)) = &error {
        red(ui, text.clone());
    }

    // Phase 1: Identity, Keys, Contents (read the tree, collect what the user asks for).
    let kind = selection_kind(buttons, settings, selection);
    let intents = match kind {
        SelectionKind::Root => {
            let mut intents = Vec::new();
            section(ui, "Contents", true, |ui| level_contents(ui, settings, buttons, &[], &mut intents));
            intents
        }
        SelectionKind::ClusterItem => {
            item_sections(ui, settings, buttons, selection, bindings.as_deref_mut(), ids.as_mut().map(|(i, a)| (&mut **i, *a)))
        }
        _ => button_sections(ui, settings, buttons, selection, bindings.as_deref_mut(), ids.as_mut().map(|(i, a)| (&mut **i, *a))),
    };
    {
        let mut applied = Applied { buttons, settings, selection, stashes: &mut stashes, out: &mut out };
        for intent in intents {
            // A renumbered cluster item keeps its label binding.
            let renumber = match &intent {
                Intent::ItemNumber(k, to) => button_at(applied.buttons, &applied.selection.path)
                    .and_then(|b| b.cluster_items().get(*k))
                    .map(|i| (applied.selection.path.clone(), i.number, *to)),
                _ => None,
            };
            match applied.apply(intent) {
                Ok(()) => {
                    error = None;
                    if let Some((path, from, to)) = renumber {
                        if let Some(b) = bindings.as_deref_mut() {
                            b.renumber_item(&path, from, to);
                        }
                        if let Some((i, _)) = ids.as_mut() {
                            i.renumber_item(&path, from, to);
                        }
                    }
                }
                Err(reason) => error = Some((applied.selection.clone(), reason)),
            }
        }
    }
    if let Some(b) = bindings.as_deref_mut() {
        for edit in &out.edits {
            b.follow(edit);
        }
        b.prune(buttons);
    }
    if let Some((i, _)) = ids.as_mut() {
        for edit in &out.edits {
            i.follow(edit);
        }
        i.prune(buttons);
    }
    if out.changed && !out.edits.is_empty() {
        ui.ctx().request_repaint();
    }

    // Phase 2: the caller's own controls for the selected button.
    let kind = selection_kind(buttons, settings, selection);
    if !matches!(kind, SelectionKind::Root | SelectionKind::ClusterItem) {
        let path = selection.path.clone();
        if let Some(button) = button_at_mut(buttons, &path) {
            extras(ui, &path, button);
        }
    }

    // Phase 3: the menu-wide settings that concern the selection.
    if menu_wide {
        let fields = relevant_fields(settings, kind);
        section(ui, "Menu-wide settings that affect this", false, |ui| {
            for field in fields {
                let changed = match bindings.as_deref_mut() {
                    Some(b) => field_editor_bound(ui, settings, field, b),
                    None => field_editor(ui, settings, field),
                };
                if changed {
                    out.settings_changed = true;
                    if field == SettingsField::SubPieStyle
                        && settings.sub_pies.style == SubPieStyle::FanOut
                        && let Some(text) = set_menu_sub_pie_style(settings, buttons, SubPieStyle::FanOut)
                    {
                        out.changed = true;
                        out.notice = Some(text.clone());
                        notice = Some(text);
                    }
                }
            }
        });
    }

    ui.data_mut(|d| {
        d.insert_temp(stash_id, stashes);
        match error {
            Some(e) => {
                d.insert_temp(error_id, e);
            }
            None => {
                d.remove_temp::<(Selection, String)>(error_id);
            }
        }
        match notice {
            Some(n) => {
                d.insert_temp(notice_id, n);
            }
            None => {
                d.remove_temp::<String>(notice_id);
            }
        }
    });
    out
}

/// A tree editor over the real [`PieButton`] data: [`button_tree`] and [`button_inspector`]
/// (without the menu-wide settings section) stacked, with the selection kept in egui's memory.
/// Buttons involved in a [`crate::PieMenu::check_buttons`] problem are marked in the tree.
/// `auto_mnemonics` should be the menu's `input.auto_mnemonics`. Returns `true` if the buttons
/// changed. For a layout of your own (tree on one side, inspector on another, a preview) use
/// the two pieces directly.
pub fn buttons_editor(ui: &mut Ui, buttons: &mut Vec<PieButton>, auto_mnemonics: bool) -> bool {
    let mut settings = PieMenuSettings::default();
    settings.input.auto_mnemonics = auto_mnemonics;
    let id = ui.id().with("pie_buttons_editor");
    let mut selection: Selection = ui.data(|d| d.get_temp(id)).unwrap_or_default();
    button_tree(ui, &settings, buttons, &mut selection);
    ui.separator();
    let out = inspect(ui, &mut settings, buttons, &mut selection, None, None, |_, _, _| {}, false);
    ui.data_mut(|d| d.insert_temp(id, selection));
    out.changed
}

#[cfg(test)]
mod tests {
    #![allow(clippy::field_reassign_with_default)]
    use super::*;
    use crate::ClusterItem;
    use egui::Context;
    use SettingsField as F;

    /// Legal under the default settings; one node of every [`SelectionKind`].
    fn tree() -> Vec<PieButton> {
        vec![
            PieButton::new(PieDirection::North).with_label("Copy").with_mnemonic('c'),
            PieButton::new(PieDirection::East).with_label("Fan").with_fan_out(vec![
                PieButton::back(PieDirection::NorthEast),
                PieButton::new(PieDirection::East).with_label("A"),
                PieButton::new(PieDirection::South).with_label("B"),
            ]),
            PieButton::new(PieDirection::South).with_label("More").with_children(vec![
                PieButton::back(PieDirection::North).with_label("Up"),
                PieButton::new(PieDirection::South).with_label("Deep"),
            ]),
            PieButton::cluster(PieDirection::West).with_label("Format").with_items(vec![
                ClusterItem::new(7, "Bold").with_mnemonic('b'),
                ClusterItem::new(8, "Italic"),
            ]),
        ]
    }

    fn every_selection(buttons: &[PieButton]) -> Vec<Selection> {
        fn walk(level: &[PieButton], prefix: &mut Vec<usize>, out: &mut Vec<Selection>) {
            for (i, b) in level.iter().enumerate() {
                prefix.push(i);
                out.push(Selection::button(prefix.clone()));
                for k in 0..b.cluster_items().len() {
                    out.push(Selection::item(prefix.clone(), k));
                }
                walk(b.children(), prefix, out);
                prefix.pop();
            }
        }
        let mut out = vec![Selection::root()];
        walk(buttons, &mut Vec::new(), &mut out);
        out
    }

    fn run(f: impl FnOnce(&mut Ui)) {
        let ctx = Context::default();
        let mut f = Some(f);
        let mut out = ctx.run_ui(Default::default(), |ui| {
            if let Some(f) = f.take() {
                f(ui);
            }
        });
        out.textures_delta.clear();
    }

    #[test]
    fn subtree_filter_keeps_the_way_to_a_match() {
        let t = tree();
        let named = |label: &'static str| move |b: &PieButton| b.label == label;
        // "Deep" is below "More" (index 2): that branch matches, the others do not.
        assert!(subtree_matches(&named("Deep"), &t[2]));
        assert!(!subtree_matches(&named("Deep"), &t[0]));
        assert!(!subtree_matches(&named("Deep"), &t[1]));
        // A button matches itself.
        assert!(subtree_matches(&named("Copy"), &t[0]));
        assert!(!subtree_matches(&|_: &PieButton| false, &t[2]));
    }

    #[test]
    fn filtered_tree_draws() {
        let t = tree();
        let s = PieMenuSettings::default();
        run(|ui| {
            let mut sel = Selection::root();
            button_tree_filtered(ui, &s, &t, &mut sel, &|b| b.label == "Deep");
            assert_eq!(sel, Selection::root());
        });
    }

    #[test]
    fn selection_kinds() {
        let s = PieMenuSettings::default();
        let t = tree();
        let kind = |sel: &Selection| selection_kind(&t, &s, sel);
        assert_eq!(kind(&Selection::root()), SelectionKind::Root);
        assert_eq!(kind(&Selection::button([0])), SelectionKind::Action);
        assert_eq!(kind(&Selection::button([1])), SelectionKind::FanOut);
        assert_eq!(kind(&Selection::button([1, 0])), SelectionKind::FanChild, "also a back button in a fan");
        assert_eq!(kind(&Selection::button([1, 1])), SelectionKind::FanChild);
        assert_eq!(kind(&Selection::button([2])), SelectionKind::ReplaceSubPie);
        assert_eq!(kind(&Selection::button([2, 0])), SelectionKind::Back);
        assert_eq!(kind(&Selection::button([2, 1])), SelectionKind::ReplaceChild);
        assert_eq!(kind(&Selection::button([3])), SelectionKind::Cluster);
        assert_eq!(kind(&Selection::item([3], 1)), SelectionKind::ClusterItem);
        assert_eq!(kind(&Selection::button([9, 9])), SelectionKind::Root, "a missing node counts as the menu");
        // The menu-wide style decides for sub-pies without a style of their own.
        let mut fan = s.clone();
        fan.sub_pies.style = SubPieStyle::FanOut;
        assert_eq!(selection_kind(&t, &fan, &Selection::button([2])), SelectionKind::FanOut);
        assert_eq!(selection_kind(&t, &fan, &Selection::button([2, 1])), SelectionKind::FanChild);
        // A back button at the root acts like an action.
        let root_back = [PieButton::back(PieDirection::North)];
        assert_eq!(selection_kind(&root_back, &s, &Selection::button([0])), SelectionKind::Action);
    }

    #[test]
    fn clamping_a_selection() {
        let t = tree();
        assert_eq!(Selection::button([2, 5]).clamped(&t), Selection::button([2]));
        assert_eq!(Selection::button([7]).clamped(&t), Selection::root());
        assert_eq!(Selection::item([3], 1).clamped(&t), Selection::item([3], 1));
        assert_eq!(Selection::item([3], 5).clamped(&t), Selection::button([3]));
        assert_eq!(Selection::item([0], 0).clamped(&t), Selection::button([0]), "no items there");
        assert_eq!(Selection::default(), Selection::root());
    }

    #[test]
    fn preview_targets() {
        let s = PieMenuSettings::default();
        let t = tree();
        let target = |sel: Selection| preview_target(&t, &s, &sel);
        let p = |level: &[usize], fan, highlight, item| PreviewTarget {
            level_path: level.to_vec(),
            fan_trigger: fan,
            highlight,
            item,
        };
        assert_eq!(target(Selection::root()), PreviewTarget::default());
        assert_eq!(target(Selection::button([0])), p(&[], None, Some(0), None));
        assert_eq!(target(Selection::button([1])), p(&[], Some(1), None, None), "fan-out: parent ring, fan open");
        assert_eq!(target(Selection::button([1, 1])), p(&[], Some(1), Some(1), None), "fan child: same, highlighted");
        assert_eq!(target(Selection::button([2])), p(&[2], None, None, None), "replace sub-pie: its own ring");
        assert_eq!(target(Selection::button([2, 1])), p(&[2], None, Some(1), None));
        assert_eq!(target(Selection::button([2, 0])), p(&[2], None, Some(0), None), "back button");
        assert_eq!(target(Selection::button([3])), p(&[], None, Some(3), None));
        assert_eq!(target(Selection::item([3], 1)), p(&[], None, Some(3), Some(1)));
        // Nested: a fan-out inside a replace sub-pie.
        let nested = vec![PieButton::new(PieDirection::South).with_children(vec![
            PieButton::new(PieDirection::East).with_fan_out(vec![PieButton::new(PieDirection::East)]),
            PieButton::new(PieDirection::North),
        ])];
        assert_eq!(preview_target(&nested, &s, &Selection::button([0, 0, 0])), p(&[0], Some(0), Some(0), None));
        assert_eq!(preview_target(&nested, &s, &Selection::button([0, 0])), p(&[0], Some(0), None, None));
        // A selection that no longer exists previews the nearest node.
        assert_eq!(target(Selection::button([2, 9])), p(&[2], None, None, None));
    }

    #[test]
    fn relevance_table() {
        let mut s = PieMenuSettings::default();
        let hints = [F::HintColor, F::HintSpacing, F::HintSide, F::HintDigits];
        let with_hints = |head: &[SettingsField], tail: &[SettingsField]| {
            [head, &hints[..], tail].concat()
        };
        assert_eq!(
            relevant_fields(&s, SelectionKind::Action),
            with_hints(&[F::TextColor, F::AutoMnemonics, F::UseNumberKeys], &[F::HoverEmphasis, F::HoverDuration])
        );
        assert_eq!(
            relevant_fields(&s, SelectionKind::ReplaceChild),
            with_hints(&[F::TextColor, F::AutoMnemonics, F::UseNumberKeys], &[F::SubPieHoverEmphasis, F::HoverDuration])
        );
        assert_eq!(
            relevant_fields(&s, SelectionKind::Back),
            with_hints(
                &[F::TextColor, F::AutoMnemonics, F::UseNumberKeys],
                &[F::SubPieHoverEmphasis, F::HoverDuration, F::SubPieCenter, F::SubPieHoverDelay]
            )
        );
        assert_eq!(
            relevant_fields(&s, SelectionKind::ReplaceSubPie),
            with_hints(
                &[F::TextColor, F::AutoMnemonics, F::UseNumberKeys],
                &[
                    F::HoverEmphasis,
                    F::SubPieHoverEmphasis,
                    F::HoverDuration,
                    F::SubPieStyle,
                    F::SubPieCenter,
                    F::SubPieHoverDelay,
                    F::TitleShow
                ]
            )
        );
        assert_eq!(
            relevant_fields(&s, SelectionKind::FanOut),
            with_hints(
                &[F::TextColor, F::AutoMnemonics, F::UseNumberKeys],
                &[
                    F::HoverEmphasis,
                    F::SubPieHoverEmphasis,
                    F::HoverDuration,
                    F::SubPieStyle,
                    F::FanOutDelay,
                    F::FanOutRadius,
                    F::FanOutOpenerAction
                ]
            )
        );
        assert_eq!(
            relevant_fields(&s, SelectionKind::FanChild),
            with_hints(&[F::SubPieHoverEmphasis, F::FanOutRadius, F::TextColor], &[])
        );
        assert_eq!(relevant_fields(&s, SelectionKind::Cluster), [F::AutoMnemonics, F::UseMnemonicKeys, F::SelectOnPrimaryClick]);
        assert_eq!(relevant_fields(&s, SelectionKind::ClusterItem), [F::AutoMnemonics, F::UseMnemonicKeys]);
        let root = relevant_fields(&s, SelectionKind::Root);
        for g in [SettingsGroup::Ring, SettingsGroup::CenterIndicator, SettingsGroup::Title] {
            assert!(g.fields().iter().all(|f| root.contains(f)), "{g:?}");
        }
        assert!(root.contains(&F::HoverEmphasis) && root.contains(&F::HoverDuration) && root.contains(&F::AutoMnemonics));
        assert!(!root.contains(&F::SubPieHoverEmphasis) && !root.contains(&F::TextColor));
        // Number hints only while number keys are on; and no field twice.
        s.input.use_number_keys = false;
        for kind in [SelectionKind::Action, SelectionKind::FanChild, SelectionKind::ReplaceSubPie] {
            let fields = relevant_fields(&s, kind);
            assert!(hints.iter().all(|h| !fields.contains(h)), "{kind:?}");
        }
        assert_eq!(relevant_fields(&s, SelectionKind::FanChild), [F::SubPieHoverEmphasis, F::FanOutRadius, F::TextColor]);
        for kind in [
            SelectionKind::Root,
            SelectionKind::Action,
            SelectionKind::Back,
            SelectionKind::ReplaceSubPie,
            SelectionKind::FanOut,
            SelectionKind::ReplaceChild,
            SelectionKind::FanChild,
            SelectionKind::Cluster,
            SelectionKind::ClusterItem,
        ] {
            let fields = relevant_fields(&PieMenuSettings::default(), kind);
            let mut sorted = fields.clone();
            sorted.sort_by_key(|f| f.path());
            sorted.dedup();
            assert_eq!(sorted.len(), fields.len(), "{kind:?} lists a field twice");
        }
    }

    #[test]
    fn the_marked_tree_draws() {
        let t = tree();
        let s = PieMenuSettings::default();
        let mut sel = Selection::button([2, 1]);
        run(|ui| button_tree_marked(ui, &s, &t, &mut sel, None, &[vec![2, 1]]));
        run(|ui| button_tree_marked(ui, &s, &t, &mut sel, Some(&|b: &PieButton| b.label == "Deep"), &[vec![0]]));
        assert_eq!(sel, Selection::button([2, 1]));
    }

    #[test]
    fn the_bound_inspector_draws_and_keeps_bindings() {
        use super::super::bindings::{Binding, BindingMode};
        let mut buttons = tree();
        let mut bindings = Bindings::new();
        // Every node's label, a cluster item's and a settings field bound.
        for sel in every_selection(&buttons) {
            let target = match sel.item {
                Some(k) => BindTarget::ItemLabel(sel.path.clone(), button_at(&buttons, &sel.path).unwrap().cluster_items()[k].number),
                None if !sel.path.is_empty() => BindTarget::Label(sel.path.clone()),
                None => BindTarget::Field(F::TitleTextColor),
            };
            bindings.set(target, Binding::new("tr(\"x\")", BindingMode::Live));
        }
        let mut ids = ButtonIds::new();
        ids.set(IdKey { path: vec![0], item: None }, "a.b");
        let before = (format!("{buttons:?}"), bindings.clone(), ids.clone());
        for sel in every_selection(&buttons) {
            let mut settings = PieMenuSettings::default();
            let mut selection = sel.clone();
            let mut seen = None;
            run(|ui| {
                let mut actions = Actions::new();
                actions.add("a.b");
                seen = Some(button_inspector_bound(ui, &mut settings, &mut buttons, &mut selection, &mut bindings, &mut ids, &actions, |_, _, _| {}))
            });
            assert_eq!(seen, Some(InspectorOutput::default()), "{sel:?}");
        }
        assert_eq!(before, (format!("{buttons:?}"), bindings, ids));
    }

    #[test]
    fn every_selection_draws_and_untouched_means_unchanged() {
        let mut fan_menu = PieMenuSettings::default();
        fan_menu.sub_pies.style = SubPieStyle::FanOut;
        let mut numbers_off = PieMenuSettings::default();
        numbers_off.input.use_number_keys = false;
        for base in [PieMenuSettings::default(), fan_menu, numbers_off] {
            let mut t = tree();
            // A tree with problems must not panic either.
            let mut bad = tree();
            bad.push(PieButton::new(PieDirection::North).with_label("Dup").with_mnemonic('c'));
            for buttons in [&mut t, &mut bad] {
                for sel in every_selection(buttons) {
                    let mut settings = base.clone();
                    let mut selection = sel.clone();
                    let before = (format!("{buttons:?}"), format!("{settings:?}"));
                    let mut seen = None;
                    run(|ui| {
                        button_tree(ui, &settings, buttons, &mut selection);
                        let mut extras_for = None;
                        let out = button_inspector(ui, &mut settings, buttons, &mut selection, |_, path, _| {
                            extras_for = Some(path.to_vec());
                        });
                        seen = Some((out, extras_for));
                    });
                    let (out, extras_for) = seen.unwrap();
                    assert_eq!(out, InspectorOutput::default(), "{sel:?}");
                    assert_eq!(selection, sel, "{sel:?}");
                    assert_eq!(before, (format!("{buttons:?}"), format!("{settings:?}")), "{sel:?}");
                    let expect = (!sel.path.is_empty() && sel.item.is_none()).then(|| sel.path.clone());
                    assert_eq!(extras_for, expect, "extras run for a selected button only");
                }
            }
        }
        // The all-in-one editor.
        let mut t = tree();
        let before = format!("{t:?}");
        let mut changed = true;
        run(|ui| changed = buttons_editor(ui, &mut t, true));
        assert!(!changed);
        assert_eq!(before, format!("{t:?}"));
    }

    fn apply(buttons: &mut Vec<PieButton>, selection: &mut Selection, intent: Intent) -> (Result<(), String>, InspectorOutput) {
        let settings = PieMenuSettings::default();
        let mut stashes = StashMap::default();
        let mut out = InspectorOutput::default();
        let r = Applied { buttons, settings: &settings, selection, stashes: &mut stashes, out: &mut out }.apply(intent);
        (r, out)
    }

    #[test]
    fn intents_report_tree_edits() {
        let mut t = tree();
        // Remove: later siblings shift, the parent is selected.
        let mut sel = Selection::button([2, 0]);
        let (r, out) = apply(&mut t, &mut sel, Intent::Remove);
        assert_eq!(r, Ok(()));
        assert_eq!(out.edits, [TreeEdit::Removed(vec![2, 0])]);
        assert_eq!(sel, Selection::button([2]));
        assert_eq!(t[2].children().len(), 1);
        // The last child stays: refused.
        let mut sel = Selection::button([2, 0]);
        let (r, out) = apply(&mut t, &mut sel, Intent::Remove);
        assert!(r.is_err() && out.edits.is_empty() && !out.changed);
        // Add appends and selects.
        let mut sel = Selection::root();
        let (r, out) = apply(&mut t, &mut sel, Intent::Add(vec![], AddKind::Action));
        assert_eq!(r, Ok(()));
        assert_eq!(out.edits, [TreeEdit::Added(vec![4])]);
        assert_eq!(sel, Selection::button([4]));
        // Converting a sub-pie drops its children and reports it.
        let mut sel = Selection::button([2]);
        let (_, out) = apply(&mut t, &mut sel, Intent::Kind(KindChoice::Action));
        assert_eq!(out.edits, [TreeEdit::Dropped(vec![2])]);
        assert!(t[2].children().is_empty());
        // Becoming a fan-out drops the back child facing the parent, reported as removals.
        let mut t = vec![new_sub_pie_for(PieDirection::East, false, SubPieStyle::Replace)];
        let mut sel = Selection::button([0]);
        let (r, out) = apply(&mut t, &mut sel, Intent::Style(Some(SubPieStyle::FanOut)));
        assert_eq!(r, Ok(()));
        assert_eq!(out.edits, [TreeEdit::Removed(vec![0, 5])]);
        // Refusals come with the reason and change nothing.
        let mut t = tree();
        let mut sel = Selection::button([1, 1]);
        let (r, out) = apply(&mut t, &mut sel, Intent::Direction(PieDirection::West));
        assert_eq!(r, Err("faces the parent".to_string()));
        assert!(!out.changed);
        let (r, _) = apply(&mut t, &mut Selection::button([1]), Intent::Mnemonic(MnemonicChange::Key('c')));
        assert_eq!(r, Err("'c' is taken by \"Copy\"".to_string()));
        let (r, _) = apply(&mut t, &mut Selection::item([3], 0), Intent::ItemNumber(0, 8));
        assert!(r.unwrap_err().contains("Italic"));
        // Items: add selects the new one, removing the selected one selects the cluster.
        let mut sel = Selection::button([3]);
        assert!(apply(&mut t, &mut sel, Intent::AddItem).0.is_ok());
        assert_eq!((sel.item, t[3].cluster_items().len()), (Some(2), 3));
        assert!(apply(&mut t, &mut sel, Intent::RemoveItem(2)).0.is_ok());
        assert_eq!(sel.item, None);
    }

    #[test]
    fn stash_map_follows_removals() {
        let mut m = StashMap::default();
        for key in [vec![1], vec![2], vec![2, 0], vec![3, 1], vec![0]] {
            m.0.insert(key, Stash::default());
        }
        m.removed(&[1]);
        let mut keys: Vec<_> = m.0.keys().cloned().collect();
        keys.sort();
        assert_eq!(keys, [vec![0], vec![1], vec![1, 0], vec![2, 1]]);
        m.dropped_below(&[1]);
        let mut keys: Vec<_> = m.0.keys().cloned().collect();
        keys.sort();
        assert_eq!(keys, [vec![0], vec![1], vec![2, 1]]);
    }

    #[test]
    fn converting_back_through_the_inspector_restores_children() {
        let mut t = tree();
        let before = format!("{:?}", t[2]);
        let settings = PieMenuSettings::default();
        let mut stashes = StashMap::default();
        let mut out = InspectorOutput::default();
        let mut sel = Selection::button([2]);
        for k in [KindChoice::Action, KindChoice::SubPie] {
            Applied { buttons: &mut t, settings: &settings, selection: &mut sel, stashes: &mut stashes, out: &mut out }
                .apply(Intent::Kind(k))
                .unwrap();
        }
        assert_eq!(format!("{:?}", t[2]), before);
    }

    #[test]
    fn the_tree_marks_problems() {
        let s = PieMenuSettings::default();
        let mut t = tree();
        assert_eq!(problem_button_paths(&tree_problems(&s, &t)), Vec::<Vec<usize>>::new());
        // A clash deep in a fan-out marks that child (and so its ancestors in the tree).
        if let PieButtonKind::SubPie { children, .. } = &mut t[1].kind {
            children[2].direction = PieDirection::East;
        }
        let bad = problem_button_paths(&tree_problems(&s, &t));
        assert!(bad.contains(&vec![1, 1]) || bad.contains(&vec![1, 2]), "{bad:?}");
        // The tree draws the marked tree without a panic.
        let mut selection = Selection::root();
        run(|ui| button_tree(ui, &s, &t, &mut selection));
    }
}
