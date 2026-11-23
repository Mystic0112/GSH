//! Conservative access checks for agent initiated database commands.
//!
//! These checks intentionally favor false rejection over allowing a command
//! whose effects cannot be established from its syntax.

use serde_json::Value;

const SQL_READ_START: &[&str] = &[
    "select", "with", "explain", "show", "describe", "desc", "pragma", "values",
];
const SQL_WRITES: &[&str] = &[
    "insert", "update", "delete", "merge", "replace", "upsert", "drop", "create", "alter",
    "truncate", "grant", "revoke", "vacuum", "attach", "detach", "reindex", "copy", "call", "exec",
    "execute", "set", "lock", "rename", "into",
];
const PURE_FUNCTIONS: &[&str] = &[
    "abs",
    "avg",
    "cast",
    "ceil",
    "ceiling",
    "coalesce",
    "concat",
    "count",
    "date",
    "datetime",
    "floor",
    "format",
    "ifnull",
    "json",
    "json_array",
    "json_extract",
    "json_object",
    "length",
    "lower",
    "max",
    "min",
    "nullif",
    "printf",
    "round",
    "substr",
    "substring",
    "sum",
    "time",
    "trim",
    "typeof",
    "upper",
];
const SQLITE_READ_PRAGMAS: &[&str] = &[
    "collation_list",
    "compile_options",
    "database_list",
    "foreign_key_check",
    "foreign_key_list",
    "function_list",
    "index_info",
    "index_list",
    "index_xinfo",
    "integrity_check",
    "module_list",
    "pragma_list",
    "quick_check",
    "schema_version",
    "table_info",
    "table_list",
    "table_xinfo",
    "user_version",
];

/// Returns the Flutter-compatible error text when SQL is not provably read-only.
pub fn sql_read_violation(sql: &str) -> Option<String> {
    // MySQL executes these comment forms as SQL. Treat them as opaque rather
    // than stripping them like ordinary comments and accidentally approving a
    // hidden write or side-effecting function.
    if sql.contains("/*!") || sql.contains("/*+") {
        return Some(sql_violation("executable SQL comments are not allowed"));
    }
    let statements = match tokenize_sql(sql) {
        Some(statements) => statements,
        None => return Some(sql_violation("SQL could not be safely analyzed")),
    };
    for tokens in statements {
        let Some(first) = tokens.first().map(String::as_str) else {
            continue;
        };
        if !SQL_READ_START.contains(&first) {
            return Some(sql_violation(&format!(
                "\"{}\" is not allowed on a read-only connection",
                first.to_uppercase()
            )));
        }
        if first == "pragma" {
            let pragma = tokens.get(1).map(String::as_str).unwrap_or_default();
            if tokens.iter().any(|token| token == "=") || !SQLITE_READ_PRAGMAS.contains(&pragma) {
                return Some(sql_violation(
                    "PRAGMA assignment is not allowed on a read-only connection",
                ));
            }
            continue;
        }
        if first == "show" || first == "describe" || first == "desc" {
            continue;
        }
        if first == "explain" && tokens.iter().any(|token| token == "analyze") {
            return Some(sql_violation(
                "EXPLAIN ANALYZE is not allowed on a read-only connection",
            ));
        }
        if let Some(word) = tokens
            .iter()
            .find(|token| SQL_WRITES.contains(&token.as_str()))
        {
            return Some(sql_violation(&format!(
                "statement contains \"{}\" — not allowed on a read-only connection",
                word.to_uppercase()
            )));
        }
        for (index, token) in tokens.iter().enumerate() {
            if token == ":=" {
                return Some(sql_violation(
                    "session-variable assignment is not allowed on a read-only connection",
                ));
            }
            if token.starts_with('@') && tokens.get(index + 1).is_some_and(|next| next == "=") {
                return Some(sql_violation(
                    "session-variable assignment is not allowed on a read-only connection",
                ));
            }
            if tokens.get(index + 1).is_some_and(|next| next == "(")
                && !PURE_FUNCTIONS.contains(&token.as_str())
                && !["in", "exists", "over", "filter"].contains(&token.as_str())
            {
                return Some(sql_violation(&format!(
                    "function \"{}\" is not approved for a read-only connection",
                    token.to_uppercase()
                )));
            }
        }
    }
    None
}

fn sql_violation(reason: &str) -> String {
    format!(
        "read_only_connection: {reason} — enable Read & write on this connection in the Database panel"
    )
}

