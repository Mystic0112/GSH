//! Fonte declarativa única para a barra de menus Linux e seus atalhos.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    NewTerminal,
    #[cfg(target_os = "linux")]
    NewRemoteTerminal,
    NewChat,
    NewChatRight,
    OpenWorkspace,
    OpenSettings,
    CheckForUpdates,
    Quit,
    MinimizeWindow,
    ZoomWindow,
    SaveFile,
    FocusSearch,
    ToggleWorkspacePanel,
    ToggleFiles,
    SplitRight,
    SplitDown,
    PreviousWorkspace,
    NextWorkspace,
    ZoomIn,
    ZoomOut,
    ActualSize,
    SelectTab(u8),
    SelectLastTab,
}

#[derive(Debug, Clone, Copy)]
pub struct Entry {
    pub group: &'static str,
    pub id: &'static str,
    pub label: &'static str,
    pub shortcut: Option<&'static str>,
    pub action: Action,
}

pub const ENTRIES: &[Entry] = &[
    Entry {
        group: "Arquivo",
        id: "newTerminal",
        label: "Novo terminal",
        shortcut: Some("Ctrl+T"),
        action: Action::NewTerminal,
    },
    #[cfg(target_os = "linux")]
    Entry {
        group: "Arquivo",
        id: "newRemoteTerminal",
        label: "Novo terminal remoto por SSH",
        shortcut: None,
        action: Action::NewRemoteTerminal,
    },
    Entry {
        group: "Arquivo",
        id: "newChat",
        label: "Novo chat",
        shortcut: Some("Ctrl+Shift+L"),
        action: Action::NewChat,
    },
    Entry {
        group: "Arquivo",
        id: "newChatRight",
        label: "Novo chat à direita",
        shortcut: Some("Ctrl+Shift+J"),
        action: Action::NewChatRight,
    },
    Entry {
        group: "Arquivo",
        id: "openWorkspace",
        label: "Abrir workspace",
        shortcut: Some("Ctrl+O"),
        action: Action::OpenWorkspace,
    },
    Entry {
        group: "GSH",
        id: "openSettings",
        label: "Configurações",
        shortcut: Some("Ctrl+,"),
        action: Action::OpenSettings,
    },
    Entry {
        group: "GSH",
        id: "checkForUpdates",
        label: "Verificar atualizações…",
        shortcut: None,
        action: Action::CheckForUpdates,
    },
    Entry {
        group: "GSH",
        id: "quit",
        label: "Sair",
        shortcut: None,
        action: Action::Quit,
    },
    Entry {
        group: "Arquivo",
        id: "save",
        label: "Salvar",
        shortcut: Some("Ctrl+S"),
        action: Action::SaveFile,
    },
    Entry {
        group: "Exibir",
        id: "toggleWorkspacePanel",
        label: "Alternar painel de workspaces",
        shortcut: Some("Ctrl+B"),
        action: Action::ToggleWorkspacePanel,
    },
    Entry {
        group: "Exibir",
        id: "toggleFiles",
        label: "Alternar arquivos",
        shortcut: Some("Ctrl+Shift+B"),
        action: Action::ToggleFiles,
    },
    Entry {
        group: "Exibir",
        id: "searchInFiles",
        label: "Buscar em arquivos",
        shortcut: Some("Ctrl+Shift+F"),
        action: Action::FocusSearch,
    },
    Entry {
        group: "Exibir",
        id: "splitRight",
        label: "Dividir à direita",
        shortcut: Some("Ctrl+D"),
        action: Action::SplitRight,
    },
    Entry {
        group: "Exibir",
        id: "splitDown",
        label: "Dividir abaixo",
        shortcut: Some("Ctrl+Shift+D"),
        action: Action::SplitDown,
    },
    Entry {
        group: "Exibir",
        id: "previousWorkspace",
        label: "Workspace anterior",
        shortcut: Some("Ctrl+Shift+N"),
        action: Action::PreviousWorkspace,
    },
    Entry {
        group: "Exibir",
        id: "nextWorkspace",
        label: "Próximo workspace",
        shortcut: Some("Ctrl+Shift+M"),
        action: Action::NextWorkspace,
    },
    Entry {
        group: "Exibir",
        id: "zoomIn",
        label: "Aumentar zoom",
        shortcut: Some("Ctrl+="),
        action: Action::ZoomIn,
    },
    Entry {
        group: "Exibir",
        id: "zoomOut",
        label: "Diminuir zoom",
        shortcut: Some("Ctrl+-"),
        action: Action::ZoomOut,
    },
    Entry {
        group: "Exibir",
        id: "actualSize",
        label: "Tamanho original",
        shortcut: Some("Ctrl+0"),
        action: Action::ActualSize,
    },
    Entry {
        group: "Janela",
        id: "selectTab1",
        label: "Selecionar aba 1",
        shortcut: Some("Ctrl+1"),
        action: Action::SelectTab(0),
    },
    Entry {
        group: "Janela",
        id: "selectTab2",
        label: "Selecionar aba 2",
        shortcut: Some("Ctrl+2"),
        action: Action::SelectTab(1),
    },
    Entry {
        group: "Janela",
        id: "selectTab3",
        label: "Selecionar aba 3",
        shortcut: Some("Ctrl+3"),
        action: Action::SelectTab(2),
    },
    Entry {
        group: "Janela",
        id: "selectTab4",
        label: "Selecionar aba 4",
        shortcut: Some("Ctrl+4"),
        action: Action::SelectTab(3),
    },
    Entry {
        group: "Janela",
        id: "selectTab5",
        label: "Selecionar aba 5",
        shortcut: Some("Ctrl+5"),
        action: Action::SelectTab(4),
    },
    Entry {
        group: "Janela",
        id: "selectTab6",
        label: "Selecionar aba 6",
        shortcut: Some("Ctrl+6"),
        action: Action::SelectTab(5),
    },
    Entry {
        group: "Janela",
        id: "selectTab7",
        label: "Selecionar aba 7",
        shortcut: Some("Ctrl+7"),
        action: Action::SelectTab(6),
    },
    Entry {
        group: "Janela",
        id: "selectTab8",
        label: "Selecionar aba 8",
        shortcut: Some("Ctrl+8"),
        action: Action::SelectTab(7),
    },
    Entry {
        group: "Janela",
        id: "selectLastTab",
        label: "Última aba",
        shortcut: Some("Ctrl+9"),
        action: Action::SelectLastTab,
    },
    Entry {
        group: "Janela",
        id: "minimizeWindow",
        label: "Minimizar janela",
        shortcut: None,
        action: Action::MinimizeWindow,
    },
    Entry {
        group: "Janela",
        id: "zoomWindow",
        label: "Alternar tamanho da janela",
        shortcut: None,
        action: Action::ZoomWindow,
    },
];

