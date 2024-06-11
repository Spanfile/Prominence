use prominence::image::io::Reader as ImageReader;

const BLACK_MAX_LIGHTNESS: f32 = 0.02;
const WHITE_MIN_LIGHTNESS: f32 = 0.90;

// this filter uses the same approach as the default filter in prominence, except it allows more
// darker colors and blocks more lighter colors
struct CustomFilter;
impl prominence::Filter for CustomFilter {
    fn is_allowed(&self, _: (u8, u8, u8), (_, _, l): (f32, f32, f32)) -> bool {
        !is_black(l) && !is_white(l)
    }
}

fn is_black(l: f32) -> bool {
    l <= BLACK_MAX_LIGHTNESS
}

fn is_white(l: f32) -> bool {
    l >= WHITE_MIN_LIGHTNESS
}

fn main() {
    let reader = ImageReader::open("ab67616d0000b2732cd7888600aafe2eb8b6be9f.jpg").unwrap();
    let img = reader.decode().unwrap();
    let buf = img.to_rgb8();

    let palette = prominence::PaletteBuilder::from_image(buf)
        .clear_filters() // remove the default filter
        .add_filter(CustomFilter) // add our custom filter
        .generate();

    println!("{:#?}", palette);
}