/// Tokenizes SQL without interpreting comments or quoted text as executable syntax.
/// A `None` result means malformed quoting/comment nesting and is denied.
fn tokenize_sql(sql: &str) -> Option<Vec<Vec<String>>> {
    let bytes = sql.as_bytes();
    let mut statements = Vec::new();
    let mut tokens = Vec::new();
    let mut at = 0;
    let mut depth = 0usize;
    while let Some(byte) = bytes.get(at).copied() {
        match byte {
            b' ' | b'\t' | b'\r' | b'\n' => at += 1,
            b'-' if bytes.get(at + 1) == Some(&b'-') => {
                at += 2;
                while bytes.get(at).is_some_and(|b| *b != b'\n') {
                    at += 1;
                }
            }
            b'/' if bytes.get(at + 1) == Some(&b'*') => {
                at += 2;
                let mut closed = false;
                while at < bytes.len() {
                    if bytes.get(at) == Some(&b'*') && bytes.get(at + 1) == Some(&b'/') {
                        at += 2;
                        closed = true;
                        break;
                    }
                    at += 1;
                }
                if !closed {
                    return None;
                }
            }
            b'\'' | b'"' | b'`' | b'[' => {
                let close = if byte == b'[' { b']' } else { byte };
                let mut closed = false;
                at += 1;
                while let Some(next) = bytes.get(at).copied() {
                    if next == close {
                        if bytes.get(at + 1) == Some(&close) {
                            at += 2;
                            continue;
                        }
                        at += 1;
                        closed = true;
                        break;
                    }
                    at += 1;
                }
                if !closed {
                    return None;
                }
                tokens.push("<quoted>".to_string());
            }
            b';' if depth == 0 => {
                if !tokens.is_empty() {
                    statements.push(std::mem::take(&mut tokens));
                }
                at += 1;
            }
            b'(' => {
                depth += 1;
                tokens.push("(".to_string());
                at += 1;
            }
            b')' => {
                depth = depth.checked_sub(1)?;
                tokens.push(")".to_string());
                at += 1;
            }
            b'=' => {
                tokens.push("=".to_string());
                at += 1;
            }
            b':' if bytes.get(at + 1) == Some(&b'=') => {
                tokens.push(":=".to_string());
                at += 2;
            }
            b if b.is_ascii_alphanumeric() || b == b'_' || b == b'$' || b == b'@' => {
                let start = at;
                while bytes.get(at).is_some_and(|next| {
                    next.is_ascii_alphanumeric() || *next == b'_' || *next == b'$' || *next == b'@'
                }) {
                    at += 1;
                }
                let word = std::str::from_utf8(bytes.get(start..at)?).ok()?;
                tokens.push(word.to_ascii_lowercase());
            }
            _ => at += 1,
        }
    }
    if depth != 0 {
        return None;
    }
    if !tokens.is_empty() {
        statements.push(tokens);
    }
    Some(statements)
}

/// Returns an error for Redis commands not in the safe read whitelist.
pub fn redis_read_violation(parts: &[String]) -> Option<String> {
    let Some(command) = parts.first().map(|part| part.to_ascii_lowercase()) else {
        return Some(
            "read_only_connection: an empty command is not allowed on a read-only connection — enable Read & write on this connection in the Database panel".to_string(),
        );
    };
    let safe = [
        "get",
        "mget",
        "strlen",
        "getrange",
        "exists",
        "type",
        "ttl",
        "pttl",
        "keys",
        "scan",
        "randomkey",
        "dbsize",
        "memory",
        "object",
        "dump",
        "hget",
        "hmget",
        "hgetall",
        "hkeys",
        "hvals",
        "hlen",
        "hexists",
        "hscan",
        "hstrlen",
        "hrandfield",
        "lrange",
        "llen",
        "lindex",
        "lpos",
        "smembers",
        "sismember",
        "smismember",
        "scard",
        "srandmember",
        "sscan",
        "sinter",
        "sunion",
        "sdiff",
        "sintercard",
        "zrange",
        "zrangebyscore",
        "zrangebylex",
        "zrevrange",
        "zcard",
        "zcount",
        "zscore",
        "zmscore",
        "zrank",
        "zrevrank",
        "zscan",
        "zrandmember",
        "zlexcount",
        "xrange",
        "xrevrange",
        "xlen",
        "xread",
        "xinfo",
        "bitcount",
        "bitpos",
        "getbit",
        "pfcount",
        "ping",
        "echo",
        "info",
        "time",
        "command",
        "client",
        "config",
    ];
    let safe_subcommands = [
        "get", "list", "info", "getname", "usage", "stats", "doctor", "help",
    ];
    let subcommand = parts.get(1).map(|part| part.to_ascii_lowercase());
    let command_safe = safe.contains(&command.as_str());
    let subcommand_safe = !["config", "client", "memory"].contains(&command.as_str())
        || subcommand
            .as_deref()
            .is_none_or(|sub| safe_subcommands.contains(&sub));
    if command_safe && subcommand_safe {
        return None;
    }
    let label = match (parts.first(), parts.get(1)) {
        (Some(command), Some(sub)) if command_safe => {
            format!("{} {}", command.to_uppercase(), sub.to_uppercase())
        }
        (Some(command), _) => command.to_uppercase(),
        _ => return None,
    };
    Some(format!(
        "read_only_connection: \"{label}\" is not allowed on a read-only connection — enable Read & write on this connection in the Database panel"
    ))
}

