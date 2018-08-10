extern crate amethyst;

use amethyst::ecs::prelude::*;

#[derive(Clone)]
pub struct HitObject {
    pub red: bool,
    pub time: f64,
    pub big: bool,
}

impl Component for HitObject {
    type Storage = VecStorage<HitObject>;
}
