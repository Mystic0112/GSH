use std::sync::atomic::{AtomicU64, Ordering};

use super::*;

static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

struct TempTreeDir {
    path: PathBuf,
}

impl TempTreeDir {
    fn new() -> Self {
        let id = TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("gsh-file-tree-test-{}-{}", std::process::id(), id));
        fs::create_dir_all(&path).expect("create temp test dir");
        Self { path }
    }
}

impl Drop for TempTreeDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn lista_diretorios_primeiro_depois_arquivos_em_ordem_alfabetica_e_ignora_vcs() {
    let tmp = TempTreeDir::new();
    fs::create_dir_all(tmp.path.join("zeta")).unwrap();
    fs::create_dir_all(tmp.path.join("Alpha")).unwrap();
    fs::create_dir_all(tmp.path.join(".git")).unwrap();
    fs::create_dir_all(tmp.path.join(".hg")).unwrap();
    fs::create_dir_all(tmp.path.join(".svn")).unwrap();
    fs::write(tmp.path.join(".DS_Store"), b"").unwrap();
    fs::write(tmp.path.join(".env"), b"KEY=1").unwrap();
    fs::write(tmp.path.join("main.rs"), b"fn main() {}").unwrap();
    fs::write(tmp.path.join("Cargo.toml"), b"[package]").unwrap();

    let entries = read_dir_sorted(&tmp.path).unwrap();
    let names: Vec<(&str, bool)> = entries
        .iter()
        .map(|e| (e.name.as_str(), e.is_dir))
        .collect();

    assert_eq!(
        names,
        vec![
            ("Alpha", true),
            ("zeta", true),
            (".env", false),
            ("Cargo.toml", false),
            ("main.rs", false),
        ]
    );
}

#[test]
fn carrega_subpastas_de_forma_lazy_ao_expandir_e_destaca_arquivo_selecionado() {
    let tmp = TempTreeDir::new();
    let src_dir = tmp.path.join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(src_dir.join(".git")).unwrap();
    let lib_file = src_dir.join("lib.rs");
    fs::write(&lib_file, b"pub fn ok() {}").unwrap();

    let mut tree = FileTree::from_root(&tmp.path);
    // Subpasta `src` ainda não foi lida do disco (lazy!)
    assert!(!tree.is_dir_cached(&src_dir));
    assert_eq!(tree.visible_rows().len(), 1);

    // Expande `src`: agora lê `src` e exibe `lib.rs` (ignorando `src/.git`)
    tree.toggle_dir(&src_dir);
    assert!(tree.is_dir_cached(&src_dir));
    assert!(tree.is_expanded(&src_dir));
    assert_eq!(tree.visible_rows().len(), 2);

    // Seleciona `lib.rs`
    let hook = tree.select_file(lib_file.clone());
    assert_eq!(hook, ViewerHook::OpenFile(lib_file.clone()));
    assert_eq!(tree.selected_file(), Some(lib_file.as_path()));

    // Recolhe `src`: volta a ter apenas 1 linha visível
    tree.toggle_dir(&src_dir);
    assert!(!tree.is_expanded(&src_dir));
    assert_eq!(tree.visible_rows().len(), 1);
}

#[test]
fn erro_de_leitura_vira_estado_tipado_sem_panic() {
    let missing = std::env::temp_dir().join("gsh-caminho-que-nao-existe-999999");
    let tree = FileTree::from_root(&missing);
    let rows = tree.visible_rows();
    assert_eq!(
        rows,
        vec![TreeRow::Error {
            depth: 0,
            error: TreeReadError::NotFound,
        }]
    );
}

#[test]
fn keeps_folder_selection_for_header_create_and_rejects_context_outside_root() {
    let tmp = TempTreeDir::new();
    let folder = tmp.path.join("folder");
    fs::create_dir_all(&folder).unwrap();
    let file = folder.join("note.md");
    fs::write(&file, b"note").unwrap();
    let outside = tmp.path.with_file_name(format!(
        "{}-outside",
        tmp.path.file_name().unwrap_or_default().to_string_lossy()
    ));
    fs::create_dir_all(&outside).unwrap();

    let mut tree = FileTree::from_root(&tmp.path);
    tree.toggle_dir(&folder);
    assert_eq!(tree.selected_path(), Some(folder.as_path()));
    assert_eq!(tree.create_target_dir(), Some(folder.as_path()));
    tree.select_file(file.clone());
    assert_eq!(tree.create_target_dir(), Some(folder.as_path()));

    tree.show_context_menu(file.clone());
    assert_eq!(tree.context_target(), Some(file.as_path()));
    tree.show_context_menu(outside);
    assert_eq!(tree.context_target(), Some(file.as_path()));
    fs::remove_dir_all(tmp.path.with_file_name(format!(
        "{}-outside",
        tmp.path.file_name().unwrap_or_default().to_string_lossy()
    )))
    .unwrap();
}
