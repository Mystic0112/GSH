//! Árvore binária de divisões de um workspace (porta de `pane_node.dart`).
//!
//! Uma folha ([`Leaf`]) é um contêiner de abas; uma divisão ([`Split`]) reparte o
//! espaço entre dois nós, lado a lado ([`SplitDir::Vertical`]) ou empilhados
//! ([`SplitDir::Horizontal`]). Fechar uma folha faz o irmão ocupar o lugar da
//! divisão. O JSON é o mesmo do `paneNodeToJson` do Flutter.

use serde::{Deserialize, Serialize};

use crate::ids::{PaneId, TabId};

/// Limites do divisor (o Flutter trava o `frac` em 0.16..0.84 ao arrastar).
pub const MIN_FRAC: f64 = 0.16;
pub const MAX_FRAC: f64 = 0.84;
/// Proporção de uma divisão recém-criada.
pub const DEFAULT_FRAC: f64 = 0.5;

/// `Vertical` = filhos lado a lado; `Horizontal` = empilhados (nome do Flutter).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SplitDir {
    Vertical,
    Horizontal,
}

/// Direção do foco entre folhas (atalhos de setas).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneMove {
    Left,
    Right,
    Up,
    Down,
}

/// Folha: abas em ordem e a ativa. Invariante mantida pelas operações: `tabs`
/// nunca fica vazia e `active` está em `tabs`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Leaf {
    pub id: PaneId,
    pub tabs: Vec<TabId>,
    pub active: TabId,
}

/// Divisão: `a` e `b` na proporção `frac` (0..1) na direção `dir`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Split {
    pub id: PaneId,
    pub dir: SplitDir,
    pub frac: f64,
    pub a: Box<PaneNode>,
    pub b: Box<PaneNode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "k", rename_all = "lowercase")]
pub enum PaneNode {
    Leaf(Leaf),
    Split(Split),
}

impl Default for PaneNode {
    /// Folha vazia e sem ids: só serve de valor temporário em `mem::take`.
    fn default() -> Self {
        Self::Leaf(Leaf {
            id: PaneId::default(),
            tabs: Vec::new(),
            active: TabId::default(),
        })
    }
}

impl PaneNode {
    pub fn id(&self) -> &PaneId {
        match self {
            Self::Leaf(l) => &l.id,
            Self::Split(s) => &s.id,
        }
    }

    /// Folhas da esquerda/cima para a direita/baixo.
    pub fn leaves(&self) -> Vec<&Leaf> {
        let mut out = Vec::new();
        self.collect_leaves(&mut out);
        out
    }

