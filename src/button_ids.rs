//! [`ButtonIds`]: stable string ids for the buttons and cluster items of a tree, kept next to
//! it by path (as menus loaded from TOML carry them), so a selection can be told apart by id
//! rather than by a path that changes when buttons are added or moved.
use crate::PieButton;
use crate::dev_tool::TreeEdit;

/// What an id belongs to: the button at `path`, or the item `item` (its number) of the
/// cluster at `path`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IdKey {
    pub path: Vec<usize>,
    pub item: Option<usize>,
}

/// Ids of the buttons and cluster items of one tree, by path.
///
/// After a selection, [`ButtonIds::selected`]`(&buttons, menu.selected_path())` gives the id
/// of what was picked, a button or a cluster item.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct ButtonIds {
    entries: Vec<(IdKey, String)>,
}

/// Equal when they hold the same ids in any order.
impl PartialEq for ButtonIds {
    fn eq(&self, other: &Self) -> bool {
        self.entries.len() == other.entries.len() && self.entries.iter().all(|(k, v)| other.get(k) == Some(v.as_str()))
    }
}

impl Eq for ButtonIds {}

impl ButtonIds {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&IdKey, &str)> {
        self.entries.iter().map(|(k, v)| (k, v.as_str()))
    }

    fn get(&self, key: &IdKey) -> Option<&str> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }

    /// Sets (an empty `id` removes) the id of what `key` names.
    pub fn set(&mut self, key: IdKey, id: impl Into<String>) {
        let id = id.into();
        self.entries.retain(|(k, _)| *k != key);
        if !id.is_empty() {
            self.entries.push((key, id));
        }
    }

    /// The id of the button at `path`.
    pub fn button(&self, path: &[usize]) -> Option<&str> {
        self.get(&IdKey { path: path.to_vec(), item: None })
    }

    /// The id of the item numbered `number` of the cluster at `path`.
    pub fn item(&self, path: &[usize], number: usize) -> Option<&str> {
        self.get(&IdKey { path: path.to_vec(), item: Some(number) })
    }

    /// The id of what a selection picked, from [`crate::PieMenu::selected_path`]: the button
    /// at that path, or, when the path ends inside a cluster (`[.., cluster, item number]`),
    /// that item.
    pub fn selected(&self, buttons: &[PieButton], path: &[usize]) -> Option<&str> {
        if let Some((number, cluster)) = path.split_last()
            && crate::button_at(buttons, cluster).is_some_and(PieButton::is_cluster)
        {
            return self.item(cluster, *number);
        }
        self.button(path)
    }

    /// Where `id` is (the first entry with it).
    pub fn find(&self, id: &str) -> Option<&IdKey> {
        self.entries.iter().find(|(_, v)| v == id).map(|(k, _)| k)
    }

    /// Follows a change to the tree's shape (from [`crate::dev_tool::InspectorOutput::edits`]),
    /// as [`crate::dev_tool::Bindings::follow`] does for bindings.
    pub fn follow(&mut self, edit: &TreeEdit) {
        self.entries.retain_mut(|(key, _)| follow_key(&mut key.path, key.item.is_some(), edit));
    }

    /// Renumbers the item `from` of the cluster at `path` to `to`.
    pub fn renumber_item(&mut self, path: &[usize], from: usize, to: usize) {
        for (key, _) in &mut self.entries {
            if key.path == path && key.item == Some(from) {
                key.item = Some(to);
            }
        }
    }

    /// Forgets the ids of buttons and items that are not in `buttons`.
    pub fn prune(&mut self, buttons: &[PieButton]) {
        self.entries.retain(|(key, _)| match (crate::button_at(buttons, &key.path), key.item) {
            (Some(_), None) => true,
            (Some(b), Some(n)) => b.cluster_items().iter().any(|i| i.number == n),
            (None, _) => false,
        });
    }
}

/// Moves a path-keyed entry (`item`: it belongs to a cluster item at `key`) along a tree edit.
/// Returns `false` if the entry goes.
pub(crate) fn follow_key(key: &mut [usize], item: bool, edit: &TreeEdit) -> bool {
    match edit {
        TreeEdit::Removed(path) => {
            let Some((last, parent)) = path.split_last() else { return true };
            if key.starts_with(path) {
                return false;
            }
            let depth = parent.len();
            if key.len() > depth && key.starts_with(parent) && key[depth] > *last {
                key[depth] -= 1;
            }
            true
        }
        // The children below went; a cluster's items went too (or it became a cluster).
        TreeEdit::Dropped(path) => !((key.len() > path.len() || item) && key.starts_with(path)),
        TreeEdit::Added(path) => !key.starts_with(path),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ClusterItem, PieDirection};

    fn tree() -> Vec<PieButton> {
        vec![
            PieButton::new(PieDirection::North).with_label("Copy"),
            PieButton::cluster(PieDirection::South).with_items(vec![ClusterItem::new(1, "Bold"), ClusterItem::new(2, "Italic")]),
        ]
    }

    #[test]
    fn lookups_and_selection() {
        let mut ids = ButtonIds::new();
        ids.set(IdKey { path: vec![0], item: None }, "edit.copy");
        ids.set(IdKey { path: vec![1], item: Some(2) }, "fmt.italic");
        let t = tree();
        assert_eq!(ids.selected(&t, &[0]), Some("edit.copy"));
        assert_eq!(ids.selected(&t, &[1, 2]), Some("fmt.italic"), "a cluster item's path ends with its number");
        assert_eq!(ids.selected(&t, &[1, 1]), None);
        assert_eq!(ids.find("fmt.italic"), Some(&IdKey { path: vec![1], item: Some(2) }));
        ids.set(IdKey { path: vec![0], item: None }, "");
        assert_eq!(ids.len(), 1, "an empty id removes");
    }

    #[test]
    fn ids_follow_the_tree() {
        let mut ids = ButtonIds::new();
        for (p, item, id) in [(vec![0], None, "a"), (vec![1], None, "b"), (vec![1, 0], None, "b0"), (vec![2], Some(1), "c1")] {
            ids.set(IdKey { path: p, item }, id);
        }
        ids.follow(&TreeEdit::Removed(vec![0]));
        assert_eq!(ids.button(&[0]), Some("b"));
        assert_eq!(ids.button(&[0, 0]), Some("b0"));
        assert_eq!(ids.item(&[1], 1), Some("c1"));
        ids.follow(&TreeEdit::Dropped(vec![1]));
        assert_eq!(ids.item(&[1], 1), None, "the cluster's items went");
        ids.follow(&TreeEdit::Dropped(vec![0]));
        assert_eq!(ids.button(&[0]), Some("b"), "the converted button keeps its id");
        assert_eq!(ids.button(&[0, 0]), None);
        ids.renumber_item(&[0], 1, 2);
        ids.prune(&tree());
        assert_eq!(ids.len(), 1);
    }
}
