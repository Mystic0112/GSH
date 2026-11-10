//! Modelo estático sem dependência da interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Office,
    Farm,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Plot(u8),
    Desk,
    Plant,
    Path,
    Wall,
    Barn,
    Pond,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Prop {
    pub kind: Kind,
    pub x: i32,
    pub y: i32,
}

pub fn props(mode: Mode) -> Vec<Prop> {
    let mut items = Vec::new();
    match mode {
        Mode::Farm => {
            items.extend([
                Prop {
                    kind: Kind::Barn,
                    x: -135,
                    y: -60,
                },
                Prop {
                    kind: Kind::Pond,
                    x: -130,
                    y: 65,
                },
                Prop {
                    kind: Kind::Path,
                    x: -150,
                    y: 5,
                },
            ]);
            for row in 0..3 {
                for col in 0..3 {
                    items.push(Prop {
                        kind: Kind::Plot(((row * 3 + col) % 3) as u8),
                        x: -15 + col * 60,
                        y: -55 + row * 46,
                    });
                }
            }
        }
        Mode::Office => {
            items.push(Prop {
                kind: Kind::Wall,
                x: -165,
                y: -100,
            });
            for row in 0..3 {
                for col in 0..3 {
                    items.push(Prop {
                        kind: Kind::Desk,
                        x: -15 + col * 60,
                        y: -55 + row * 46,
                    });
                }
            }
            for y in [-65, 0, 65] {
                items.push(Prop {
                    kind: Kind::Plant,
                    x: -130,
                    y,
                });
            }
        }
    }
    items
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn worlds_have_nine_grid_positions() {
        for mode in [Mode::Farm, Mode::Office] {
            let count = props(mode)
                .iter()
                .filter(|p| matches!(p.kind, Kind::Plot(_) | Kind::Desk))
                .count();
            assert_eq!(count, 9);
        }
    }
}
