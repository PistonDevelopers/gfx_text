//! Construct bitmap font using FreeType library.
//! Generates raw texture data for the given font and collects information
//! about available font characters to map them into texture.

use std::cmp::max;
use std::iter::{repeat, FromIterator};
use std::collections::{HashMap, HashSet};
use std::char::from_u32;
use ::freetype as ft;
use ::freetype::Face;

#[derive(Debug)]
pub struct BitmapFont {
    width: u16,
    height: u16,
    chars: HashMap<char, BitmapChar>,
    image: Vec<u8>,
}

#[derive(Debug)]
pub struct BitmapChar {
    // Real glyph's coordinates in pixels.
    pub x_offset: i32,
    pub y_offset: i32,
    pub x_advance: i32,
    pub width: i32,
    pub height: i32,
    // Precalculated scaled positions in texture.
    pub tex: [f32; 2],
    pub tex_width: f32,
    pub tex_height: f32,
    // This field is used only while building the texture.
    data: Option<Vec<u8>>,
}

/// Represents possible errors that may occur during the font loading.
#[derive(Debug)]
pub enum FontError {
    /// Character set is empty
    EmptyFont,
    /// FreeType library error
    FreetypeError(ft::Error),
}

impl From<ft::Error> for FontError {
    fn from(e: ft::Error) -> FontError { FontError::FreetypeError(e) }
}

pub type FontResult = Result<BitmapFont, FontError>;

impl BitmapFont {
    pub fn from_path(path: &str, font_size: u8, chars: Option<&[char]>) -> FontResult {
        let library = try!(ft::Library::init());
        let face = try!(library.new_face(path, 0));
        Self::new(face, font_size, chars)
    }

    pub fn from_bytes(data: &[u8], font_size: u8, chars: Option<&[char]>) -> FontResult {
        let library = try!(ft::Library::init());
        let face = try!(library.new_memory_face(data, 0));
        Self::new(face, font_size, chars)
    }

