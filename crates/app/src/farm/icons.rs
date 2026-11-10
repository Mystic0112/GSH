// Ported from clodfarm (MIT), via farm_sprites_icons.dart.
mod data_1;
mod data_2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FarmIcon {
    Terminal,
    Zzz,
    Pause,
    Alert,
    Dots,
    Scroll,
    Quill,
    Globe,
    Gear,
    Tasks,
    Heart,
    Flame,
    Envelope,
    Parcel,
}

struct IconData {
    rows: &'static [&'static str],
    palette: &'static [(u8, u32)],
}

impl FarmIcon {
    fn data(self) -> &'static IconData {
        match self {
            Self::Terminal => &data_1::TERMINAL,
            Self::Zzz => &data_1::ZZZ,
            Self::Pause => &data_1::PAUSE,
            Self::Alert => &data_1::ALERT,
            Self::Dots => &data_1::DOTS,
            Self::Scroll => &data_1::SCROLL,
            Self::Quill => &data_1::QUILL,
            Self::Globe => &data_2::GLOBE,
            Self::Gear => &data_2::GEAR,
            Self::Tasks => &data_2::TASKS,
            Self::Heart => &data_2::HEART,
            Self::Flame => &data_2::FLAME,
            Self::Envelope => &data_2::ENVELOPE,
            Self::Parcel => &data_2::PARCEL,
        }
    }

    /// RGBA pixels in row-major order, one output pixel per Dart sprite pixel.
    pub fn rgba(self) -> [u8; 10 * 10 * 4] {
        let data = self.data();
        let mut rgba = [0; 10 * 10 * 4];
        for (ch, pixel) in data
            .rows
            .iter()
            .flat_map(|row| row.bytes())
            .zip(rgba.as_chunks_mut::<4>().0.iter_mut())
        {
            if let Some((_, argb)) = data.palette.iter().find(|(key, _)| *key == ch) {
                let [a, r, g, b] = argb.to_be_bytes();
                pixel.copy_from_slice(&[r, g, b, a]);
            }
        }
        rgba
    }
}

#[cfg(test)]
mod tests {
    use super::FarmIcon;

    const ALL: [FarmIcon; 14] = [
        FarmIcon::Terminal,
        FarmIcon::Zzz,
        FarmIcon::Pause,
        FarmIcon::Alert,
        FarmIcon::Dots,
        FarmIcon::Scroll,
        FarmIcon::Quill,
        FarmIcon::Globe,
        FarmIcon::Gear,
        FarmIcon::Tasks,
        FarmIcon::Heart,
        FarmIcon::Flame,
        FarmIcon::Envelope,
        FarmIcon::Parcel,
    ];

    #[test]
    fn grids_are_rectangular_and_characters_are_mapped() {
        for icon in ALL {
            let data = icon.data();
            assert_eq!(data.rows.len(), 10, "{icon:?}");
            for row in data.rows {
                assert_eq!(row.len(), 10, "{icon:?}");
                for ch in row.bytes() {
                    assert!(
                        ch == b'.' || data.palette.iter().any(|(key, _)| *key == ch),
                        "{icon:?}: {ch}"
                    );
                }
            }
        }
    }

    #[test]
    fn raster_matches_dart_pixel_counts_and_rgba_order() {
        // Counts independently obtained from the Dart grids ('.' is transparent).
        for (icon, count) in [(FarmIcon::Terminal, 90), (FarmIcon::Heart, 46)] {
            assert_eq!(
                icon.rgba()
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .filter(|pixel| pixel[3] != 0)
                    .count(),
                count
            );
        }
        assert_eq!(&FarmIcon::Terminal.rgba()[..4], &[0x1b, 0x1f, 0x2a, 0xff]);
        assert_eq!(&FarmIcon::Heart.rgba()[..4], &[0, 0, 0, 0]);
    }
}
