//! Modelos da aba Galeria (`gallery_template.dart` do Flutter): um documento
//! especial do GSH que a galeria cria na raiz do workspace, com extensão
//! própria e conteúdo inicial. Só dados e regra de nome, sem I/O nem UI.
//!
//! A ordem de [`GalleryTemplate::ALL`] é a ordem dos cartões. O conteúdo de
//! cada modelo é o texto do Dart, byte a byte, em `assets/gallery/*.tpl`.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GalleryTemplate {
    DbQuery,
    Kanban,
    Notebook,
    Html,
    Panel,
    HttpRequest,
    Tasks,
    Layout,
    WorkspaceEnv,
}

impl GalleryTemplate {
    pub const ALL: [Self; 9] = [
        Self::DbQuery,
        Self::Kanban,
        Self::Notebook,
        Self::Html,
        Self::Panel,
        Self::HttpRequest,
        Self::Tasks,
        Self::Layout,
        Self::WorkspaceEnv,
    ];

    /// Nome sugerido sem extensão (`dev` -> `dev.ckp`, `dev-2.ckp` se já existe).
    pub fn base_name(self) -> &'static str {
        match self {
            Self::DbQuery => "query",
            Self::Kanban => "board",
            Self::Notebook => "welcome",
            Self::Html => "view",
            Self::Panel => "dashboard",
            Self::HttpRequest => "requests",
            Self::Tasks => "tasks",
            Self::Layout => "dev",
            Self::WorkspaceEnv => "",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::DbQuery => "dbq",
            Self::Kanban => "kanban",
            Self::Notebook => "md",
            Self::Html => "html",
            Self::Panel => "panel",
            Self::HttpRequest => "http",
            Self::Tasks => "json",
            Self::Layout => "ckp",
            Self::WorkspaceEnv => "env.gsh",
        }
    }

    /// Subpasta (relativa à raiz) onde o arquivo mora; vazio = raiz.
    pub fn relative_dir(self) -> &'static str {
        match self {
            Self::Notebook => "notes.notebook",
            Self::Tasks => ".gsh",
            _ => "",
        }
    }

    /// `true` = o nome é contrato do app (`.gsh/tasks.json`): se já existe,
    /// abre o existente em vez de criar um `-2`.
    pub fn fixed_name(self) -> bool {
        matches!(self, Self::Notebook | Self::Tasks | Self::WorkspaceEnv)
    }

    /// `true` = o documento é a pasta (`relative_dir`), não o arquivo (caderno).
    pub fn opens_parent(self) -> bool {
        matches!(self, Self::Notebook)
    }

    /// Ícone do cartão: nome do SVG do tema de ícones, ou `gsh_logo`.
    pub fn icon(self) -> &'static str {
        match self {
            Self::DbQuery => "database",
            Self::Kanban => "todo",
            Self::Notebook => "gsh-notebook",
            Self::Html => "html",
            Self::Panel => "gsh-panel",
            Self::HttpRequest => "http",
            Self::Tasks => "console",
            Self::Layout | Self::WorkspaceEnv => "gsh_logo",
        }
    }

    /// Conteúdo inicial do arquivo.
    pub fn content(self) -> &'static str {
        match self {
            Self::DbQuery => include_str!("../assets/gallery/db_query.tpl"),
            Self::Kanban => include_str!("../assets/gallery/kanban.tpl"),
            Self::Notebook => include_str!("../assets/gallery/notebook.tpl"),
            Self::Html => include_str!("../assets/gallery/html.tpl"),
            Self::Panel => include_str!("../assets/gallery/panel.tpl"),
            Self::HttpRequest => include_str!("../assets/gallery/http_request.tpl"),
            Self::Tasks => include_str!("../assets/gallery/tasks.tpl"),
            Self::Layout => include_str!("../assets/gallery/layout.tpl"),
            Self::WorkspaceEnv => include_str!("../assets/gallery/workspace_env.tpl"),
        }
    }

    /// `.env.gsh` tem `base_name` vazio: o nome é só o "ponto + extensão".
    pub fn file_name(self) -> String {
        match self.base_name() {
            "" => format!(".{}", self.extension()),
            base => format!("{base}.{}", self.extension()),
        }
    }

    /// Caminho relativo à raiz, com a subpasta quando houver (`.gsh/tasks.json`).
    pub fn relative_path(self) -> String {
        match self.relative_dir() {
            "" => self.file_name(),
            dir => format!("{dir}/{}", self.file_name()),
        }
    }

    /// Texto à direita do título do cartão: a pasta (caderno), `.ext` (raiz) ou
    /// o caminho relativo (subpasta).
    pub fn card_path(self) -> String {
        if self.opens_parent() {
            self.relative_dir().to_owned()
        } else if self.relative_dir().is_empty() {
            format!(".{}", self.extension())
        } else {
            self.relative_path()
        }
    }

    /// Primeiro nome livre dado o conjunto de `taken` (basenames da pasta,
    /// comparados sem case): `dev.ckp`, `dev-2.ckp`, `dev-3.ckp`...
    pub fn unique_file_name<I>(self, taken: I) -> String
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let lower: Vec<String> = taken
            .into_iter()
            .map(|name| name.as_ref().to_lowercase())
            .collect();
        let is_taken = |name: &str| lower.contains(&name.to_lowercase());
        let file_name = self.file_name();
        if !is_taken(&file_name) {
            return file_name;
        }
        (2u64..)
            .map(|n| format!("{}-{n}.{}", self.base_name(), self.extension()))
            .find(|candidate| !is_taken(candidate))
            .unwrap_or(file_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_file_name_returns_base_name_when_free() {
        assert_eq!(
            GalleryTemplate::Layout.unique_file_name(["a.txt"]),
            "dev.ckp"
        );
    }

    #[test]
    fn unique_file_name_suffixes_when_taken_ignoring_case() {
        assert_eq!(
            GalleryTemplate::Layout.unique_file_name(["DEV.CKP"]),
            "dev-2.ckp"
        );
        assert_eq!(
            GalleryTemplate::Kanban.unique_file_name(["board.kanban", "board-2.kanban"]),
            "board-3.kanban"
        );
    }

    #[test]
    fn tasks_lives_under_gsh_with_a_fixed_name() {
        assert_eq!(GalleryTemplate::Tasks.relative_path(), ".gsh/tasks.json");
        assert!(GalleryTemplate::Tasks.fixed_name());
        assert_eq!(GalleryTemplate::Html.relative_path(), "view.html");
    }

    #[test]
    fn every_template_has_content_and_an_icon() {
        assert_eq!(GalleryTemplate::ALL.len(), 9);
        for template in GalleryTemplate::ALL {
            assert!(!template.content().is_empty(), "{template:?}");
            assert!(!template.icon().is_empty(), "{template:?}");
            assert!(
                template
                    .file_name()
                    .ends_with(&format!(".{}", template.extension())),
                "{template:?}"
            );
        }
    }

    #[test]
    fn workspace_env_is_a_dotfile_and_notebook_opens_its_folder() {
        assert_eq!(GalleryTemplate::WorkspaceEnv.file_name(), ".env.gsh");
        assert_eq!(
            GalleryTemplate::Notebook.relative_path(),
            "notes.notebook/welcome.md"
        );
        assert!(GalleryTemplate::Notebook.opens_parent());
    }

    #[test]
    fn card_path_matches_the_flutter_card() {
        assert_eq!(GalleryTemplate::Kanban.card_path(), ".kanban");
        assert_eq!(GalleryTemplate::Notebook.card_path(), "notes.notebook");
        assert_eq!(GalleryTemplate::Tasks.card_path(), ".gsh/tasks.json");
        assert_eq!(GalleryTemplate::WorkspaceEnv.card_path(), ".env.gsh");
    }

    #[test]
    fn contents_are_the_dart_text() {
        assert_eq!(
            GalleryTemplate::DbQuery.content(),
            "-- limit: 100\nSELECT 1;\n"
        );
        assert_eq!(
            GalleryTemplate::HttpRequest.content(),
            "### Ping\nGET https://httpbin.org/get\nAccept: application/json\n"
        );
        assert!(GalleryTemplate::Panel
            .content()
            .starts_with("---\ntitle: Dashboard\nreload: true\ncwd: .\n---\n<!doctype html>\n"));
        assert!(GalleryTemplate::Tasks
            .content()
            .starts_with("{\n  // .gsh/tasks.json"));
        assert!(GalleryTemplate::Layout
            .content()
            .ends_with("    command: claude\n"));
    }
}
