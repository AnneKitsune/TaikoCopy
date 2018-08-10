extern crate amethyst;
extern crate futures;
extern crate rayon;

use std::sync::Arc;

use amethyst::assets::{AssetStorage, Handle, Loader};
use amethyst::audio::output::Output;
use amethyst::audio::WavFormat;
use amethyst::audio::{AudioSink, OggFormat, Source};
use amethyst::core::cgmath::{Matrix4, Vector3};
use amethyst::core::timing::Stopwatch;
use amethyst::core::transform::{GlobalTransform, Transform};
use amethyst::ecs::prelude::*;
use amethyst::prelude::*;
use amethyst::renderer::*;
use amethyst::shrev::EventChannel;
use amethyst::ui::*;
use amethyst_extra::*;

use rayon::ThreadPool;

use amethyst::winit::VirtualKeyCode;

use amethyst::core::shred::*;

use resources::*;
use systems::*;
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
        //let loader = world.read_resource::<Loader>();
        let asset_loader = world.read_resource::<AssetLoader>();

        // Unwrap because we know the path is right.
        let hitsound_normal = asset_loader
            .load(
                "audio/taiko-normal-hitnormal.wav",
                WavFormat,
                (),
                &mut world.write_resource(),
                &mut world.write_resource(),
                &world.read_resource(),
            )
            .unwrap();
        let hitsound_clap = asset_loader
            .load(
                "audio/taiko-normal-hitclap.wav",
                WavFormat,
                (),
                &mut world.write_resource(),
                &mut world.write_resource(),
                &world.read_resource(),
            )
            .unwrap();
        let hitsound_finish = asset_loader
            .load(
                "audio/taiko-normal-hitfinish.wav",
                WavFormat,
                (),
                &mut world.write_resource(),
                &mut world.write_resource(),
                &world.read_resource(),
            )
            .unwrap();
        let hitsound_whistle = asset_loader
            .load(
                "audio/taiko-normal-hitwhistle.wav",
                WavFormat,
                (),
                &mut world.write_resource(),
                &mut world.write_resource(),
                &world.read_resource(),
            )
            .unwrap();
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

