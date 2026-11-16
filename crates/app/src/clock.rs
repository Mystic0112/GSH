//! Hora local `hh:mm` de um instante (`_hhmm` de `agent_file_marks.dart`, sobre `DateTime` local).
//!
//! A biblioteca padrão não conhece fuso. No Unix o deslocamento vem de `localtime_r`; onde
//! não há (Windows) cai em UTC, sem erro.

/// `hh:mm` na hora local de `epoch_ms`.
pub fn hhmm(epoch_ms: u64) -> String {
    let secs = i64::try_from(epoch_ms / 1000).unwrap_or(i64::MAX);
    hhmm_with_offset(secs, local_offset_secs(secs))
}

/// `hh:mm` de `epoch_secs` deslocado de `offset_secs` em relação a UTC.
fn hhmm_with_offset(epoch_secs: i64, offset_secs: i64) -> String {
    let day_secs = epoch_secs.saturating_add(offset_secs).rem_euclid(86_400);
    format!("{:02}:{:02}", day_secs / 3600, day_secs % 3600 / 60)
}

/// Data e hora local de um instante (`DateTime.toLocal()` do Dart), com o fuso de `hhmm`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalTime {
    pub year: i64,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
}

/// `epoch_ms` na hora local.
pub fn local_time(epoch_ms: u64) -> LocalTime {
    let secs = i64::try_from(epoch_ms / 1000).unwrap_or(i64::MAX);
    local_time_with_offset(secs, local_offset_secs(secs))
}

fn local_time_with_offset(epoch_secs: i64, offset_secs: i64) -> LocalTime {
    let t = epoch_secs.saturating_add(offset_secs);
    let days = t.div_euclid(86_400);
    let day_secs = t.rem_euclid(86_400);
    // `civil_from_days` de Howard Hinnant.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    LocalTime {
        year,
        month: u32::try_from(month).unwrap_or(1),
        day: u32::try_from(day).unwrap_or(1),
        hour: u32::try_from(day_secs / 3600).unwrap_or(0),
        minute: u32::try_from(day_secs % 3600 / 60).unwrap_or(0),
    }
}

// `time_t` e `tm_gmtoff` já são i64 em alvos de 64 bits, mas não nos de 32.
#[cfg(unix)]
#[allow(clippy::useless_conversion)]
fn local_offset_secs(epoch_secs: i64) -> i64 {
    let t = libc::time_t::try_from(epoch_secs).unwrap_or_default();
    // SAFETY: `tm` é um struct C de inteiros (e um ponteiro), válido zerado; `localtime_r`
    // só lê `t` e escreve em `tm`, ambos vivos e exclusivos nesta chamada, e é a variante
    // reentrante (não usa o buffer estático de `localtime`).
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    let filled = unsafe { libc::localtime_r(&t, &mut tm) };
    if filled.is_null() {
        0
    } else {
        i64::from(tm.tm_gmtoff)
    }
}

#[cfg(not(unix))]
fn local_offset_secs(_epoch_secs: i64) -> i64 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formata_com_zero_a_esquerda_e_aplica_o_fuso() {
        // 2026-01-01 03:05:00 UTC
        let t = 1_767_236_700;
        assert_eq!(hhmm_with_offset(t, 0), "03:05");
        assert_eq!(hhmm_with_offset(t, -3 * 3600), "00:05");
        assert_eq!(hhmm_with_offset(t, -4 * 3600), "23:05");
        assert_eq!(hhmm_with_offset(t, 5 * 3600 + 30 * 60), "08:35");
    }

    #[test]
    fn data_local_com_fuso_e_virada_de_dia() {
        // 2026-01-01 03:05:00 UTC
        let t = 1_767_236_700;
        let utc = local_time_with_offset(t, 0);
        assert_eq!(
            (utc.year, utc.month, utc.day, utc.hour, utc.minute),
            (2026, 1, 1, 3, 5)
        );
        // -4h cruza a meia-noite: 2025-12-31 23:05
        let w = local_time_with_offset(t, -4 * 3600);
        assert_eq!(
            (w.year, w.month, w.day, w.hour, w.minute),
            (2025, 12, 31, 23, 5)
        );
        // bissexto: 2024-02-29 12:00 UTC
        let l = local_time_with_offset(1_709_208_000, 0);
        assert_eq!((l.year, l.month, l.day), (2024, 2, 29));
    }

    #[test]
    fn hhmm_local_tem_formato_valido() {
        let s = hhmm(1_767_236_700_000);
        assert_eq!(s.len(), 5);
        assert_eq!(s.get(2..3), Some(":"));
    }
}
