//! Domínio puro do GSH: sem I/O, sem UI. Os tipos reais entram no card "[F1] Modelo de domínio".

/// Versão do crate, exposta para a CLI e para diagnósticos.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_not_empty() {
        assert!(!VERSION.is_empty());
    }
}