/// Validates the editable `Ctrl+...` binding syntax. Linux maps the primary modifier to Ctrl.
pub fn valid_binding(value: &str) -> bool {
    let mut has_key = false;
    let mut ctrl = false;
    let mut shift = false;
    let mut alt = false;
    for part in value.split('+') {
        let normalized = part.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "ctrl" if !ctrl => ctrl = true,
            "shift" if !shift => shift = true,
            "alt" if !alt => alt = true,
            key_part if !has_key && (key_part.len() == 1 || key_part == "comma") => has_key = true,
            _ => return false,
        }
    }
    ctrl && has_key
}

pub fn binding<'a>(
    entry: &'a Entry,
    overrides: &'a std::collections::BTreeMap<String, String>,
) -> Option<&'a str> {
    overrides
        .get(entry.id)
        .map(String::as_str)
        .filter(|value| valid_binding(value))
        .or(entry.shortcut)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{binding, valid_binding, Action, Entry};

    #[test]
    fn accepts_only_supported_configurable_linux_bindings() {
        for value in [
            "Ctrl+T",
            "Ctrl+Shift+1",
            "Ctrl+Alt+U",
            "Ctrl+,",
            "Ctrl+comma",
        ] {
            assert!(valid_binding(value), "{value}");
        }
        for value in ["T", "Ctrl+Alt+Shift", "Ctrl+T+X", "Ctrl+Ctrl+T", "Ctrl+"] {
            assert!(!valid_binding(value), "{value}");
        }
    }

    #[test]
    fn invalid_override_falls_back_to_the_declared_shortcut() {
        let entry = Entry {
            group: "Arquivo",
            id: "newTerminal",
            label: "Novo terminal",
            shortcut: Some("Ctrl+T"),
            action: Action::NewTerminal,
        };
        let overrides = BTreeMap::from([("newTerminal".to_owned(), "Alt+T".to_owned())]);
        assert_eq!(binding(&entry, &overrides), Some("Ctrl+T"));
    }
}
