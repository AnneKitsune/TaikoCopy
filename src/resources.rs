extern crate amethyst;

use amethyst::audio::SourceHandle;
use amethyst::core::timing::Stopwatch;
use amethyst::ecs::prelude::*;
use amethyst::renderer::Material;

use std::collections::VecDeque;

use components::*;

pub struct StopwatchWrapper {
    pub stopwatch: Stopwatch,
}

pub struct Sounds {
    pub normal: SourceHandle,
    pub clap: SourceHandle,
    pub finish: SourceHandle,
    pub whistle: SourceHandle,
}

#[derive(Clone, Default)]
pub struct BeatMap {
    pub name: String,
    pub songpath: String,
    pub objects: Vec<HitObject>,
    pub maxhitoffset: f64,
}

pub struct HitResultTextures {
    pub miss: Material,
    pub good: Material,
    pub perfect: Material,
}
impl Component for HitResultTextures {
    type Storage = VecStorage<HitResultTextures>;
}

pub enum HitResult {
    Perfect,
    Good,
    Miss,
}

impl Default for HitResult {
    fn default() -> Self {
        HitResult::Miss
    }
}

impl Component for BeatMap {
    type Storage = VecStorage<BeatMap>;
}

#[derive(Default)]
pub struct HitObjectQueue {
    pub queue: VecDeque<HitObject>,
}
impl HitObjectQueue {
    pub fn new() -> HitObjectQueue {
        HitObjectQueue {
            queue: VecDeque::new(),
        }
    }
}
impl Component for HitObjectQueue {
    type Storage = VecStorage<HitObjectQueue>;
}

#[derive(Default)]
pub struct HitOffsets {
    pub offsets: Vec<Option<f64>>,
}

impl Component for HitOffsets {
    type Storage = VecStorage<HitOffsets>;
}

#[derive(Default)]
pub struct UserSettings {
    pub offset: f64,
}
