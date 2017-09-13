//! A copy of the osu!taiko gamemode implemented using amethyst

//#![deny(missing_docs,dead_code)]
extern crate amethyst;
extern crate futures;
extern crate rayon;
extern crate time;

use amethyst::assets::{AssetFuture, BoxedErr};
use amethyst::assets::Loader;
use amethyst::assets::formats::audio::{OggFormat, WavFormat};
use amethyst::audio::{Dj, AudioContext, Source};
use amethyst::audio::output::{default_output, Output};
use amethyst::audio::play::play_once;
use amethyst::ecs::{Component, Fetch, FetchMut, Join, System, VecStorage, WriteStorage};
use amethyst::ecs::audio::DjSystem;
use amethyst::ecs::input::{Bindings, InputHandler};
use amethyst::ecs::rendering::{Factory, MeshComponent, MaterialComponent};
use amethyst::ecs::transform::{Transform, LocalTransform, Child, Init, TransformSystem};
use amethyst::prelude::*;
use amethyst::renderer::Config as DisplayConfig;
use amethyst::renderer::prelude::*;
use amethyst::timing::{Time, Stopwatch};
use futures::{Future, IntoFuture};

use std::ops::{Add, Sub};
use std::time::Instant;
use time::PreciseTime;
use std::fs::File;
use std::io::prelude::*;
use std::collections::HashMap;
use std::collections::VecDeque;


#[derive(Clone)]
struct HitObject {
    red: bool,
    time: f32,
    big: bool,
}

impl Component for HitObject {
    type Storage = VecStorage<HitObject>;
}

struct BeatMap {
    name: String,
    pub songpath: String,
    objects: Vec<HitObject>,
    maxhitoffset: f32,
}

impl Component for BeatMap {
    type Storage = VecStorage<BeatMap>;
}

struct HitObjectQueue {
    pub queue: VecDeque<HitObject>,
}
impl HitObjectQueue {
    fn new() -> HitObjectQueue {
        return HitObjectQueue { queue: VecDeque::new() };
    }
}
impl Component for HitObjectQueue {
    type Storage = VecStorage<HitObjectQueue>;
}

struct Sounds {
    hitsound: Source,
}

struct Game;

impl State for Game {
    fn on_start(&mut self, engine: &mut Engine) {
        let beatmap = readBeatmap();

        let (music, hitsound) = {
            let mut loader = engine.world.write_resource::<Loader>();
            loader.register(AudioContext::new());

            let music: Source = loader
                .load_from(beatmap.songpath.clone(), OggFormat, "")
                .wait()
                .unwrap();

            let hitsound: Source = loader
                .load_from(
                    format!(
                        "{}/resources/assets/hitsound.wav",
                        env!("CARGO_MANIFEST_DIR")
                    ),
                    WavFormat,
                    "",
                )
                .wait()
                .unwrap();
            (music, hitsound)
        };

        engine.world.add_resource(Sounds { hitsound });

        let have_output = engine.world.read_resource::<Option<Output>>().is_some();

        let (red_hit_mesh, red_hit_mtl) =
            gen_complete_rect([0.005, 0.15], [1.0, 0., 0., 1.0], engine);
        let (blue_hit_mesh, blue_hit_mtl) =
            gen_complete_rect([0.005, 0.15], [0., 0., 1.0, 1.0], engine);
        let (hit_judgement_mesh, hit_judgement_mtl) =
            gen_complete_rect([0.001, 0.25], [0., 1., 0., 1.], engine);

        //Maybe putting it last in the method would reduce the offset??
        if have_output {
            let mut dj = engine.world.write_resource::<Dj>();
            dj.set_volume(0.15);
            let music = music.clone();
            dj.set_picker(Box::new(move |ref mut dj| {
                dj.append(&music).expect("Decoder error occurred!");
                true
            }));
        }

        let world = &mut engine.world;

        let mut stopwatch = Stopwatch::new();
        stopwatch.start();
        world.add_resource(stopwatch);

        world.add_resource(Camera {
            eye: [0., 0., 1.0].into(),
            proj: Projection::orthographic(0.0, 1.0, 1.0, 0.0).into(),
            forward: [0., 0., -1.0].into(),
            right: [1.0, 0.0, 0.0].into(),
            up: [0., 1.0, 0.].into(),
        });
        let mut input = InputHandler::new();
        input.bindings = Bindings::load(format!(
            "{}/resources/input.ron",
            env!("CARGO_MANIFEST_DIR")
        ));

        world.add_resource(input);
        world.add_resource(Time::default());
        world.add_resource(HitObjectQueue::new());


        world.register::<Child>();
        world.register::<Init>();
        world.register::<LocalTransform>();

        for hit in &beatmap.objects {
            let mut tr = LocalTransform::default();
            tr.translation = [0.0, 0.5, 0.0];
            let mtl = if hit.red {
                red_hit_mtl.clone()
            } else {
                blue_hit_mtl.clone()
            };
            world
                .create_entity()
                .with(red_hit_mesh.clone())
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
        //Place your cursor around where you hear the note actually being played,
        //then count the bars.

        //Jojolepro's result -> 13 bars = -300 ms.  My normal offset on osu is -25 ms
        for i in 1..20 {
            let mut tr = LocalTransform::default();
            tr.translation = [0.3 - (0.0125 as f32 * i as f32), 0.5, 0.0];
            world
                .create_entity()
                .with(hit_judgement_mesh.clone())
                .with(hit_judgement_mtl.clone())
                .with(tr)
                .with(Transform::default())
                .build();
        }

        world.add_resource(beatmap);
    }

    fn handle_event(&mut self, _: &mut Engine, event: Event) -> Trans {
        match event {
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::KeyboardInput {
                        input: KeyboardInput { virtual_keycode: Some(VirtualKeyCode::Escape), .. }, ..
                    } |
                    WindowEvent::Closed => Trans::Quit,
                    _ => Trans::None,
                }
            }
            _ => Trans::None,
        }
    }
}


