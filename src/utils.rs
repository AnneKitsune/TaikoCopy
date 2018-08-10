extern crate amethyst;
extern crate itertools;

//use self::itertools::Itertools;

use amethyst::assets::{AssetStorage, Handle, Loader,SimpleFormat};
use amethyst::renderer::{
    Event, KeyboardInput, Material, MaterialDefaults, Mesh, PngFormat, PosTex, Texture,
    TextureMetadata, WindowEvent,
};
use amethyst::winit::VirtualKeyCode;
use amethyst::Result;
use amethyst_extra::*;

use std::fs;
use std::fs::File;
use std::io::Read;
use std::ops::{Add, Sub};

use components::*;
use resources::*;

#[derive(PartialEq)]
pub enum RemovalLayer {
    SongSelect,
    Gameplay,
}

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

/*#[derive(Clone)]
pub struct BeatMapFormat;

impl SimpleFormat<BeatMap> for BeatMapFormat {
    const NAME: &'static str = "osu";
    type Options = ();
    fn import(&self, bytes: Vec<u8>, _: ()) -> Result<BeatMap> {
        let content = str::from_utf8(bytes.as_slice())?;
        //
        let mut hitobjects: Vec<HitObject> = vec![];
    let mut mode = "";
    let mut songpath = "";
    for line in content.lines() {
        if line == "[HitObjects]" {
            mode = "HitObjects";
        } else if line == "[General]" {
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
                _ => (false, false), // We don't know what this is, but it happens. Probably sliders.
            };
            hitobjects.push(HitObject {
                red: red,
                time: osu_to_real_time(
                    split[2 as usize]
                        .parse::<i32>()
                        .expect("Failed to parse hitobject time as i32."),
                ),
                big: big,
            });
        }
    }
        //
        Ok(BeatMap {
            name: String::from(songpath),
            songpath: format!("{}/{}", folder, songpath),
            objects: hitobjects,
            maxhitoffset: 0.05,
        })
    }
}*/

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
        } else if line == "[General]" {
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
                _ => (false, false), // We don't know what this is, but it happens. Probably sliders.
            };
            hitobjects.push(HitObject {
                red: red,
                time: osu_to_real_time(
                    split[2 as usize]
                        .parse::<i32>()
                        .expect("Failed to parse hitobject time as i32."),
                ),
                big: big,
            });
        }
    }
    Some(BeatMap {
        name: String::from(songpath),
        songpath: format!("{}/{}", folder, songpath),
        objects: hitobjects,
        maxhitoffset: 0.05,
    })
}

pub fn osu_to_real_time(time: i32) -> f64 {
    time as f64 / 1000.0
}