    fn get_all_face_chars<'a>(face: &mut Face<'a>) -> HashSet<char> {
        let mut result = HashSet::new();
        let mut index = 0;
        let mut face_ptr = face.raw_mut();
        unsafe {
            let mut code = ft::ffi::FT_Get_First_Char(face_ptr, &mut index);
            while index != 0 {
                from_u32(code as u32).map(|ch| result.insert(ch));
                code = ft::ffi::FT_Get_Next_Char(face_ptr, code, &mut index);
            }
        }
        result
    }

    // FIXME(Kagami): Profile and optimize this function!
    // TODO(Kagami): Limit too huge textures? We should keep it less than
    // 8k X 8k in general.
    // TODO(Kagami): Add bunch of asserts to check for negative values and
    // overflows.
    /// Construct new BitMap font using provided parameters (this is general
    /// method, called via `from_` helpers).
    fn new<'a>(mut face: ft::Face<'a>, font_size: u8, chars: Option<&[char]>) -> FontResult {
        let needed_chars = chars
            .map(|sl| HashSet::from_iter(sl.iter().cloned()))
            .unwrap_or_else(|| Self::get_all_face_chars(&mut face));
        if needed_chars.is_empty() {
            return Err(FontError::EmptyFont);
        }

        try!(face.set_pixel_sizes(0, font_size as u32));

        // FreeType representation of rendered glyph 'j':
        //
        // b_left   w
        // +-----+-----+-----+
        // |     |     |     | font_size - bitmap_top()
        // +-----+-----+-----+
        // |     |  x  |     |
        // |     |     |     |
        // |     |  x  |     | bitmap_top()
        // |     |  x  |     |
        // |     |  x  |     |
        // |     |  x  |     |
        // +-----+--x--+-----+
        // |     | x   |     | rows() - bitmap_top()
        // |     |x    |     |
        // +-----------+-----+
        //      advance.x
        //
        // (Read <http://www.freetype.org/freetype2/docs/glyphs/glyphs-3.html>
        // for more details.)
        //
        // Notes:
        // * Width/height of the rendered glyph generally smaller than the the
        //   specified font size
        // * But if we add x/y offsets to the real glyph's dimensions it might
        //   go beyound that limits (e.g. chars like 'j', 'q')
        // * `bottom_left()` may be less than zero for some tight characters
        //   (too push it to the previous one)
        // * Theoretically `bitmap_top()` may be bigger than the `font_size`
        //
        // For simplicity we use fixed box height to store characters in the
        // texture (extended with blank pixels downwards), but width may vary:
        //
        //         width()
        //  +-----+-------+
        //  |  x  |   x   |
        //  |     |       |
        //  |  x  |   x   |
        //  |  x  |   x   | rows()
        //  |  x  |   x   |
        //  |  x  |   x   |
        //  +-----+  x    |
        //  |     | x     |
        //  |     +-------+
        //  |     |       | ch_box_height - rows()
        //  +-----+-------+
        //
        // To construct the optimal texture (i.e. square enought and with box
        // height of the highest character) we need to do several passes.

        // In first pass we collect information about the chars and store their
        // raw bitmap data. It gives us max character height and summary width
        // of all characters.

        let chars_len = needed_chars.len();
        let mut chars_info = HashMap::with_capacity(chars_len);
        let mut sum_image_width = 0;
        let mut max_ch_width = 0;
        let mut ch_box_height = 0;

        // debug!("Start building the bitmap (chars: {})", chars_len);

        for ch in needed_chars {
            try!(face.load_char(ch as usize, ft::face::RENDER));
            let glyph = face.glyph();
            let bitmap = glyph.bitmap();
            let ch_width = bitmap.width();
            let ch_height = bitmap.rows();
            let ch_x_offset = glyph.bitmap_left();
            let ch_y_offset = font_size as i32 - glyph.bitmap_top();
            let ch_x_advance = (glyph.advance().x >> 6) as i32;
            let buffer = bitmap.buffer();
            let ch_data = Vec::from(buffer);

            chars_info.insert(ch, BitmapChar {
                x_offset: ch_x_offset,
                y_offset: ch_y_offset,
                x_advance: ch_x_advance,
                width: ch_width,
                height: ch_height,
                // We'll need to fix that fields later:
                tex: [0.0, 0.0],
                tex_width: 0.0,
                tex_height: 0.0,
                data: Some(ch_data),
            });

            sum_image_width += ch_width;
            max_ch_width = max(max_ch_width, ch_width);
            ch_box_height = max(ch_box_height, ch_height);
        }

        // In second pass we map character boxes with varying width onto the
        // fixed quad texture image and build the final texture image.
        //
        // We start with optimist (square) assumption about texture dimensions
        // and adjust the image's height and size while filling the rows.
        //
        // TODO(Kagami): We may try some cool CS algorithm to fit char boxes
        // into the quad texture space with the best level of compression.
        // Though current level of inefficiency is good enough.

        let ideal_image_size = sum_image_width * ch_box_height;
        let ideal_image_width = (ideal_image_size as f32).sqrt() as i32;
        let image_width = max(max_ch_width, ideal_image_width);
        let assumed_size = ideal_image_size as f32 * 1.5;
        let assumed_ch_in_row = image_width as f32 / max_ch_width as f32;
        let mut image = Vec::with_capacity(assumed_size as usize);
        let mut chars_row = Vec::with_capacity(assumed_ch_in_row as usize);
        let mut cursor_x = 0;
        let mut image_height = 0;

        let dump_row = |image: &mut Vec<u8>, chars_row: &Vec<(i32, i32, Vec<u8>)>| {
            // Copy character data into the image row by row:
            //
            //       image_width
            // +-------+---------+---+
            // |   x   |    x    |   |
            // |       |         |   |
            // |   x   |    x    |   | ch_box_height
            // |   x   |    x    |   |
            // |   x   |    x    |   |
            // |   x   |   x     |   |
            // |       |  x      |   |
            // +-------+---------+---+
            //                     ^--- image_width - width_ch_i - width_ch_j
            for i in 0..ch_box_height {
                let mut x = 0;
                for &(width, height, ref data) in chars_row {
                   if i >= height {
                       image.extend(repeat(0).take(width as usize));
                   } else {
                       let skip = i * width;
                       debug_assert!(data.len() >= (skip + width) as usize);
                       let line = data.iter().skip(skip as usize).take(width as usize);
                       image.extend(line.cloned());
                   };
                   x += width;
                }
                let cols_to_fill = image_width - x;
                image.extend(repeat(0).take(cols_to_fill as usize));
            }
        };

        // debug!("Placing chars onto a plane");

        // Hashmap doesn't preserve the order but we don't need it anyway.
        for (_, ch_info) in chars_info.iter_mut() {
            if cursor_x + ch_info.width > image_width {
                dump_row(&mut image, &chars_row);
                chars_row.clear();
                cursor_x = 0;
                image_height += ch_box_height;
            }
            let ch_data = ch_info.data.take().unwrap();
            chars_row.push((ch_info.width, ch_info.height, ch_data));
            ch_info.tex = [cursor_x as f32, image_height as f32];
            cursor_x += ch_info.width;
        }
        dump_row(&mut image, &chars_row);
        image_height += ch_box_height;

        // Finally, we just precalculate some fields to make it easier to use
        // our font.

        for (_, ch_info) in chars_info.iter_mut() {
            ch_info.tex[0] /= image_width as f32;
            ch_info.tex[1] /= image_height as f32;
            ch_info.tex_width = ch_info.width as f32 / image_width as f32;
            ch_info.tex_height = ch_info.height as f32 / image_height as f32;
        }

        // info!("Image width: {}, image height: {}, total size: {}",
        //     image_width, image_height, image.len());

        Ok(BitmapFont {
            width: image_width as u16,
            height: image_height as u16,
            chars: chars_info,
            image: image,
        })
    }

    pub fn get_width(&self) -> u16 {
        self.width
    }

    pub fn get_height(&self) -> u16 {
        self.height
    }

    /// Return 8-bit texture raw data (grayscale).
    pub fn get_image(&self) -> &[u8] {
        &self.image
    }

    pub fn find_char(&self, ch: char) -> Option<&BitmapChar> {
        self.chars.get(&ch)
    }
}
