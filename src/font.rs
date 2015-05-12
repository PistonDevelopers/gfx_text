//! Construct bitmap font using FreeType library.
//! Generates raw texture data for the given font and collects information
//! about available font characters to map them into texture.

use std::io;
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
}

#[derive(Debug)]
pub enum FontError {
    IoError(io::Error),
    FreetypeError(ft::Error),
}

impl From<io::Error> for FontError {
    fn from(e: io::Error) -> FontError { FontError::IoError(e) }
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
        unsafe {
            let mut code = ft::ffi::FT_Get_First_Char(face.raw_mut(), &mut index);
            while index != 0 {
                from_u32(code as u32).map(|ch| { result.insert(ch) });
                code = ft::ffi::FT_Get_Next_Char(face.raw_mut(), code, &mut index);
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
            // Short-circuit.
            return Ok(BitmapFont {
                width: 0,
                height: 0,
                chars: HashMap::new(),
                image: Vec::new(),
            })
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
        let mut chars_data = HashMap::with_capacity(chars_len);
        let mut sum_image_width = 0;
        let mut max_ch_width = 0;
        let mut ch_box_height = 0;

        // println!("Start building the bitmap (chars: {})", chars_len);

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
            });
            chars_data.insert(ch, ch_data);

            sum_image_width += ch_width;
            max_ch_width = max(max_ch_width, ch_width);
            ch_box_height = max(ch_box_height, ch_height);
        }

        // In second pass we map character boxes with varying width onto the
        // fixed quad texture image.
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
        let mut tiles = vec![Vec::new()];
        let mut cursor_x = 0;
        let mut row = 0;

        // println!("Placing chars onto a plane");

        // Hashmap doesn't preserve the order but we don't need it anyway.
        for (ch, ch_info) in chars_info.iter_mut() {
            ch_info.tex = [cursor_x as f32, (row * ch_box_height) as f32];
            if cursor_x + ch_info.width > image_width {
                cursor_x = 0;
                row += 1;
                tiles.push(Vec::new());
                ch_info.tex = [cursor_x as f32, (row * ch_box_height) as f32];
            }
            cursor_x += ch_info.width;
            // FIXME(Kagami): We can't store char data in tiles vector itself
            // because of borrow checking. So next pass will need to look up
            // char data hashmap again. It should be fast (O(1)) but still
            // annoying.
            tiles[row as usize].push((ch_info.width, ch_info.height, *ch));
        }

        // Finally, we build the resuling image by copying glyphs pixel data to
        // the their cells and also fill the empty space.

        let image_height = (row + 1) * ch_box_height;
        let mut image = Vec::with_capacity((image_width * image_height) as usize);

        // println!("Building the final image");

        for tiles_row in tiles {
            for row in 0..ch_box_height {
                let mut cursor_x = 0;
                for (width, height, ch) in tiles_row.iter().cloned() {
                    if height <= row {
                        image.extend(repeat(0).take(width as usize));
                    } else {
                        let tile = chars_data.get(&ch).unwrap();
                        let skip = row * width;
                        let tile_row = tile.iter().skip(skip as usize).take(width as usize);
                        image.extend(tile_row.cloned());
                    };
                    cursor_x += width;
                }
                let cols_to_fill = image_width - cursor_x;
                image.extend(repeat(0).take(cols_to_fill as usize));
            }
        }

        // Precalculate some fields to make it easier to use our font.
        for (_, ch_info) in chars_info.iter_mut() {
            ch_info.tex[0] /= image_width as f32;
            ch_info.tex[1] /= image_height as f32;
            ch_info.tex_width = ch_info.width as f32 / image_width as f32;
            ch_info.tex_height = ch_info.height as f32 / image_height as f32;
        }

        // println!("Image width: {}, image height: {}", image_width, image_height);

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
