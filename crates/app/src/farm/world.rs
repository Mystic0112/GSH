//! Modelo estático sem dependência da interface.
//! A fazenda vem do layout em `backdrop.rs`; aqui fica só o escritório.
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
    Wall,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Prop {
    pub kind: Kind,
    pub x: i32,
    pub y: i32,
}

pub fn office_props() -> Vec<Prop> {
    let mut items = vec![Prop {
        kind: Kind::Wall,
        x: -165,
        y: -100,
    }];
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
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn office_has_nine_desks() {
        let count = office_props()
            .iter()
            .filter(|p| p.kind == Kind::Desk)
            .count();
        assert_eq!(count, 9);
    }
}
