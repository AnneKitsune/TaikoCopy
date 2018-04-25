extern crate amethyst;
extern crate futures;
extern crate rayon;

use std::sync::Arc;

use amethyst::core::Time;
use amethyst::core::cgmath::{Matrix4, Vector3};
use amethyst::input::InputHandler;
use amethyst::input::InputEvent;

use amethyst::assets::{AssetStorage, Handle, Loader};
use amethyst::audio::{AudioSink, OggFormat, Source, SourceHandle};
use amethyst::audio::output::Output;

use amethyst::renderer::{Camera, Event, Factory, KeyboardInput, Material, MeshHandle, PngFormat,
                         Projection, TextureMetadata, WindowEvent};
use amethyst::core::transform::{GlobalTransform, Transform};
use amethyst::prelude::*;
use amethyst::core::timing::Stopwatch;
use futures::Future;
use amethyst::ecs::prelude::*;
use amethyst::shrev::{EventChannel, ReaderId};

use rayon::ThreadPool;

use amethyst::winit::VirtualKeyCode;

use amethyst::core::shred::*;

use systems::*;
use resources::*;
use utils::*;

pub struct GameState {
    dispatch: ParSeq<Arc<rayon::ThreadPool>, GameSystem>,
    audio_handle: Handle<Source>,
}

impl GameState {
    pub fn new(world: &mut World, audio_handle: Handle<Source>) -> GameState {
        GameState {
            dispatch: ParSeq::new(
                GameSystem {
                    reader_id: None,
                    start_time: 0.0,
                },
                world.read_resource::<Arc<ThreadPool>>().clone(),
            ),
            audio_handle,
        }
    }
    pub fn load_sounds(world: &World) -> Sounds {
        let loader = world.read_resource::<Loader>();
        let hitsound_normal = wav_from_file(
            "resources/audio/taiko-normal-hitnormal.wav",
            &loader,
            &world.read_resource(),
        );
        let hitsound_clap = wav_from_file(
            "resources/audio/taiko-normal-hitclap.wav",
            &loader,
            &world.read_resource(),
        );
        let hitsound_finish = wav_from_file(
            "resources/audio/taiko-normal-hitfinish.wav",
            &loader,
            &world.read_resource(),
        );
        let hitsound_whistle = wav_from_file(
            "resources/audio/taiko-normal-hitwhistle.wav",
            &loader,
            &world.read_resource(),
        );
        Sounds {
            normal: hitsound_normal,
            clap: hitsound_clap,
            finish: hitsound_finish,
            whistle: hitsound_whistle,
        }
    }
    pub fn load_hit_results(
        hit_results_path: String,
        world: &World,
    ) -> (Material, Material, Material) {
        let loader = world.read_resource::<Loader>();
        (
            material_from_png_simple(
                &format!("{}/taiko-hit0.png", hit_results_path),
                &loader,
                &world.read_resource(),
                &world.read_resource(),
            ),
            material_from_png_simple(
                &format!("{}/taiko-hit100.png", hit_results_path),
                &loader,
                &world.read_resource(),
                &world.read_resource(),
            ),
            material_from_png_simple(
                &format!("{}/taiko-hit300.png", hit_results_path),
                &loader,
                &world.read_resource(),
                &world.read_resource(),
            ),
        )
    }
}

