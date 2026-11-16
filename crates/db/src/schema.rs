//! Introspecção de schema (`db_schema_sql.dart` e `database_viewmodel.dart`): tabelas e colunas
//! de um banco, na forma única que a árvore do painel consome.

use crate::connection::Engine;
use crate::error::DbError;
use crate::result::{Cell, QueryResult};

/// Tabela (ou view). `schema` é o namespace do motor (`public`); `None` no SQLite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaTable {
    pub name: String,
    pub schema: Option<String>,
    pub is_view: bool,
}

/// Coluna de uma tabela.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaColumn {
    pub name: String,
    pub ty: String,
    pub primary_key: bool,
}

/// SQL que lista as tabelas (`table` é `None`) ou as colunas de `table`.
pub fn schema_sql(
    engine: Engine,
    table: Option<&str>,
    schema: Option<&str>,
) -> Result<String, DbError> {
    let table = table.map(safe_ident).transpose()?;
    match engine {
        Engine::Sqlite => Ok(match table {
            None => "SELECT name AS \"table\", NULL AS \"schema\", type \
                FROM sqlite_master \
                WHERE type IN ('table','view') AND name NOT LIKE 'sqlite_%' \
                ORDER BY name"
                .to_string(),
            Some(t) => format!(
                "SELECT name AS \"column\", type, \
                (CASE \"notnull\" WHEN 0 THEN 1 ELSE 0 END) AS nullable, \
                pk AS \"primaryKey\" FROM pragma_table_info('{t}')"
            ),
        }),
        Engine::Postgres => {
            let s = safe_ident(schema.unwrap_or("public"))?;
            Ok(match table {
                None => "SELECT table_name AS \"table\", table_schema AS \"schema\", \
                    CASE table_type WHEN 'VIEW' THEN 'view' ELSE 'table' END \
                    AS type FROM information_schema.tables \
                    WHERE table_schema NOT IN ('pg_catalog','information_schema') \
                    ORDER BY table_schema, table_name"
                    .to_string(),
                Some(t) => format!(
                    "SELECT c.column_name AS \"column\", c.data_type AS type, \
                    CASE WHEN c.is_nullable = 'YES' THEN 1 ELSE 0 END \
                    AS nullable, \
                    CASE WHEN pk.column_name IS NULL THEN 0 ELSE 1 END \
                    AS \"primaryKey\" \
                    FROM information_schema.columns c \
                    LEFT JOIN (\
                     SELECT kcu.column_name \
                     FROM information_schema.table_constraints tc \
                     JOIN information_schema.key_column_usage kcu \
                       ON kcu.constraint_name = tc.constraint_name \
                      AND kcu.table_schema = tc.table_schema \
                     WHERE tc.constraint_type = 'PRIMARY KEY' \
                      AND tc.table_schema = '{s}' AND tc.table_name = '{t}'\
                    ) pk ON pk.column_name = c.column_name \
                    WHERE c.table_schema = '{s}' AND c.table_name = '{t}' \
                    ORDER BY c.ordinal_position"
                ),
            })
        }
        Engine::Mysql => Ok(match table {
            None => "SELECT table_name AS `table`, table_schema AS `schema`, \
                CASE table_type WHEN 'VIEW' THEN 'view' ELSE 'table' END \
                AS type FROM information_schema.tables \
                WHERE table_schema = DATABASE() ORDER BY table_name"
                .to_string(),
            Some(t) => format!(
                "SELECT column_name AS `column`, data_type AS type, \
                CASE is_nullable WHEN 'YES' THEN 1 ELSE 0 END AS nullable, \
                CASE column_key WHEN 'PRI' THEN 1 ELSE 0 END \
                AS `primaryKey` FROM information_schema.columns \
                WHERE table_schema = DATABASE() AND table_name = '{t}' \
                ORDER BY ordinal_position"
            ),
        }),
        Engine::Mssql => {
            let s = safe_ident(schema.unwrap_or("dbo"))?;
            Ok(match table {
                None => "SELECT TABLE_NAME AS [table], TABLE_SCHEMA AS [schema], \
                    CASE TABLE_TYPE WHEN 'VIEW' THEN 'view' ELSE 'table' END \
                    AS type FROM INFORMATION_SCHEMA.TABLES \
                    ORDER BY TABLE_SCHEMA, TABLE_NAME"
                    .to_string(),
                Some(t) => format!(
                    "SELECT c.COLUMN_NAME AS [column], c.DATA_TYPE AS type, \
                    CASE c.IS_NULLABLE WHEN 'YES' THEN 1 ELSE 0 END \
                    AS nullable, \
                    CASE WHEN pk.COLUMN_NAME IS NULL THEN 0 ELSE 1 END \
                    AS [primaryKey] \
                    FROM INFORMATION_SCHEMA.COLUMNS c \
                    LEFT JOIN (\
                     SELECT ku.COLUMN_NAME \
                     FROM INFORMATION_SCHEMA.TABLE_CONSTRAINTS tc \
                     JOIN INFORMATION_SCHEMA.KEY_COLUMN_USAGE ku \
                       ON ku.CONSTRAINT_NAME = tc.CONSTRAINT_NAME \
                     WHERE tc.CONSTRAINT_TYPE = 'PRIMARY KEY' \
                      AND tc.TABLE_SCHEMA = '{s}' AND tc.TABLE_NAME = '{t}'\
                    ) pk ON pk.COLUMN_NAME = c.COLUMN_NAME \
                    WHERE c.TABLE_SCHEMA = '{s}' AND c.TABLE_NAME = '{t}' \
                    ORDER BY c.ORDINAL_POSITION"
                ),
            })
        }
        other => Err(DbError::UnsupportedEngine(other)),
    }
}

