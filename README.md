# gfx_text [![Build Status](https://travis-ci.org/PistonDevelopers/gfx_text.png?branch=master)](https://travis-ci.org/PistonDevelopers/gfx_text) [![crates.io](https://img.shields.io/crates/v/gfx_text.svg)](https://crates.io/crates/gfx_text)

Library for drawing text for [gfx-rs](https://github.com/gfx-rs/gfx-rs) graphics API. Uses [freetype-rs](https://github.com/PistonDevelopers/freetype-rs) underneath.

## Usage

Basic usage:

```rust
// Initialize text renderer.
let mut text = gfx_text::new(factory).build().unwrap();

// In render loop:

// Add some text 10 pixels down and right from the top left screen corner.
text.add(
    "The quick brown fox jumps over the lazy dog",  // Text to add
    [10, 10],                                       // Position
    [0.65, 0.16, 0.16, 1.0],                        // Text color
);

// Draw text.
text.draw(&mut stream);
```

See [API documentation](http://docs.piston.rs/gfx_text/gfx_text/) for overview of all available methods.

You can skip default font by disabling `include-font` feature:

```
[dependencies.gfx_text]
version = "*"
default-features = false
```

## Examples

See [this example](./examples/styles.rs) on how to draw text in various styles: different sizes, colors, fonts, etc.

Output:

[![](https://raw.githubusercontent.com/PistonDevelopers/gfx_text/images/styles.png)](https://raw.githubusercontent.com/PistonDevelopers/gfx_text/images/styles.png)

## License

* gfx_text licensed under [MIT License](./LICENSE)
* Included by default Noto Sans font uses [Apache License 2.0](./assets/LICENSE.txt)
* Ubuntu Font used in examples has [Ubuntu Font Licence 1.0](./examples/assets/LICENSE.txt)
