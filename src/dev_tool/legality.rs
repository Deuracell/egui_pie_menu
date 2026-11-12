//! The rules of what the editor may offer, as pure functions over the button tree: which
//! directions and kinds a button may take, what adding, removing and converting do, how
//! mnemonics and item numbers stay unique. Every operation either leaves
//! [`tree_problems`] empty (when it was empty before) or refuses with a reason.
use super::KindChoice;
use crate::mnemonic::{fold_char, is_mnemonic_letter, resolve_all};
use crate::settings::*;
use crate::{ButtonProblem, ClusterItem, Mnemonic, PieButton, PieButtonKind, PieMenu};

// ---------------------------------------------------------------------------------------
// Navigating the tree
// ---------------------------------------------------------------------------------------

/// The buttons of the level reached through the sub-pies at `parent_path` (empty: the root level).
pub(crate) fn level_at<'a>(buttons: &'a [PieButton], parent_path: &[usize]) -> Option<&'a [PieButton]> {
    let mut level = buttons;
    for &i in parent_path {
        level = match &level.get(i)?.kind {
            PieButtonKind::SubPie { children, .. } => children,
            _ => return None,
        };
    }
    Some(level)
}

/// The button at `path` (its own index last).
pub(crate) fn button_at<'a>(buttons: &'a [PieButton], path: &[usize]) -> Option<&'a PieButton> {
    let (last, parent) = path.split_last()?;
    level_at(buttons, parent)?.get(*last)
}

/// The vector holding the level at `parent_path`.
pub(crate) fn level_mut<'a>(buttons: &'a mut Vec<PieButton>, parent_path: &[usize]) -> Option<&'a mut Vec<PieButton>> {
    let mut level = buttons;
    for &i in parent_path {
        level = match &mut level.get_mut(i)?.kind {
            PieButtonKind::SubPie { children, .. } => children,
            _ => return None,
        };
    }
    Some(level)
}

/// The button at `path`, mutably.
pub(crate) fn button_at_mut<'a>(buttons: &'a mut Vec<PieButton>, path: &[usize]) -> Option<&'a mut PieButton> {
    let (last, parent) = path.split_last()?;
    level_mut(buttons, parent)?.get_mut(*last)
}

/// The style a sub-pie button really has (its own, else the menu's); `None` for other kinds.
pub(crate) fn effective_style(button: &PieButton, settings: &PieMenuSettings) -> Option<SubPieStyle> {
    match &button.kind {
        PieButtonKind::SubPie { style, .. } => Some(style.unwrap_or(settings.sub_pies.style)),
        _ => None,
    }
}

/// What kind of level a list of buttons is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LevelKind {
    /// The menu's own ring.
    Root,
    /// The children of a replace-style sub-pie.
    Replace,
    /// The children of a fan-out whose opening button points in this direction.
    Fan(PieDirection),
}

/// The kind of the level whose buttons are the children of the button at `parent_path`.
pub(crate) fn level_kind(buttons: &[PieButton], settings: &PieMenuSettings, parent_path: &[usize]) -> LevelKind {
    match button_at(buttons, parent_path) {
        None => LevelKind::Root,
        Some(opener) => match effective_style(opener, settings) {
            Some(SubPieStyle::FanOut) => LevelKind::Fan(opener.direction),
            _ => LevelKind::Replace,
        },
    }
}

fn level_candidates(kind: LevelKind) -> Vec<PieDirection> {
    match kind {
        LevelKind::Fan(opener) => opener.fan_out_slots().to_vec(),
        _ => PieDirection::ALL.to_vec(),
    }
}

fn step_of(d: PieDirection) -> usize {
    PieDirection::ALL.iter().position(|x| *x == d).unwrap_or(0)
}

/// `d` turned clockwise by `steps` eighths of a circle.
fn rotated(d: PieDirection, steps: usize) -> PieDirection {
    PieDirection::ALL[(step_of(d) + steps) % 8]
}

fn name_of(label: &str) -> String {
    if label.is_empty() { "(no label)".to_string() } else { format!("\"{label}\"") }
}

// ---------------------------------------------------------------------------------------
// Problems
// ---------------------------------------------------------------------------------------

/// A [`PieMenu`] with these settings, for [`PieMenu::check_buttons`] and friends.
pub(crate) fn problem_menu_for(settings: &PieMenuSettings) -> PieMenu {
    PieMenu::new().with_settings(settings.clone())
}

/// Every problem of the whole tree: `(path of the sub-pie button whose children form the
/// level, problem)`, the root level having the empty path. Unlike [`PieMenu::check_buttons`]
/// (one level) this descends into all sub-pies.
pub(crate) fn tree_problems(settings: &PieMenuSettings, buttons: &[PieButton]) -> Vec<(Vec<usize>, ButtonProblem)> {
    fn walk(menu: &PieMenu, level: &[PieButton], path: &mut Vec<usize>, out: &mut Vec<(Vec<usize>, ButtonProblem)>) {
        out.extend(menu.check_buttons(level).into_iter().map(|p| (path.clone(), p)));
        for (i, b) in level.iter().enumerate() {
            if let PieButtonKind::SubPie { children, .. } = &b.kind {
                path.push(i);
                walk(menu, children, path, out);
                path.pop();
            }
        }
    }
    let menu = problem_menu_for(settings);
    let mut out = Vec::new();
    walk(&menu, buttons, &mut Vec::new(), &mut out);
    out
}

/// The paths of the buttons involved in a problem of the tree (the sub-pie levels' parents
/// included, so a whole branch can be marked).
pub(crate) fn problem_button_paths(problems: &[(Vec<usize>, ButtonProblem)]) -> Vec<Vec<usize>> {
    let mut out: Vec<Vec<usize>> = Vec::new();
    for (level, problem) in problems {
        // An `InFanOut` wraps a problem among the children; mark those, not the opener.
        let (inner, nested) = match problem {
            ButtonProblem::InFanOut { parent, problem } => (&**problem, Some(*parent)),
            other => (other, None),
        };
        let mut base = level.clone();
        if let Some(parent) = nested {
            base.push(parent);
        }
        for i in inner.buttons() {
            let mut path = base.clone();
            path.push(i);
            out.push(path);
        }
    }
    out
}

// ---------------------------------------------------------------------------------------
// Directions
// ---------------------------------------------------------------------------------------

/// The directions the button at `path` may take: the ones no sibling uses, plus its own; in a
/// fan-out only the five [`PieDirection::fan_out_slots`]. (The inspector asks per direction
/// through [`direction_block_reason`].)
#[cfg(test)]
pub(crate) fn allowed_directions(buttons: &[PieButton], settings: &PieMenuSettings, path: &[usize]) -> Vec<PieDirection> {
    let Some((own, parent)) = path.split_last() else { return Vec::new() };
    let Some(level) = level_at(buttons, parent) else { return Vec::new() };
    let own_dir = level.get(*own).map(|b| b.direction);
    level_candidates(level_kind(buttons, settings, parent))
        .into_iter()
        .filter(|d| Some(*d) == own_dir || level.iter().all(|b| b.direction != *d))
        .collect()
}

