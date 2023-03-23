extern crate sdl2;

#[cfg(all(unix, not(target_os = "macos")))]
use dbus::{
    arg::messageitem::{MessageItem, MessageItemArray},
    ffidisp::Connection,
    Message,
};

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
const VELOCITY_SPEED: i32 = 3;
const TEXT_PADDING: f32 = 0.1;
const TEXT_SIZE: f32 = 0.8;
const DVD_FONT_SCALE: f32 = 0.25;

#[derive(PartialEq)]
enum DisplayMode {
    Default,
    DVD,
}

#[derive(Clone, Copy, Debug)]
struct Velocity {
    x: i32,
    y: i32,
}

#[derive(Debug)]
struct TimerDisplay {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    velocity: Option<Velocity>,
}

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
    let mut args = ::std::env::args();
    let mut timer: Option<f64> = None;
    let mut display_mode = DisplayMode::Default;

    // Shift one to move off the executable name
    args.next();

    for arg in args {
        match arg.as_str() {
            "--dvd" => display_mode = DisplayMode::DVD,
            _ => timer = Some(parse_timer(&arg)?),
        }
    }

    if timer == None {
        return Err("Missing timer".to_string());
    }

    // Redeclare the timer so we can just reference the value directly
    let mut timer = timer.unwrap();
    let mut timer_display = TimerDisplay {
        x: 0,
        y: 0,
        width: 0,
        height: 0,
        velocity: None,
    };

    // Add a velocity to the timer_display since it'll be bouncin' around the place
    if display_mode == DisplayMode::DVD {
        timer_display.velocity = Some(Velocity {
            x: VELOCITY_SPEED,
            y: VELOCITY_SPEED,
        });
    }

    let mut window_width: i32 = WIDTH as i32;
    let mut window_height: i32 = HEIGHT as i32;

    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let window = video_subsystem
        .window("timer", window_width as u32, window_height as u32)
        .position_centered()
        .resizable()
        .build()
        .map_err(|e| e.to_string())?;

    let font_bytes = include_bytes!("../assets/Roboto-Regular.ttf");
    let ttf_handler = ttf::init().unwrap();
    let font = ttf_handler.load_font_from_rwops(RWops::from_bytes(font_bytes).unwrap(), 512)?;

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
            canvas
                .window_mut()
                .flash(sdl2::video::FlashOperation::UntilFocused)?;
            user_notified_finished_timer = true;

            // For XDG desktops (besides macOS), we can use D-Bus to send a
            // Desktop notification and let the user know that the timer
            // has finished. This code should be moved into a module.
            #[cfg(all(unix, not(target_os = "macos")))]
            {
                let connection = Connection::get_private(dbus::ffidisp::BusType::Session)
                    .map_err(|e| e.to_string())?;

                let mut message = Message::new_method_call(
                    "org.freedesktop.Notifications",
                    "/org/freedesktop/Notifications",
                    "org.freedesktop.Notifications",
                    "Notify",
                )?;

                let program_name = "timer";
                let id: u32 = 0;
                let icon = "";
                let summary = "Timer";
                let body = "Time's up!";
                let actions =
                    MessageItem::Array(MessageItemArray::new(vec![], "as".into()).unwrap());
                let hints =
                    MessageItem::Array(MessageItemArray::new(vec![], "a{sv}".into()).unwrap());
                let timeout = 5000;

                message.append_items(&[
                    program_name.clone().into(),
                    id.into(),
                    icon.into(),
                    summary.into(),
                    body.into(),
                    actions,
                    hints,
                    timeout.into(),
                ]);

                connection
                    .send(message)
                    .map_err(|_| String::from("Could not send Desktop Notification Message"))?;
            }

            #[cfg(target_os = "macos")]
            {
                let bundle = mac_notification_sys::get_bundle_identifier_or_default("iterm");
                mac_notification_sys::set_application(&bundle).unwrap();
                let _ = mac_notification_sys::Notification::new()
                    .title("Timer")
                    .message("Time's up!")
                    .sound("Ping")
                    .send()
                    .unwrap();
            }
        }

        /****************************
         *** POLL EVENTS ************
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
                        window_width = w;
                        window_height = h;
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

        match display_mode {
            DisplayMode::DVD => {
                timer_display.x = timer_display.x + timer_display.velocity.unwrap().x;
                timer_display.y = timer_display.y + timer_display.velocity.unwrap().y;
                timer_display.width = (window_width as f32 * DVD_FONT_SCALE) as u32;
                timer_display.height = (window_height as f32 * DVD_FONT_SCALE) as u32;

                if timer_display.x <= 0 {
                    timer_display.velocity.as_mut().unwrap().x = VELOCITY_SPEED;
                }

                if (timer_display.x + timer_display.width as i32) >= window_width {
                    timer_display.velocity.as_mut().unwrap().x = -VELOCITY_SPEED;
                }

                // The font has some padding above it. To make the timer properly hit the top of
                // the window by ignoring the padding, we need to calculate the space between
                // the font ascent and the font's top. This will give us the padding value.
                let font_padding_above_ascent_percentage =
                    (font.height() - font.ascent()) as f32 / font.height() as f32;
                let padding =
                    ((timer_display.height as f32) * font_padding_above_ascent_percentage) as i32;
                if (timer_display.y + padding) <= 0 {
                    timer_display.velocity.as_mut().unwrap().y = VELOCITY_SPEED;
                }

                // There is also some padding under the font's baseline which makes the bounce
                // occur earlier than it should. Here we'll take the baseline and add it to
                // `y` on the timer_display to find where the true bottom of the text is.
                //
                // NOTE: If we were to ever add characters that go below
                // baseline, then the bounce effect would break since
                // we're calcluating the bounce from the baseline.
                let font_height_from_baseline_percentage =
                    (font.height() + font.descent()) as f32 / font.height() as f32;
                let true_height =
                    ((timer_display.height as f32) * font_height_from_baseline_percentage) as i32;
                if (timer_display.y + true_height) >= window_height {
                    timer_display.velocity.as_mut().unwrap().y = -VELOCITY_SPEED;
                }
            }
            DisplayMode::Default => {
                // Calculate the time display based on the window width and
                // height. We run this every frame just in case the user
                // has resized the window which changes the font size.
                timer_display.x = (window_width as f32 * TEXT_PADDING) as i32;
                timer_display.y = (window_height as f32 * TEXT_PADDING) as i32;
                timer_display.width = (window_width as f32 * TEXT_SIZE) as u32;
                timer_display.height = (window_height as f32 * TEXT_SIZE) as u32;
            }
        }

        // Once `active_timer` is false, we flash the completed
        // timer on the screen every half second; so we need
        // to set `flash_timer` every half second for it.
        let flash_timer = (blink_timer % 1.0) < 0.5;

        if active_timer || flash_timer {
            canvas
                .copy(
                    &texture,
                    None,
                    Rect::new(
                        timer_display.x,
                        timer_display.y,
                        timer_display.width,
                        timer_display.height,
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
