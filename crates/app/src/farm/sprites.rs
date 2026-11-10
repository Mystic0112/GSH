// Portado de clodfarm (MIT), via farm_sprites_crops.dart no Flutter.
//! Sprites estáticos derivados de `_bezel`, `monitorOff` e `_crop` do Flutter.
// As letras indexam a paleta em theme.rs; ponto é transparente.
pub const MONITOR: &[&str] = &[
    "bbbbbbbbbbbbbb",
    "bssssssssssssb",
    "bssssssssrrssb",
    "bssssssssssrsb",
    "bssssssssssssb",
    "bssssssssssssb",
    "bssssssssssssb",
    "bbbbbbbbbbbbab",
    "......mm......",
    "....nnnnnn....",
];
#[cfg(test)]
pub const ALL: &[&[&str]] = &[MONITOR];
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sprite_rows_are_rectangular() {
        for sprite in ALL {
            let width = sprite.first().map_or(0, |row| row.len());
            assert!(width > 0 && sprite.iter().all(|row| row.len() == width));
        }
    }
}
