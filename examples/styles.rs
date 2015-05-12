extern crate gfx;
extern crate gfx_window_glutin;
extern crate glutin;
extern crate gfx_text;

use gfx::traits::{IntoCanvas, Stream};
use gfx_window_glutin as gfxw;
use glutin::{WindowBuilder, Event, VirtualKeyCode};

const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const BROWN: [f32; 4] = [0.65, 0.16, 0.16, 1.0];
const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
const BLUE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
const FONT_PATH: &'static str = "examples/assets/Ubuntu-R.ttf";

fn main() {
    let mut canvas = {
        let window = WindowBuilder::new()
            .with_dimensions(640, 480)
            .with_title(format!("gfx_text example"))
            .build()
            .unwrap();
        gfxw::init(window).into_canvas()
    };

    let mut normal_text = gfx_text::new(&mut canvas.factory).unwrap();
    let mut big_text = gfx_text::new(&mut canvas.factory).with_size(20).unwrap();
    let mut custom_font_text = gfx_text::new(&mut canvas.factory)
        .with_size(25)
        .with_font(FONT_PATH)
        .unwrap();

    'main: loop {
        for event in canvas.output.window.poll_events() {
            match event {
                Event::Closed => break 'main,
                Event::KeyboardInput(_, _, Some(VirtualKeyCode::Escape)) => break 'main,
                _ => {},
            }
        }
        canvas.clear(gfx::ClearData {color: WHITE, depth: 1.0, stencil: 0});

        normal_text.draw("The quick brown fox jumps over the lazy dog", [10, 10], BROWN);
        normal_text.draw("The quick red fox jumps over the lazy dog", [30, 30], RED);
        normal_text.draw_end(&mut canvas).unwrap();

        big_text.draw("The big brown fox jumps over the lazy dog", [50, 50], BROWN);
        big_text.draw_end(&mut canvas).unwrap();

        custom_font_text.draw("The custom blue fox jumps over the lazy dog", [10, 80], BLUE);
        custom_font_text.draw_end(&mut canvas).unwrap();

        canvas.present();
    }
}
