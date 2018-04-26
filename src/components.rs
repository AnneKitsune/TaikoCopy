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

pub struct ManiaHitObject{
    // 512 / first data column
    lane: i32,
}

pub struct TaikoHitObject{
    red: bool,
    big: bool,
}
