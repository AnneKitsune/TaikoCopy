//! A copy of the osu!taiko gamemode implemented using amethyst

//#![deny(missing_docs,dead_code)]
extern crate amethyst;
extern crate futures;
extern crate rayon;
extern crate time;
extern crate winit;
extern crate rusttype;
extern crate imagefmt;

use amethyst::assets::{AssetFuture, BoxedErr,Format,Loader,Context};
use amethyst::assets::formats::audio::{OggFormat, WavFormat};
use amethyst::assets::formats::textures::{ImageData,ImageError,ImageFuture};
use amethyst::audio::{Dj, AudioContext, Source};
use amethyst::audio::output::{default_output, Output};
use amethyst::audio::play::play_once;
use amethyst::ecs::{Component, Fetch, FetchMut, Join, System, VecStorage, WriteStorage};
use amethyst::ecs::audio::DjSystem;
use amethyst::ecs::Entities;
use amethyst::ecs::input::{Bindings, InputHandler,InputBundle};
use amethyst::ecs::rendering::{Factory, MeshComponent, MaterialComponent,TextureContext,TextureComponent,RenderBundle};
use amethyst::ecs::transform::{Transform, LocalTransform, Child, Init, TransformSystem,TransformBundle};
use amethyst::prelude::*;
use amethyst::renderer::Config as DisplayConfig;
use amethyst::renderer::prelude::*;
use amethyst::timing::{Time, Stopwatch};
use futures::{Future, IntoFuture};
use amethyst::assets::formats::textures::PngFormat;
use amethyst::ecs::ECSBundle;
use amethyst::ecs::audio::DjBundle;

use amethyst::Result;

use std::ops::{Add, Sub};
use std::time::{Instant,Duration};
use std::fs::File;
use std::io::prelude::*;
use std::collections::{HashMap,VecDeque};

use amethyst::input::*;

use amethyst::util::time::*;
use amethyst::ecs::util::resources::*;
use amethyst::ecs::util::systems::*;

use winit::VirtualKeyCode;

use rusttype::{FontCollection, Scale, point, PositionedGlyph};

use imagefmt::{Image,ColFmt};

use rayon::ThreadPool;


fn main() {
    use amethyst::assets::Directory;

    let path = format!("{}/resources/config.ron", env!("CARGO_MANIFEST_DIR"));
    let cfg = DisplayConfig::load(path);
    let assets_dir = format!("{}/resources/assets/", env!("CARGO_MANIFEST_DIR"));

    let input_path = format!(
            "{}/resources/input.ron",
            env!("CARGO_MANIFEST_DIR")
        );
    type DrawFlat = pass::DrawFlat<PosNormTex, MeshComponent, MaterialComponent, Transform>;
    let mut game = Application::build(Game).unwrap()
        .with_bundle(FPSCounterBundle::new(20)).expect("Failed to create FPSCounterBundle")
        .with_bundle(InputBundle::new().with_bindings_from_file(&input_path)).expect("Failed to load input bindings")
        .with_bundle(GameBundle).expect("Failed to build game system")
        .with_bundle(TransformBundle::new().with_dep(&["game_system"])).expect("Failed to build transform bundle")
        .with_bundle(DjBundle::new()).expect("Failed to build dj bundle")
        .with_bundle(
            RenderBundle::new(
                Pipeline::build().with_stage(
                    Stage::with_backbuffer()
                        .clear_target([255.0,105.0,180.0, 1.0], 1.0)
                        .with_pass(DrawFlat::new()),
                ),
            ).with_config(cfg),
        ).unwrap()
        .with_store("assets", Directory::new(assets_dir));
    game.build().expect("Failed to build game").run();
}

//---------------idk how to classify this :/
struct Sounds {
    normal: Source,
    clap: Source,
    finish: Source,
    whistle: Source,
}

//----------------GAME STRUCTS-----------
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
        HitObjectQueue { queue: VecDeque::new() }
    }
}
impl Component for HitObjectQueue {
    type Storage = VecStorage<HitObjectQueue>;
}