struct GameSystem;

impl<'a> System<'a> for GameSystem {
    type SystemData = (WriteStorage<'a, HitObject>,
     WriteStorage<'a, LocalTransform>,
     Fetch<'a, Camera>,
     Fetch<'a, Time>,
     Fetch<'a, InputHandler>,
     Fetch<'a, Sounds>,
     Fetch<'a, Option<Output>>,
     Fetch<'a, BeatMap>,
     Fetch<'a, Stopwatch>,
     FetchMut<'a, HitObjectQueue>);
    fn run(
        &mut self,
        (mut hitobjects,
         mut transforms,
         cam,
         time,
         input,
         sounds,
         audio_output,
         beatmap,
         stopwatch,
         mut hitqueue):
         Self::SystemData,
){
        let curTime = stopwatch.elapsed();
        let curTime = curTime.as_secs() as f32 + (curTime.subsec_nanos() as f32 / 1_000_000_000.0);
        println!("CurTime: {}", curTime);
        for (obj, tr) in (&mut hitobjects, &mut transforms).join() {
            tr.translation[0] = (obj.time - curTime) * 0.50 + 0.3; //TEMPORARY. TO TEST HIT JUDGEMENT
        }
    }
}


fn getKeyPressType(z: bool, x: bool, two: bool, three: bool) -> (bool, bool) {
    let dual = (z && x) || (two && three);

    let red = z || x;
    (red, dual)
}

/*fn window(){
    let mut window: PistonWindow = WindowSettings::new("Taiko Copy :D",
    [800,600]).resizable(true).exit_on_esc(true).build().unwrap();




    for hit in &beatmap.objects{
        objectsqueue.push_back(hit.clone());
    }
    let mut hitoffsets:Vec<Option<f64>> = Vec::new();




    //key,was pressed   if key doesn't exists, key is not pressed
    let mut keys:HashMap<i32,bool> = HashMap::new();

    while let Some(event) = window.next(){
        let curTime = startTime.to(PreciseTime::now()).num_milliseconds() as f64 / 1000.0;
        //println!("Time diff: {}",curTime);
        match event {
            Event::Input(input) => {
                if let Input::Press(Button::Keyboard(but)) = input {
                    println!("Pressed ");
                    if keys.get(&but.code()).is_none(){
                        keys.insert(but.code(),false);
                    }
                }
                if let Input::Release(Button::Keyboard(but)) = input {
                    println!("Released ");
                    keys.remove(&but.code());
                }
            },
            Event::Render(render)=>{
                window.draw_2d(&event,|context,graphics|{
                    clear([1.0;4],graphics);
                    for hit in &beatmap.objects{
                        let color = match hit.red{
                            true=>[1.0,0.0,0.0,1.0],
                            false=>[0.0,0.0,1.0,1.0],
                        };
                        let (sizex,sizey) = match hit.big{
                            true =>(16.0,150.0),
                            false =>(8.0,100.0),
                        };
                        rectangle(
                            color,
                            [(hit.time as f64 - curTime) * 750.0,50.0,sizex,sizey],
                            context.transform,
                            graphics
                        );
                    }
                });
            },
            Event::Update(update)=>{
                //Remove past objects that were not clicked

                //try take first
                //drop if ...
                //loop
                while let Some(head) = (&mut objectsqueue).pop_front(){

                    if head.time + beatmap.maxhitoffset < curTime as f32{
                        &hitoffsets.push(None);
                        println!("Dropped object");
                    }else{
                        objectsqueue.push_front(head);
                        break;
                    }
                }



                //add && !waspressed




                let z = keys.get(&Key::Z.code()).is_some();
                let z = z && !*keys.get(&Key::Z.code()).unwrap();
                let x = keys.get(&Key::X.code()).is_some();
                let x = x && !*keys.get(&Key::X.code()).unwrap();
                let two = keys.get(&Key::NumPad2.code()).is_some();
                let two = two && !*keys.get(&Key::NumPad2.code()).unwrap();
                let three = keys.get(&Key::NumPad3.code()).is_some();
                let three = three && !*keys.get(&Key::NumPad3.code()).unwrap();
                //println!("NumPad2 pressed: {}",keys.get(&Key::NumPad2.code()).is_some());
                if z || x || two || three {
                    let (red, dual) = getKeyPressType(z, x, two, three);

                    //Get clickable object
                    if let Some(head) = (&mut objectsqueue).pop_front() {
                        if let (Some(offset), clicked) = checkHit(&beatmap, curTime, red, dual) {
                            if clicked {
                                hitoffsets.push(Some(offset));
                            } else {
                                hitoffsets.push(None);
                            }
                        } else {
                            //Put back into list if pressed but no hitobject was found
                            hitoffsets.push(None);
                        }
                    }
                }



                //not working :(
                //keys.iter_mut().map(|(k,v)|{(k,true)});
                for (k,v) in keys.iter_mut(){
                    *v = true;
                }
            }
            _ =>{},
        }

    }
}*/

///Returns hit offset
///Found no objects to hit, no offset
///Found an object to hit, used wrong button
///Found an object to hit, used right button  (Some(offset),true)
fn checkHit(beatmap: &BeatMap, time: f64, redpressed: bool, dual: bool) -> (Option<f64>, bool) {
    for hit in &beatmap.objects {
        if value_near(time, hit.time as f64, 0.2) {
            if (hit.red && redpressed) || (!hit.red && !redpressed) {
                if (hit.big && dual) || (!hit.big && !dual) {
                    println!("GOOD HIT @ {}", time);
                    return (Some(time - hit.time as f64), true);
                } else {
                    println!("Wrong dual >_<");
                    return (Some(time - hit.time as f64), false);
                }
            }
            println!("Wrong key >_<");
            return (Some(time - hit.time as f64), false);
        }
    }
    return (None, false);
}

fn value_near<B: Add<Output = B> + Sub<Output = B> + PartialOrd + Copy>(
    number: B,
    target: B,
    margin: B,
) -> bool {
    number >= target - margin && number <= target + margin
}

fn readBeatmap() -> BeatMap {
    let folder = format!(
        "{}/resources/assets/maps/544922 t+pazolite - Intro - the other Side/",
        env!("CARGO_MANIFEST_DIR")
    );
    let mut file = File::open(format!(
        "{}t+pazolite - Intro - the other Side (Taikocracy) [Futsuu].osu",
        folder
    )).expect("Failed to open beatmap file");
    let mut content = String::new();
    file.read_to_string(&mut content).expect(
        "Failed to read beatmap file",
    );

    let mut hitobjects: Vec<HitObject> = vec![];
    let mut mode = "";
    let mut songpath = "";
    for line in content.lines() {
        if line == "[HitObjects]" {
            mode = "HitObjects";
        }
        if line.starts_with("AudioFilename:") {
            songpath = &line[15..];
        }
        if mode == "HitObjects" {
            let mut split: Vec<&str> = line.split(",").collect();

            if split.len() != 6 {
                continue;
            }
            let objecttype = split[4 as usize].parse::<u8>().expect(
                "Failed to parse as u8",
            );
            let (isRed, isBig) = match objecttype {
                0 => (true, false),//small red
                4 => (true, true),//big red
                8 => (false, false),//small blue
                12 => (false, true),//big blue
                c => panic!("Unknown hitobject color type: {}", c),
            };
            hitobjects.push(HitObject {
                red: isRed,
                time: osuToRealTime(split[2 as usize].parse::<i32>().expect(
                    "Failed to parse as u8",
                )),
                big: isBig,
            });
        }
    }
    BeatMap {
        name: String::from("Test beatmap"),
        songpath: format!("{}{}", folder, songpath),
        objects: hitobjects,
        maxhitoffset: 0.05,
    }
}
fn osuToRealTime(time: i32) -> f32 {
    time as f32 / 1000.0
}
fn load_proc_asset<T, F>(engine: &mut Engine, f: F) -> AssetFuture<T::Item>
where
    T: IntoFuture<Error = BoxedErr>,
    T::Future: 'static,
    F: FnOnce(&mut Engine) -> T,
{
    let future = f(engine).into_future();
    let future: Box<Future<Item = T::Item, Error = BoxedErr>> = Box::new(future);
    AssetFuture(future.shared())
}

fn main() {
    use amethyst::assets::Directory;

    let path = format!("{}/resources/config.ron", env!("CARGO_MANIFEST_DIR"));
    let cfg = DisplayConfig::load(path);
    let assets_dir = format!("{}/resources/assets/", env!("CARGO_MANIFEST_DIR"));
    let gamesystem = GameSystem;
    let mut game = Application::build(Game)
        .unwrap()
        .register::<HitObject>()
        .register::<HitObjectQueue>()
        .register::<BeatMap>()
        .with::<GameSystem>(gamesystem, "game_system", &[])
        .with::<TransformSystem>(TransformSystem::new(), "transform_system", &["game_system"])
        .with_renderer(
            Pipeline::build().with_stage(
                Stage::with_backbuffer()
                    .clear_target([0.0, 0.0, 0.0, 1.0], 1.0)
                    .with_model_pass(pass::DrawFlat::<PosNormTex>::new()),
            ),
            Some(cfg),
        )
        .unwrap()
        .add_store("assets", Directory::new(assets_dir));

    let audio_output = default_output();
    match audio_output {
        Some(ref output) => {
            game = game.add_resource(Dj::new(&output)).with(
                DjSystem,
                "dj_system",
                &[],
            );
        }
        None => {
            eprintln!("Audio device not found, no sound will be played.");
        }
    }
    game = game.add_resource(audio_output);
    game.build().expect("Failed to build game").run();
}


fn gen_rectangle(w: f32, h: f32) -> Vec<PosNormTex> {
    let data: Vec<PosNormTex> = vec![
        PosNormTex {
            a_position: [-w / 2., -h / 2., 0.],
            a_normal: [0., 0., 1.],
            a_tex_coord: [0., 0.],
        },
        PosNormTex {
            a_position: [w / 2., -h / 2., 0.],
            a_normal: [0., 0., 1.],
            a_tex_coord: [1., 0.],
        },
        PosNormTex {
            a_position: [w / 2., h / 2., 0.],
            a_normal: [0., 0., 1.],
            a_tex_coord: [1., 1.],
        },

        PosNormTex {
            a_position: [w / 2., h / 2., 0.],
            a_normal: [0., 0., 1.],
            a_tex_coord: [1., 1.],
        },
        PosNormTex {
            a_position: [-w / 2., h / 2., 0.],
            a_normal: [0., 0., 1.],
            a_tex_coord: [0., 1.],
        },
        PosNormTex {
            a_position: [-w / 2., -h / 2., 0.],
            a_normal: [0., 0., 1.],
            a_tex_coord: [0., 0.],
        },
    ];
    data
}
fn gen_complete_rect(
    size: [f32; 2],
    color: [f32; 4],
    engine: &mut Engine,
) -> (AssetFuture<MeshComponent>, AssetFuture<MaterialComponent>) {
    let tex = Texture::from_color_val(color);
    let mtl = MaterialBuilder::new().with_albedo(tex);
    let verts = gen_rectangle(size[0], size[1]);
    let mesh = Mesh::build(verts);
    let mesh = load_proc_asset(engine, move |engine| {
        let factory = engine.world.read_resource::<Factory>();
        factory.create_mesh(mesh).map(MeshComponent::new).map_err(
            BoxedErr::new,
        )
    });

    let mtl = load_proc_asset(engine, move |engine| {
        let factory = engine.world.read_resource::<Factory>();
        factory
            .create_material(mtl)
            .map(MaterialComponent)
            .map_err(BoxedErr::new)
    });
    (mesh, mtl)
}