/// The directions of the level at `parent_path` that nothing occupies (and a fan-out allows).
pub(crate) fn free_directions(buttons: &[PieButton], settings: &PieMenuSettings, parent_path: &[usize]) -> Vec<PieDirection> {
    let Some(level) = level_at(buttons, parent_path) else { return Vec::new() };
    level_candidates(level_kind(buttons, settings, parent_path))
        .into_iter()
        .filter(|d| level.iter().all(|b| b.direction != *d))
        .collect()
}

/// Why `to` is not available to the button at `path`, if it is not.
pub(crate) fn direction_block_reason(
    buttons: &[PieButton],
    settings: &PieMenuSettings,
    path: &[usize],
    to: PieDirection,
) -> Option<String> {
    let (own, parent) = path.split_last()?;
    let level = level_at(buttons, parent)?;
    if let LevelKind::Fan(opener) = level_kind(buttons, settings, parent)
        && opener.blocks_fan_out(to)
    {
        return Some("faces the parent".to_string());
    }
    level
        .iter()
        .enumerate()
        .find(|(i, b)| i != own && b.direction == to)
        .map(|(_, b)| format!("taken by {}", name_of(&b.label)))
}

/// Moves the button at `path` to `to`. A fan-out button takes its children along: they turn by
/// the same step, so they keep their place relative to it. Refuses a direction that is taken
/// or (in a fan-out) faces the parent.
pub(crate) fn set_direction(
    buttons: &mut Vec<PieButton>,
    settings: &PieMenuSettings,
    path: &[usize],
    to: PieDirection,
) -> Result<(), String> {
    if let Some(reason) = direction_block_reason(buttons, settings, path, to) {
        return Err(reason);
    }
    let fan = effective_style(button_at(buttons, path).ok_or("no such button")?, settings) == Some(SubPieStyle::FanOut);
    let button = button_at_mut(buttons, path).ok_or("no such button")?;
    let steps = (step_of(to) + 8 - step_of(button.direction)) % 8;
    button.direction = to;
    if fan && let PieButtonKind::SubPie { children, .. } = &mut button.kind {
        for child in children {
            child.direction = rotated(child.direction, steps);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------
// Kinds
// ---------------------------------------------------------------------------------------

/// The kinds the button at `path` may be converted to (its current kind always included): in a
/// fan-out only Action and Back; at the root no Back (unless it already is one).
pub(crate) fn allowed_kinds(buttons: &[PieButton], settings: &PieMenuSettings, path: &[usize]) -> Vec<KindChoice> {
    let Some((_, parent)) = path.split_last() else { return Vec::new() };
    let Some(button) = button_at(buttons, path) else { return Vec::new() };
    let current = super::kind_choice(button);
    KindChoice::ALL
        .into_iter()
        .filter(|k| {
            *k == current
                || match level_kind(buttons, settings, parent) {
                    LevelKind::Fan(_) => matches!(k, KindChoice::Action | KindChoice::Back),
                    LevelKind::Root => *k != KindChoice::Back,
                    LevelKind::Replace => true,
                }
        })
        .collect()
}

/// A ready-made sub-pie at `dir`: five children "Item 1".."Item 5" in the five fan-out slots
/// (valid for both styles) and, for a replace-style one, a back button opposite `dir`. A fan
/// is `with_fan_out`; a replace-style one follows the menu (`None`) unless the menu's own style
/// is a fan-out, when it is pinned to [`SubPieStyle::Replace`].
pub(crate) fn new_sub_pie_for(dir: PieDirection, fan: bool, menu_style: SubPieStyle) -> PieButton {
    let mut b = super::new_sub_pie(dir, fan);
    if !fan && menu_style == SubPieStyle::FanOut {
        b = b.with_sub_pie_style(Some(SubPieStyle::Replace));
    }
    b
}

/// What a button had before a conversion dropped it, to bring back on converting back.
#[derive(Clone, Debug, Default)]
pub(crate) struct Stash {
    pub(crate) sub_pie: Option<(Vec<PieButton>, Option<SubPieStyle>)>,
    pub(crate) items: Option<Vec<ClusterItem>>,
}

/// Converts `button` to another kind, keeping direction, label, mnemonic and enabled.
///
/// Leaving a sub-pie or cluster stashes its children / items in `stash`; entering one takes
/// them back out (a restored sub-pie that would be illegal now is replaced by a fresh one).
/// A fresh sub-pie is [`new_sub_pie_for`] the button's direction, in the menu's style; a fresh
/// cluster has one item numbered 0 and no mnemonic. Returns `true` when children (a whole
/// subtree) were dropped. Does nothing when the kind is already `to`.
pub(crate) fn convert_kind_in(
    button: &mut PieButton,
    to: KindChoice,
    settings: &PieMenuSettings,
    stash: &mut Stash,
) -> bool {
    if super::kind_choice(button) == to {
        return false;
    }
    let mut dropped = false;
    match std::mem::take(&mut button.kind) {
        PieButtonKind::SubPie { children, style } => {
            dropped = !children.is_empty();
            stash.sub_pie = Some((children, style));
        }
        PieButtonKind::Cluster(items) => stash.items = Some(items),
        PieButtonKind::Action | PieButtonKind::Back => {}
    }
    match to {
        KindChoice::Action => button.kind = PieButtonKind::Action,
        KindChoice::Back => button.kind = PieButtonKind::Back,
        KindChoice::SubPie => {
            let restored = stash.sub_pie.take().map(|(children, style)| PieButtonKind::SubPie { children, style });
            let mut candidate = button.clone();
            if let Some(kind) = restored {
                candidate.kind = kind;
                if tree_problems(settings, std::slice::from_ref(&candidate)).is_empty() {
                    button.kind = candidate.kind;
                    return dropped;
                }
            }
            let fan = settings.sub_pies.style == SubPieStyle::FanOut;
            button.kind = new_sub_pie_for(button.direction, fan, settings.sub_pies.style).kind;
        }
        KindChoice::Cluster => {
            button.kind = PieButtonKind::Cluster(stash.items.take().unwrap_or_else(|| vec![ClusterItem::new(0, "Item")]));
            button.mnemonic = Mnemonic::None;
        }
    }
    dropped
}

// ---------------------------------------------------------------------------------------
// Adding and removing
// ---------------------------------------------------------------------------------------

/// What "Add" can create.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AddKind {
    Action,
    Back,
    SubPie,
    FanOut,
    Cluster,
}

impl AddKind {
    pub(crate) fn name(self) -> &'static str {
        match self {
            AddKind::Action => "Button",
            AddKind::Back => "Back button",
            AddKind::SubPie => "Sub-pie",
            AddKind::FanOut => "Fan-out",
            AddKind::Cluster => "Cluster",
        }
    }
}

/// The kinds that can be added to the level at `parent_path`: none when it has no free
/// direction (8 buttons, 5 in a fan-out); in a fan-out only a plain or back button; a back
/// button only below the root and only if the level has none yet.
pub(crate) fn addable_kinds(buttons: &[PieButton], settings: &PieMenuSettings, parent_path: &[usize]) -> Vec<AddKind> {
    let Some(level) = level_at(buttons, parent_path) else { return Vec::new() };
    if free_directions(buttons, settings, parent_path).is_empty() {
        return Vec::new();
    }
    let kind = level_kind(buttons, settings, parent_path);
    let mut out = vec![AddKind::Action];
    if kind != LevelKind::Root && !level.iter().any(|b| b.is_back()) {
        out.push(AddKind::Back);
    }
    if !matches!(kind, LevelKind::Fan(_)) {
        out.extend([AddKind::SubPie, AddKind::FanOut, AddKind::Cluster]);
    }
    out
}

/// Appends a new button of `kind` to the level at `parent_path`, in its first free direction
/// (a back button prefers the one opposite the sub-pie it leaves). Returns its index, or `None`
/// if that kind cannot be added now ([`addable_kinds`]).
pub(crate) fn add_button(
    buttons: &mut Vec<PieButton>,
    settings: &PieMenuSettings,
    parent_path: &[usize],
    kind: AddKind,
) -> Option<usize> {
    if !addable_kinds(buttons, settings, parent_path).contains(&kind) {
        return None;
    }
    let free = free_directions(buttons, settings, parent_path);
    let mut dir = *free.first()?;
    if kind == AddKind::Back
        && let Some(parent) = button_at(buttons, parent_path)
        && free.contains(&parent.direction.opposite())
    {
        dir = parent.direction.opposite();
    }
    let button = match kind {
        AddKind::Action => PieButton::new(dir).with_label("New"),
        AddKind::Back => PieButton::back(dir),
        AddKind::SubPie => new_sub_pie_for(dir, false, settings.sub_pies.style),
        AddKind::FanOut => new_sub_pie_for(dir, true, settings.sub_pies.style),
        AddKind::Cluster => {
            PieButton::cluster(dir).with_label("Cluster").with_items(vec![ClusterItem::new(0, "Item")])
        }
    };
    let level = level_mut(buttons, parent_path)?;
    level.push(button);
    Some(level.len() - 1)
}

/// Why the button at `path` cannot be removed: the menu keeps at least one button and a
/// sub-pie at least one child (convert it to a plain button instead).
pub(crate) fn remove_block_reason(buttons: &[PieButton], path: &[usize]) -> Option<&'static str> {
    let (_, parent) = path.split_last()?;
    let level = level_at(buttons, parent)?;
    if level.len() > 1 {
        None
    } else if parent.is_empty() {
        Some("the menu needs at least one button")
    } else {
        Some("a sub-pie needs at least one button; convert it to a plain button instead")
    }
}

