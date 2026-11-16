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
    fn hhmm_local_tem_formato_valido() {
        let s = hhmm(1_767_236_700_000);
        assert_eq!(s.len(), 5);
        assert_eq!(s.get(2..3), Some(":"));
    }
}
