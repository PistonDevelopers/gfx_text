extern crate piston_window;
extern crate gfx_text;

use piston_window::*;
use gfx_text::{HorizontalAnchor, VerticalAnchor};

const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const BROWN: [f32; 4] = [0.65, 0.16, 0.16, 1.0];
const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
const BLUE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
const FONT_PATH: &'static str = "examples/assets/Ubuntu-R.ttf";

fn main() {
    let title = "gfx_text example";
    let mut window: PistonWindow = WindowSettings::new(title, [640, 480])
        .exit_on_esc(true)
        .build()
        .unwrap_or_else(|e| panic!("Failed to build PistonWindow: {}", e));

    let mut encoder: GfxEncoder = window.factory.create_command_buffer().into();

    let hdpi = window.draw_size().width / window.size().width;

    let mut normal_text = gfx_text::new(window.factory.clone())
        .with_size((16.0 * hdpi) as u8).unwrap();
    let mut big_text = gfx_text::new(window.factory.clone())
        .with_size((20.0 * hdpi) as u8).unwrap();
    let mut custom_font_text = gfx_text::new(window.factory.clone())
        .with_size((25.0 * hdpi) as u8)
        .with_font(FONT_PATH)
        .unwrap();

    let main_color = window.output_color.clone();
    let mut counter: u32 = 0;

    while let Some(e) = window.next() {
        window.draw_3d(&e, |window| {
            let pos = |p: [i32; 2]| [(p[0] as f64 * hdpi) as i32, (p[1] as f64 * hdpi) as i32];

            encoder.clear(&main_color, WHITE);

            counter += 1;

            normal_text.add("The quick brown fox jumps over the lazy dog", pos([10, 10]), BROWN);
            normal_text.add("The quick red fox jumps over the lazy dog", pos([30, 30]), RED);
            normal_text.add_anchored("hello centred world", pos([320, 240]), HorizontalAnchor::Center, VerticalAnchor::Center, BLUE);
            normal_text.add_anchored(&format!("Count: {}", counter), pos([0, 479]), HorizontalAnchor::Left, VerticalAnchor::Bottom, BLUE);

            big_text.add("The big brown fox jumps over the lazy dog", pos([50, 50]), BROWN);

            custom_font_text.add("The custom blue fox jumps over the lazy dog", pos([10, 80]), BLUE);
            custom_font_text.add_anchored("I live in the bottom right", pos([639, 479]), HorizontalAnchor::Right, VerticalAnchor::Bottom, RED);

            normal_text.draw(&mut encoder, &main_color).unwrap();
            big_text.draw(&mut encoder, &main_color).unwrap();
            custom_font_text.draw(&mut encoder, &main_color).unwrap();

            encoder.flush(&mut window.device);
        });
    }
}
