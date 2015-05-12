#![feature(custom_attribute)]
#![feature(plugin)]
#![plugin(gfx_macros)]

extern crate gfx;
extern crate freetype;

use std::mem;
use std::marker::PhantomData;
use gfx::{Resources, Factory, CommandBuffer, Output, Device, Canvas};
use gfx::{PrimitiveType, ProgramError, DrawError};
use gfx::traits::{FactoryExt, ToSlice, Stream};
use gfx::handle::{Program, Buffer, IndexBuffer, Texture};
use gfx::batch::Error as BatchError;
use gfx::shade::TextureParam;
use gfx::tex::{self, TextureError};
mod font;
use font::BitmapFont;
pub use font::FontError;

const DEFAULT_FONT_SIZE: u8 = 16;
const DEFAULT_FONT_DATA: &'static [u8] = include_bytes!("../assets/NotoSans-Regular.ttf");
const DEFAULT_BUFFER_SIZE: usize = 128;
const DEFAULT_OUTLINE_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
const DEFAULT_PROJECTION: [[f32; 4]; 4] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

#[derive(Debug)]
pub enum Error {
    ProgramError(ProgramError),
    FontError(FontError),
    TextureError(TextureError),
}

impl From<ProgramError> for Error {
    fn from(e: ProgramError) -> Error { Error::ProgramError(e) }
}

impl From<FontError> for Error {
    fn from(e: FontError) -> Error { Error::FontError(e) }
}

impl From<TextureError> for Error {
    fn from(e: TextureError) -> Error { Error::TextureError(e) }
}

type IndexT = u32;

pub struct Renderer<R: Resources> {
    program: Program<R>,
    draw_state: gfx::DrawState,
    vertex_data: Vec<Vertex>,
    vertex_buffer: Buffer<R, Vertex>,
    index_data: Vec<IndexT>,
    index_buffer: IndexBuffer<R, IndexT>,
    font_bitmap: BitmapFont,
    params: ShaderParams<R>,
}

pub struct RendererBuilder<'r, R: Resources, F: Factory<R> + 'r> {
    factory: &'r mut F,
    font_size: u8,
    // NOTE(Kagami): Better to use `P: AsRef<OsStr>` but since we store path in
    // the intermediate builder structure, Rust will unable to infer type
    // without manual annotation which is much worse. Anyway, it's possible to
    // just pass raw bytes.
    font_path: Option<&'r str>,
    font_data: &'r [u8],
    outline_width: Option<u8>,
    outline_color: [f32; 4],
    buffer_size: usize,
    chars: Option<&'r [char]>,
    // XXX(Kagami): Shut up the Rust complains about unused R. We can't use
    // just `factory: &mut Factory<R>` because it doesn't work with lifetimes
    // (complains about the Marker associated type). Is there any better way?
    _r: PhantomData<R>,
}

/// Create a new text renderer builder.
pub fn new<'r, R: Resources, F: Factory<R>> (factory: &'r mut F) -> RendererBuilder<'r, R, F> {
    // Default renderer settings.
    RendererBuilder {
        factory: factory,
        font_size: DEFAULT_FONT_SIZE,
        font_path: None,  // Default font will be used
        font_data: DEFAULT_FONT_DATA,
        outline_width: None,  // No outline by default
        outline_color: DEFAULT_OUTLINE_COLOR,
        buffer_size: DEFAULT_BUFFER_SIZE,
        chars: None,  // Place all available font chars into texture
        _r: PhantomData,
    }
}

