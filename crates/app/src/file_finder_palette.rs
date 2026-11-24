//! Quick-open por nome de arquivo, alinhado ao `FileSearcherImpl` do Flutter.

use std::path::{Path, PathBuf};

const MAX_FILES: usize = 6000;
const LIMIT: usize = 12;
pub const INPUT_ID: &str = "file-finder-input";
const IGNORED: &[&str] = &[
    "node_modules",
    "build",
    ".dart_tool",
    ".next",
    "dist",
    "out",
    "Pods",
    "DerivedData",
    ".gradle",
    ".venv",
    "venv",
    "__pycache__",
    "target",
    "vendor",
    "coverage",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Open,
    Close,
    Query(String),
    DebouncedQuery(String, u64),
    Up,
    Down,
    Pick,
    PickAt(usize),
}

#[derive(Debug, Clone)]
pub struct State {
    pub root: PathBuf,
    pub query: String,
    pub files: Vec<String>,
    pub results: Vec<String>,
    pub selected: usize,
    pub generation: u64,
}

impl State {
    pub fn from_files(root: PathBuf, files: Vec<String>) -> Self {
        let results = rank(&files, "");
        Self {
            root,
            query: String::new(),
            files,
            results,
            selected: 0,
            generation: 0,
        }
    }

    pub fn apply(&mut self, action: Action) -> Option<PathBuf> {
        match action {
            Action::Query(query) => {
                self.query = query;
                self.generation = self.generation.wrapping_add(1);
            }
            Action::DebouncedQuery(query, generation) if generation == self.generation => {
                self.query = query;
                self.results = rank(&self.files, &self.query);
                self.selected = 0;
            }
            Action::DebouncedQuery(_, _) => {}
            Action::Up => self.selected = self.selected.saturating_sub(1),
            Action::Down => {
                self.selected = self
                    .selected
                    .saturating_add(1)
                    .min(self.results.len().saturating_sub(1))
            }
            Action::Pick => {
                return self
                    .results
                    .get(self.selected)
                    .map(|path| self.root.join(path))
            }
            Action::PickAt(index) => {
                return self.results.get(index).map(|path| self.root.join(path))
            }
            Action::Open | Action::Close => {}
        }
        None
    }
}

pub fn rank(files: &[String], query: &str) -> Vec<String> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return files.iter().take(LIMIT).cloned().collect();
    }
    let mut ranked: Vec<(u8, usize, &str)> = files
        .iter()
        .filter_map(|path| {
            let lower = path.to_lowercase();
            let base = lower.rsplit('/').next().unwrap_or(&lower);
            let score = if base.starts_with(&query) {
                0
            } else if base.contains(&query) {
                1
            } else if lower.contains(&query) {
                2
            } else {
                return None;
            };
            Some((score, path.len(), path.as_str()))
        })
        .collect();
    ranked.sort_by_key(|(score, length, _)| (*score, *length));
    ranked
        .into_iter()
        .take(LIMIT)
        .map(|(_, _, path)| path.to_owned())
        .collect()
}

pub fn list_files(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while !stack.is_empty() && out.len() < MAX_FILES {
        let Some(dir) = stack.pop() else {
            break;
        };
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                if !name.starts_with('.') && !IGNORED.contains(&name) {
                    stack.push(path);
                }
            } else if kind.is_file() && name != ".DS_Store" && out.len() < MAX_FILES {
                if let Ok(relative) = path.strip_prefix(root) {
                    out.push(relative.to_string_lossy().replace('\\', "/"));
                }
            }
        }
    }
    out.sort();
    out
}

pub fn view<'a>(state: &'a State) -> iced::Element<'a, Action> {
    use iced::widget::{button, column, container, row, scrollable, text, text_input};
    use iced::{Fill, Theme};
    let input = text_input("Go to file…", &state.query)
        .id(INPUT_ID)
        .on_input(Action::Query)
        .on_submit(Action::Pick)
        .padding([9, 10]);
    let mut rows = column![].spacing(0);
    for (index, path) in state.results.iter().enumerate() {
        let name = path.rsplit('/').next().unwrap_or(path);
        let directory = path.rsplit_once('/').map_or("", |(parent, _)| parent);
        let icon = crate::file_icons::view_sized(name, false, false, 16.0);
        let line = row![
            icon,
            text(name).size(13),
            text(directory)
                .size(11)
                .color(crate::theme::palette().text2),
        ]
        .spacing(8)
        .padding([7, 12]);
        rows = rows.push(
            button(line)
                .width(Fill)
                .on_press(Action::PickAt(index))
                .style(move |theme: &Theme, status| {
                    if index == state.selected {
                        iced::widget::button::secondary(theme, status)
                    } else {
                        iced::widget::button::text(theme, status)
                    }
                }),
        );
    }
    let results: iced::Element<'a, Action> = if state.results.is_empty() {
        container(
            text("No files.")
                .size(crate::theme::SIZE_LABEL)
                .color(crate::theme::palette().text2),
        )
        .padding([4, 14])
        .into()
    } else {
        scrollable(rows).height(iced::Length::Shrink).into()
    };
    container(column![container(input).padding(10), results].spacing(0))
        .width(560)
        .max_height(420)
        .style(|_: &Theme| container::Style {
            background: Some(crate::theme::palette().panel.into()),
            border: iced::Border {
                radius: 10.0.into(),
                width: 1.0,
                color: crate::theme::palette().border,
            },
            shadow: iced::Shadow {
                color: crate::theme::palette().shadow,
                offset: iced::Vector::new(0.0, 10.0),
                blur_radius: 24.0,
            },
            ..Default::default()
        })
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranks_basename_prefix_then_contains_then_path() {
        let files = vec![
            "docs/readme.md".into(),
            "readme-old.md".into(),
            "src/reader.rs".into(),
            "src/other.rs".into(),
        ];
        assert_eq!(
            rank(&files, "read"),
            vec!["readme-old.md", "src/reader.rs", "docs/readme.md"]
        );
    }

    #[test]
    fn stale_debounce_does_not_replace_newer_results_and_pick_is_workspace_relative() {
        let root = PathBuf::from("/workspace");
        let mut state = State::from_files(
            root.clone(),
            vec!["src/main.rs".into(), "docs/readme.md".into()],
        );
        let _ = state.apply(Action::Query("read".into()));
        let stale = state.generation;
        let _ = state.apply(Action::Query("main".into()));
        let current = state.generation;
        let _ = state.apply(Action::DebouncedQuery("read".into(), stale));
        assert_eq!(state.results.len(), 2);
        let _ = state.apply(Action::DebouncedQuery("main".into(), current));
        assert_eq!(state.results, vec!["src/main.rs"]);
        assert_eq!(state.apply(Action::Pick), Some(root.join("src/main.rs")));
    }

    #[test]
    fn skips_hidden_and_heavy_directories() {
        let root = std::env::temp_dir().join(format!("gsh-file-finder-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::create_dir_all(root.join("target")).unwrap();
        std::fs::create_dir_all(root.join(".cache")).unwrap();
        std::fs::write(root.join("src/main.rs"), "").unwrap();
        std::fs::write(root.join("target/large"), "").unwrap();
        std::fs::write(root.join(".cache/hidden"), "").unwrap();
        assert_eq!(list_files(&root), vec!["src/main.rs"]);
        let _ = std::fs::remove_dir_all(root);
    }
}
