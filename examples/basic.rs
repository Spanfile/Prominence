use prominence::image::io::Reader as ImageReader;

fn main() {
    let reader = ImageReader::open("ab67616d0000b2732cd7888600aafe2eb8b6be9f.jpg").unwrap();
    let img = reader.decode().unwrap();
    let buf = img.to_rgb8();

    let palette = prominence::PaletteBuilder::from_image(buf).generate();

    println!("{:#?}", palette);
}
