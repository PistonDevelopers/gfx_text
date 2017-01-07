// extern crate env_logger;
extern crate gfx;
extern crate gfx_device_gl;
extern crate shader_version;
extern crate glutin_window;
extern crate glutin;
extern crate piston;
extern crate gfx_text;

use gfx::traits::Device;
use gfx_text::{HorizontalAnchor, VerticalAnchor};
use glutin_window::GlutinWindow;
use shader_version::OpenGL;
use piston::window::{WindowSettings, OpenGLWindow};
use piston::input::*;
use piston::event_loop::*;

const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const BROWN: [f32; 4] = [0.65, 0.16, 0.16, 1.0];
const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
const BLUE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
const FONT_PATH: &'static str = "examples/assets/Ubuntu-R.ttf";

fn main() {

    // Create Window
    let samples = 0;
    let mut window: GlutinWindow = WindowSettings::new(
            "gfx_text example",
            [640, 480]
        )
        .samples(samples)
        .opengl(OpenGL::V3_2)
        .vsync(false)
        .exit_on_esc(true)
        .build()
        .unwrap();

    // OpenGL Context
    let (mut device, mut factory) = gfx_device_gl::create(|s| {
        window.get_proc_address(s) as *const _
    });

    let main_color = create_main_target((640, 480, 1, samples.into()));

    let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();

    let mut normal_text = gfx_text::new(factory.clone()).unwrap();
    let mut big_text = gfx_text::new(factory.clone()).with_size(20).unwrap();
    let mut custom_font_text = gfx_text::new(factory.clone())
        .with_size(25)
        .with_font(FONT_PATH)
        .unwrap();

    let mut counter: u32 = 0;
    let mut events = window.events();

    'main: loop {
        for event in events.next(&mut window) {
            match event {
                Event::Render(_) => {

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
                    device.cleanup();

                },
                _ => {}
            }
        }

    }
}

fn create_main_target(dim: gfx::texture::Dimensions) -> gfx::handle::RenderTargetView<gfx_device_gl::Resources, gfx::format::Srgba8> {

    use gfx::memory::Typed;
    use gfx::format::{DepthStencil, Format, Formatted, Srgba8};

    let color_format: Format = <Srgba8 as Formatted>::get_format();
    let depth_format: Format = <DepthStencil as Formatted>::get_format();
    let (output_color, _) = gfx_device_gl::create_main_targets_raw(
        dim,
        color_format.0,
        depth_format.0
    );

    Typed::new(output_color)
}

