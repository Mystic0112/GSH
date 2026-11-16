//! Resultado normalizado de uma consulta (`db_result.dart`): o que a grade e o JSON conhecem.

use std::time::Duration;

use serde::ser::{Serialize, SerializeMap, Serializer};

/// Valor de uma célula, já no conjunto normalizado.
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    Null,
    Int(i64),
    Real(f64),
    Bool(bool),
    Text(String),
    Blob(Vec<u8>),
}

impl Cell {
    /// Texto da grade (`_cellText`): `NULL`, `blob N B` ou o valor como o Dart o imprime.
    pub fn display(&self) -> String {
        match self {
            Self::Null => "NULL".to_string(),
            Self::Int(v) => v.to_string(),
            Self::Real(v) => dart_double(*v),
            Self::Bool(v) => v.to_string(),
            Self::Text(v) => v.clone(),
            Self::Blob(v) => format!("blob {} B", v.len()),
        }
    }

    /// Números alinham à direita na grade.
    pub fn is_number(&self) -> bool {
        matches!(self, Self::Int(_) | Self::Real(_))
    }

    /// Tipo inferido pelo valor (`_inferType`); vazio para blob.
    fn type_name(&self) -> Option<&'static str> {
        match self {
            Self::Null => None,
            Self::Int(_) => Some("INTEGER"),
            Self::Real(_) => Some("REAL"),
            Self::Bool(_) => Some("BOOL"),
            Self::Text(_) => Some("TEXT"),
            Self::Blob(_) => Some(""),
        }
    }
}

impl Serialize for Cell {
    /// Célula como JSON da aba (`_jsonCell`): blob vira `<blob N bytes>`.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Null => serializer.serialize_none(),
            Self::Int(v) => serializer.serialize_i64(*v),
            Self::Real(v) => serializer.serialize_f64(*v),
            Self::Bool(v) => serializer.serialize_bool(*v),
            Self::Text(v) => serializer.serialize_str(v),
            Self::Blob(v) => serializer.serialize_str(&format!("<blob {} bytes>", v.len())),
        }
    }
}

/// `double.toString()` do Dart: `1.0`, `1.5`, `1e+21`, `1e-7`.
pub fn dart_double(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_string();
    }
    if value.is_infinite() {
        return if value > 0.0 { "Infinity" } else { "-Infinity" }.to_string();
    }
    let magnitude = value.abs();
    if magnitude != 0.0 && !(1e-6..1e21).contains(&magnitude) {
        // `{:e}` dá `1.5e21`/`1e-7`; o Dart escreve o sinal do expoente e a mantissa com `.0`
        // só quando há fração.
        let text = format!("{value:e}");
        return match text.split_once('e') {
            Some((mantissa, exponent)) => {
                let sign = if exponent.starts_with('-') { "" } else { "+" };
                format!("{mantissa}e{sign}{exponent}")
            }
            None => text,
        };
    }
    let text = value.to_string();
    if text.contains('.') {
        text
    } else {
        format!("{text}.0")
    }
}

/// Coluna: nome e tipo inferido (texto livre, só para exibição).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    pub name: String,
    pub ty: String,
}

/// Resultado de uma execução.
#[derive(Debug, Clone, PartialEq)]
pub struct QueryResult {
    pub columns: Vec<Column>,
    pub rows: Vec<Vec<Cell>>,
    pub elapsed: Duration,
    /// O limite cortou o cursor; nunca truncar em silêncio.
    pub truncated: bool,
    /// Só em DML (`execute`).
    pub affected_rows: Option<u64>,
}

