extern crate amethyst;

use amethyst::assets::AssetStorage;
use amethyst::audio::output::Output;
use amethyst::audio::Source;
use amethyst::core::timing::Time;
use amethyst::core::transform::Transform;
use amethyst::ecs::prelude::*;
use amethyst::input::InputEvent;
use amethyst::input::InputHandler;
use amethyst::shrev::{EventChannel, ReaderId};
use amethyst::winit::VirtualKeyCode;

use components::*;
use resources::*;
use utils::*;

pub struct GameSystem {
    pub reader_id: Option<ReaderId<InputEvent<String>>>,
    pub start_time: f64,
}

impl<'a> System<'a> for GameSystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, HitObject>,
        WriteStorage<'a, Transform>,
        Read<'a, AssetStorage<Source>>,
        Read<'a, Time>,
        Read<'a, InputHandler<String, String>>,
        ReadExpect<'a, Sounds>,
        Option<Read<'a, Output>>,
        Read<'a, BeatMap>,
        Write<'a, EventChannel<InputEvent<String>>>,
        Write<'a, HitObjectQueue>,
        Write<'a, HitOffsets>,
        Write<'a, UserSettings>,
    );
    fn run(
        &mut self,
        (
            entities,
            mut hitobjects,
            mut transforms,
            audio,
            time,
            _input,
            sounds,
            audio_output,
            beatmap,
            mut events,
            mut hitqueue,
            mut hitoffsets,
            mut user_settings,
        ): Self::SystemData,
    ) {
        if self.reader_id.is_none() {
            self.reader_id = Some(events.register_reader());
        }

        if self.start_time <= 0.0 {
            self.start_time = time.absolute_time_seconds();
        }

        let cur_time = time.absolute_time_seconds() - self.start_time;

        let cur_time = cur_time + user_settings.offset;

        let (mut r1, mut r2, mut b1, mut b2, mut offset_up, mut offset_down) =
            (false, false, false, false, false, false);
        for ev in events.read(self.reader_id.as_mut().unwrap()) {
            match ev {
                &InputEvent::KeyPressed { key_code, .. } => match key_code {
                    VirtualKeyCode::Z => r1 = true,
                    VirtualKeyCode::X => r2 = true,
                    VirtualKeyCode::N => b1 = true,
                    VirtualKeyCode::M => b2 = true,
                    VirtualKeyCode::Equals => offset_up = true,
                    VirtualKeyCode::Subtract => offset_down = true,
                    _ => {}
                },
                &InputEvent::KeyReleased { .. } => {}
                _ => {}
            }
        }

        if offset_up {
            user_settings.offset = user_settings.offset + 0.005;
            println!("Offset: {} ms", user_settings.offset * 1000.0);
        } else if offset_down {
            user_settings.offset = user_settings.offset - 0.005;
            println!("Offset: {} ms", user_settings.offset * 1000.0);
        }

        let mut dropped_offsets = Vec::new();
        while let Some(head) = (&mut hitqueue.queue).pop_front() {
            if head.time + beatmap.maxhitoffset < cur_time {
                hitoffsets.offsets.push(None);
                dropped_offsets.push(head.time);
            } else {
                // put back into the list
                hitqueue.queue.push_front(head);
                break;
            }
        }

        if r1 || r2 || b1 || b2 {
            let (red, dual) = get_key_press_type(r1, r2, b1, b2);

            if let Some(output) = audio_output {
                if red {
                    output.play_once(
                        audio
                            .get(&sounds.normal)
                            .expect("Failed to find normal hitsound"),
                        1.0,
                    );
                } else {
                    output.play_once(
                        audio
                            .get(&sounds.clap)
                            .expect("Failed to find clap hitsound"),
                        1.0,
                    );
                }
                if dual {
                    output.play_once(
                        audio
                            .get(&sounds.finish)
                            .expect("Failed to find finish hitsound"),
                        1.0,
                    );
                }
            } else {
                error!("Failed to find audio `Output` from system.");
            }

            //Get clickable object
            if let Some(head) = (&mut hitqueue.queue).pop_front() {
                if let (Some(offset), clicked) = check_hit(&beatmap, &head, cur_time, red, dual) {
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
        'outer: for (entity, obj, tr) in (&*entities, &mut hitobjects, &mut transforms).join() {
            //Drop objects that weren't clicked fast enough
            for dropped_offset in dropped_offsets.iter() {
                if *dropped_offset == obj.time {
                    //Drop visual object
                    //println!("Dropped entity");
                    match entities.delete(entity) {
                        Ok(_) => {}
                        Err(err) => {
                            error!("Failed to delete entity {:?} because {:?}", entity, err)
                        }
                    }
                    //continue 'outer;
                }
            }
            //Update object position
            tr.translation[0] = (((obj.time - cur_time) * 0.50) + 0.3) as f32; //TEMPORARY. TO TEST HIT JUDGEMENT
        }
    }
}
