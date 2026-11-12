//! Modelo estático sem dependência da interface.
//! O desenho vem de `backdrop.rs`; aqui ficam o modo e os postos de cultivo.
use super::crops::CropStage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Office,
    Farm,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Plot(CropStage),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Prop {
    pub kind: Kind,
    pub x: i32,
    pub y: i32,
}
