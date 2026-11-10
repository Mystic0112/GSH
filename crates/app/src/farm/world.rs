//! Modelo estático sem dependência da interface.
use super::crops::CropStage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Office,
    Farm,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Plot(CropStage),
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

/// `farm_world.dart`: field origin + (19 + crowLane, 15), then 60 × 46.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Plot {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

pub fn plots(field_x: i32, field_y: i32, columns: i32, rows: i32) -> Vec<Plot> {
    (0..rows)
        .flat_map(|row| {
            (0..columns).map(move |col| Plot {
                x: field_x + 41 + col * 60,
                y: field_y + 15 + row * 46,
                width: 30,
                height: 16,
            })
        })
        .collect()
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
            for (i, plot) in plots(-56, -70, 3, 3).into_iter().enumerate() {
                let stage = match i % 3 {
                    0 => CropStage::Seed,
                    1 => CropStage::Sprout,
                    _ => CropStage::Grow,
                };
                items.push(Prop {
                    kind: Kind::Plot(stage),
                    x: plot.x,
                    y: plot.y,
                });
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
    fn dart_plot_positions() {
        let plots = plots(100, 200, 3, 3);
        assert_eq!(
            plots.first(),
            Some(&Plot {
                x: 141,
                y: 215,
                width: 30,
                height: 16
            })
        );
        assert_eq!(
            plots.last(),
            Some(&Plot {
                x: 261,
                y: 307,
                width: 30,
                height: 16
            })
        );
    }
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