struct HitOffsets{
    pub offsets: Vec<Option<f32>>,
}
impl HitOffsets{
    fn new()->HitOffsets{
        HitOffsets{
            offsets: Vec::new(),
        }
    }
}
impl Component for HitOffsets{
    type Storage = VecStorage<HitOffsets>;
}

struct UserSettings{
    pub offset: f32,
}

//-------------------------//

//--------------GAME INIT------------
struct GameBundle;

impl<'a, 'b, T> ECSBundle<'a, 'b, T> for GameBundle {
    fn build(
        &self,
        builder: ApplicationBuilder<'a, 'b, T>,
    ) -> Result<ApplicationBuilder<'a, 'b, T>> {
        Ok(
            builder
                .with_resource(HitOffsets::new())
                .with_resource(Time::default())
                .register::<HitObject>()
                .register::<HitObjectQueue>()
                .register::<BeatMap>()
                .with(GameSystem, "game_system", &[]),
        )
    }
}


struct Game;

impl State for Game {
    fn on_start(&mut self, engine: &mut Engine) {
        let beatmap = read_beatmap();

        let (music,hitsound_normal,hitsound_clap,hitsound_finish,hitsound_whistle) = {
            let mut loader = engine.world.write_resource::<Loader>();
            loader.register(AudioContext::new());

            let music: Source = loader
                .load_from(beatmap.songpath.clone(), OggFormat, "").wait().unwrap();

            let hitsound_normal: Source = wav_from_file("resources/audio/taiko-normal-hitnormal.wav",&loader);
            let hitsound_clap: Source = wav_from_file("resources/audio/taiko-normal-hitclap.wav",&loader);
            let hitsound_finish: Source = wav_from_file("resources/audio/taiko-normal-hitfinish.wav",&loader);
            let hitsound_whistle: Source = wav_from_file("resources/audio/taiko-normal-hitwhistle.wav",&loader);
            (music,hitsound_normal,hitsound_clap,hitsound_finish,hitsound_whistle)
        };

        engine.world.add_resource(Sounds {
            normal: hitsound_normal,
            clap: hitsound_clap,
            finish: hitsound_finish,
            whistle: hitsound_whistle,
         });

        let have_output = engine.world.read_resource::<Option<Output>>().is_some();

        let (big_hit_mesh, red_hit_mtl) =
            gen_complete_rect([0.010, 0.25], [1.0, 0., 0., 1.0], engine);
        let (small_hit_mesh, blue_hit_mtl) =
            gen_complete_rect([0.005, 0.15], [0., 0., 1.0, 1.0], engine);
        let (hit_judgement_mesh, hit_judgement_mtl) =
            gen_complete_rect([0.001, 0.25], [0., 1., 0., 1.], engine);

        
        if have_output {
            let mut dj = engine.world.write_resource::<Dj>();
            dj.set_volume(0.15);
            let music = music.clone();
            dj.set_picker(Box::new(move |ref mut dj| {
                dj.append(&music).expect("Decoder error occurred!");
                true
            }));
        }


        let text_material:AssetFuture<MaterialComponent> = load_material(engine, "/home/jojolepro/share/Prog/Rust/taiko-copy/resources/fonts/Arial.ttf", TTFFormat);
        //let text_material:AssetFuture<MaterialComponent> = load_material(engine, "/home/jojolepro/share/Prog/Rust/taiko-copy/resources/assets/Sword.png", PngFormat);
        let text_mesh:AssetFuture<MeshComponent> = {
            let verts = gen_rectangle(1., 1.);
            let mesh = Mesh::build(verts);
            let mesh = load_proc_asset(engine, move |engine| {
                let factory = engine.world.read_resource::<Factory>();
                factory.create_mesh(mesh).map(MeshComponent::new).map_err(
                    BoxedErr::new,
                )
            });
            mesh
        };

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
        /*let mut input = InputHandler::new();
        input.bindings = Bindings::load(format!(
            "{}/resources/input.ron",
            env!("CARGO_MANIFEST_DIR")
        ));

        world.add_resource(input);
        world.add_resource(Time::default());*/
        //world.add_resource(HitOffsets::new());


        let mut hitqueue = HitObjectQueue::new();
        for hit in &beatmap.objects {
            hitqueue.queue.push_back(hit.clone());

            let mut tr = LocalTransform::default();
            tr.translation = [0.0, 0.5, 0.0];
            let mtl = if hit.red {
                red_hit_mtl.clone()
            } else {
                blue_hit_mtl.clone()
            };
            let mesh = if hit.big{
                big_hit_mesh.clone()
            }else{
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

        /*let mut tr = LocalTransform::default();
        tr.translation = [0.5, 0.5, 0.1];
        world
            .create_entity()
            .with(text_mesh.clone())
            .with(text_material.clone())
            .with(tr)
            .with(Transform::default())
            .build();*/

        world.add_resource(hitqueue);

        world.add_resource(UserSettings{
            offset:0.0,
        });

        //add hit judgement On Time
        // 0.5 screen/sec, 25 ms = 0.0125 screens

        //Count bars from right to left. 1rst one is at the exact time the note hits,
        //then each is -25 ms from the one at the right of it.
        //Place your cursor around where you hear the note actually being played,
        //then count the bars.

        //Jojolepro's result -> 13 bars = -300 ms.  My normal offset on osu is -25 ms
        //With --release -> -75 to -125 ms
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

        let mut tr = LocalTransform::default();
            tr.translation = [0.3, 0.5, 0.0];
            world
                .create_entity()
                .with(hit_judgement_mesh.clone())
                .with(hit_judgement_mtl.clone())
                .with(tr)
                .with(Transform::default())
                .build();


        let mut tr = LocalTransform::default();
            tr.translation = [0.3 - (0.0125 * 2.), 0.5, 0.0];
            world
                .create_entity()
                .with(hit_judgement_mesh.clone())
                .with(hit_judgement_mtl.clone())
                .with(tr)
                .with(Transform::default())
                .build();

        world.add_resource(beatmap);

        let mut tr = LocalTransform::default();
            tr.translation = [0.3 - (0.0125 * -2.), 0.5, 0.0];
            world
                .create_entity()
                .with(hit_judgement_mesh.clone())
                .with(hit_judgement_mtl.clone())
                .with(tr)
                .with(Transform::default())
                .build();
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

//-------------------------//


//----------------GAME SYSTEMS----------

struct GameSystem;

impl<'a> System<'a> for GameSystem {
    type SystemData = (
     Entities<'a>,
     WriteStorage<'a, HitObject>,
     WriteStorage<'a, LocalTransform>,
     Fetch<'a, Camera>,
     Fetch<'a, Time>,
     Fetch<'a, InputHandler>,
     Fetch<'a, Sounds>,
     Fetch<'a, Option<Output>>,
     Fetch<'a, BeatMap>,
     Fetch<'a, Stopwatch>,
     FetchMut<'a, HitObjectQueue>,
     FetchMut<'a, HitOffsets>,
     FetchMut<'a, UserSettings>,
     );
    fn run(
        &mut self,
        (
         entities,
         mut hitobjects,
         mut transforms,
         cam,
         time,
         input,
         sounds,
         audio_output,
         beatmap,
         stopwatch,
         mut hitqueue,
         mut hitoffsets,
         mut user_settings,
         ):
         Self::SystemData,
    ){
        let cur_time = stopwatch.elapsed();
        let cur_time = duration_to_secs(cur_time);

        let cur_time = cur_time + user_settings.offset;

        let r1 = pressed(VirtualKeyCode::Z,&*input);
        let r2 = pressed(VirtualKeyCode::X,&*input);
        let b1 = pressed(VirtualKeyCode::N,&*input);
        let b2 = pressed(VirtualKeyCode::M,&*input);
        //println!("{} {} {} {}",r1,r2,b1,b2);

        let offset_up = pressed(VirtualKeyCode::Equals,&*input);
        let offset_down = pressed(VirtualKeyCode::Subtract,&*input);

        if offset_up{
            user_settings.offset = user_settings.offset + 0.005;
            println!("Offset: {} ms",user_settings.offset*1000.0);
        }else if offset_down{
            user_settings.offset = user_settings.offset - 0.005;
            println!("Offset: {} ms",user_settings.offset*1000.0);
        }

        let mut dropped_offsets = Vec::new();
        while let Some(head) = (&mut hitqueue.queue).pop_front(){
            if head.time + beatmap.maxhitoffset < cur_time as f32{
                hitoffsets.offsets.push(None);
                dropped_offsets.push(head.time);
                //println!("Dropped object");
            }else{
                hitqueue.queue.push_front(head);
                break;
            }
        }

        if r1 || r2 || b1 || b2 {
            let (red, dual) = get_key_press_type(r1,r2,b1,b2);

            if let Some(ref output) = *audio_output {
                if red{
                    play_once(&sounds.normal, 0.10, &output);
                }else{
                    play_once(&sounds.clap, 0.10, &output);
                }
                if dual{
                    play_once(&sounds.finish, 0.10, &output);
                }
            }

            //Get clickable object
            if let Some(head) = (&mut hitqueue.queue).pop_front() {
                if let (Some(offset), clicked) = check_hit(&beatmap,&head, cur_time, red, dual) {
                    if clicked {
                        hitoffsets.offsets.push(Some(offset));
                    } else {
                        hitoffsets.offsets.push(None);
                    }
                    dropped_offsets.push(head.time);
                } else {
                    //Put back into list if pressed but no hitobject was found
                    hitqueue.queue.push_front(head);
                }
            }
        }

        //println!("cur_time: {}", cur_time);
        'outer: for (entity,obj, tr) in (&*entities,&mut hitobjects, &mut transforms).join() {
            //Drop objects that weren't clicked fast enough
            for dropped_offset in dropped_offsets.iter(){
                    if *dropped_offset == obj.time{
                        //Drop visual object
                        //println!("Dropped entity");
                        entities.delete(entity);
                        //continue 'outer;
                    }
            }
            //Update object position
            tr.translation[0] = ((obj.time - cur_time) * 0.50) + 0.3; //TEMPORARY. TO TEST HIT JUDGEMENT
        }
    }
}

//-------------------------//

//----------GAME LOCAL FUNCTIONS---------

fn pressed(key:VirtualKeyCode,input:&InputHandler)->bool{
    input.key_is(key,ButtonState::Pressed(ChangeState::ThisFrame))
}

fn get_key_press_type(z: bool, x: bool, two: bool, three: bool) -> (bool, bool) {
    let dual = (z && x) || (two && three);
    let red = z || x;
    (red, dual)
}

///Returns hit offset
///Found no objects to hit, no offset  (None,false)
///Found an object to hit, used wrong button (Some(offset),false)
///Found an object to hit, used right button  (Some(offset),true)
fn check_hit(beatmap: &BeatMap,hit:&HitObject, time: f32, redpressed: bool, dual: bool) -> (Option<f32>, bool) {
    //for hit in &beatmap.objects {
        if value_near(time, hit.time, beatmap.maxhitoffset) {
            if (hit.red && redpressed) || (!hit.red && !redpressed) {
                if (hit.big && dual) || (!hit.big && !dual) {
                    println!("GOOD HIT @ {}, hit.time {}", time,hit.time);
                    return (Some(time - hit.time), true);
                } else {
                    println!("Wrong dual @ {}, hit.time {}",time,hit.time);
                    return (Some(time - hit.time), false);
                }
            }
            println!("Wrong key >_<");
            return (Some(time - hit.time), false);
        }
    //}
    return (None, false);
}

fn read_beatmap() -> BeatMap {
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
            let split: Vec<&str> = line.split(",").collect();

            if split.len() != 6 {
                continue;
            }
            let objecttype = split[4 as usize].parse::<u8>().expect(
                "Failed to parse as u8",
            );
            let (red, big) = match objecttype {
                0 => (true, false),//small red
                4 => (true, true),//big red
                8 => (false, false),//small blue
                12 => (false, true),//big blue
                c => panic!("Unknown hitobject color type: {}", c),
            };
            hitobjects.push(HitObject {
                red: red,
                time: osu_to_real_time(split[2 as usize].parse::<i32>().expect(
                    "Failed to parse as u8",
                )),
                big: big,
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
fn osu_to_real_time(time: i32) -> f32 {
    time as f32 / 1000.0
}
//-------------------------//

//----------UTILS----------

struct FPSCounterBundle{
    samplesize:usize,
}
impl FPSCounterBundle{
    fn new(samplesize:usize) -> Self {
        Self {
            samplesize:samplesize,
        }
    }
}
impl<'a, 'b, T> ECSBundle<'a, 'b, T> for FPSCounterBundle {
    fn build(
        &self,
        builder: ApplicationBuilder<'a, 'b, T>,
    ) -> Result<ApplicationBuilder<'a, 'b, T>> {
        Ok(
            builder
                .with_resource(FPSCounter::new(20))
                .with::<FPSCounterSystem>(FPSCounterSystem, "fps_counter_system", &[])
        )
    }
}

fn load_material<F>(engine: &mut Engine, albedo: &str, format: F) -> AssetFuture<MaterialComponent>
where
    F: Format + 'static,
    F::Data: Into<<TextureContext as Context>::Data>,
{
    let future = {
        let factory = engine.world.read_resource::<Factory>();
        factory.create_material(MaterialBuilder::new()).map_err(
            BoxedErr::new,
        )
    }.join({
        let loader = engine.world.read_resource::<Loader>();
        loader.load_from::<TextureComponent, _, _, _>(albedo, format, "")
    })
        .map(|(mut mtl, albedo)| {
            mtl.albedo = albedo.0.inner();
            MaterialComponent(mtl)
        });
    AssetFuture::from_future(future)
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

fn value_near<B: Add<Output = B> + Sub<Output = B> + PartialOrd + Copy>(
    number: B,
    target: B,
    margin: B,
) -> bool {
    number >= target - margin && number <= target + margin
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

fn wav_from_file(path:&str,loader:&Loader)->Source{
    loader.load_from(
            format!(
                "{}/{}",
                env!("CARGO_MANIFEST_DIR"),
                path,
            ),
            WavFormat,
            "",
        ).wait().unwrap()
}

//---------------------------//

//--------------TTF----------

struct TTFFormat;

impl Format for TTFFormat {
    const EXTENSIONS: &'static [&'static str] = &["ttf"];
    type Data = ImageData;
    type Error = ImageError;
    type Result = ImageFuture;

    fn parse(&self, bytes: Vec<u8>, pool: &ThreadPool) -> Self::Result {
        ImageFuture::spawn(pool, move || {
            load_ttf(bytes).map(|raw| ImageData { raw })
        })
    }
}


fn load_ttf(bytes:Vec<u8>)->imagefmt::Result<Image<u8>>{

    //let font_data = include_bytes!("/home/jojolepro/share/Prog/Rust/taiko-copy/resources/fonts/Arial.ttf");
    let font_data = bytes;
    let collection = FontCollection::from_bytes(&font_data[..] as &[u8]);
    let font = collection.into_font().unwrap(); // only succeeds if collection consists of one font

    // Desired font pixel height
    let height: f32 = 600.0; //default: 12.4 to get 80 chars across (fits most terminals); adjust as desired
    let pixel_height = height.ceil() as usize;

    // 2x scale in x direction to counter the aspect ratio of monospace characters.
    let scale = Scale { x: height*2.0, y: height };

    // The origin of a line of text is at the baseline (roughly where non-descending letters sit).
    // We don't want to clip the text, so we shift it down with an offset when laying it out.
    // v_metrics.ascent is the distance between the baseline and the highest edge of any glyph in
    // the font. That's enough to guarantee that there's no clipping.
    let v_metrics = font.v_metrics(scale);
    let offset = point(0.0, v_metrics.ascent);

    // Glyphs to draw for "RustType". Feel free to try other strings.
    let glyphs: Vec<PositionedGlyph> = font.layout("A", scale, offset).collect();

    let width = glyphs.iter().rev()
        .filter_map(|g| g.pixel_bounding_box()
                    .map(|b| b.min.x as f32 + g.unpositioned().h_metrics().advance_width))
        .next().unwrap_or(0.0).ceil() as usize;

    //fill with 255,255,255,255 and set 4th to grayscale
    let mut pixel_data = vec![255 as u8; width * pixel_height * 4];
    for g in glyphs {
        if let Some(bb) = g.pixel_bounding_box() {
            g.draw(|x, y, v| {
                // v should be in the range 0.0 to 1.0
                //let i = (v*mapping_scale + 0.5) as usize;
                // so something's wrong if you get $ in the output.
                //let c = mapping.get(i).cloned().unwrap_or(b'$');
                let c = (v * 255.0).ceil() as u8;

                let x = x as i32 + bb.min.x;
                let y = y as i32 + bb.min.y;
                // There's still a possibility that the glyph clips the boundaries of the bitmap
                if x >= 0 && x < width as i32 && y >= 0 && y < pixel_height as i32 {
                    let x = x as usize;
                    let y = y as usize;
                    println!("x {}, y {}, v {}",x,y,v);
                    println!("Setting pixel alpha, {}:{}",((x + y * width) * 4 + 3 ),c);
                    pixel_data[(x + y * width) * 4 + 3 ] = c;

                    //FOR TESTING
                    pixel_data[(x + y * width) * 4 ] = c;
                    pixel_data[(x + y * width) * 4 +1] = c;
                    pixel_data[(x + y * width) * 4 +2] = c;
                }
            })
        }
    }

    Ok(Image::<u8> {
        w   : width as usize,
        h   : pixel_height as usize,
        fmt : ColFmt::RGBA,
        buf : pixel_data,
    })

    // Find the most visually pleasing width to display
    /*let width = glyphs.iter().rev()
        .filter_map(|g| g.pixel_bounding_box()
                    .map(|b| b.min.x as f32 + g.unpositioned().h_metrics().advance_width))
        .next().unwrap_or(0.0).ceil() as usize;

    println!("width: {}, height: {}", width, pixel_height);*/


/*
    // Rasterise directly into ASCII art.
    let mut pixel_data = vec![b'@'; width * pixel_height];
    let mapping = b"@%#x+=:-. "; // The approximation of greyscale
    let mapping_scale = (mapping.len()-1) as f32;
    for g in glyphs {
        if let Some(bb) = g.pixel_bounding_box() {
            g.draw(|x, y, v| {
                // v should be in the range 0.0 to 1.0
                let i = (v*mapping_scale + 0.5) as usize;
                // so something's wrong if you get $ in the output.
                let c = mapping.get(i).cloned().unwrap_or(b'$');
                let x = x as i32 + bb.min.x;
                let y = y as i32 + bb.min.y;
                // There's still a possibility that the glyph clips the boundaries of the bitmap
                if x >= 0 && x < width as i32 && y >= 0 && y < pixel_height as i32 {
                    let x = x as usize;
                    let y = y as usize;
                    pixel_data[(x + y * width)] = c;
                }
            })
        }
    }

    // Print it out
    let stdout = ::std::io::stdout();
    let mut handle = stdout.lock();
    for j in 0..pixel_height {
        handle.write(&pixel_data[j*width..(j+1)*width]).unwrap();
        handle.write(b"\n").unwrap();
    }
    */
}
//-------------------------//