impl<'a, 'b> State<GameData<'a, 'b>> for GameState {
    fn on_start(&mut self, data: StateData<GameData<'a, 'b>>) {
        self.dispatch.setup(&mut data.world.res);

        let beatmap = data.world
            .res
            .try_fetch::<BeatMap>()
            .expect("Can't fetch beatmap from resources.")
            .clone();
        /*let hit_results_path = data.world
            .fetch::<AssetLoader>()
            .resolve_path("hit_results")
            .expect("Failed to find hit_results path")
            .clone();*/

        //let music:SourceHandle = world.read_resource::<Loader>().load(beatmap.songpath.clone(), OggFormat, (),(),&world.read_resource());

        let sounds = GameState::load_sounds(&data.world);

        //let (miss, good, perfect) = GameState::load_hit_results(hit_results_path, &data.world);

        let big_hit_mesh = gen_rectangle_mesh(
            0.01,
            0.25,
            &data.world.read_resource::<Loader>(),
            &data.world.read_resource(),
        );
        let small_hit_mesh = gen_rectangle_mesh(
            0.005,
            0.15,
            &data.world.read_resource::<Loader>(),
            &data.world.read_resource(),
        );
        let hit_judgement_mesh = gen_rectangle_mesh(
            0.001,
            0.25,
            &data.world.read_resource::<Loader>(),
            &data.world.read_resource(),
        );

        let red_hit_mtl = material_from_color(
            [1.0, 0.0, 0.0, 1.0],
            &data.world.read_resource::<Loader>(),
            &data.world.read_resource(),
            &data.world.read_resource(),
        );
        let blue_hit_mtl = material_from_color(
            [0.0, 0.0, 1.0, 1.0],
            &data.world.read_resource::<Loader>(),
            &data.world.read_resource(),
            &data.world.read_resource(),
        );
        let hit_judgement_mtl = material_from_color(
            [0.0, 1.0, 0.0, 1.0],
            &data.world.read_resource::<Loader>(),
            &data.world.read_resource(),
            &data.world.read_resource(),
        );

        /*data.world.add_resource(HitResultTextures {
            miss,
            good,
            perfect,
        });*/

        data.world.add_resource(sounds);

        if let Some(output) = data.world.res.try_fetch::<Output>() {
            let mut sink = data.world.write_resource::<AudioSink>();
            sink.set_volume(0.5);
            let m = data.world.read_resource::<AssetStorage<Source>>();
            output.play_once(m.get(&self.audio_handle).expect("Can't find music"), 1.0);
        } else {
            error!("Failed to find audio `Output`.");
        }

        let mut stopwatch = StopwatchWrapper {
            stopwatch: Stopwatch::new(),
        };
        stopwatch.stopwatch.start();
        data.world.add_resource(stopwatch);

        // TODO: Remove this. It needs to be created only once.
        data.world
            .create_entity()
            .with(Camera::from(Projection::orthographic(0.0, 1.0, 1.0, 0.0)))
            .with(GlobalTransform(
                Matrix4::from_translation(Vector3::new(0.0, 0.0, 1.0)).into(),
            ))
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
            data.world
                .create_entity()
                .with(mesh)
                .with(mtl)
                .with(hit.clone())
                .with(tr)
                .with(GlobalTransform::default())
                .with(Removal::new(RemovalLayer::Gameplay))
                .build();
        }
        data.world.add_resource(hitqueue);

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
        data.world
            .create_entity()
            .with(hit_judgement_mesh.clone())
            .with(hit_judgement_mtl.clone())
            .with(tr)
            .with(GlobalTransform::default())
            .with(Removal::new(RemovalLayer::Gameplay))
            .build();
    }
    
    fn on_stop(&mut self, mut data: StateData<GameData>) {
        exec_removal(&data.world.read_resource(), &data.world.read_storage(), RemovalLayer::Gameplay);
    }
    
    fn update(&mut self, mut data: StateData<GameData<'a, 'b>>) -> Trans<GameData<'a, 'b>> {
        data.data.update(&mut data.world);
        self.dispatch.dispatch(&mut data.world.res);
        Trans::None
    }
    fn handle_event(
        &mut self,
        _: StateData<GameData<'a, 'b>>,
        event: Event,
    ) -> Trans<GameData<'a, 'b>> {
        if key_pressed_from_event(VirtualKeyCode::Escape, &event) {
            return Trans::Pop;
        } else if window_closed(&event) {
            return Trans::Quit;
        }
        Trans::None
    }
}

pub struct MenuState {
    ui_events: Option<ReaderId<UiEvent>>,
    all_maps: Vec<BeatMap>,
    button_entities: Vec<Entity>,
}

impl MenuState {
    pub fn new() -> Self {
        MenuState {
            ui_events: None,
            all_maps: vec![],
            button_entities: vec![],
        }
    }
}

impl<'a, 'b> State<GameData<'a, 'b>> for MenuState {
    fn on_start(&mut self, mut data: StateData<GameData<'a, 'b>>) {
        data.world.register::<Removal<RemovalLayer>>();
        let map_folder = &data.world
            .read_resource::<AssetLoader>()
            .resolve_path("maps")
            .expect("Failed to find maps folder");
        let beatmaps = beatmap_list(&map_folder);
        self.ui_events = Some(
            data.world
                .write_resource::<EventChannel<UiEvent>>()
                .register_reader(),
        );

        // Now we have our map list. Create the selection buttons.
        let font = data.world.read_resource::<AssetLoader>().load(
            "fonts/Arial.ttf",
            TtfFormat,
            (),
            &mut data.world.write_resource(),
            &mut data.world.write_resource(),
            &data.world.read_resource(),
        );
        
        self.button_entities.clear();
        if let Some(font) = font {
            for (i, b) in beatmaps.iter().enumerate() {
                println!("Found beatmap: {}", b.songpath);
                /*world.create_entity()
                .with(UiTransform::new("song_select",Anchor::TopMiddle, 0.0, -80.0 * (i as f32 + 1.0), 0.0, 1000.0, 80.0, i))
                .with(UiText::new(font,""))
                .build();*/
                let entity = UiButtonBuilder::new(b.songpath.clone(), b.name.clone())
                    .with_anchor(Anchor::TopMiddle)
                    .with_position(0.0, 40.0 + 100.0 * (i as f32 + 1.0))
                    .with_size(1000.0, 80.0)
                    .with_font_size(25.0)
                    .with_tab_order(i as i32)
                    .with_font(font.clone())
                    .with_text_color([0.0, 1.0, 0.0, 1.0])
                    .build_from_world(&mut data.world);
                self.button_entities.push(entity);
            }
        } else {
            error!("Failed to load font for song_select buttons. Skipping...");
        }

        self.all_maps = beatmaps;
        //data.world.add_resource(beatmaps.swap_remove(1)); //tephereth

        //world.add_resource(beatmaps.swap_remove(3));//Unpleasant Sonata

        //data.world.add_resource(EventChannel::<HitResult>::new());
    }
    
