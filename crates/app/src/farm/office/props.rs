//! Tipos e probabilidades de `_officeProps` no Dart.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Plant,
    Shelf,
    Pouf,
    Lamp,
    Bin,
    Cooler,
    Sofa,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Prop {
    pub kind: Kind,
    pub x: f64,
    pub y: f64,
    pub radius: f64,
    pub variant: u8,
}

pub const KINDS: [(Kind, f64, f64); 7] = [
    (Kind::Plant, 0.36, 6.0),
    (Kind::Shelf, 0.18, 7.0),
    (Kind::Pouf, 0.14, 5.0),
    (Kind::Lamp, 0.12, 3.0),
    (Kind::Bin, 0.08, 3.0),
    (Kind::Cooler, 0.06, 4.0),
    (Kind::Sofa, 0.06, 10.0),
];
