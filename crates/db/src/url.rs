//! Leitura mínima de URIs como o `Uri.parse` do Dart, o bastante para as URLs de conexão.
//!
//! Não valida nada além do que o Flutter valida: porta não numérica falha (é o que dispara a
//! re-codificação da senha em `normalize`); o resto é aceito como veio.

/// Partes de uma URL de conexão.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParsedUrl {
    pub scheme: String,
    /// `user:senha`, ainda codificado; vazio sem `@`.
    pub user_info: String,
    pub host: String,
    pub port: Option<u16>,
    pub path: String,
    pub query: String,
}

/// A URL não tem a forma `scheme://...` ou a porta não é um número.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UrlError;

/// Quebra `text` em partes.
pub fn parse(text: &str) -> Result<ParsedUrl, UrlError> {
    let (without_fragment, _) = text.split_once('#').unwrap_or((text, ""));
    let (before_query, query) = without_fragment
        .split_once('?')
        .unwrap_or((without_fragment, ""));
    let mut url = ParsedUrl {
        query: query.to_string(),
        ..ParsedUrl::default()
    };
    let rest = match before_query.split_once(':') {
        Some((scheme, rest)) if is_scheme(scheme) => {
            url.scheme = scheme.to_ascii_lowercase();
            rest
        }
        _ => before_query,
    };
    let Some(after_slashes) = rest.strip_prefix("//") else {
        url.path = rest.to_string();
        return Ok(url);
    };
    let (authority, path) = match after_slashes.find('/') {
        Some(at) => after_slashes.split_at(at),
        None => (after_slashes, ""),
    };
    url.path = path.to_string();
    let host_port = match authority.rsplit_once('@') {
        Some((info, host_port)) => {
            url.user_info = info.to_string();
            host_port
        }
        None => authority,
    };
    let (host, port) = split_host_port(host_port)?;
    url.host = host.to_ascii_lowercase();
    url.port = port;
    Ok(url)
}

fn is_scheme(text: &str) -> bool {
    let mut chars = text.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}

fn split_host_port(text: &str) -> Result<(&str, Option<u16>), UrlError> {
    let (host, port) = if let Some(inner) = text.strip_prefix('[') {
        match inner.split_once(']') {
            Some((host, after)) => (host, after.strip_prefix(':')),
            None => return Err(UrlError),
        }
    } else {
        match text.rsplit_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (text, None),
        }
    };
    match port {
        None | Some("") => Ok((host, None)),
        Some(digits) if digits.bytes().all(|b| b.is_ascii_digit()) => digits
            .parse::<u16>()
            .map(|port| (host, Some(port)))
            .map_err(|_| UrlError),
        Some(_) => Err(UrlError),
    }
}

/// `Uri.encodeComponent`: tudo vira `%XX` (UTF-8, maiúsculas) menos `A-Za-z0-9-_.!~*'()`.
pub fn encode_component(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&byte) {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// `Uri.decodeComponent`; `%` mal formado fica como veio.
pub fn decode_component(text: &str) -> String {
    String::from_utf8_lossy(&decode_bytes(text, false)).into_owned()
}

fn decode_bytes(text: &str, plus_is_space: bool) -> Vec<u8> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while let Some(&byte) = bytes.get(at) {
        let escaped = (byte == b'%')
            .then(|| {
                let hi = bytes.get(at + 1).copied().and_then(hex_value)?;
                let lo = bytes.get(at + 2).copied().and_then(hex_value)?;
                Some(hi * 16 + lo)
            })
            .flatten();
        match escaped {
            Some(value) => {
                out.push(value);
                at += 3;
            }
            None if plus_is_space && byte == b'+' => {
                out.push(b' ');
                at += 1;
            }
            None => {
                out.push(byte);
                at += 1;
            }
        }
    }
    out
}

fn hex_value(byte: u8) -> Option<u8> {
    char::from(byte)
        .to_digit(16)
        .and_then(|digit| u8::try_from(digit).ok())
}

/// Pares da query (`Uri.queryParameters`): `+` vira espaço e `%XX` é decodificado.
pub fn query_params(query: &str) -> Vec<(String, String)> {
    query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            (
                String::from_utf8_lossy(&decode_bytes(key, true)).into_owned(),
                String::from_utf8_lossy(&decode_bytes(value, true)).into_owned(),
            )
        })
        .collect()
}

/// Torna parseável uma URL editada à mão com a senha sem codificar (`user:8nJM9g8%?FC(@host`).
///
/// Se o parse estrito falha, recodifica o `userinfo` (até o último `@` da autoridade) e
/// revalida; ainda inválida, devolve o erro.
pub fn normalize(url: &str) -> Result<String, UrlError> {
    if parse(url).is_ok() {
        return Ok(url.to_string());
    }
    let separator = url.find("://").ok_or(UrlError)?;
    let start = separator + 3;
    let tail = url.get(start..).ok_or(UrlError)?;
    // Sem `?` na senha a autoridade acaba na primeira `/`; com ela, o último `@` antes da `/`.
    let path_at = tail.find('/');
    let authority = match path_at {
        Some(at) => tail.get(..at).ok_or(UrlError)?,
        None => tail,
    };
    let at = authority.rfind('@').ok_or(UrlError)?;
    let user_info = authority.get(..at).ok_or(UrlError)?;
    let encoded = match user_info.split_once(':') {
        Some((user, pass)) => format!("{}:{}", encode_component(user), encode_component(pass)),
        None => encode_component(user_info),
    };
    let rest = tail.get(at..).ok_or(UrlError)?.to_string();
    let rebuilt = format!("{}{encoded}{rest}", url.get(..start).ok_or(UrlError)?);
    parse(&rebuilt)?;
    Ok(rebuilt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_a_postgres_url() {
        let url = parse("postgres://ana:s%40@Db.Local:5433/app?sslmode=require").unwrap();
        assert_eq!(url.scheme, "postgres");
        assert_eq!(url.user_info, "ana:s%40");
        assert_eq!(url.host, "db.local");
        assert_eq!(url.port, Some(5433));
        assert_eq!(url.path, "/app");
        assert_eq!(url.query, "sslmode=require");
    }

    #[test]
    fn non_numeric_port_fails_like_dart() {
        assert!(parse("postgres://user:8nJM9g8%?FC(@host/db").is_err());
    }

    #[test]
    fn normalize_reencodes_a_raw_password() {
        // a `?` da senha encerra a autoridade no parse estrito; a recodificação resolve
        let raw = "postgres://user:p%?w(@host:5432/db";
        let fixed = normalize(raw).unwrap();
        assert_eq!(fixed, "postgres://user:p%25%3Fw(@host:5432/db");
        let url = parse(&fixed).unwrap();
        assert_eq!(decode_component(&url.user_info), "user:p%?w(");
    }

    #[test]
    fn component_codec_round_trips() {
        let text = "a b/ç?&=";
        assert_eq!(decode_component(&encode_component(text)), text);
        assert_eq!(encode_component("a b"), "a%20b");
        assert_eq!(encode_component("(x)!"), "(x)!");
    }

    #[test]
    fn query_params_decode_plus_and_percent() {
        assert_eq!(
            query_params("a=1&b=x+y&c=%C3%A7"),
            vec![
                ("a".into(), "1".into()),
                ("b".into(), "x y".into()),
                ("c".into(), "ç".into())
            ]
        );
    }
}
