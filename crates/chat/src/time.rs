//! Instantes do transcript e do `get_usage` (`DateTime.tryParse` do Dart): RFC 3339 -> epoch ms.
//!
//! Só aritmética de calendário, sem fuso local nem relógio: quem formata na hora local é a UI.

/// Dias desde 1970-01-01 de uma data civil (algoritmo `days_from_civil` de Howard Hinnant).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// `2026-10-25T07:30:00.337736+00:00`, `...Z` ou com `-03:00`: epoch em milissegundos (UTC).
/// `None` se não for uma data nesse formato. Sem fuso, vale UTC (o Dart assumiria local; o
/// Claude Code sempre manda com fuso).
pub fn parse_rfc3339_ms(text: &str) -> Option<u64> {
    let s = text.trim();
    let (date, rest) = s.split_once(['T', ' '])?;
    let mut d = date.split('-');
    let year: i64 = d.next()?.parse().ok()?;
    let month: i64 = d.next()?.parse().ok()?;
    let day: i64 = d.next()?.parse().ok()?;
    if d.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let tz_at = rest.find(['Z', 'z', '+', '-']);
    let (clock, tz) = match tz_at {
        Some(i) => (rest.get(..i)?, rest.get(i..)?),
        None => (rest, ""),
    };
    let (hms, frac) = clock.split_once('.').unwrap_or((clock, ""));
    let mut t = hms.split(':');
    let hour: i64 = t.next()?.parse().ok()?;
    let minute: i64 = t.next()?.parse().ok()?;
    let second: i64 = t.next().unwrap_or("0").parse().ok()?;
    if t.next().is_some() || hour > 23 || minute > 59 || second > 60 {
        return None;
    }
    let millis: i64 = {
        let digits: String = frac.chars().take(3).collect();
        if digits.is_empty() {
            0
        } else {
            format!("{digits:0<3}").parse().ok()?
        }
    };
    let offset_secs: i64 = match tz.chars().next() {
        None | Some('Z' | 'z') => 0,
        Some(sign) => {
            let body = tz.get(1..)?;
            let (h, m) = body
                .split_once(':')
                .unwrap_or((body.get(..2)?, body.get(2..)?));
            let h: i64 = h.parse().ok()?;
            let m: i64 = if m.is_empty() { 0 } else { m.parse().ok()? };
            let total = h * 3600 + m * 60;
            if sign == '-' {
                -total
            } else {
                total
            }
        }
    };
    let secs = days_from_civil(year, month, day) * 86_400 + hour * 3600 + minute * 60 + second
        - offset_secs;
    u64::try_from(secs.checked_mul(1000)?.checked_add(millis)?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parseia_z_offset_e_fracao() {
        // 2026-01-01T03:05:00Z
        assert_eq!(
            parse_rfc3339_ms("2026-01-01T03:05:00Z"),
            Some(1_767_236_700_000)
        );
        assert_eq!(
            parse_rfc3339_ms("2026-01-01T00:05:00.250-03:00"),
            Some(1_767_236_700_250)
        );
        // do fixture do get_usage do Flutter: hora 7 em UTC.
        let ms = parse_rfc3339_ms("2026-10-25T07:30:00.337736+00:00").unwrap_or(0);
        assert_eq!(ms / 1000 % 86_400 / 3600, 7);
        assert_eq!(parse_rfc3339_ms("lixo"), None);
        assert_eq!(parse_rfc3339_ms(""), None);
    }
}