impl<'r, R: Resources, F: Factory<R>> RendererBuilder<'r, R, F> {
    /// Specify custom size.
    pub fn with_size(mut self, size: u8) -> RendererBuilder<'r, R, F> {
        self.font_size = size;
        self
    }

    /// Specify custom font by path.
    pub fn with_font(mut self, path: &'r str) -> RendererBuilder<'r, R, F> {
        self.font_path = Some(path);
        self
    }

    /// Pass raw font data.
    pub fn with_font_data(mut self, data: &'r [u8]) -> RendererBuilder<'r, R, F> {
        self.font_data = data;
        self
    }

    /// Specify outline width and color.
    pub fn with_outline(mut self, width: u8, color: [f32; 4]) -> RendererBuilder<'r, R, F> {
        self.outline_width = Some(width);
        self.outline_color = color;
        self
    }

    /// Specify custom initial buffer size.
    pub fn with_buffer_size(mut self, size: usize) -> RendererBuilder<'r, R, F> {
        self.buffer_size = size;
        self
    }

    /// Make available only provided characters in font texture instead of
    /// loading all existing from the font face.
    pub fn with_chars(mut self, chars: &'r [char]) -> RendererBuilder<'r, R, F> {
        self.chars = Some(chars);
        self
    }

    /// Build a new text renderer instance using current settings.
    pub fn build(self) -> Result<Renderer<R>, Error> {
        let program = try!(self.factory.link_program(VERTEX_SRC, FRAGMENT_SRC));
        let state = gfx::DrawState::new().blend(gfx::BlendPreset::Alpha);
        let vertex_buffer = self.factory.create_buffer(
            self.buffer_size,
            gfx::BufferUsage::Dynamic,
        );
        let index_buffer = create_index_buffer(
            self.factory,
            self.buffer_size,
            gfx::BufferUsage::Dynamic,
        );

        // Initialize bitmap font.
        // TODO(Kagami): Outline!
        // TODO(Kagami): More granulated font settings, e.g. antialiasing,
        // hinting, kerning, etc.
        let font_bitmap = try!(match self.font_path {
            Some(path) =>
                BitmapFont::from_path(path, self.font_size, self.chars),
            None =>
                BitmapFont::from_bytes(self.font_data, self.font_size, self.chars),
        });
        // FIXME(Kagami): Seems like blending R8 texture with RGBA8
        // user-specified font color cause artifacts, so converting to
        // full-component image as for now.
        let image24: Vec<_> = font_bitmap.get_image().iter().flat_map(|&i|
            Some(0).into_iter()  // R
            .chain(Some(0))      // G
            .chain(Some(0))      // B
            .chain(Some(i))      // A
        ).collect();
        let font_texture = try!(create_texture_rgba8_static(
            self.factory,
            font_bitmap.get_width(),
            font_bitmap.get_height(),
            &image24,
        ));
        let sampler = self.factory.create_sampler(
            tex::SamplerInfo::new(tex::FilterMethod::Bilinear,
                                  tex::WrapMode::Clamp)
        );

        Ok(Renderer {
            program: program,
            draw_state: state,
            vertex_data: Vec::new(),
            vertex_buffer: vertex_buffer,
            index_data: Vec::new(),
            index_buffer: index_buffer,
            font_bitmap: font_bitmap,
            params: ShaderParams {
                color: (font_texture, Some(sampler)),
                screen_size: [0.0, 0.0],
                proj: DEFAULT_PROJECTION,
            },
        })
    }

    /// Just an alias for `builder.build().unwrap()`.
    pub fn unwrap(self) -> Renderer<R> {
        self.build().unwrap()
    }
}

impl<R: Resources> Renderer<R> {
    /// Add some text to the current draw scene relative to the top left corner
    /// of the screen using pixel coords.
    pub fn draw(&mut self, text: &str, pos: [i32; 2], color: [f32; 4]) {
        self.draw_generic(text, Ok(pos), color)
    }

    /// Add some text to the draw scene using absolute world coords.
    pub fn draw_at(&mut self, text: &str, pos: [f32; 3], color: [f32; 4]) {
        self.draw_generic(text, Err(pos), color)
    }

