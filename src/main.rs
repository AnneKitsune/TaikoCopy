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
extern crate amethyst_extra;
extern crate core;

use amethyst::audio::AudioBundle;
use amethyst::audio::Source;
use amethyst::core::transform::TransformBundle;
use amethyst::input::InputBundle;
use amethyst::prelude::*;
use amethyst::renderer::*;
use amethyst::ui::{FontAsset, UiBundle};
use amethyst_extra::*;

mod components;
mod resources;
mod states;
mod systems;
mod utils;

use states::*;

fn main() -> amethyst::Result<()> {
    amethyst::start_logger(Default::default());
    
    let base_path = get_working_dir();
    let asset_loader = AssetLoader::new(&format!("{}/assets", base_path).to_string(), "base");
    let display_config_path = asset_loader.resolve_path("config/display.ron").unwrap();
    let key_bindings_path = asset_loader.resolve_path("config/input.ron").unwrap();

    let game_data_builder = GameDataBuilder::default()
        .with_bundle(InputBundle::<String, String>::new().with_bindings_from_file(&key_bindings_path)?)?
        .with_bundle(TransformBundle::new())?
        .with_bundle(AudioBundle::new(|music: &mut Music| music.music.next()))?
        .with_bundle(UiBundle::<String, String>::new())?
        .with(NormalOrthoCameraSystem::default(), "normal_cam", &[])
        .with_basic_renderer(display_config_path, DrawFlat::<PosTex>::new().with_transparency(ColorMask::all(), ALPHA, None), true)?;
    let resources_directory = format!("");
    Application::build(resources_directory, MenuState::new())?
        .with_resource(asset_loader)
        .with_resource(AssetLoaderInternal::<Mesh>::new())
        .with_resource(AssetLoaderInternal::<Texture>::new())
        .with_resource(AssetLoaderInternal::<Source>::new())
        .with_resource(AssetLoaderInternal::<FontAsset>::new())
        .with_resource(Music {
            music: vec![].into_iter().cycle(),
        })
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
