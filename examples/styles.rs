// extern crate env_logger;
extern crate gfx;
extern crate gfx_window_glutin;
extern crate glutin;
extern crate gfx_text;

use gfx::Device;
use gfx_window_glutin as gfxw;
use gfx_text::{HorizontalAnchor, VerticalAnchor};
use glutin::{
    WindowBuilder, Event, VirtualKeyCode,
    WindowEvent, KeyboardInput, EventsLoop,
    GL_CORE
};

const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const BROWN: [f32; 4] = [0.65, 0.16, 0.16, 1.0];
const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
const BLUE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
const FONT_PATH: &'static str = "examples/assets/Ubuntu-R.ttf";

fn main() {
    // env_logger::init().unwrap();

    let mut events_loop = EventsLoop::new();
    let context = glutin::ContextBuilder::new()
        .with_gl(GL_CORE);
    let (window, mut device, mut factory, main_color, _) = {
        let builder = WindowBuilder::new()
            .with_dimensions((640, 480).into())
            .with_title(format!("gfx_text example"));
        gfxw::init::<gfx::format::Rgba8, gfx::format::Depth>(builder, context, &events_loop).unwrap()
    };
    let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();

    let mut normal_text = gfx_text::new(factory.clone()).unwrap();
    let mut big_text = gfx_text::new(factory.clone()).with_size(20).unwrap();
    let mut custom_font_text = gfx_text::new(factory.clone())
        .with_size(25)
        .with_font(FONT_PATH)
        .unwrap();

    let mut counter: u32 = 0;
    let mut exit = false;

    'main: loop {
        events_loop.poll_events(|event| {
            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    exit = true;
                    return;
                }
                Event::WindowEvent {
                    event: WindowEvent::KeyboardInput {
                        input: KeyboardInput {
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        },
                        ..
                    },
                    ..
                } => {
                    exit = true;
                    return;
                }
                _ => {},
            }
        });

        if exit {break 'main;}

        counter += 1;

        normal_text.add("The quick brown fox jumps over the lazy dog", [10, 10], BROWN);
        normal_text.add("The quick red fox jumps over the lazy dog", [30, 30], RED);
        normal_text.add_anchored("hello centred world", [320, 240], HorizontalAnchor::Center, VerticalAnchor::Center, BLUE);
        normal_text.add_anchored(&format!("Count: {}", counter), [0, 479], HorizontalAnchor::Left, VerticalAnchor::Bottom, BLUE);

        big_text.add("The big brown fox jumps over the lazy dog", [50, 50], BROWN);

        custom_font_text.add("The custom blue fox jumps over the lazy dog", [10, 80], BLUE);
        custom_font_text.add_anchored("I live in the bottom right", [639, 479], HorizontalAnchor::Right, VerticalAnchor::Bottom, RED);

        encoder.clear(&main_color, WHITE);

        normal_text.draw(&mut encoder, &main_color).unwrap();
        big_text.draw(&mut encoder, &main_color).unwrap();
        custom_font_text.draw(&mut encoder, &main_color).unwrap();

        encoder.flush(&mut device);
        window.swap_buffers().unwrap();
        device.cleanup();
    }
}
