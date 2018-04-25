extern crate amethyst;

use amethyst::audio::SourceHandle;
use amethyst::ecs::prelude::*;
use amethyst::renderer::{Material, MeshHandle};
use amethyst::core::transform::Transform;
use amethyst::core::timing::Stopwatch;

use std::collections::VecDeque;
use std::collections::HashMap;
use std::path::Path;

use amethyst::prelude::*;

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
impl HitOffsets {
    pub fn new() -> HitOffsets {
        HitOffsets {
            offsets: Vec::new(),
        }
    }
}
impl Component for HitOffsets {
    type Storage = VecStorage<HitOffsets>;
}

pub struct SystemTracker {
    pub game: bool,
}
impl SystemTracker {
    pub fn new() -> SystemTracker {
        SystemTracker { game: false }
    }
}
impl Component for SystemTracker {
    type Storage = VecStorage<SystemTracker>;
}

#[derive(Default)]
pub struct UserSettings {
    pub offset: f64,
}

pub struct Paths {
    paths: HashMap<String, String>,
}
impl Component for Paths {
    type Storage = VecStorage<Paths>;
}
impl Paths {
    pub fn from_file(cfg_file: &str) -> Paths {
        let path = Path::new(&cfg_file);
        let mut paths: HashMap<String, String> = HashMap::load(path);
        for (_, v) in paths.iter_mut() {
            if v.starts_with("./") {
                v.remove(0);
                *v = format!("{}{}", env!("CARGO_MANIFEST_DIR"), v);
            }
        }
        Paths { paths: paths }
    }
    pub fn paths(&self) -> &HashMap<String, String> {
        &self.paths
    }
    pub fn path(&self, key: &str) -> Option<&String> {
        self.paths.get(key)
    }
}