/// Removes the button at `path` (later siblings shift down by one). Refuses per
/// [`remove_block_reason`].
pub(crate) fn remove_button(buttons: &mut Vec<PieButton>, path: &[usize]) -> bool {
    if remove_block_reason(buttons, path).is_some() {
        return false;
    }
    let Some((last, parent)) = path.split_last() else { return false };
    match level_mut(buttons, parent) {
        Some(level) if *last < level.len() => {
            level.remove(*last);
            true
        }
        _ => false,
    }
}

/// Why a cluster's item cannot be removed (a cluster keeps at least one).
pub(crate) fn item_remove_block_reason(item_count: usize) -> Option<&'static str> {
    (item_count <= 1).then_some("a cluster needs at least one item")
}

/// The number a new item of a cluster gets: one more than the largest in use.
pub(crate) fn next_item_number(items: &[ClusterItem]) -> usize {
    items.iter().map(|i| i.number + 1).max().unwrap_or(0)
}

/// Appends an item with the next free number to the cluster at `path`; returns its index.
pub(crate) fn add_item(buttons: &mut Vec<PieButton>, path: &[usize]) -> Option<usize> {
    match &mut button_at_mut(buttons, path)?.kind {
        PieButtonKind::Cluster(items) => {
            let n = next_item_number(items);
            items.push(ClusterItem::new(n, "Item"));
            Some(items.len() - 1)
        }
        _ => None,
    }
}

/// Removes item `k` of the cluster at `path`. Refuses the last item.
pub(crate) fn remove_item(buttons: &mut Vec<PieButton>, path: &[usize], k: usize) -> bool {
    match button_at_mut(buttons, path).map(|b| &mut b.kind) {
        Some(PieButtonKind::Cluster(items)) if k < items.len() && item_remove_block_reason(items.len()).is_none() => {
            items.remove(k);
            true
        }
        _ => false,
    }
}

/// Gives item `k` the number `n`, refusing a number another item has.
pub(crate) fn set_item_number(items: &mut [ClusterItem], k: usize, n: usize) -> Result<(), String> {
    if let Some(other) = items.iter().enumerate().find(|(i, it)| *i != k && it.number == n).map(|(_, it)| it) {
        return Err(format!("number {n} is used by {}", name_of(&other.label)));
    }
    items.get_mut(k).ok_or("no such item")?.number = n;
    Ok(())
}

// ---------------------------------------------------------------------------------------
// Mnemonics and enabled
// ---------------------------------------------------------------------------------------

/// What to do with the mnemonic of a button or cluster item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MnemonicChange {
    Auto,
    None,
    /// An explicit key.
    Key(char),
    /// Switch to an explicit key, taking the first free letter of the label.
    KeyFromLabel,
}

/// Who holds an explicit key in a level.
struct Holder {
    key: char,
    label: String,
    /// `(button index, item index)`.
    at: (usize, Option<usize>),
}

/// The explicit keys of all enabled owners of the level (buttons, and items of enabled clusters).
fn explicit_holders(level: &[PieButton]) -> Vec<Holder> {
    let mut out = Vec::new();
    for (i, b) in level.iter().enumerate() {
        if !b.enabled {
            continue;
        }
        if b.is_cluster() {
            for (k, item) in b.cluster_items().iter().enumerate() {
                if let (true, Mnemonic::Key(c)) = (item.enabled, item.mnemonic) {
                    out.push(Holder { key: fold_char(c), label: item.label.clone(), at: (i, Some(k)) });
                }
            }
        } else if let Mnemonic::Key(c) = b.mnemonic {
            out.push(Holder { key: fold_char(c), label: b.label.clone(), at: (i, None) });
        }
    }
    out
}

/// An enabled explicit key of the owner `(button, item)` (any item when `None` and the button
/// is a cluster) that another enabled owner of the level holds too: `(key, other's label)`.
fn explicit_clash(level: &[PieButton], button: usize, item: Option<usize>) -> Option<(char, String)> {
    let holders = explicit_holders(level);
    holders
        .iter()
        .filter(|h| h.at.0 == button && (item.is_none() || h.at.1 == item))
        .find_map(|mine| holders.iter().find(|o| o.key == mine.key && o.at != mine.at).map(|o| (mine.key, o.label.clone())))
}

/// The mnemonic of the button (or item) at `path` / `item`, mutably.
fn mnemonic_slot<'a>(buttons: &'a mut Vec<PieButton>, path: &[usize], item: Option<usize>) -> Result<&'a mut Mnemonic, String> {
    let button = button_at_mut(buttons, path).ok_or("no such button")?;
    match (&mut button.kind, item) {
        (PieButtonKind::Cluster(items), Some(k)) => Ok(&mut items.get_mut(k).ok_or("no such item")?.mnemonic),
        (PieButtonKind::Cluster(_), None) => Err("a cluster has no mnemonic of its own; its items have".to_string()),
        (_, Some(_)) => Err("not a cluster".to_string()),
        (_, None) => Ok(&mut button.mnemonic),
    }
}