/// Mongo command name is its first ordered key. Aggregation stages are checked recursively.
pub fn mongo_read_violation(command: &[(String, Value)]) -> Option<String> {
    let Some((name, value)) = command.first() else {
        return Some(
            "read_only_connection: an empty command is not allowed on a read-only connection — enable Read & write on this connection in the Database panel".to_string(),
        );
    };
    let normalized = name.to_ascii_lowercase();
    let safe = [
        "find",
        "aggregate",
        "count",
        "distinct",
        "listcollections",
        "listindexes",
        "listdatabases",
        "collstats",
        "dbstats",
        "explain",
        "buildinfo",
        "hello",
        "ismaster",
        "ping",
        "serverstatus",
    ];
    if !safe.contains(&normalized.as_str()) {
        return Some(format!(
            "read_only_connection: \"{name}\" is not allowed on a read-only connection — enable Read & write on this connection in the Database panel"
        ));
    }
    if normalized == "aggregate" && mongo_pipeline_writes(value) {
        return Some(
            "read_only_connection: aggregate with $out/$merge writes to a collection — not allowed on a read-only connection".to_string(),
        );
    }
    None
}

fn mongo_pipeline_writes(value: &Value) -> bool {
    match value {
        Value::Array(items) => items.iter().any(mongo_pipeline_writes),
        Value::Object(map) => map.iter().any(|(key, value)| {
            matches!(key.as_str(), "$out" | "$merge") || mongo_pipeline_writes(value)
        }),
        _ => false,
    }
}

/// Mongo commands that must target the admin database.
pub fn mongo_forced_database(command: &[(String, Value)]) -> Option<&'static str> {
    command
        .first()
        .filter(|(name, _)| name.eq_ignore_ascii_case("listDatabases"))
        .map(|_| "admin")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_sql_write_tricks_and_malformed_input() {
        for sql in [
            "WITH gone AS (DELETE FROM users RETURNING *) SELECT * FROM gone",
            "SELECT 1; DROP TABLE users",
            "SELECT 1 /* comment */; -- hidden\n DELETE FROM users",
            "EXPLAIN ANALYZE DELETE FROM users",
            "SELECT id INTO backup FROM users",
            "SELECT nextval('seq')",
            "SELECT dangerous_function()",
            "SELECT @session_value := 1",
            "SELECT @session_value = 1",
            "SELECT 1 /*!40101 ; DROP TABLE users */",
            "SELECT /*+ READ_CONSISTENCY(WEAK) */ 1",
        ] {
            assert!(sql_read_violation(sql).is_some(), "accepted: {sql}");
        }
        assert!(sql_read_violation("SELECT 'DELETE; -- harmless'").is_none());
        assert!(sql_read_violation("SELECT COUNT(*) FROM users").is_none());
    }

    #[test]
    fn redis_and_mongo_are_whitelist_classified() {
        assert!(redis_read_violation(&["GET".into(), "k".into()]).is_none());
        assert!(redis_read_violation(&["CONFIG".into(), "SET".into()]).is_some());
        assert!(redis_read_violation(&["FLUSHALL".into()]).is_some());
        assert!(redis_read_violation(&[]).is_some());
        assert!(mongo_read_violation(&[]).is_some());
        let list = vec![("listDatabases".to_string(), Value::from(1))];
        assert_eq!(mongo_forced_database(&list), Some("admin"));
        assert!(mongo_read_violation(&list).is_none());
        let write = vec![("delete".to_string(), Value::from("users"))];
        assert!(mongo_read_violation(&write).is_some());
        let aggregate = vec![(
            "aggregate".to_string(),
            serde_json::json!({"pipeline":[{"$out":"archive"}]}),
        )];
        assert!(mongo_read_violation(&aggregate).is_some());
    }
}
