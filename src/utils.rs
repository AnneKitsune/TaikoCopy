extern crate amethyst;
extern crate itertools;

use self::itertools::Itertools;

use amethyst::assets::{AssetStorage, Handle, Loader};
use amethyst::renderer::{ImageError,Event,KeyboardInput, Material, MaterialDefaults, Mesh,
                         PngFormat, PosTex, Texture, TextureData, TextureMetadata,WindowEvent};
use amethyst::audio::WavFormat;
use amethyst::audio::Source;
use amethyst::input::InputEvent;
use amethyst::prelude::*;
use amethyst::winit::VirtualKeyCode;

use std::io::Read;
use std::ops::{Add, Sub};
use std::fs::File;
use std::fs;


use resources::*;
use components::*;

pub fn get_key_press_type(z: bool, x: bool, two: bool, three: bool) -> (bool, bool) {
    let dual = (z && x) || (two && three);
    let red = z || x;
    (red, dual)
}

///Returns hit offset
///Found no objects to hit, no offset  (None,false)
///Found an object to hit, used wrong button (Some(offset),false)
///Found an object to hit, used right button  (Some(offset),true)
pub fn check_hit(
    beatmap: &BeatMap,
    hit: &HitObject,
    time: f64,
    redpressed: bool,
    dual: bool,
) -> (Option<f64>, bool) {
    //for hit in &beatmap.objects {
    if value_near(time, hit.time, beatmap.maxhitoffset) {
        if (hit.red && redpressed) || (!hit.red && !redpressed) {
            if (hit.big && dual) || (!hit.big && !dual) {
                println!("GOOD HIT @ {}, hit.time {}", time, hit.time);
                return (Some(time - hit.time), true);
            } else {
                println!("Wrong dual @ {}, hit.time {}", time, hit.time);
                return (Some(time - hit.time), false);
            }
        }
        println!("Wrong key >_<");
        return (Some(time - hit.time), false);
    }
    //}
    return (None, false);
}

pub fn beatmap_list(maps_folder: &String) -> Vec<BeatMap> {
    //let paths = fs::read_dir(maps_folder).expect(&*format!("Failed to read map folder @ {}",maps_folder));
    /*for path in paths{
        println!("Paths: {}",path.unwrap().path().display());
    }*/
    list_directory(maps_folder)
        .into_iter()
        .map(|m| {
            println!("BeatMap folder path {}", m);
            list_directory(&m)
                .into_iter()
                .filter(|diff| diff.ends_with(".osu"))
                .map(|diff| {
                    println!("BeatMap diff path {}", m);
                    read_beatmap(&m, &diff)
                })
                .flatten()
                .collect::<Vec<BeatMap>>()
        })
        .flatten()
        .collect()
}

pub fn list_directory(dir: &String) -> Vec<String> {
    fs::read_dir(dir)
        .expect(&*format!("Failed to read directory {}", dir))
        .map(|e| {
            String::from(
                e.expect("Failed to read file path.")
                    .path()
                    .to_str()
                    .unwrap(),
            )
        })
        .collect()
}

pub fn read_beatmap(folder_path: &String, difficulty_path: &String) -> Option<BeatMap> {
    let folder = folder_path;
    let mut file = File::open(format!("{}", difficulty_path)).expect("Failed to open beatmap file");
    let mut content = String::new();
    file.read_to_string(&mut content)
        .expect("Failed to read beatmap file");

    let mut hitobjects: Vec<HitObject> = vec![];
    let mut mode = "";
    let mut songpath = "";
    for line in content.lines() {
        if line == "[HitObjects]" {
            mode = "HitObjects";
        }else if line == "[General]"{
            mode = "General";
        }
        if mode == "General" {
            if line.starts_with("AudioFilename:") {
                songpath = &line[15..];
                if !songpath.ends_with(".ogg") {
                    return None;
                }
            }
            if line.starts_with("Mode:") {
                // 1=taiko,3=mania
                if &line[6..] != "1" {
                    return None;
                }
            }
        }
        if mode == "HitObjects" {
            let split: Vec<&str> = line.split(",").collect();

            if split.len() != 6 {
                continue;
            }
            let objecttype = split[4 as usize]
                .parse::<u8>()
                .expect("Failed to parse as u8");
            let (red, big) = match objecttype {
                0 => (true, false),  //small red
                4 => (true, true),   //big red
                8 => (false, false), //small blue
                12 => (false, true), //big blue
                c => (false,false)//panic!("Unknown hitobject color type: {}", c),
            };
            hitobjects.push(HitObject {
                red: red,
                time: osu_to_real_time(
                    split[2 as usize]
                        .parse::<i32>()
                        .expect("Failed to parse as u8"),
                ),
                big: big,
            });
        }
    }
    Some(BeatMap {
        name: String::from("Test beatmap"),
        songpath: format!("{}/{}", folder, songpath),
        objects: hitobjects,
        maxhitoffset: 0.05,
    })
}
pub fn osu_to_real_time(time: i32) -> f64 {
    time as f64 / 1000.0
}


