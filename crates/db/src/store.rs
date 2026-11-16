//! Conexões do workspace em `.gsh/databases.json` (versionado) e `.gsh/databases.local.json`
//! (fora do git, sobrepõe por nome), mais os SQLite **detectados** no repo (`db_connection_store_impl.dart`).
//!
//! O arquivo é lido e gravado no mesmo formato do Flutter: mesmas chaves, mesma ordem, recuo de
//! 2 espaços e `\n` no fim.

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::Path;

use serde::Serialize;
use serde_json::Value;

use crate::connection::{Access, Connection, Engine, Entry, Origin};
use crate::error::StoreError;

const DIR_NAME: &str = ".gsh";
const FILE_NAME: &str = "databases.json";
const LOCAL_FILE_NAME: &str = "databases.local.json";

/// Profundidade máxima da varredura de SQLite (raiz = 0).
const SCAN_DEPTH: usize = 3;

/// Diretórios nunca varridos (os que começam com `.` também não).
const SKIP_DIRS: [&str; 4] = ["build", "node_modules", "Pods", "target"];

const SQLITE_EXTENSIONS: [&str; 4] = ["db", "sqlite", "sqlite3", "db3"];
const SQLITE_MAGIC: &[u8] = b"SQLite format 3";

/// Caminho do `databases.json` relativo à raiz do workspace.
pub const RELATIVE_PATH: &str = ".gsh/databases.json";

/// Carrega registradas, locais e detectadas, em ordem alfabética.
pub fn load(root: &Path) -> Vec<Connection> {
    let registered = read_file(&root.join(DIR_NAME).join(FILE_NAME), Origin::Registered);
    let local = read_file(&root.join(DIR_NAME).join(LOCAL_FILE_NAME), Origin::Local);

    // por nome, a local sobrepõe a registrada
    let mut by_name: BTreeMap<String, Connection> = BTreeMap::new();
    for connection in registered.into_iter().chain(local) {
        by_name.insert(connection.name.clone(), connection);
    }

    // detectadas: sem repetir caminho já registrado nem nome em uso
    let known: Vec<String> = by_name
        .values()
        .filter(|c| c.engine == Engine::Sqlite)
        .map(|c| normalize(root, c.sqlite_path()))
        .collect();
    for path in detect_sqlites(root) {
        if known.contains(&normalize(root, &path)) {
            continue;
        }
        let name = path.rsplit('/').next().unwrap_or(&path).to_string();
        if by_name.contains_key(&name) {
            continue;
        }
        by_name.insert(
            name.clone(),
            Connection::sqlite(&name, &path, Origin::Detected, Access::Read, true),
        );
    }
    let mut all: Vec<Connection> = by_name.into_values().collect();
    all.sort_by(Connection::compare_by_name);
    all
}

/// Grava as conexões `Registered` em `.gsh/databases.json` (as locais e detectadas ficam de fora).
pub fn save(root: &Path, connections: &[Connection]) -> Result<(), StoreError> {
    let dir = root.join(DIR_NAME);
    fs::create_dir_all(&dir).map_err(|e| StoreError::io("create .gsh", &e))?;
    fs::write(dir.join(FILE_NAME), encode_databases_json(connections))
        .map_err(|e| StoreError::io("write databases.json", &e))
}

#[derive(Serialize)]
struct Document<'a> {
    databases: Vec<Entry<'a>>,
}

/// Conteúdo do `databases.json`: só as registradas, ordenadas por nome (o arquivo é versionado;
/// gravar na ordem da memória geraria diff de reordenação a cada salvamento).
pub fn encode_databases_json(connections: &[Connection]) -> String {
    let mut registered: Vec<&Connection> = connections
        .iter()
        .filter(|c| c.origin == Origin::Registered)
        .collect();
    registered.sort_by(|a, b| Connection::compare_by_name(a, b));
    let document = Document {
        databases: registered.into_iter().map(Entry::from).collect(),
    };
    let mut text = serde_json::to_string_pretty(&document).unwrap_or_default();
    text.push('\n');
    text
}

/// Lê um `databases.json` (`{"databases":[...]}` ou lista crua). Entrada inválida não derruba as
/// demais; JSON quebrado vira lista vazia (o usuário corrige no editor).
pub fn parse_databases_json(content: &str, origin: Origin) -> Vec<Connection> {
    let Ok(decoded) = serde_json::from_str::<Value>(content) else {
        return Vec::new();
    };
    let list = match &decoded {
        Value::Object(map) => map.get("databases"),
        other => Some(other),
    };
    let Some(Value::Array(items)) = list else {
        return Vec::new();
    };
    items
        .iter()
        .filter(|item| item.is_object())
        .filter_map(|item| Connection::from_json(item, origin).ok())
        .collect()
}

fn read_file(path: &Path, origin: Origin) -> Vec<Connection> {
    fs::read_to_string(path)
        .map(|content| parse_databases_json(&content, origin))
        .unwrap_or_default()
}

fn normalize(root: &Path, path: &str) -> String {
    let drive = path.as_bytes().get(1) == Some(&b':')
        && path.as_bytes().first().is_some_and(u8::is_ascii_alphabetic);
    if path.starts_with('/') || drive {
        path.to_string()
    } else {
        format!("{}/{path}", root.to_string_lossy())
    }
}

/// SQLite do workspace: extensão candidata e cabeçalho mágico confirmado (um `.db` qualquer não
/// basta). Caminhos relativos à raiz, ordenados.
fn detect_sqlites(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    walk(root, root, 0, &mut found);
    found.sort();
    found
}

