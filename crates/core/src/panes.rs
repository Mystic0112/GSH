//! Modelo puro de panes, splits e abas dentro de um workspace.
//!
//! Espelha `pane_node.dart`: não conhece renderer, sessão de terminal, arquivo
//! nem persistência. A camada do app converte seus ids concretos em `TabId`.

use serde_json::{json, Value};
use thiserror::Error;

pub type TabId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    Vertical,
    Horizontal,
}

impl SplitDirection {
    fn as_str(self) -> &'static str {
        match self {
            Self::Vertical => "vertical",
            Self::Horizontal => "horizontal",
        }
    }

    fn dart_name(self) -> &'static str {
        match self {
            Self::Vertical => "SplitDir.vertical",
            Self::Horizontal => "SplitDir.horizontal",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneLeaf {
    pub id: String,
    pub tabs: Vec<TabId>,
    pub active: TabId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PaneSplit {
    pub id: String,
    pub direction: SplitDirection,
    pub fraction: f64,
    pub a: Box<PaneNode>,
    pub b: Box<PaneNode>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PaneNode {
    Leaf(PaneLeaf),
    Split(PaneSplit),
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PaneModelError {
    #[error("pane node must be an object")]
    NotObject,
    #[error("pane node is missing `{0}`")]
    Missing(&'static str),
    #[error("pane node has an invalid `{0}`")]
    Invalid(&'static str),
}

impl PaneNode {
    pub fn leaf(id: impl Into<String>, tabs: Vec<TabId>, active: impl Into<String>) -> Self {
        Self::Leaf(PaneLeaf {
            id: id.into(),
            tabs,
            active: active.into(),
        })
    }

    pub fn leaves(&self) -> Vec<&PaneLeaf> {
        let mut out = Vec::new();
        self.collect_leaves(&mut out);
        out
    }

    fn collect_leaves<'a>(&'a self, out: &mut Vec<&'a PaneLeaf>) {
        match self {
            Self::Leaf(leaf) => out.push(leaf),
            Self::Split(split) => {
                split.a.collect_leaves(out);
                split.b.collect_leaves(out);
            }
        }
    }

    pub fn find_leaf(&self, id: &str) -> Option<&PaneLeaf> {
        self.leaves().into_iter().find(|leaf| leaf.id == id)
    }

    pub fn find_split(&self, id: &str) -> Option<&PaneSplit> {
        match self {
            Self::Leaf(_) => None,
            Self::Split(split) if split.id == id => Some(split),
            Self::Split(split) => split.a.find_split(id).or_else(|| split.b.find_split(id)),
        }
    }

    pub fn set_fraction(&self, id: &str, fraction: f64) -> Self {
        match self {
            Self::Leaf(_) => self.clone(),
            Self::Split(split) if split.id == id => {
                let mut split = split.clone();
                split.fraction = fraction;
                Self::Split(split)
            }
            Self::Split(split) => Self::Split(PaneSplit {
                id: split.id.clone(),
                direction: split.direction,
                fraction: split.fraction,
                a: Box::new(split.a.set_fraction(id, fraction)),
                b: Box::new(split.b.set_fraction(id, fraction)),
            }),
        }
    }

    pub fn update_leaf(&self, id: &str, update: impl Fn(&PaneLeaf) -> PaneLeaf + Copy) -> Self {
        match self {
            Self::Leaf(leaf) if leaf.id == id => Self::Leaf(update(leaf)),
            Self::Leaf(_) => self.clone(),
            Self::Split(split) => Self::Split(PaneSplit {
                id: split.id.clone(),
                direction: split.direction,
                fraction: split.fraction,
                a: Box::new(split.a.update_leaf(id, update)),
                b: Box::new(split.b.update_leaf(id, update)),
            }),
        }
    }

    pub fn split_leaf(
        &self,
        leaf_id: &str,
        direction: SplitDirection,
        new_leaf: PaneLeaf,
        split_id: Option<&str>,
        before: bool,
    ) -> Self {
        match self {
            Self::Leaf(leaf) if leaf.id == leaf_id => {
                let id = split_id
                    .map(str::to_owned)
                    .unwrap_or_else(|| format!("sp_{leaf_id}_{}", direction.dart_name()));
                let (a, b) = if before {
                    (new_leaf, leaf.clone())
                } else {
                    (leaf.clone(), new_leaf)
                };
                Self::Split(PaneSplit {
                    id,
                    direction,
                    fraction: 0.5,
                    a: Box::new(Self::Leaf(a)),
                    b: Box::new(Self::Leaf(b)),
                })
            }
            Self::Leaf(_) => self.clone(),
            Self::Split(split) => Self::Split(PaneSplit {
                id: split.id.clone(),
                direction: split.direction,
                fraction: split.fraction,
                a: Box::new(split.a.split_leaf(
                    leaf_id,
                    direction,
                    new_leaf.clone(),
                    split_id,
                    before,
                )),
                b: Box::new(
                    split
                        .b
                        .split_leaf(leaf_id, direction, new_leaf, split_id, before),
                ),
            }),
        }
    }

    pub fn remove_leaf(&self, id: &str) -> Self {
        match self {
            Self::Leaf(_) => self.clone(),
            Self::Split(split) => {
                if matches!(split.a.as_ref(), Self::Leaf(leaf) if leaf.id == id) {
                    return split.b.as_ref().clone();
                }
                if matches!(split.b.as_ref(), Self::Leaf(leaf) if leaf.id == id) {
                    return split.a.as_ref().clone();
                }
                Self::Split(PaneSplit {
                    id: split.id.clone(),
                    direction: split.direction,
                    fraction: split.fraction,
                    a: Box::new(split.a.remove_leaf(id)),
                    b: Box::new(split.b.remove_leaf(id)),
                })
            }
        }
    }

    pub fn neighbor_leaf(&self, from_leaf_id: &str, movement: PaneMove) -> Option<String> {
        let mut path = Vec::new();
        if !self.path_to(from_leaf_id, &mut path) {
            return None;
        }
        let direction = match movement {
            PaneMove::Left | PaneMove::Right => SplitDirection::Vertical,
            PaneMove::Up | PaneMove::Down => SplitDirection::Horizontal,
        };
        let toward_b = matches!(movement, PaneMove::Right | PaneMove::Down);
        for (split, took_a) in path.into_iter().rev() {
            if split.direction != direction {
                continue;
            }
            if toward_b && took_a {
                return Some(split.b.nearest_leaf(direction, true));
            }
            if !toward_b && !took_a {
                return Some(split.a.nearest_leaf(direction, false));
            }
        }
        None
    }

    fn path_to<'a>(&'a self, id: &str, path: &mut Vec<(&'a PaneSplit, bool)>) -> bool {
        match self {
            Self::Leaf(leaf) => leaf.id == id,
            Self::Split(split) => {
                path.push((split, true));
                if split.a.path_to(id, path) {
                    return true;
                }
                path.pop();
                path.push((split, false));
                if split.b.path_to(id, path) {
                    return true;
                }
                path.pop();
                false
            }
        }
    }

    fn nearest_leaf(&self, direction: SplitDirection, prefer_a: bool) -> String {
        match self {
            Self::Leaf(leaf) => leaf.id.clone(),
            Self::Split(split) => {
                let next = if split.direction == direction {
                    if prefer_a {
                        &split.a
                    } else {
                        &split.b
                    }
                } else {
                    &split.a
                };
                next.nearest_leaf(direction, prefer_a)
            }
        }
    }

    pub fn to_json(&self) -> Value {
        match self {
            Self::Leaf(leaf) => json!({
                "k": "leaf", "id": leaf.id, "tabs": leaf.tabs, "active": leaf.active
            }),
            Self::Split(split) => json!({
                "k": "split", "id": split.id, "dir": split.direction.as_str(),
                "frac": split.fraction, "a": split.a.to_json(), "b": split.b.to_json()
            }),
        }
    }

    pub fn from_json(value: &Value) -> Result<Self, PaneModelError> {
        if !value.is_object() {
            return Err(PaneModelError::NotObject);
        }
        let id = string_field(value, "id")?;
        if value.get("k").and_then(Value::as_str) == Some("split") {
            let direction = match string_field(value, "dir")?.as_str() {
                "vertical" => SplitDirection::Vertical,
                "horizontal" => SplitDirection::Horizontal,
                _ => return Err(PaneModelError::Invalid("dir")),
            };
            let fraction = value
                .get("frac")
                .ok_or(PaneModelError::Missing("frac"))?
                .as_f64()
                .ok_or(PaneModelError::Invalid("frac"))?;
            return Ok(Self::Split(PaneSplit {
                id,
                direction,
                fraction,
                a: Box::new(Self::from_json(
                    value.get("a").ok_or(PaneModelError::Missing("a"))?,
                )?),
                b: Box::new(Self::from_json(
                    value.get("b").ok_or(PaneModelError::Missing("b"))?,
                )?),
            }));
        }
        let tabs = value
            .get("tabs")
            .ok_or(PaneModelError::Missing("tabs"))?
            .as_array()
            .ok_or(PaneModelError::Invalid("tabs"))?
            .iter()
            .map(|tab| {
                tab.as_str()
                    .map(str::to_owned)
                    .ok_or(PaneModelError::Invalid("tabs"))
            })
            .collect::<Result<_, _>>()?;
        Ok(Self::Leaf(PaneLeaf {
            id,
            tabs,
            active: string_field(value, "active")?,
        }))
    }
}

fn string_field(value: &Value, field: &'static str) -> Result<String, PaneModelError> {
    value
        .get(field)
        .ok_or(PaneModelError::Missing(field))?
        .as_str()
        .map(str::to_owned)
        .ok_or(PaneModelError::Invalid(field))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneMove {
    Left,
    Right,
    Up,
    Down,
}

pub fn reorder_tabs(tabs: &[TabId], tab_id: &str, index: isize) -> Vec<TabId> {
    let mut out = tabs.to_vec();
    let Some(from) = out.iter().position(|tab| tab == tab_id) else {
        return out;
    };
    let tab = out.remove(from);
    let index = index.max(0) as usize;
    let to = if index > from { index - 1 } else { index }.min(out.len());
    out.insert(to, tab);
    out
}