    fn draw_generic(&mut self, text: &str, pos: Result<[i32; 2], [f32; 3]>, color: [f32; 4]) {
        let (screen_pos, world_pos, screen_rel) = match pos {
            Ok(screen_pos) => (screen_pos, [0.0, 0.0, 0.0], 1),
            Err(world_pos) => ([0, 0], world_pos, 0),
        };
        let (mut x, y) = (screen_pos[0] as f32, screen_pos[1] as f32);
        let mut index = self.vertex_data.len() as u32;
        for ch in text.chars() {
            let ch_info = match self.font_bitmap.find_char(ch) {
                Some(info) => info,
                // Skip unknown chars from text string. Probably it would be
                // better to place some "?" mark instead but it may not exist
                // in the font too.
                None => continue,
            };
            let x_offset = x + ch_info.x_offset as f32;
            let y_offset = y + ch_info.y_offset as f32;
            let tex = ch_info.tex;

            // Top-left point, index 0.
            self.vertex_data.push(Vertex {
                pos: [x_offset, y_offset],
                tex: [tex[0], tex[1]],
                world_pos: world_pos,
                screen_rel: screen_rel,
                color: color,
            });
            // Bottom-left point, index 1.
            self.vertex_data.push(Vertex {
                pos: [x_offset, y_offset + ch_info.height as f32],
                tex: [tex[0], tex[1] + ch_info.tex_height],
                world_pos: world_pos,
                screen_rel: screen_rel,
                color: color,
            });
            // Bottom-right point, index 2.
            self.vertex_data.push(Vertex {
                pos: [x_offset + ch_info.width as f32, y_offset + ch_info.height as f32],
                tex: [tex[0] + ch_info.tex_width, tex[1] + ch_info.tex_height],
                world_pos: world_pos,
                screen_rel: screen_rel,
                color: color,
            });
            // Top-right point, index 3.
            self.vertex_data.push(Vertex {
                pos: [x_offset + ch_info.width as f32, y_offset],
                tex: [tex[0] + ch_info.tex_width, tex[1]],
                world_pos: world_pos,
                screen_rel: screen_rel,
                color: color,
            });

            // Top-left triangle.
            // 0--3
            // | /
            // |/
            // 1
            self.index_data.push(index + 0);
            self.index_data.push(index + 1);
            self.index_data.push(index + 3);
            // Bottom-right triangle.
            //    3
            //   /|
            //  / |
            // 1--2
            self.index_data.push(index + 3);
            self.index_data.push(index + 1);
            self.index_data.push(index + 2);

            index += 4;
            x += ch_info.x_advance as f32;
        }
    }

    /// End with drawing, clear internal state and return resulting batch.
    pub fn draw_end<
        C: CommandBuffer<R>,
        O: Output<R>,
        D: Device<Resources=R, CommandBuffer=C>,
        F: Factory<R>,
    > (
        &mut self,
        canvas: &mut Canvas<O, D, F>
    ) -> Result<(), DrawError<BatchError>> {
        self.draw_end_at(canvas, DEFAULT_PROJECTION)
    }

    /// End with drawing using provided projection matrix.
    pub fn draw_end_at<
        C: CommandBuffer<R>,
        O: Output<R>,
        D: Device<Resources=R, CommandBuffer=C>,
        F: Factory<R>,
    > (
        &mut self,
        canvas: &mut Canvas<O, D, F>,
        proj: [[f32; 4]; 4]
    ) -> Result<(), DrawError<BatchError>> {
        let ver_len = self.vertex_data.len();
        let ver_buf_len = self.vertex_buffer.len();
        let ind_len = self.index_data.len();
        let ind_buf_len = self.index_buffer.len();

        // Reallocate buffers if there is no enough space for data.
        if ver_len > ver_buf_len {
            self.vertex_buffer = canvas.factory.create_buffer(
                grow_buffer_size(ver_buf_len, ver_len),
                gfx::BufferUsage::Dynamic);
        }
        if ind_len > ind_buf_len {
            self.index_buffer = create_index_buffer(
                &mut canvas.factory,
                grow_buffer_size(ind_buf_len, ind_len),
                gfx::BufferUsage::Dynamic);
        }
        // Move vertex/index data.
        canvas.factory.update_buffer(&self.vertex_buffer, &self.vertex_data, 0);
        update_index_buffer(&mut canvas.factory, &self.index_buffer, &self.index_data, 0);
        // Clear state.
        self.vertex_data.clear();
        self.index_data.clear();

        let nv = ind_len as gfx::VertexCount;
        let mesh = gfx::Mesh::from_format(self.vertex_buffer.clone(), nv);
        let slice = self.index_buffer.to_slice(PrimitiveType::TriangleList);
        self.params.screen_size = {
            let (w, h) = canvas.output.get_size();
            [w as f32, h as f32]
        };
        self.params.proj = proj;
        let batch = gfx::batch::bind(
            &self.draw_state,
            &mesh,
            slice,
            &self.program,
            &self.params);

        canvas.draw(&batch)
    }
}

