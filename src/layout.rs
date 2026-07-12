#[derive(Copy, Clone)]
pub struct Coord {
    pub x: f32,
    pub y: f32,
    pub z: f32
}

pub trait Layout {
    fn get_coord(&mut self, index: usize) -> Coord;
}

pub struct Linear {}

impl Layout for Linear {
    fn get_coord(&mut self, index: usize) -> Coord {
        Coord {
            x: index as f32,
            y: 0.0,
            z: 0.0
        }
    }
}
