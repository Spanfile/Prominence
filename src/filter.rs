const BLACK_MAX_LIGHTNESS: f32 = 0.05;
const WHITE_MIN_LIGHTNESS: f32 = 0.95;

pub trait Filter {
    fn is_allowed(&self, rgb: (u8, u8, u8), hsl: (f32, f32, f32)) -> bool;
}

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