/// Identificador seguro para interpolar: só letra, dígito, `_`, `.` e `$`.
fn safe_ident(name: &str) -> Result<&str, DbError> {
    let ok = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '$'));
    if ok {
        Ok(name)
    } else {
        Err(DbError::InvalidIdentifier(name.to_string()))
    }
}

/// Cita um identificador na sintaxe do motor (tabela CamelCase quebra sem aspas no Postgres).
pub fn quote_ident(engine: Engine, ident: &str) -> String {
    match engine {
        Engine::Mysql => format!("`{}`", ident.replace('`', "``")),
        Engine::Mssql => format!("[{}]", ident.replace(']', "]]")),
        _ => format!("\"{}\"", ident.replace('"', "\"\"")),
    }
}

/// Tabela citada, prefixada pelo schema quando houver.
pub fn qualified_ident(engine: Engine, schema: Option<&str>, table: &str) -> String {
    match schema {
        Some(schema) if !schema.is_empty() => {
            format!(
                "{}.{}",
                quote_ident(engine, schema),
                quote_ident(engine, table)
            )
        }
        _ => quote_ident(engine, table),
    }
}

/// Texto de uma célula como o Dart a interpola (`'${row[i]}'`).
fn text_of(cell: &Cell) -> String {
    match cell {
        Cell::Null => "null".to_string(),
        other => other.display(),
    }
}

fn index_of(result: &QueryResult, name: &str) -> Option<usize> {
    result.columns.iter().position(|column| column.name == name)
}

fn cell(row: &[Cell], index: usize) -> Option<&Cell> {
    row.get(index)
}

/// Tabelas a partir do resultado de [`schema_sql`] sem `table`.
pub fn tables_from(result: &QueryResult) -> Vec<SchemaTable> {
    let table = index_of(result, "table").unwrap_or(0);
    let schema = index_of(result, "schema");
    let kind = index_of(result, "type");
    result
        .rows
        .iter()
        .map(|row| SchemaTable {
            name: cell(row, table).map(text_of).unwrap_or_default(),
            schema: schema
                .and_then(|index| cell(row, index))
                .filter(|cell| !matches!(cell, Cell::Null))
                .map(text_of),
            is_view: kind
                .and_then(|index| cell(row, index))
                .is_some_and(|cell| text_of(cell) == "view"),
        })
        .collect()
}