impl State for GameState {
    fn on_start(&mut self, world: &mut World) {
        self.dispatch.setup(&mut world.res);

        let beatmap = world
            .res
            .try_fetch::<BeatMap>()
            .expect("Can't fetch beatmap from resources.")
            .clone();
        let hit_results_path = world
            .res
            .try_fetch::<Paths>()
            .expect("Can't fetch folder paths.")
            .path("hit_results")
            .expect("Failed to find hit_results path")
            .clone();

        //let music:SourceHandle = world.read_resource::<Loader>().load(beatmap.songpath.clone(), OggFormat, (),(),&world.read_resource());

        let sounds = GameState::load_sounds(&world);

        let (miss, good, perfect) = GameState::load_hit_results(hit_results_path, &world);

        let mesh = gen_rectangle_mesh(
            0.05,
            0.05,
            &world.read_resource::<Loader>(),
            &world.read_resource(),
        );

        let big_hit_mesh = gen_rectangle_mesh(
            0.01,
            0.25,
            &world.read_resource::<Loader>(),
            &world.read_resource(),
        );
        let small_hit_mesh = gen_rectangle_mesh(
            0.005,
            0.15,
            &world.read_resource::<Loader>(),
            &world.read_resource(),
        );
        /*let hit_judgement_mesh = gen_rectangle_mesh(
            0.001,
            0.25,
            &world.read_resource::<Loader>(),
            &world.read_resource(),
        );*/
        let hit_judgement_mesh = gen_rectangle_mesh(
            20.0,
            20.0,
            &world.read_resource::<Loader>(),
            &world.read_resource(),
        );

        let red_hit_mtl = material_from_color(
            [1.0, 0.0, 0.0, 1.0],
            &world.read_resource::<Loader>(),
            &world.read_resource(),
            &world.read_resource(),
        );
        let blue_hit_mtl = material_from_color(
            [0.0, 0.0, 1.0, 1.0],
            &world.read_resource::<Loader>(),
            &world.read_resource(),
            &world.read_resource(),
        );
        let hit_judgement_mtl = material_from_color(
            [0.0, 1.0, 0.0, 1.0],
            &world.read_resource::<Loader>(),
            &world.read_resource(),
            &world.read_resource(),
        );


        world.add_resource(HitResultTextures {
            miss,
            good,
            perfect,
        });

        world.add_resource(sounds);

        if let Some(ref output) = world.read_resource::<Option<Output>>().as_ref() {
            let mut sink = world.write_resource::<AudioSink>();
            sink.set_volume(0.25);
            let m = world.read_resource::<AssetStorage<Source>>();
            output.play_once(m.get(&self.audio_handle).expect("Can't find music"), 0.2);
        }

        let mut stopwatch = StopwatchWrapper {
            stopwatch: Stopwatch::new(),
        };
        stopwatch.stopwatch.start();
        world.add_resource(stopwatch);

        let mut tr = Transform::default();
        tr.translation = [0.0,0.0,-0.5].into();

        world
            .create_entity()
            .with(Camera::from(Projection::orthographic(0.0, 1.0, 1.0, 0.0)))
            .with(tr)
            .with(GlobalTransform::default())
            .build();

        let mut hitqueue = HitObjectQueue::new();
        for hit in &beatmap.objects {
            hitqueue.queue.push_back(hit.clone());

            let mut tr = Transform::default();
            tr.translation = [0.0, 0.5, 0.0].into();
            let mtl = if hit.red {
                red_hit_mtl.clone()
            } else {
                blue_hit_mtl.clone()
            };
            let mesh = if hit.big {
                big_hit_mesh.clone()
            } else {
                small_hit_mesh.clone()
            };
            world
                .create_entity()
                .with(mesh)
                .with(mtl)
                .with(hit.clone())
                .with(tr)
                .with(Transform::default())
                .build();
        }

        //add hit judgement On Time
        // 0.5 screen/sec, 25 ms = 0.0125 screens

        //Count bars from right to left. 1rst one is at the exact time the note hits,
        //then each is -25 ms from the one at the right of it.
        /*for i in 1..20 {
            let mut tr = LocalTransform::default();
            tr.translation = [0.3 - (0.0125 as f32 * i as f32), 0.5, 0.0];
            world
                .create_entity()
                .with(hit_judgement_mesh.clone())
                .with(hit_judgement_mtl.clone())
                .with(tr)
                .with(Transform::default())
                .build();
        }*/

        let mut tr = Transform::default();
        tr.translation = [0.3, 0.5, 0.0].into();
        world
            .create_entity()
            .with(hit_judgement_mesh.clone())
            .with(hit_judgement_mtl.clone())
            .with(tr)
            .with(Transform::default())
            .build();
    }

    fn update(&mut self, world: &mut World) -> Trans {
        self.dispatch.dispatch(&mut world.res);
        Trans::None
    }
    fn handle_event(&mut self, _: &mut World, event: Event) -> Trans {
        if key_pressed_from_event(VirtualKeyCode::Escape,&event) || window_closed(&event){
            return Trans::Quit;
        }
        Trans::None
    }
}

pub struct MenuState;

impl State for MenuState {
    fn on_start(&mut self, world: &mut World) {
        let map_folder = &world
            .res
            .try_fetch::<Paths>()
            .expect("Can't fetch folder paths.")
            .path("maps")
            .expect("Can't find the map folder path")
            .clone();
        let mut beatmaps = beatmap_list(&map_folder);
        for b in &beatmaps {
            println!("Found beatmap: {}", b.songpath);
        }
        //world.add_resource(beatmaps.swap_remove(1));
        world.add_resource(beatmaps.swap_remove(3));//Unpleasant Sonata

        world.add_resource(EventChannel::<HitResult>::new());
    }
    fn handle_event(&mut self, world: &mut World, event: Event) -> Trans {
        if key_pressed_from_event(VirtualKeyCode::Space,&event){
            return Trans::Switch(Box::new(BeatmapLoadState { audio_handle: None }));
        }
        if window_closed(&event){
            return Trans::Quit;
        }
        Trans::None
    }
}

pub struct BeatmapLoadState {
    audio_handle: Option<Handle<Source>>,
}

impl State for BeatmapLoadState {
    fn on_start(&mut self, world: &mut World) {
        if self.audio_handle.is_none() {
            let beatmap = world
                .res
                .try_fetch::<BeatMap>()
                .expect("Can't fetch beatmap from resources.")
                .clone();
            self.audio_handle = Some(world.read_resource::<Loader>().load(
                beatmap.songpath.clone(),
                OggFormat,
                (),
                (),
                &world.read_resource(),
            ));
        }
    }
    fn update(&mut self, world: &mut World) -> Trans {
        if world
            .read_resource::<AssetStorage<Source>>()
            .get(&self.audio_handle.clone().unwrap())
            .is_some()
        {
            Trans::Switch(Box::new(GameState::new(
                world,
                self.audio_handle.clone().unwrap(),
            )))
        } else {
            Trans::None
        }
    }
}