    fn collect_leaves<'a>(&'a self, out: &mut Vec<&'a Leaf>) {
        match self {
            Self::Leaf(l) => out.push(l),
            Self::Split(s) => {
                s.a.collect_leaves(out);
                s.b.collect_leaves(out);
            }
        }
    }

    pub fn find_leaf(&self, id: &PaneId) -> Option<&Leaf> {
        match self {
            Self::Leaf(l) => (l.id == *id).then_some(l),
            Self::Split(s) => s.a.find_leaf(id).or_else(|| s.b.find_leaf(id)),
        }
    }

    pub fn find_leaf_mut(&mut self, id: &PaneId) -> Option<&mut Leaf> {
        match self {
            Self::Leaf(l) => (l.id == *id).then_some(l),
            Self::Split(s) => match s.a.find_leaf_mut(id) {
                Some(l) => Some(l),
                None => s.b.find_leaf_mut(id),
            },
        }
    }

    pub fn find_split_mut(&mut self, id: &PaneId) -> Option<&mut Split> {
        match self {
            Self::Leaf(_) => None,
            Self::Split(s) => {
                if s.id == *id {
                    return Some(s);
                }
                match s.a.find_split_mut(id) {
                    Some(found) => Some(found),
                    None => s.b.find_split_mut(id),
                }
            }
        }
    }

    /// Folha que contém a aba.
    pub fn leaf_of_tab(&self, tab: &TabId) -> Option<&Leaf> {
        match self {
            Self::Leaf(l) => l.tabs.contains(tab).then_some(l),
            Self::Split(s) => s.a.leaf_of_tab(tab).or_else(|| s.b.leaf_of_tab(tab)),
        }
    }

    /// Divide a folha `id` em `dir`, pondo `new_leaf` depois (direita/baixo) ou,
    /// com `before`, antes (esquerda/cima). Folha inexistente: árvore intacta.
    pub fn split_leaf(
        self,
        id: &PaneId,
        dir: SplitDir,
        new_leaf: Leaf,
        split_id: PaneId,
        before: bool,
    ) -> Self {
        let mut slot = Some(new_leaf);
        self.split_inner(id, dir, &mut slot, &split_id, before)
    }

    fn split_inner(
        self,
        id: &PaneId,
        dir: SplitDir,
        new_leaf: &mut Option<Leaf>,
        split_id: &PaneId,
        before: bool,
    ) -> Self {
        match self {
            Self::Leaf(l) if l.id == *id => match new_leaf.take() {
                Some(fresh) => {
                    let (a, b) = if before {
                        (Self::Leaf(fresh), Self::Leaf(l))
                    } else {
                        (Self::Leaf(l), Self::Leaf(fresh))
                    };
                    Self::Split(Split {
                        id: split_id.clone(),
                        dir,
                        frac: DEFAULT_FRAC,
                        a: Box::new(a),
                        b: Box::new(b),
                    })
                }
                None => Self::Leaf(l),
            },
            Self::Leaf(l) => Self::Leaf(l),
            Self::Split(s) => Self::Split(Split {
                id: s.id,
                dir: s.dir,
                frac: s.frac,
                a: Box::new(s.a.split_inner(id, dir, new_leaf, split_id, before)),
                b: Box::new(s.b.split_inner(id, dir, new_leaf, split_id, before)),
            }),
        }
    }

    /// Remove a folha `id`: o irmão toma o lugar da divisão. Uma folha raiz (ou
    /// um id inexistente) deixa a árvore como está.
    pub fn remove_leaf(self, id: &PaneId) -> Self {
        match self {
            Self::Leaf(_) => self,
            Self::Split(s) => {
                let is_target = |n: &Self| matches!(n, Self::Leaf(l) if l.id == *id);
                if is_target(&s.a) {
                    return *s.b;
                }
                if is_target(&s.b) {
                    return *s.a;
                }
                Self::Split(Split {
                    id: s.id,
                    dir: s.dir,
                    frac: s.frac,
                    a: Box::new(s.a.remove_leaf(id)),
                    b: Box::new(s.b.remove_leaf(id)),
                })
            }
        }
    }

    /// Folha vizinha de `from` na direção `mv`, ou `None` na borda. Sem
    /// geometria: sobe até o ancestral do eixo certo em que `from` está do lado
    /// de partida, cruza para o irmão e desce colando na borda compartilhada
    /// (no eixo cruzado pega sempre `a`, determinístico).
    pub fn neighbor_leaf(&self, from: &PaneId, mv: PaneMove) -> Option<&PaneId> {
        let mut path = Vec::new();
        if !self.path_to(from, &mut path) {
            return None;
        }
        let axis = match mv {
            PaneMove::Left | PaneMove::Right => SplitDir::Vertical,
            PaneMove::Up | PaneMove::Down => SplitDir::Horizontal,
        };
        let toward_b = matches!(mv, PaneMove::Right | PaneMove::Down);
        for (split, took_a) in path.into_iter().rev() {
            if split.dir != axis {
                continue;
            }
            if toward_b && took_a {
                return split.b.descend_nearest(axis, true);
            }
            if !toward_b && !took_a {
                return split.a.descend_nearest(axis, false);
            }
        }
        None
    }

    /// Divisões atravessadas da raiz até a folha `id` (e o lado tomado).
    fn path_to<'a>(&'a self, id: &PaneId, out: &mut Vec<(&'a Split, bool)>) -> bool {
        match self {
            Self::Leaf(l) => l.id == *id,
            Self::Split(s) => {
                out.push((s, true));
                if s.a.path_to(id, out) {
                    return true;
                }
                out.pop();
                out.push((s, false));
                if s.b.path_to(id, out) {
                    return true;
                }
                out.pop();
                false
            }
        }
    }

    fn descend_nearest(&self, axis: SplitDir, prefer_a: bool) -> Option<&PaneId> {
        let mut node = self;
        while let Self::Split(s) = node {
            // `prefer_a` só vale no eixo do movimento; no cruzado é sempre `a`.
            node = if s.dir == axis && !prefer_a {
                &s.b
            } else {
                &s.a
            };
        }
        match node {
            Self::Leaf(l) => Some(&l.id),
            Self::Split(_) => None,
        }
    }
}