    fn on_resume(&mut self, mut data: StateData<GameData>) {
        self.on_start(data);
    }
    
    fn handle_event(
        &mut self,
        _: StateData<GameData<'a, 'b>>,
        event: Event,
    ) -> Trans<GameData<'a, 'b>> {
        /*if key_pressed_from_event(VirtualKeyCode::Space, &event) {
            println!("Starting my dude");
            return Trans::Push(Box::new(BeatmapLoadState { audio_handle: None }));
        }
        if window_closed(&event) {
            return Trans::Quit;
        }*/
        Trans::None
    }
    fn update(&mut self, mut data: StateData<GameData<'a, 'b>>) -> Trans<GameData<'a, 'b>> {
        data.data.update(&mut data.world);
        let mut found_map = None;
        for ev in data.world
            .read_resource::<EventChannel<UiEvent>>()
            .read(&mut self.ui_events.as_mut().unwrap())
        {
            match ev.event_type {
                UiEventType::Click => {
                    let songpath = data.world
                        .read_storage::<UiTransform>()
                        .get(ev.target)
                        .unwrap()
                        .id
                        .clone();
                    if let Some(map) = self.all_maps.iter().find(|m| m.songpath == *songpath) {
                        found_map = Some(map.clone());
                    } else {
                        error!("Could not find selected map {} when clicking the select button. Was it removed?",songpath);
                    }
                }
                _ => {}
            }
        }
        if let Some(map) = found_map {
            data.world.add_resource(map);
            data.world
                .delete_entities(&self.button_entities)
                .expect("Failed to cleanup MenuState buttons");
            return Trans::Push(Box::new(BeatmapLoadState { audio_handle: None }));
        }

        Trans::None
    }
}

pub struct BeatmapLoadState {
    audio_handle: Option<Handle<Source>>,
}

impl<'a, 'b> State<GameData<'a, 'b>> for BeatmapLoadState {
    fn on_start(&mut self, data: StateData<GameData<'a, 'b>>) {
        if self.audio_handle.is_none() {
            let beatmap = data.world
                .res
                .try_fetch::<BeatMap>()
                .expect("Can't fetch beatmap from resources.")
                .clone();

            /*let music = data.world.read_resource::<AssetLoader>()
                .load("")*/

            self.audio_handle = Some(data.world.read_resource::<Loader>().load(
                beatmap.songpath.clone(),
                OggFormat,
                (),
                (),
                &data.world.read_resource(),
            ));
        }
    }
    fn update(&mut self, mut data: StateData<GameData<'a, 'b>>) -> Trans<GameData<'a, 'b>> {
        data.data.update(&mut data.world);
        if data.world
            .read_resource::<AssetStorage<Source>>()
            .get(&self.audio_handle.as_ref().unwrap())
            .is_some()
        {
            Trans::Switch(Box::new(GameState::new(
                data.world,
                self.audio_handle.as_ref().unwrap().clone(),
            )))
        } else {
            Trans::None
        }
    }
}
