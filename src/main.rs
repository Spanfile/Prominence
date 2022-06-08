use prominence::image::io::Reader as ImageReader;

fn main() {
    let reader = ImageReader::open("ab67616d0000b273e53ce7e1e6354cb3b03facf8.jpg").unwrap();
    let img = reader.decode().unwrap();
    let buf = img.to_rgb8();

    let palette = prominence::PaletteBuilder::from_image(buf)
        .resize_image_area(0)
        .generate();

    println!("{:#?}", palette);
}