/// Reposiciona `tab` no slot `index` (0..=len, semântica "soltar no slot i": o
/// índice vale na lista **antes** da remoção). Aba ausente: lista intacta.
pub fn reorder_tabs(tabs: &[TabId], tab: &TabId, index: usize) -> Vec<TabId> {
    let mut out = tabs.to_vec();
    let Some(from) = out.iter().position(|t| t == tab) else {
        return out;
    };
    let moved = out.remove(from);
    let mut to = index;
    if to > from {
        to -= 1; // o item saiu antes do alvo: desloca uma posição
    }
    out.insert(to.min(out.len()), moved);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf(id: &str, tabs: &[&str]) -> PaneNode {
        PaneNode::Leaf(Leaf {
            id: PaneId::new(id),
            tabs: tabs.iter().map(|t| TabId::new(*t)).collect(),
            active: TabId::new(tabs.first().copied().unwrap_or("")),
        })
    }

    fn split(id: &str, dir: SplitDir, a: PaneNode, b: PaneNode) -> PaneNode {
        PaneNode::Split(Split {
            id: PaneId::new(id),
            dir,
            frac: DEFAULT_FRAC,
            a: Box::new(a),
            b: Box::new(b),
        })
    }

    fn ids(node: &PaneNode) -> Vec<String> {
        node.leaves().iter().map(|l| l.id.to_string()).collect()
    }

    #[test]
    fn split_depois_e_antes() {
        let l = |id| Leaf {
            id: PaneId::new(id),
            tabs: vec![TabId::new("x")],
            active: TabId::new("x"),
        };
        let after = leaf("p1", &["a"]).split_leaf(
            &PaneId::new("p1"),
            SplitDir::Vertical,
            l("p2"),
            PaneId::new("sp1"),
            false,
        );
        assert_eq!(ids(&after), ["p1", "p2"]);
        let before = leaf("p1", &["a"]).split_leaf(
            &PaneId::new("p1"),
            SplitDir::Horizontal,
            l("p2"),
            PaneId::new("sp1"),
            true,
        );
        assert_eq!(ids(&before), ["p2", "p1"]);
        assert!(
            matches!(&before, PaneNode::Split(s) if s.dir == SplitDir::Horizontal && s.frac == DEFAULT_FRAC)
        );
    }

    #[test]
    fn split_de_folha_inexistente_nao_muda_nada() {
        let tree = leaf("p1", &["a"]);
        let out = tree.clone().split_leaf(
            &PaneId::new("nope"),
            SplitDir::Vertical,
            Leaf {
                id: PaneId::new("p2"),
                tabs: vec![TabId::new("x")],
                active: TabId::new("x"),
            },
            PaneId::new("sp1"),
            false,
        );
        assert_eq!(out, tree);
    }

    #[test]
    fn remove_folha_faz_irmao_expandir_inclusive_subarvore() {
        let inner = split(
            "sp2",
            SplitDir::Horizontal,
            leaf("p2", &["b"]),
            leaf("p3", &["c"]),
        );
        let tree = split("sp1", SplitDir::Vertical, leaf("p1", &["a"]), inner.clone());
        assert_eq!(tree.remove_leaf(&PaneId::new("p1")), inner);
    }

    #[test]
    fn remove_folha_funda_colapsa_so_a_divisao_pai() {
        let inner = split(
            "sp2",
            SplitDir::Horizontal,
            leaf("p2", &["b"]),
            leaf("p3", &["c"]),
        );
        let tree = split("sp1", SplitDir::Vertical, leaf("p1", &["a"]), inner);
        let out = tree.remove_leaf(&PaneId::new("p3"));
        assert_eq!(ids(&out), ["p1", "p2"]);
        assert!(matches!(&out, PaneNode::Split(s) if s.id.as_str() == "sp1"));
    }

    #[test]
    fn remove_folha_raiz_ou_inexistente_e_noop() {
        let solo = leaf("p1", &["a"]);
        assert_eq!(solo.clone().remove_leaf(&PaneId::new("p1")), solo);
        let tree = split(
            "sp1",
            SplitDir::Vertical,
            leaf("p1", &["a"]),
            leaf("p2", &["b"]),
        );
        assert_eq!(tree.clone().remove_leaf(&PaneId::new("zz")), tree);
    }

    #[test]
    fn vizinhos_por_direcao() {
        // [ p1 | (p2 / p3) ]
        let tree = split(
            "sp1",
            SplitDir::Vertical,
            leaf("p1", &["a"]),
            split(
                "sp2",
                SplitDir::Horizontal,
                leaf("p2", &["b"]),
                leaf("p3", &["c"]),
            ),
        );
        let n = |from: &str, mv| {
            tree.neighbor_leaf(&PaneId::new(from), mv)
                .map(PaneId::to_string)
        };
        assert_eq!(n("p1", PaneMove::Right).as_deref(), Some("p2"));
        assert_eq!(n("p3", PaneMove::Left).as_deref(), Some("p1"));
        assert_eq!(n("p2", PaneMove::Down).as_deref(), Some("p3"));
        assert_eq!(n("p3", PaneMove::Up).as_deref(), Some("p2"));
        assert_eq!(n("p1", PaneMove::Left), None);
        assert_eq!(n("p1", PaneMove::Down), None);
        assert_eq!(n("p3", PaneMove::Right), None);
        assert_eq!(n("zz", PaneMove::Right), None);
    }

    #[test]
    fn vizinho_desce_colando_na_borda() {
        // [ (p1 | p2) / p3 ]: descendo de p3 pra cima cai em p1 (eixo cruzado = a);
        // [ (p1 | p2) | p3 ]: indo p3 pra esquerda cai em p2 (borda compartilhada).
        let stacked = split(
            "sp1",
            SplitDir::Horizontal,
            split(
                "sp2",
                SplitDir::Vertical,
                leaf("p1", &["a"]),
                leaf("p2", &["b"]),
            ),
            leaf("p3", &["c"]),
        );
        assert_eq!(
            stacked
                .neighbor_leaf(&PaneId::new("p3"), PaneMove::Up)
                .map(PaneId::to_string)
                .as_deref(),
            Some("p1")
        );
        let row = split(
            "sp1",
            SplitDir::Vertical,
            split(
                "sp2",
                SplitDir::Vertical,
                leaf("p1", &["a"]),
                leaf("p2", &["b"]),
            ),
            leaf("p3", &["c"]),
        );
        assert_eq!(
            row.neighbor_leaf(&PaneId::new("p3"), PaneMove::Left)
                .map(PaneId::to_string)
                .as_deref(),
            Some("p2")
        );
    }

    #[test]
    fn reorder_semantica_de_slot() {
        let t = |s: &str| TabId::new(s);
        let tabs = [t("a"), t("b"), t("c")];
        let show = |v: Vec<TabId>| v.iter().map(TabId::to_string).collect::<Vec<_>>().join("");
        assert_eq!(show(reorder_tabs(&tabs, &t("a"), 3)), "bca"); // fim
        assert_eq!(show(reorder_tabs(&tabs, &t("c"), 0)), "cab"); // começo
        assert_eq!(show(reorder_tabs(&tabs, &t("b"), 1)), "abc"); // mesmo slot
        assert_eq!(show(reorder_tabs(&tabs, &t("a"), 2)), "bac");
        assert_eq!(show(reorder_tabs(&tabs, &t("a"), 99)), "bca"); // clamp
        assert_eq!(show(reorder_tabs(&tabs, &t("zz"), 0)), "abc"); // ausente
    }

    #[test]
    fn json_igual_ao_do_flutter() {
        let tree = split(
            "sp1",
            SplitDir::Vertical,
            leaf("p1", &["a", "b"]),
            leaf("p2", &["c"]),
        );
        let json = serde_json::to_value(&tree).ok();
        assert_eq!(
            json,
            Some(serde_json::json!({
                "k": "split", "id": "sp1", "dir": "vertical", "frac": 0.5,
                "a": {"k": "leaf", "id": "p1", "tabs": ["a", "b"], "active": "a"},
                "b": {"k": "leaf", "id": "p2", "tabs": ["c"], "active": "c"},
            }))
        );
        let back: Result<PaneNode, _> = serde_json::from_value(json.unwrap_or_default());
        assert_eq!(back.ok(), Some(tree));
    }
}
