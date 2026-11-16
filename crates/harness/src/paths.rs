//! `p.normalize` e `p.relative` do pacote `path` do Dart, só lexicais (sem disco).

use std::path::{Component, Path, PathBuf};

/// Tira `.`, resolve `..` e a barra final, sem consultar o disco.
pub(crate) fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => match out.components().next_back() {
                Some(Component::Normal(_)) => {
                    out.pop();
                }
                // `/..` é `/`
                Some(Component::RootDir | Component::Prefix(_)) => {}
                _ => out.push(".."),
            },
            other => out.push(other.as_os_str()),
        }
    }
    if out.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        out
    }
}

/// `path` relativo a `from` (`../CLAUDE.md`, `.claude/CLAUDE.md`).
pub(crate) fn relative(path: &Path, from: &Path) -> PathBuf {
    let path = normalize(path);
    let from = normalize(from);
    let a: Vec<Component> = path.components().collect();
    let b: Vec<Component> = from.components().collect();
    let common = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let mut out = PathBuf::new();
    for _ in b.iter().skip(common) {
        out.push("..");
    }
    for component in a.iter().skip(common) {
        out.push(component.as_os_str());
    }
    if out.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_resolves_dots_and_parents() {
        assert_eq!(normalize(Path::new("/w/./a/../b/")), PathBuf::from("/w/b"));
        assert_eq!(normalize(Path::new("/..")), PathBuf::from("/"));
        assert_eq!(normalize(Path::new("a/../..")), PathBuf::from(".."));
    }

    #[test]
    fn relative_climbs_and_descends() {
        assert_eq!(
            relative(Path::new("/r/CLAUDE.md"), Path::new("/r/w")),
            PathBuf::from("../CLAUDE.md")
        );
        assert_eq!(
            relative(Path::new("/r/w/.claude/CLAUDE.md"), Path::new("/r/w")),
            PathBuf::from(".claude/CLAUDE.md")
        );
        assert_eq!(
            relative(Path::new("/r/w"), Path::new("/r/w")),
            PathBuf::from(".")
        );
    }
}