/// The first free letter of `label` (word initials first, then the rest), else the first free
/// letter a..z; `taken` are folded.
pub(crate) fn first_free_key(label: &str, taken: &[char]) -> Option<char> {
    let initials = label.split(|c: char| !c.is_alphanumeric()).filter_map(|w| w.chars().next());
    initials
        .chain(label.chars())
        .filter(|c| is_mnemonic_letter(*c))
        .map(fold_char)
        .chain('a'..='z')
        .find(|c| !taken.contains(c))
}

/// Changes the mnemonic of a button or (with `item`) a cluster item.
///
/// An explicit key another enabled owner of the level has is refused ("taken by ..."); a
/// character that is no letter too. Switching to a key picks the first free letter of the label
/// (free of explicit and generated keys alike).
pub(crate) fn set_mnemonic(
    buttons: &mut Vec<PieButton>,
    settings: &PieMenuSettings,
    path: &[usize],
    item: Option<usize>,
    change: MnemonicChange,
) -> Result<(), String> {
    let (own, parent) = path.split_last().ok_or("no such button")?;
    let key = match change {
        MnemonicChange::Auto => Mnemonic::Auto,
        MnemonicChange::None => Mnemonic::None,
        MnemonicChange::Key(c) => {
            if !is_mnemonic_letter(c) {
                return Err(format!("'{c}' is not a letter"));
            }
            let level = level_at(buttons, parent).ok_or("no such level")?;
            if let Some(h) = explicit_holders(level).iter().find(|h| h.key == fold_char(c) && h.at != (*own, item)) {
                return Err(format!("'{c}' is taken by {}", name_of(&h.label)));
            }
            Mnemonic::Key(c)
        }
        MnemonicChange::KeyFromLabel => {
            let level = level_at(buttons, parent).ok_or("no such level")?;
            let mut probe = level.to_vec();
            let button = probe.get_mut(*own).ok_or("no such button")?;
            let label = match (&mut button.kind, item) {
                (PieButtonKind::Cluster(items), Some(k)) => {
                    let it = items.get_mut(k).ok_or("no such item")?;
                    it.mnemonic = Mnemonic::None;
                    it.label.clone()
                }
                (PieButtonKind::Cluster(_), None) => return Err("a cluster has no mnemonic of its own".to_string()),
                (_, _) => {
                    button.mnemonic = Mnemonic::None;
                    button.label.clone()
                }
            };
            let (per_button, per_item) = resolve_all(&probe, settings.input.auto_mnemonics);
            let taken: Vec<char> = per_button.iter().flatten().chain(per_item.iter().flatten().flatten()).copied().collect();
            let c = first_free_key(&label, &taken).ok_or("no free letter left")?;
            Mnemonic::Key(c)
        }
    };
    *mnemonic_slot(buttons, path, item)? = key;
    Ok(())
}