// Some missing helpers.

fn create_index_buffer<R: Resources, F: Factory<R>, T: Copy>(
    factory: &mut F,
    num: usize,
    usage: gfx::BufferUsage,
) -> IndexBuffer<R, T> {
    IndexBuffer::from_raw(factory.create_buffer_raw(num * mem::size_of::<T>(), usage))
}

fn update_index_buffer<R: Resources, F: Factory<R>, T: Copy>(
    factory: &mut F,
    buf: &IndexBuffer<R, IndexT>,
    data: &[T],
    offset_elements: usize
) {
    factory.update_buffer_raw(
        buf.raw(),
        gfx::as_byte_slice(data),
        mem::size_of::<T>() * offset_elements)
}

fn grow_buffer_size(mut current_size: usize, desired_size: usize) -> usize {
    if current_size < 1 {
        current_size = 1;
    }
    while current_size < desired_size {
        current_size *= 2;
    }
    current_size
}

// Helper from FactoryExt with the same name create minmaps and we don't need
// them.
fn create_texture_rgba8_static<R: Resources, F: Factory<R>>(
    factory: &mut F,
    width: u16,
    height: u16,
    data: &[u8]
) -> Result<Texture<R>, TextureError>{
    let texture = try!(factory.create_texture_rgba8(width, height));
    try!(factory.update_texture_raw(
        &texture,
        &texture.get_info().to_image_info(),
        data,
        None,
    ));
    Ok(texture)
}

// TODO(Kagami): Use simple macroses instead, see
// <https://github.com/gfx-rs/gfx-rs/pull/725> for details.

#[vertex_format]
#[derive(Debug, Clone, Copy)]
struct Vertex {
    #[name = "a_Pos"]
    pos: [f32; 2],
    #[name = "a_TexCoord"]
    tex: [f32; 2],
    #[name = "a_World_Pos"]
    world_pos: [f32; 3],
    #[name = "a_Screen_Rel"]
    screen_rel: i32,  // Should be bool but gfx-rs doesn't support it
    #[name = "a_Color"]
    color: [f32; 4],
}

#[shader_param]
struct ShaderParams<R: Resources> {
    #[name = "t_Color"]
    color: TextureParam<R>,
    #[name = "u_Screen_Size"]
    screen_size: [f32; 2],
    #[name = "u_Proj"]
    proj: [[f32; 4]; 4],
}

const VERTEX_SRC: &'static [u8] = b"
    #version 150 core

    in vec2 a_Pos;
    in vec4 a_Color;
    in vec2 a_TexCoord;
    in vec4 a_World_Pos;
    in int a_Screen_Rel;
    out vec4 v_Color;
    out vec2 v_TexCoord;
    uniform vec2 u_Screen_Size;
    uniform mat4 u_Proj;

    void main() {
        // On-screen offset from text origin.
        vec2 v_Screen_Offset = vec2(
            2 * a_Pos.x / u_Screen_Size.x - 1,
            1 - 2 * a_Pos.y / u_Screen_Size.y
        );
        vec4 v_Screen_Pos = u_Proj * a_World_Pos;
        vec2 v_World_Offset = a_Screen_Rel == 0
            // Perspective divide to get normalized device coords.
            ? vec2 (
                v_Screen_Pos.x / v_Screen_Pos.z + 1,
                v_Screen_Pos.y / v_Screen_Pos.z - 1
            ) : vec2(0.0, 0.0);

        v_Color = a_Color;
        v_TexCoord = a_TexCoord;
        gl_Position = vec4(v_World_Offset + v_Screen_Offset, 0.0, 1.0);
    }
";

const FRAGMENT_SRC: &'static [u8] = b"
    #version 150 core

    in vec4 v_Color;
    in vec2 v_TexCoord;
    out vec4 o_Color;
    uniform sampler2D t_Color;

    void main() {
        vec4 t_Font_Color = texture2D(t_Color, v_TexCoord);
        o_Color = vec4(v_Color.rgb, t_Font_Color.a * v_Color.a);
    }
";
