const BLACK_MAX_LIGHTNESS: f32 = 0.05;
const WHITE_MIN_LIGHTNESS: f32 = 0.95;

/// A trait used to implement filters for the image quantization process.
///
/// During the image quantization process, filters are used to remove colors from the quantization
/// process, and to remove final color swatches that may have their average color end up as
/// filtered. This trait allows the library consumer to implement custom filters.
///
/// See [`crate::PaletteBuilder::add_filter`] on how to add filters to the quantization process.
pub trait Filter {
    /// Return whether a given color should be allowed or not. The same color is given in both sRGB
    /// and HSL for convenience.
    fn is_allowed(&self, rgb: (u8, u8, u8), hsl: (f32, f32, f32)) -> bool;
}

/// The default filter included in every [`crate::PaletteBuilder`] by default.
///
/// This filter will disallow colors very close to black, colors very close to white, and colors
/// near the red I line, whatever that is.
#[derive(Debug)]
pub struct DefaultFilter;
impl Filter for DefaultFilter {
    fn is_allowed(&self, _: (u8, u8, u8), (h, s, l): (f32, f32, f32)) -> bool {
        !is_black(l) && !is_white(l) && !is_near_red_i_line(h, s)
    }
}

fn is_black(l: f32) -> bool {
    l <= BLACK_MAX_LIGHTNESS
}

fn is_white(l: f32) -> bool {
    l >= WHITE_MIN_LIGHTNESS
}

fn is_near_red_i_line(h: f32, s: f32) -> bool {
    (10.0..=37.0).contains(&h) && s <= 0.82
}