fn walk(root: &Path, dir: &Path, depth: usize, found: &mut Vec<String>) {
    if depth > SCAN_DEPTH {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        if kind.is_dir() {
            if name.starts_with('.') || SKIP_DIRS.contains(&name.as_str()) {
                continue;
            }
            walk(root, &path, depth + 1, found);
        } else if kind.is_file() {
            let extension = name
                .rsplit_once('.')
                .map(|(_, ext)| ext.to_lowercase())
                .unwrap_or_default();
            if SQLITE_EXTENSIONS.contains(&extension.as_str()) && has_sqlite_magic(&path) {
                let relative = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                found.push(relative);
            }
        }
    }
}

fn has_sqlite_magic(path: &Path) -> bool {
    let mut head = vec![0_u8; SQLITE_MAGIC.len()];
    fs::File::open(path)
        .and_then(|mut file| file.read_exact(&mut head))
        .is_ok()
        && head == SQLITE_MAGIC
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Arquivo gerado pelo `encodeDatabasesJson` do Flutter (`dart:convert`): túnel SSH, aspas,
    /// barra invertida, tabulação, acentos e entradas fora de ordem na origem.
    const FLUTTER_FIXTURE: &str = include_str!("../tests/flutter-databases.json");

    #[test]
    fn reads_and_writes_the_flutter_file_byte_for_byte() {
        let connections = parse_databases_json(FLUTTER_FIXTURE, Origin::Registered);
        assert_eq!(connections.len(), 4);
        assert_eq!(connections[3].name, "Zeta \"quoted\" ação");
        assert_eq!(connections[0].access, Access::ReadWrite);
        assert!(!connections[0].agents);
        assert_eq!(connections[0].ssh.as_ref().map(|s| s.port), Some(2222));
        assert_eq!(encode_databases_json(&connections), FLUTTER_FIXTURE);
    }

    #[test]
    fn output_is_sorted_even_when_the_input_is_not() {
        let mut connections = parse_databases_json(FLUTTER_FIXTURE, Origin::Registered);
        connections.reverse();
        assert_eq!(encode_databases_json(&connections), FLUTTER_FIXTURE);
    }

    #[test]
    fn empty_list_matches_dart() {
        assert_eq!(encode_databases_json(&[]), "{\n  \"databases\": []\n}\n");
    }

    #[test]
    fn only_registered_connections_are_written() {
        let local = Connection::sqlite("l", "l.db", Origin::Local, Access::Read, true);
        let detected = Connection::sqlite("d", "d.db", Origin::Detected, Access::Read, true);
        let kept = Connection::sqlite("k", "k.db", Origin::Registered, Access::Read, true);
        let text = encode_databases_json(&[local, detected, kept]);
        assert!(text.contains("\"k\"") && !text.contains("\"l\"") && !text.contains("\"d\""));
    }

    #[test]
    fn bare_list_broken_json_and_bad_entries_are_tolerated() {
        let bare = r#"[{"name":"a","url":"sqlite:a.db"}, 5, {"name":"x","url":"oracle://h/d"}]"#;
        let parsed = parse_databases_json(bare, Origin::Local);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].origin, Origin::Local);
        assert!(parse_databases_json("{not json", Origin::Registered).is_empty());
        assert!(parse_databases_json(r#"{"databases": 3}"#, Origin::Registered).is_empty());
    }

    fn sqlite_file(path: &Path) {
        let connection = rusqlite::Connection::open(path).unwrap();
        connection
            .execute_batch("CREATE TABLE t (n INTEGER)")
            .unwrap();
    }

    #[test]
    fn load_merges_local_over_registered_and_detects_sqlite_by_magic_header() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join(".gsh")).unwrap();
        fs::write(
            root.join(".gsh/databases.json"),
            r#"{"databases":[
                {"name":"analytics","url":"postgres://ana@db.internal/analytics"},
                {"name":"app","url":"sqlite:data/app.db"}]}"#,
        )
        .unwrap();
        fs::write(
            root.join(".gsh/databases.local.json"),
            r#"{"databases":[{"name":"analytics","url":"postgres://me@localhost/dev"}]}"#,
        )
        .unwrap();
        fs::create_dir_all(root.join("data")).unwrap();
        sqlite_file(&root.join("data/app.db")); // já registrada: não duplica
        sqlite_file(&root.join("data/other.sqlite3"));
        fs::create_dir_all(root.join("node_modules/x")).unwrap();
        sqlite_file(&root.join("node_modules/x/skip.db")); // pasta ignorada
        fs::create_dir_all(root.join(".hidden")).unwrap();
        sqlite_file(&root.join(".hidden/skip.db")); // pasta oculta
        fs::write(root.join("fake.db"), "not a database").unwrap(); // sem cabeçalho mágico
        fs::create_dir_all(root.join("a/b/c/d")).unwrap();
        sqlite_file(&root.join("a/b/c/d/deep.db")); // fundo demais (profundidade 4)

        let all = load(root);
        let summary: Vec<_> = all
            .iter()
            .map(|c| (c.name.as_str(), c.origin, c.url.as_str()))
            .collect();
        assert_eq!(
            summary,
            [
                ("analytics", Origin::Local, "postgres://me@localhost/dev"),
                ("app", Origin::Registered, "sqlite:data/app.db"),
                (
                    "other.sqlite3",
                    Origin::Detected,
                    "sqlite:data/other.sqlite3"
                ),
            ]
        );
    }

    #[test]
    fn save_creates_the_directory_and_writes_only_registered() {
        let dir = tempfile::tempdir().unwrap();
        let connections = parse_databases_json(FLUTTER_FIXTURE, Origin::Registered);
        save(dir.path(), &connections).unwrap();
        let written = fs::read_to_string(dir.path().join(RELATIVE_PATH)).unwrap();
        assert_eq!(written, FLUTTER_FIXTURE);
        assert_eq!(load(dir.path()).len(), 4);
    }
}