/// Enables or disables the button (or cluster item). Enabling one whose explicit key another
/// enabled owner now holds is refused.
pub(crate) fn set_enabled(
    buttons: &mut Vec<PieButton>,
    path: &[usize],
    item: Option<usize>,
    enabled: bool,
) -> Result<(), String> {
    let (own, parent) = path.split_last().ok_or("no such button")?;
    let own = *own;
    let flag = |buttons: &mut Vec<PieButton>, value: bool| -> Result<(), String> {
        let button = button_at_mut(buttons, path).ok_or("no such button")?;
        match (&mut button.kind, item) {
            (PieButtonKind::Cluster(items), Some(k)) => items.get_mut(k).ok_or("no such item")?.enabled = value,
            _ => button.enabled = value,
        }
        Ok(())
    };
    let before = {
        let button = button_at(buttons, path).ok_or("no such button")?;
        match (&button.kind, item) {
            (PieButtonKind::Cluster(items), Some(k)) => items.get(k).ok_or("no such item")?.enabled,
            _ => button.enabled,
        }
    };
    flag(buttons, enabled)?;
    if enabled
        && !before
        && let Some((key, other)) = level_at(buttons, parent).and_then(|level| explicit_clash(level, own, item))
    {
        flag(buttons, before)?;
        return Err(format!("its key '{key}' is taken by {}", name_of(&other)));
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------
// Sub-pie style
// ---------------------------------------------------------------------------------------

/// What turning a sub-pie into a fan-out needs: the Back children facing the parent are
/// dropped (a fan closes by itself); anything else that does not fit makes it impossible.
/// Returns the indices (ascending) of the children to drop, or why it cannot be.
pub(crate) fn fan_out_plan(button: &PieButton) -> Result<Vec<usize>, String> {
    let children = button.children();
    let mut remove = Vec::new();
    for (i, child) in children.iter().enumerate() {
        if matches!(child.kind, PieButtonKind::SubPie { .. } | PieButtonKind::Cluster(_)) {
            return Err(format!("{} is a sub-pie or cluster; a fan-out holds plain buttons only", name_of(&child.label)));
        }
        if button.direction.blocks_fan_out(child.direction) {
            if child.is_back() {
                remove.push(i);
            } else {
                return Err(format!("{} faces the parent; move it first", name_of(&child.label)));
            }
        }
    }
    if remove.len() == children.len() {
        return Err("no button would be left".to_string());
    }
    Ok(remove)
}

/// Whether the sub-pie `button` may take `style` (`None`: follow the menu), with the reason
/// when it may not.
pub(crate) fn style_availability(button: &PieButton, settings: &PieMenuSettings, style: Option<SubPieStyle>) -> Result<(), String> {
    if style.unwrap_or(settings.sub_pies.style) == SubPieStyle::FanOut {
        fan_out_plan(button).map(|_| ())
    } else {
        Ok(())
    }
}

/// Sets the style of the sub-pie at `path`. Becoming a fan-out drops its Back children facing
/// the parent ([`fan_out_plan`]); their indices (ascending, in the old numbering) are returned
/// so side tables can follow. Refuses with the reason when it cannot.
pub(crate) fn set_sub_pie_style(
    buttons: &mut Vec<PieButton>,
    settings: &PieMenuSettings,
    path: &[usize],
    style: Option<SubPieStyle>,
) -> Result<Vec<usize>, String> {
    let button = button_at_mut(buttons, path).ok_or("no such button")?;
    if !matches!(button.kind, PieButtonKind::SubPie { .. }) {
        return Err("not a sub-pie".to_string());
    }
    let remove = if style.unwrap_or(settings.sub_pies.style) == SubPieStyle::FanOut {
        fan_out_plan(button)?
    } else {
        Vec::new()
    };
    if let PieButtonKind::SubPie { children, style: own } = &mut button.kind {
        for i in remove.iter().rev() {
            children.remove(*i);
        }
        *own = style;
    }
    Ok(remove)
}

/// Sets the menu-wide [`PieMenuSubPieSettings::style`] and keeps `buttons` legal. Going to a fan-out, every sub-pie that
/// follows the menu but cannot be a fan-out (children facing the parent, nested sub-pies) is
/// pinned to [`SubPieStyle::Replace`] so the tree stays legal; the notice says which. `None`
/// when nothing was pinned.
pub fn set_menu_sub_pie_style(
    settings: &mut PieMenuSettings,
    buttons: &mut [PieButton],
    style: SubPieStyle,
) -> Option<String> {
    settings.sub_pies.style = style;
    if style != SubPieStyle::FanOut {
        return None;
    }
    fn pin(menu: &PieMenu, level: &mut [PieButton], pinned: &mut Vec<String>) {
        for b in level {
            let direction = b.direction;
            let label = b.label.clone();
            if let PieButtonKind::SubPie { children, style } = &mut b.kind {
                if style.is_none()
                    && !children.is_empty()
                    && !crate::button::fan_out_problems(direction, children, &menu.mnemonics(children)).is_empty()
                {
                    *style = Some(SubPieStyle::Replace);
                    pinned.push(name_of(&label));
                }
                pin(menu, children, pinned);
            }
        }
    }
    let menu = problem_menu_for(settings);
    let mut pinned = Vec::new();
    pin(&menu, buttons, &mut pinned);
    (!pinned.is_empty()).then(|| {
        format!(
            "{} cannot open as a fan-out (children facing the parent or nested) and stay{} replace-style: {}",
            if pinned.len() == 1 { "1 sub-pie" } else { "Some sub-pies" },
            if pinned.len() == 1 { "s" } else { "" },
            pinned.join(", ")
        )
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::field_reassign_with_default)]
    use super::*;
    use crate::{ClusterItem, PieButton};

    fn labels(level: &[PieButton]) -> Vec<&str> {
        level.iter().map(|b| b.label.as_str()).collect()
    }

    fn fan_settings() -> PieMenuSettings {
        let mut s = PieMenuSettings::default();
        s.sub_pies.style = SubPieStyle::FanOut;
        s
    }

    /// A tree with every feature, legal under the default settings.
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
                PieButton::new(PieDirection::South).with_label("Deep").with_children(vec![
                    PieButton::new(PieDirection::West).with_label("Leaf"),
                    PieButton::new(PieDirection::East).with_label("Fan2").with_fan_out(vec![
                        PieButton::new(PieDirection::East).with_label("F1"),
                        PieButton::new(PieDirection::NorthEast).with_label("F2"),
                    ]),
                ]),
            ]),
            PieButton::cluster(PieDirection::West).with_label("Format").with_items(vec![
                ClusterItem::new(7, "Bold").with_mnemonic('b'),
                ClusterItem::new(8, "Italic"),
                ClusterItem::new(9, "Strike").with_enabled(false),
            ]),
        ]
    }

    #[test]
    fn the_test_tree_is_legal() {
        for auto in [false, true] {
            let mut s = PieMenuSettings::default();
            s.input.auto_mnemonics = auto;
            assert_eq!(tree_problems(&s, &tree()), vec![]);
        }
    }

    #[test]
    fn directions_free_ones_plus_own_and_fan_slots() {
        let s = PieMenuSettings::default();
        let t = tree();
        let root = allowed_directions(&t, &s, &[0]);
        assert_eq!(root, [PieDirection::North, PieDirection::NorthEast, PieDirection::SouthEast, PieDirection::SouthWest, PieDirection::NorthWest]);
        assert_eq!(free_directions(&t, &s, &[]), [PieDirection::NorthEast, PieDirection::SouthEast, PieDirection::SouthWest, PieDirection::NorthWest]);
        // Fan child "A" (East, opener East): slots N, NE, E, SE, S; NE and S are used.
        let fan = allowed_directions(&t, &s, &[1, 1]);
        assert_eq!(fan, [PieDirection::North, PieDirection::East, PieDirection::SouthEast]);
        assert_eq!(free_directions(&t, &s, &[1]), [PieDirection::North, PieDirection::SouthEast]);
        assert_eq!(direction_block_reason(&t, &s, &[1, 1], PieDirection::West).as_deref(), Some("faces the parent"));
        assert_eq!(direction_block_reason(&t, &s, &[1, 1], PieDirection::South).as_deref(), Some("taken by \"B\""));
        assert_eq!(direction_block_reason(&t, &s, &[1, 1], PieDirection::North), None);
        // A level under a replace sub-pie is free of the fan restriction.
        assert_eq!(level_kind(&t, &s, &[2]), LevelKind::Replace);
        assert_eq!(level_kind(&t, &s, &[1]), LevelKind::Fan(PieDirection::East));
        assert_eq!(level_kind(&t, &s, &[]), LevelKind::Root);
        // The menu's own fan style turns style-less sub-pies into fan levels.
        assert_eq!(level_kind(&t, &fan_settings(), &[2]), LevelKind::Fan(PieDirection::South));
    }

    #[test]
    fn moving_a_fan_out_turns_its_children_with_it() {
        let s = PieMenuSettings::default();
        let mut t = tree();
        set_direction(&mut t, &s, &[1], PieDirection::SouthEast).unwrap();
        let dirs: Vec<_> = t[1].children().iter().map(|c| c.direction).collect();
        assert_eq!(dirs, [PieDirection::East, PieDirection::SouthEast, PieDirection::SouthWest]);
        assert_eq!(tree_problems(&s, &t), vec![]);
        // Refused moves change nothing.
        let before = format!("{t:?}");
        assert!(set_direction(&mut t, &s, &[1], PieDirection::North).is_err());
        assert!(set_direction(&mut t, &s, &[1, 1], PieDirection::West).is_err());
        assert_eq!(before, format!("{t:?}"));
        // A replace sub-pie's children stay put.
        set_direction(&mut t, &s, &[2], PieDirection::SouthWest).unwrap();
        assert_eq!(t[2].children()[0].direction, PieDirection::North);
    }

    #[test]
    fn kinds_by_level() {
        let s = PieMenuSettings::default();
        let t = tree();
        assert_eq!(allowed_kinds(&t, &s, &[0]), [KindChoice::Action, KindChoice::SubPie, KindChoice::Cluster]);
        // The back button of a replace level: everything.
        assert_eq!(allowed_kinds(&t, &s, &[2, 0]), KindChoice::ALL);
        // Fan children: Action and Back only.
        assert_eq!(allowed_kinds(&t, &s, &[1, 1]), [KindChoice::Action, KindChoice::Back]);
        // A root Back already there stays selectable.
        let root_back = vec![PieButton::back(PieDirection::North), PieButton::new(PieDirection::East)];
        assert_eq!(allowed_kinds(&root_back, &s, &[0]), [KindChoice::Action, KindChoice::Back, KindChoice::SubPie, KindChoice::Cluster]);
        // A (wrong) cluster in a fan keeps its own kind in the list.
        let bad = vec![PieButton::new(PieDirection::East).with_fan_out(vec![PieButton::cluster(PieDirection::East)])];
        assert_eq!(allowed_kinds(&bad, &s, &[0, 0]), [KindChoice::Action, KindChoice::Back, KindChoice::Cluster]);
    }

    #[test]
    fn converting_to_a_sub_pie_is_legal_in_every_direction_and_style() {
        for menu_style in [SubPieStyle::Replace, SubPieStyle::FanOut] {
            let mut s = PieMenuSettings::default();
            s.sub_pies.style = menu_style;
            for dir in PieDirection::ALL {
                let mut b = PieButton::new(dir).with_label("X");
                let mut stash = Stash::default();
                assert!(!convert_kind_in(&mut b, KindChoice::SubPie, &s, &mut stash));
                assert_eq!(tree_problems(&s, std::slice::from_ref(&b)), vec![], "{dir:?} {menu_style:?}");
                assert_eq!(effective_style(&b, &s), Some(menu_style));
                let kids = b.children();
                assert_eq!(kids.iter().take(5).map(|c| c.direction).collect::<Vec<_>>(), dir.fan_out_slots());
                match menu_style {
                    SubPieStyle::Replace => {
                        assert_eq!(kids.len(), 6);
                        assert!(kids[5].is_back() && kids[5].direction == dir.opposite());
                    }
                    SubPieStyle::FanOut => assert_eq!(kids.len(), 5),
                }
            }
        }
    }

    #[test]
    fn converting_back_restores_what_was_dropped() {
        let s = PieMenuSettings::default();
        let mut b = tree()[2].clone();
        let before = format!("{:?}", b.kind);
        let mut stash = Stash::default();
        assert!(convert_kind_in(&mut b, KindChoice::Action, &s, &mut stash), "children dropped");
        assert!(matches!(b.kind, PieButtonKind::Action));
        assert!(!convert_kind_in(&mut b, KindChoice::Cluster, &s, &mut stash));
        assert_eq!(b.cluster_items().len(), 1);
        assert_eq!(b.mnemonic, Mnemonic::None);
        convert_kind_in(&mut b, KindChoice::SubPie, &s, &mut stash);
        assert_eq!(format!("{:?}", b.kind), before, "same children back");
        // The cluster's items come back too.
        let mut c = tree()[3].clone();
        let items = format!("{:?}", c.cluster_items());
        let mut stash = Stash::default();
        convert_kind_in(&mut c, KindChoice::Action, &s, &mut stash);
        convert_kind_in(&mut c, KindChoice::Cluster, &s, &mut stash);
        assert_eq!(format!("{:?}", c.cluster_items()), items);
        // A stash that became illegal (the menu is a fan-out now, the children have a back
        // button facing the parent) is replaced by a fresh sub-pie.
        let mut b = tree()[2].clone().with_sub_pie_style(None);
        let mut stash = Stash::default();
        convert_kind_in(&mut b, KindChoice::Action, &s, &mut stash);
        convert_kind_in(&mut b, KindChoice::SubPie, &fan_settings(), &mut stash);
        assert_eq!(b.children().len(), 5);
        assert_eq!(tree_problems(&fan_settings(), std::slice::from_ref(&b)), vec![]);
    }

    #[test]
    fn remove_rules() {
        let t = tree();
        assert_eq!(remove_block_reason(&t, &[0]), None);
        assert_eq!(remove_block_reason(&t, &[2, 1, 0]), None);
        let one = vec![PieButton::new(PieDirection::North)];
        assert!(remove_block_reason(&one, &[0]).unwrap().contains("menu"));
        let single_child = vec![PieButton::new(PieDirection::North).with_children(vec![PieButton::new(PieDirection::South)])];
        assert!(remove_block_reason(&single_child, &[0, 0]).unwrap().contains("sub-pie"));
        let mut t2 = single_child.clone();
        assert!(!remove_button(&mut t2, &[0, 0]));
        let mut t3 = tree();
        assert!(remove_button(&mut t3, &[2, 1, 1]));
        assert_eq!(labels(t3[2].children()[1].children()), ["Leaf"]);
        assert!(item_remove_block_reason(1).is_some() && item_remove_block_reason(2).is_none());
        let mut t4 = tree();
        assert!(remove_item(&mut t4, &[3], 0));
        assert!(remove_item(&mut t4, &[3], 0));
        assert!(!remove_item(&mut t4, &[3], 0), "the last item stays");
        assert_eq!(t4[3].cluster_items().len(), 1);
    }

    #[test]
    fn add_rules() {
        let s = PieMenuSettings::default();
        let t = tree();
        assert_eq!(addable_kinds(&t, &s, &[]), [AddKind::Action, AddKind::SubPie, AddKind::FanOut, AddKind::Cluster]);
        // The replace level has a Back already: no second one offered.
        assert_eq!(addable_kinds(&t, &s, &[2]), [AddKind::Action, AddKind::SubPie, AddKind::FanOut, AddKind::Cluster]);
        // The fan has a Back (NE) too; a fan holds nothing else.
        assert_eq!(addable_kinds(&t, &s, &[1]), [AddKind::Action]);
        let mut fan = vec![PieButton::new(PieDirection::East).with_fan_out(vec![PieButton::new(PieDirection::East)])];
        assert_eq!(addable_kinds(&fan, &s, &[0]), [AddKind::Action, AddKind::Back]);
        // Back is allowed in a fan, in a free slot (the opposite one is blocked).
        let at = add_button(&mut fan, &s, &[0], AddKind::Back).unwrap();
        assert_eq!(fan[0].children()[at].direction, PieDirection::North);
        assert!(fan[0].children()[at].is_back());
        assert_eq!(tree_problems(&s, &fan), vec![]);
        // Full levels offer nothing: 8 at the root, 5 in a fan.
        let mut full: Vec<PieButton> = PieDirection::ALL.iter().map(|d| PieButton::new(*d)).collect();
        assert_eq!(addable_kinds(&full, &s, &[]), []);
        assert_eq!(add_button(&mut full, &s, &[], AddKind::Action), None);
        let slots = PieDirection::East.fan_out_slots();
        let full_fan = vec![PieButton::new(PieDirection::East).with_fan_out(slots.iter().map(|d| PieButton::new(*d)).collect())];
        assert_eq!(addable_kinds(&full_fan, &s, &[0]), []);
        // A back button added to a replace level prefers the direction opposite its sub-pie.
        let mut level = vec![PieButton::new(PieDirection::East).with_children(vec![PieButton::new(PieDirection::North)])];
        add_button(&mut level, &s, &[0], AddKind::Back).unwrap();
        assert_eq!(level[0].children()[1].direction, PieDirection::West);
        // Added sub-pies and fans are legal; with a fan-out menu the replace one is pinned.
        for settings in [PieMenuSettings::default(), fan_settings()] {
            let mut root = vec![PieButton::new(PieDirection::North)];
            for kind in [AddKind::SubPie, AddKind::FanOut, AddKind::Cluster, AddKind::Action] {
                add_button(&mut root, &settings, &[], kind).unwrap();
            }
            assert_eq!(tree_problems(&settings, &root), vec![], "{:?}", settings.sub_pies.style);
            assert_eq!(effective_style(&root[1], &settings), Some(SubPieStyle::Replace));
            assert_eq!(effective_style(&root[2], &settings), Some(SubPieStyle::FanOut));
        }
    }

    #[test]
    fn mnemonic_rules() {
        let s = PieMenuSettings::default();
        let mut t = tree();
        // 'c' is Copy's.
        let err = set_mnemonic(&mut t, &s, &[1], None, MnemonicChange::Key('C')).unwrap_err();
        assert_eq!(err, "'C' is taken by \"Copy\"");
        assert!(set_mnemonic(&mut t, &s, &[1], None, MnemonicChange::Key('b')).unwrap_err().contains("Bold"), "an item's key counts");
        assert!(set_mnemonic(&mut t, &s, &[1], None, MnemonicChange::Key('5')).is_err());
        assert!(set_mnemonic(&mut t, &s, &[3], None, MnemonicChange::Auto).is_err(), "no mnemonic on a cluster");
        // Own key again is fine.
        set_mnemonic(&mut t, &s, &[0], None, MnemonicChange::Key('c')).unwrap();
        // The first free letter of the label, skipping taken ones: "Fan" -> f.
        set_mnemonic(&mut t, &s, &[1], None, MnemonicChange::KeyFromLabel).unwrap();
        assert_eq!(t[1].mnemonic, Mnemonic::Key('f'));
        // "Copy" has c (its own, freed while probing): c again; for a new button labelled "Co" next to it: o.
        t.push(PieButton::new(PieDirection::NorthWest).with_label("Co"));
        set_mnemonic(&mut t, &s, &[4], None, MnemonicChange::KeyFromLabel).unwrap();
        assert_eq!(t[4].mnemonic, Mnemonic::Key('o'));
        // Cluster items: Italic's initial is free.
        set_mnemonic(&mut t, &s, &[3], Some(1), MnemonicChange::KeyFromLabel).unwrap();
        assert_eq!(t[3].cluster_items()[1].mnemonic, Mnemonic::Key('i'));
        assert!(set_mnemonic(&mut t, &s, &[3], Some(1), MnemonicChange::Key('c')).is_err());
        assert_eq!(tree_problems(&s, &t), vec![]);
        // With auto mnemonics on, picking skips generated keys too.
        let mut auto = PieMenuSettings::default();
        auto.input.auto_mnemonics = true;
        let mut level = vec![PieButton::new(PieDirection::North).with_label("Alpha"), PieButton::new(PieDirection::East).with_label("Axe")];
        set_mnemonic(&mut level, &auto, &[1], None, MnemonicChange::KeyFromLabel).unwrap();
        assert_eq!(level[1].mnemonic, Mnemonic::Key('x'), "a is Alpha's generated key");
        assert_eq!(tree_problems(&auto, &level), vec![]);
        // Free letter fallback when the whole label is taken.
        assert_eq!(first_free_key("ab", &['a', 'b']), Some('c'));
        assert_eq!(first_free_key("", &[]), Some('a'));
        // Enabling a disabled owner whose key was taken meanwhile is refused.
        let mut clash = vec![
            PieButton::new(PieDirection::North).with_label("One").with_mnemonic('x').with_enabled(false),
            PieButton::new(PieDirection::East).with_label("Two").with_mnemonic('x'),
        ];
        assert!(set_enabled(&mut clash, &[0], None, true).unwrap_err().contains("Two"));
        assert!(!clash[0].enabled);
        set_enabled(&mut clash, &[1], None, false).unwrap();
        set_enabled(&mut clash, &[0], None, true).unwrap();
    }

    #[test]
    fn item_number_rules() {
        let mut t = tree();
        assert_eq!(next_item_number(t[3].cluster_items()), 10);
        assert_eq!(add_item(&mut t, &[3]), Some(3));
        assert_eq!(t[3].cluster_items()[3].number, 10);
        assert_eq!(next_item_number(&[]), 0);
        assert_eq!(add_item(&mut t, &[0]), None);
        let mut items = t[3].cluster_items().to_vec();
        assert!(set_item_number(&mut items, 0, 8).unwrap_err().contains("Italic"));
        set_item_number(&mut items, 0, 7).unwrap();
        set_item_number(&mut items, 0, 20).unwrap();
        assert_eq!(items[0].number, 20);
    }

    #[test]
    fn style_change_to_a_fan_out() {
        let s = PieMenuSettings::default();
        // New sub-pies carry a Back opposite the opener: it is dropped, the rest fits.
        let mut t = vec![new_sub_pie_for(PieDirection::East, false, SubPieStyle::Replace)];
        assert_eq!(style_availability(&t[0], &s, Some(SubPieStyle::FanOut)), Ok(()));
        let removed = set_sub_pie_style(&mut t, &s, &[0], Some(SubPieStyle::FanOut)).unwrap();
        assert_eq!(removed, [5]);
        assert_eq!(t[0].children().len(), 5);
        assert_eq!(tree_problems(&s, &t), vec![]);
        // Fan-out back to replace or the menu default: always fine, nothing dropped.
        assert_eq!(set_sub_pie_style(&mut t, &s, &[0], Some(SubPieStyle::Replace)), Ok(vec![]));
        assert_eq!(set_sub_pie_style(&mut t, &s, &[0], None), Ok(vec![]));
        // A nested sub-pie child, or a plain child facing the parent, blocks it with a reason.
        let nested = [PieButton::new(PieDirection::East).with_children(vec![
            PieButton::new(PieDirection::East).with_label("In").with_children(vec![PieButton::new(PieDirection::North)]),
        ])];
        assert!(style_availability(&nested[0], &s, Some(SubPieStyle::FanOut)).unwrap_err().contains("\"In\""));
        let facing = vec![PieButton::new(PieDirection::East).with_children(vec![
            PieButton::new(PieDirection::East),
            PieButton::new(PieDirection::West).with_label("Wrong"),
        ])];
        assert!(style_availability(&facing[0], &s, Some(SubPieStyle::FanOut)).unwrap_err().contains("faces the parent"));
        let mut copy = facing.clone();
        assert!(set_sub_pie_style(&mut copy, &s, &[0], Some(SubPieStyle::FanOut)).is_err());
        assert_eq!(format!("{copy:?}"), format!("{facing:?}"));
        // Only back buttons would be left: refused.
        let only_back = [PieButton::new(PieDirection::East).with_children(vec![PieButton::back(PieDirection::West)])];
        assert!(fan_out_plan(&only_back[0]).is_err());
        // "Menu default" counts as a fan-out when the menu is one.
        assert!(style_availability(&facing[0], &fan_settings(), None).is_err());
        assert!(style_availability(&facing[0], &s, None).is_ok());
    }

    #[test]
    fn menu_wide_fan_out_style_pins_what_cannot_fan_out() {
        let mut s = PieMenuSettings::default();
        let mut t = vec![
            new_sub_pie_for(PieDirection::East, false, SubPieStyle::Replace), // has a Back opposite: blocked
            PieButton::new(PieDirection::North).with_label("Plain").with_children(vec![PieButton::new(PieDirection::North)]),
            PieButton::new(PieDirection::South).with_label("Nested").with_children(vec![
                PieButton::new(PieDirection::South).with_label("Inner").with_children(vec![PieButton::new(PieDirection::South)]),
            ]),
        ];
        let notice = set_menu_sub_pie_style(&mut s, &mut t, SubPieStyle::FanOut).expect("something is pinned");
        assert!(notice.contains("Sub-pie") || notice.contains("\"Sub-pie\""), "{notice}");
        assert_eq!(effective_style(&t[0], &s), Some(SubPieStyle::Replace));
        assert_eq!(effective_style(&t[1], &s), Some(SubPieStyle::FanOut));
        assert_eq!(effective_style(&t[2], &s), Some(SubPieStyle::Replace), "its child is a sub-pie");
        assert_eq!(effective_style(&t[2].children()[0], &s), Some(SubPieStyle::FanOut));
        assert_eq!(tree_problems(&s, &t), vec![]);
        // Back to replace pins nothing and reports nothing.
        assert_eq!(set_menu_sub_pie_style(&mut s, &mut t, SubPieStyle::Replace), None);
        assert_eq!(tree_problems(&s, &t), vec![]);
        // Nothing to pin: no notice.
        let mut ok = vec![PieButton::new(PieDirection::North).with_children(vec![PieButton::new(PieDirection::North)])];
        assert_eq!(set_menu_sub_pie_style(&mut s, &mut ok, SubPieStyle::FanOut), None);
    }

    /// Every operation the editor offers, applied to every button of several trees under
    /// several settings, leaves the tree without problems.
    #[test]
    fn every_offered_operation_keeps_the_tree_legal() {
        let mut variants: Vec<(PieMenuSettings, Vec<PieButton>)> = Vec::new();
        for style in [SubPieStyle::Replace, SubPieStyle::FanOut] {
            for auto in [false, true] {
                let mut s = PieMenuSettings::default();
                s.input.auto_mnemonics = auto;
                let mut t = tree();
                set_menu_sub_pie_style(&mut s, &mut t, style);
                variants.push((s.clone(), t));
                let mut generated = vec![PieButton::new(PieDirection::North).with_label("Start")];
                for k in [AddKind::SubPie, AddKind::FanOut, AddKind::Cluster, AddKind::Action] {
                    add_button(&mut generated, &s, &[], k);
                }
                variants.push((s, generated));
            }
        }
        let mut count = 0usize;
        for (s, t) in &variants {
            assert_eq!(tree_problems(s, t), vec![], "start: {s:?}");
            let mut paths: Vec<Vec<usize>> = Vec::new();
            fn collect(level: &[PieButton], prefix: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
                for (i, b) in level.iter().enumerate() {
                    prefix.push(i);
                    out.push(prefix.clone());
                    collect(b.children(), prefix, out);
                    prefix.pop();
                }
            }
            collect(t, &mut Vec::new(), &mut paths);
            let mut levels: Vec<Vec<usize>> = vec![Vec::new()];
            levels.extend(paths.iter().filter(|p| !button_at(t, p).unwrap().children().is_empty()).cloned());
            let check = |what: &str, applied: Vec<PieButton>| {
                assert_eq!(tree_problems(s, &applied), vec![], "{what}");
            };
            for path in &paths {
                for d in allowed_directions(t, s, path) {
                    let mut c = t.clone();
                    set_direction(&mut c, s, path, d).unwrap();
                    check(&format!("direction {path:?} {d:?}"), c);
                    count += 1;
                }
                for k in allowed_kinds(t, s, path) {
                    let mut c = t.clone();
                    let mut stash = Stash::default();
                    convert_kind_in(button_at_mut(&mut c, path).unwrap(), k, s, &mut stash);
                    check(&format!("kind {path:?} {k:?}"), c.clone());
                    // ... and back and forth through the stash.
                    for k2 in allowed_kinds(&c, s, path) {
                        let mut c2 = c.clone();
                        convert_kind_in(button_at_mut(&mut c2, path).unwrap(), k2, s, &mut stash);
                        check(&format!("kind {path:?} {k:?} then {k2:?}"), c2);
                        count += 1;
                    }
                    count += 1;
                }
                if remove_block_reason(t, path).is_none() {
                    let mut c = t.clone();
                    assert!(remove_button(&mut c, path));
                    check(&format!("remove {path:?}"), c);
                    count += 1;
                }
                if matches!(button_at(t, path).unwrap().kind, PieButtonKind::SubPie { .. }) {
                    for style in [None, Some(SubPieStyle::Replace), Some(SubPieStyle::FanOut)] {
                        let mut c = t.clone();
                        let offered = style_availability(button_at(t, path).unwrap(), s, style).is_ok();
                        let result = set_sub_pie_style(&mut c, s, path, style);
                        assert_eq!(result.is_ok(), offered);
                        if result.is_ok() {
                            check(&format!("style {path:?} {style:?}"), c);
                        }
                        count += 1;
                    }
                }
                for change in [MnemonicChange::Auto, MnemonicChange::None, MnemonicChange::KeyFromLabel, MnemonicChange::Key('z'), MnemonicChange::Key('c')] {
                    let mut c = t.clone();
                    if set_mnemonic(&mut c, s, path, None, change).is_ok() {
                        check(&format!("mnemonic {path:?} {change:?}"), c);
                    }
                    let items = button_at(t, path).unwrap().cluster_items().len();
                    for k in 0..items {
                        let mut c = t.clone();
                        if set_mnemonic(&mut c, s, path, Some(k), change).is_ok() {
                            check(&format!("item mnemonic {path:?}/{k} {change:?}"), c);
                        }
                        count += 1;
                    }
                    count += 1;
                }
                for enabled in [false, true] {
                    let mut c = t.clone();
                    if set_enabled(&mut c, path, None, enabled).is_ok() {
                        check(&format!("enabled {path:?} {enabled}"), c);
                    }
                    let items = button_at(t, path).unwrap().cluster_items().len();
                    for k in 0..items {
                        let mut c = t.clone();
                        if set_enabled(&mut c, path, Some(k), enabled).is_ok() {
                            check(&format!("item enabled {path:?}/{k} {enabled}"), c);
                        }
                    }
                }
                if button_at(t, path).unwrap().is_cluster() {
                    let mut c = t.clone();
                    add_item(&mut c, path).unwrap();
                    check(&format!("add item {path:?}"), c);
                    let n = button_at(t, path).unwrap().cluster_items().len();
                    for k in 0..n {
                        let mut c = t.clone();
                        if remove_item(&mut c, path, k) {
                            check(&format!("remove item {path:?}/{k}"), c);
                        }
                        // Number edits offered: any number not in use.
                        let mut c = t.clone();
                        if let PieButtonKind::Cluster(items) = &mut button_at_mut(&mut c, path).unwrap().kind {
                            let _ = set_item_number(items, k, 99);
                            let other = items[(k + 1) % n].number;
                            assert!(set_item_number(items, k, other).is_err() || n == 1);
                        }
                        check(&format!("number {path:?}/{k}"), c);
                    }
                }
            }
            for level in &levels {
                for k in addable_kinds(t, s, level) {
                    let mut c = t.clone();
                    add_button(&mut c, s, level, k).unwrap();
                    check(&format!("add {level:?} {k:?}"), c);
                    count += 1;
                }
            }
        }
        assert!(count > 500, "the property test exercised {count} operations");
    }
}