/// Colunas a partir do resultado de [`schema_sql`] com `table`. Só `primaryKey == 1` marca PK
/// (num PK composto o SQLite numera 1, 2, ...; as seguintes não aparecem marcadas, como no Flutter).
pub fn columns_from(result: &QueryResult) -> Vec<SchemaColumn> {
    let name = index_of(result, "column").unwrap_or(0);
    let ty = index_of(result, "type").unwrap_or(1);
    let pk = index_of(result, "primaryKey");
    result
        .rows
        .iter()
        .map(|row| SchemaColumn {
            name: cell(row, name).map(text_of).unwrap_or_default(),
            ty: cell(row, ty).map(text_of).unwrap_or_default(),
            primary_key: pk
                .and_then(|index| cell(row, index))
                .is_some_and(|cell| text_of(cell) == "1"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::result::Column;
    use std::time::Duration;

    #[test]
    fn identifiers_are_validated_before_interpolation() {
        assert!(schema_sql(Engine::Sqlite, Some("users"), None).is_ok());
        assert_eq!(
            schema_sql(Engine::Sqlite, Some("a b"), None),
            Err(DbError::InvalidIdentifier("a b".into()))
        );
        assert_eq!(
            schema_sql(Engine::Postgres, Some("t"), Some("x'; drop")),
            Err(DbError::InvalidIdentifier("x'; drop".into()))
        );
        assert!(schema_sql(Engine::Mysql, None, None).is_ok());
        assert!(schema_sql(Engine::Mssql, Some("Users"), Some("dbo")).is_ok());
        assert!(matches!(
            schema_sql(Engine::Redis, None, None),
            Err(DbError::UnsupportedEngine(Engine::Redis))
        ));
    }

    #[test]
    fn quoting_follows_the_engine() {
        assert_eq!(quote_ident(Engine::Postgres, "Us\"er"), "\"Us\"\"er\"");
        assert_eq!(quote_ident(Engine::Mysql, "a`b"), "`a``b`");
        assert_eq!(quote_ident(Engine::Mssql, "a]b"), "[a]]b]");
        assert_eq!(
            qualified_ident(Engine::Postgres, Some("sales"), "Orders"),
            "\"sales\".\"Orders\""
        );
        assert_eq!(qualified_ident(Engine::Sqlite, None, "t"), "\"t\"");
        assert_eq!(qualified_ident(Engine::Sqlite, Some(""), "t"), "\"t\"");
    }

    fn result(names: &[&str], rows: Vec<Vec<Cell>>) -> QueryResult {
        QueryResult {
            columns: names
                .iter()
                .map(|n| Column {
                    name: (*n).into(),
                    ty: String::new(),
                })
                .collect(),
            rows,
            elapsed: Duration::ZERO,
            truncated: false,
            affected_rows: None,
        }
    }

    #[test]
    fn tables_and_columns_are_read_by_column_name() {
        let tables = tables_from(&result(
            &["table", "schema", "type"],
            vec![
                vec![
                    Cell::Text("a".into()),
                    Cell::Null,
                    Cell::Text("table".into()),
                ],
                vec![
                    Cell::Text("v".into()),
                    Cell::Text("public".into()),
                    Cell::Text("view".into()),
                ],
            ],
        ));
        assert_eq!(tables[0].schema, None);
        assert!(!tables[0].is_view);
        assert_eq!(tables[1].schema.as_deref(), Some("public"));
        assert!(tables[1].is_view);

        let columns = columns_from(&result(
            &["column", "type", "nullable", "primaryKey"],
            vec![
                vec![
                    Cell::Text("id".into()),
                    Cell::Text("INTEGER".into()),
                    Cell::Int(1),
                    Cell::Int(1),
                ],
                vec![
                    Cell::Text("k2".into()),
                    Cell::Text("TEXT".into()),
                    Cell::Int(1),
                    Cell::Int(2),
                ],
            ],
        ));
        assert!(columns[0].primary_key);
        assert!(!columns[1].primary_key);
    }
}