pub fn value_near<B: Add<Output = B> + Sub<Output = B> + PartialOrd + Copy>(
    number: B,
    target: B,
    margin: B,
) -> bool {
    number >= target - margin && number <= target + margin
}

///Possible optimisation VS usability loss: Taking &Loader in parameter
pub fn texture_from_png_simple(
    path: &str,
    loader: &Loader,
    storage: &AssetStorage<Texture>,
) -> Handle<Texture> {
    loader.load(path, PngFormat, TextureMetadata::default(), (), &storage)
}
pub fn material_from_png_simple(
    path: &str,
    loader: &Loader,
    storage: &AssetStorage<Texture>,
    material_defaults: &MaterialDefaults,
) -> Material {
    material_from_texture(
        texture_from_png_simple(path, loader, storage),
        material_defaults,
    )
}
pub fn material_from_color(
    color: [f32; 4],
    loader: &Loader,
    storage: &AssetStorage<Texture>,
    material_defaults: &MaterialDefaults,
) -> Material {
    let albedo = loader.load_from_data(color.into(), (), &storage);
    material_from_texture(albedo, material_defaults)
}

pub fn material_from_texture(texture: Handle<Texture>, defaults: &MaterialDefaults) -> Material {
    Material {
        albedo: texture,
        ..defaults.0.clone()
    }
}
pub fn gen_rectangle_mesh(
    w: f32,
    h: f32,
    loader: &Loader,
    storage: &AssetStorage<Mesh>,
) -> Handle<Mesh> {
    let mut verts = gen_rectangle_vertices(w, h);
    loader.load_from_data(verts.into(), (), &storage)
}
pub fn gen_rectangle_vertices(w: f32, h: f32) -> Vec<PosTex> {
    let data: Vec<PosTex> = vec![
        PosTex {
            position: [-w / 2., -h / 2., 0.],
            tex_coord: [0., 0.],
        },
        PosTex {
            position: [w / 2., -h / 2., 0.],
            tex_coord: [1., 0.],
        },
        PosTex {
            position: [w / 2., h / 2., 0.],
            tex_coord: [1., 1.],
        },
        PosTex {
            position: [w / 2., h / 2., 0.],
            tex_coord: [1., 1.],
        },
        PosTex {
            position: [-w / 2., h / 2., 0.],
            tex_coord: [0., 1.],
        },
        PosTex {
            position: [-w / 2., -h / 2., 0.],
            tex_coord: [0., 0.],
        },
    ];
    data
}

pub fn wav_from_file(
    path: &str,
    loader: &Loader,
    storage: &AssetStorage<Source>,
) -> Handle<Source> {
    loader.load(
        format!("{}/{}", env!("CARGO_MANIFEST_DIR"), path,),
        WavFormat,
        (),
        (),
        &storage,
    )
}

pub fn key_pressed_from_event(key: VirtualKeyCode, event: &Event) -> bool{
    match event {
        &Event::WindowEvent { ref event, .. } => match event {
            &WindowEvent::KeyboardInput {
                input:
                KeyboardInput {
                    virtual_keycode: Some(k),
                    ..
                },
                ..
            } => k == key,
            _ => false,
        },
        _ => false,
    }
}

pub fn window_closed(event: &Event) -> bool{
    match event {
        &Event::WindowEvent { ref event, .. } => match event {
            &WindowEvent::Closed => true,
            _ => false,
        },
        _ => false,
    }
}
