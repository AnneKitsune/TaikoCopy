//! A clone of the osu!taiko gamemode implemented using amethyst

//#![deny(missing_docs,dead_code)]
extern crate amethyst;
extern crate futures;
extern crate imagefmt;
extern crate rayon;
extern crate rusttype;
extern crate time;
extern crate winit;
#[macro_use]
extern crate log;
extern crate core;
extern crate amethyst_extra;

use std::time::Duration;
use amethyst::audio::{AudioBundle, SourceHandle};
use amethyst::core::frame_limiter::FrameRateLimitStrategy;
use amethyst::core::transform::TransformBundle;
use amethyst::core::Time;
use amethyst::input::InputBundle;
use amethyst::prelude::*;
use amethyst::audio::Source;
use amethyst::utils::fps_counter::FPSCounterBundle;
//use amethyst::renderer::{DisplayConfig, DrawFlat, Pipeline, PosTex, RenderBundle,
//                        Stage};
use amethyst::renderer::*;
use amethyst_extra::*;
use std::env;

mod systems;
mod states;
mod resources;
mod components;
mod utils;

use states::*;
use resources::*;

fn main() -> amethyst::Result<()>{
    amethyst::start_logger(Default::default());
    // run_dir() -> String
    /*let bin_path = env::args().next().expect("Failed to get binary executable path");
    let last_slash_index = bin_path.rfind("/").expect("Failed to get last slash in binary path.");
    let mut base_path = bin_path[..last_slash_index].to_string();

    if base_path.contains("target/"){
        base_path = String::from(".");
    }*/
    let base_path = get_working_dir();
    let asset_loader = AssetLoader::new(
        &format!("{}/assets", base_path).to_string(),
        "base",
    );
    let display_config_path = asset_loader.resolve_path("config/display.ron").unwrap();
    let key_bindings_path = asset_loader.resolve_path("config/input.ron").unwrap();



    /*let path = format!("{}/resources/config.ron", env!("CARGO_MANIFEST_DIR"));
    let display_config = DisplayConfig::load(path);

    let paths = Paths::from_file(&format!("{}/paths.ron", env!("CARGO_MANIFEST_DIR")));
    let input_path = paths
        .path("input")
        .expect("Failed to find input config path")
        .clone();
    println!("{}", input_path);*/

    /*let pipe = Pipeline::build().with_stage(
        Stage::with_backbuffer()
            //.clear_target([255.0, 105.0, 180.0, 1.0], 1.0)
            .clear_target([1.0, 0.5, 0.75, 1.0], 1.0)
            .with_pass(DrawFlat::<PosTex>::new()),
    );
    //let maps_dir = format!("{}/resources/assets/maps/", env!("CARGO_MANIFEST_DIR"));
    let game = Application::build("", MenuState)
        .unwrap()
        .with_frame_limit(
            FrameRateLimitStrategy::SleepAndYield(Duration::from_millis(2)),
            144,
        )
        .with_resource(paths)
        .with_bundle(FPSCounterBundle::new(20))
        .expect("Failed to create FPSCounterBundle")
        .with_bundle(InputBundle::<String, String>::new().with_bindings_from_file(&input_path))
        .expect("Failed to load input bindings")
        .with_bundle(TransformBundle::new())
        .expect("Failed to build transform bundle")
        .with_bundle(AudioBundle::new(|music: &mut Time| None))
        .expect("Failed to build dj bundle")
        .with_bundle(RenderBundle::new(pipe, Some(display_config)))
        .expect("Failed to load render bundle");
    game.build().expect("Failed to build game").run();*/




    let game_data_builder = GameDataBuilder::default()
        .with_bundle(InputBundle::<String, String>::new().with_bindings_from_file(&key_bindings_path)?)
        .expect("Failed to load input bindings")
        .with_bundle(TransformBundle::new())
        .expect("Failed to build transform bundle")
        .with_bundle(AudioBundle::new(|music: &mut Music| music.music.next()))
        .expect("Failed to build dj bundle")
        //.with_bundle(RenderBundle::new(pipe, Some(display_config)))
        //.expect("Failed to build render bundle")
        .with_basic_renderer(display_config_path, DrawFlat::<PosTex>::new().with_transparency(ColorMask::all(), ALPHA, None), true)?;
    let resources_directory = format!("");
    Application::build(resources_directory, MenuState)?
        .with_resource(asset_loader)
        .with_resource(AssetLoaderInternal::<Mesh>::new())
        .with_resource(AssetLoaderInternal::<Texture>::new())
        .with_resource(AssetLoaderInternal::<Source>::new())
        .with_resource(Music{music: vec![].into_iter().cycle()})
        .build(game_data_builder)?
        .run();
    Ok(())
}

/*


Asset Loading API

In this document, mentally replace all references (&something) by a UUID/GUID/UID  (unique identifier).

Features:
-autoload using configured folder structure
-load from network (post release, after network is working)
-modding/override support (via next point)
-asset referencing using UUID


Folder Structure Example (configurable, of course):
Meshes -> Mesh1.obj
Textures -> tex1.png,tex2.jpg
Entities -> Entity1(&Mesh1,&tex1), Entity2(&Mesh1,&tex2)
Configurations -> display_conf.ron, user_prefs.ron
Languages->eng->language_pack1.ron
Scripts->script1.??

Networked Structure Example (flat structure, segmented by network packets):
(Mesh1,Mesh2,Texture1,Texture2,Entity1(&Mesh1,&Texture2),display_config.ron)


To keep track of which files are which type, a config structure will map folderpaths/networkoffset to a asset type. (Can be a downloadable template!)
Example:
./meshes/{mesh} -> Mesh
./languages/{lang}/{language_pack} -> Lang


Modding/Override Example
./base/meshes/mesh1.obj
./base/entities/Entity1(&mesh1)
./mod1/overrides/meshes/&mesh1.obj

*/

/*



Physics
Shape
Texture



duration
start speed
start size
start rotation
randomize rotation
start color (gradient)
gravity modifier
simulation space (local/global)
simulation speed
delta time (scaled/unscaled)
scaling mode (local/shape,hierarchy)
play on awake
emitter velocity (rigidbody/transform)
max particles
auto random seed
stop action(none/disable/destroy

Emission
rate over time
distance over time
Shape
shape (box,circle,sphere,etc..)
Emit from (volume/shell/edge)

Velocity over lifetime
Inherent velocity
Force over lifetime
Color over lifetime
Color by speed
Size over lifetime
Size by speed
Rotation over lifetime
Rotation by speed
External forces
Noise
Collision
Triggers
Sub emitters
Texture sheet animation
Lights
Trails
Custom data
Renderer

*/