impl QueryResult {
    /// Sem linhas: o Flutter também descarta as colunas (a grade mostra "Nenhuma linha").
    pub fn empty(elapsed: Duration) -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            elapsed,
            truncated: false,
            affected_rows: None,
        }
    }

    /// Resultado de DML com as linhas afetadas.
    pub fn affected(count: u64, elapsed: Duration) -> Self {
        Self {
            affected_rows: Some(count),
            ..Self::empty(elapsed)
        }
    }

    /// Monta o resultado a partir de colunas e linhas já cortadas no `limit`.
    pub fn from_rows(
        names: Vec<String>,
        rows: Vec<Vec<Cell>>,
        truncated: bool,
        elapsed: Duration,
    ) -> Self {
        if rows.is_empty() {
            return Self::empty(elapsed);
        }
        let columns = names
            .into_iter()
            .enumerate()
            .map(|(index, name)| Column {
                name,
                ty: infer_type(&rows, index),
            })
            .collect();
        Self {
            columns,
            rows,
            elapsed,
            truncated,
            affected_rows: None,
        }
    }

    /// Linhas como lista de objetos coluna→valor, com recuo de 2 espaços (`JsonEncoder.withIndent`).
    pub fn to_json(&self, max_rows: Option<usize>) -> String {
        let shown = max_rows.map_or(self.rows.len(), |max| max.min(self.rows.len()));
        let rows: Vec<RowJson<'_>> = self
            .rows
            .iter()
            .take(shown)
            .map(|cells| RowJson {
                names: &self.columns,
                cells,
            })
            .collect();
        serde_json::to_string_pretty(&rows).unwrap_or_default()
    }
}

fn infer_type(rows: &[Vec<Cell>], column: usize) -> String {
    rows.iter()
        .filter_map(|row| row.get(column))
        .find_map(Cell::type_name)
        .unwrap_or_default()
        .to_string()
}

struct RowJson<'a> {
    names: &'a [Column],
    cells: &'a [Cell],
}

impl Serialize for RowJson<'_> {
    /// Nome repetido guarda a posição da primeira e o valor da última, como o mapa do Dart.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut entries: Vec<(&str, &Cell)> = Vec::new();
        for (column, cell) in self.names.iter().zip(self.cells) {
            match entries.iter_mut().find(|(name, _)| *name == column.name) {
                Some(slot) => slot.1 = cell,
                None => entries.push((&column.name, cell)),
            }
        }
        let mut map = serializer.serialize_map(Some(entries.len()))?;
        for (name, cell) in entries {
            map.serialize_entry(name, cell)?;
        }
        map.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doubles_print_like_dart() {
        assert_eq!(dart_double(1.0), "1.0");
        assert_eq!(dart_double(-2.5), "-2.5");
        assert_eq!(dart_double(0.0), "0.0");
        assert_eq!(dart_double(1e21), "1e+21");
        assert_eq!(dart_double(1.5e-7), "1.5e-7");
        assert_eq!(dart_double(123456789012.0), "123456789012.0");
        assert_eq!(dart_double(f64::NAN), "NaN");
    }

    #[test]
    fn empty_rows_drop_the_columns() {
        let result = QueryResult::from_rows(vec!["a".into()], Vec::new(), false, Duration::ZERO);
        assert!(result.columns.is_empty());
    }

    #[test]
    fn types_come_from_the_first_non_null_value() {
        let rows = vec![
            vec![Cell::Null, Cell::Text("x".into()), Cell::Blob(vec![1])],
            vec![Cell::Int(1), Cell::Null, Cell::Null],
        ];
        let result = QueryResult::from_rows(
            vec!["a".into(), "b".into(), "c".into()],
            rows,
            false,
            Duration::ZERO,
        );
        let types: Vec<_> = result.columns.iter().map(|c| c.ty.as_str()).collect();
        assert_eq!(types, ["INTEGER", "TEXT", ""]);
    }

    #[test]
    fn json_view_matches_the_dart_encoder() {
        let rows = vec![vec![
            Cell::Int(1),
            Cell::Text("a\"b".into()),
            Cell::Blob(vec![0, 1]),
        ]];
        let result = QueryResult::from_rows(
            vec!["id".into(), "name".into(), "data".into()],
            rows,
            false,
            Duration::ZERO,
        );
        assert_eq!(
            result.to_json(None),
            "[\n  {\n    \"id\": 1,\n    \"name\": \"a\\\"b\",\n    \"data\": \"<blob 2 bytes>\"\n  }\n]"
        );
    }
}
