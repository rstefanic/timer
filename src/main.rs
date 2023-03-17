extern crate sdl2;

use sdl2::{
    event::{Event, WindowEvent},
    keyboard::Keycode,
    pixels::Color,
    rect::Rect,
    rwops::RWops,
    ttf,
};

use std::thread;
use std::time::Duration;

const NANOS_PER_SEC: u32 = 1_000_000_000;
const FPS: u32 = 60;
const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

fn parse_timer(value: &String) -> Result<f64, String> {
    let timer_string_split = value.split(':');

    if timer_string_split.clone().count() > 3 {
        return Err(
            "Invalid timer: countdown timer can only have 3 parts at most (hh:mm:ss)".to_string(),
        );
    }

    // Walk the split time string backwards and add up the seconds.
    // By doing this it's easier to convert the string into base
    // 60 since we can mulitple the iteration value by 60 ^ i.
    Ok(timer_string_split
        .rev()
        .enumerate()
        .fold(0, |acc, (i, time_string)| {
            let parsed_time_string = time_string.parse::<u32>().ok().unwrap();
            acc + parsed_time_string * u32::pow(60, i as u32)
        }) as f64)
}

fn main() -> Result<(), String> {
    let args: Vec<String> = ::std::env::args().collect();

    if args.len() < 2 {
        panic!("Invalid Argument: Time must be specified");
    }

    let mut timer = parse_timer(&args[1])?;
    let mut width: i32 = WIDTH as i32;
    let mut height: i32 = HEIGHT as i32;

    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let window = video_subsystem
        .window("timer", width as u32, height as u32)
        .position_centered()
        .resizable()
        .build()
        .map_err(|e| e.to_string())?;

    let font_bytes = include_bytes!("../assets/Roboto-Regular.ttf");
    let ttf_handler = ttf::init().unwrap();
    let font = ttf_handler.load_font_from_rwops(RWops::from_bytes(font_bytes).unwrap(), 128)?;

    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    let texture_creator = canvas.texture_creator();
    let background_color = Color::RGB(0, 0, 0);
    canvas.set_draw_color(background_color);
    canvas.clear();
    canvas.present();

    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut blink_timer = 0.0;
    let mut paused = false;
    let mut user_notified_finished_timer = false;

    'main_loop: loop {
        let active_timer = timer > 0.0;

        if !active_timer && !user_notified_finished_timer {
            canvas.window_mut().flash(sdl2::video::FlashOperation::UntilFocused)?;
            user_notified_finished_timer = true;
        }

        /****************************
         *** POLL EVENTS *************
         ****************************/

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'main_loop,
                Event::KeyDown {
                    keycode: Some(Keycode::Space),
                    ..
                } => {
                    if active_timer {
                        paused = !paused;
                    }
                }
                Event::Window { win_event, .. } => {
                    if let WindowEvent::Resized(w, h) = win_event {
                        width = w;
                        height = h;
                    }
                }
                _ => {}
            }
        }

        /****************************
         *** UPDATE TIMER ************
         ****************************/

        let sleep_time = NANOS_PER_SEC / FPS;
        thread::sleep(Duration::new(0, sleep_time));

        if active_timer && !paused {
            timer -= 1f64 / (FPS as f64);
        } else if !active_timer {
            blink_timer += 1f64 / (FPS as f64);
        }

        /****************************
         *** RENDER ******************
         ****************************/

        let time_to_format = f64::max(timer, 0.0);

        // If we don't call `floor()` on the hour and minutes, the format
        // string will round the whole number portion of the float and
        // give us oddly formatted times that don't make any sense.
        let formatted_timer = format!(
            "{:0>2.0}:{:0>2.0}:{:0>5.2}",
            (time_to_format / 60.0 / 60.0).floor(),
            (time_to_format / 60.0 % 60.0).floor(),
            time_to_format % 60.0,
        );

        let font_color = match paused {
            true => Color::RGB(120, 120, 120),
            _ => Color::RGB(255, 255, 255),
        };

        let pre_texture = font.render(&formatted_timer).solid(font_color).unwrap();
        let texture = pre_texture.as_texture(&texture_creator).unwrap();
        canvas.set_draw_color(background_color);
        canvas.clear();

        // Once `active_timer` is false, we flash the completed
        // timer on the screen every half second; so we need
        // to set `flash_timer` every half second for it.
        let flash_timer = (blink_timer % 1.0) < 0.5;

        if active_timer || flash_timer {
            const PADDING: f32 = 0.1;
            const TEXT_SIZE: f32 = 0.8;

            canvas
                .copy(
                    &texture,
                    None,
                    Rect::new(
                        (width as f32 * PADDING) as i32,
                        (height as f32 * PADDING) as i32,
                        (width as f32 * TEXT_SIZE) as u32,
                        (height as f32 * TEXT_SIZE) as u32,
                    ),
                )
                .expect("Error writing texture");
        }

        canvas.present();
    }

    Ok(())
}

#[test]
fn it_should_parse_a_time_with_only_seconds() {
    assert_eq!(10.0, parse_timer(&"10".to_string()).unwrap());
}

#[test]
fn it_should_parse_a_time_with_minutes_and_seconds() {
    assert_eq!(70.0, parse_timer(&"01:10".to_string()).unwrap());
}

#[test]
fn it_should_parse_a_time_with_hours_minutes_and_seconds() {
    assert_eq!(3670.0, parse_timer(&"01:01:10".to_string()).unwrap());
}
