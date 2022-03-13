fn main() {
    #[cfg(windows)]
    {
        build_logo_ico();
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/logo.ico");
        res.compile().expect("Could not attach exe icon");
    }
}
use image::{
    codecs::ico::{IcoEncoder, IcoFrame},
    io::Reader as ImageReader,
    ColorType,
};
fn build_logo_ico() {
    let sizes = vec![16, 32, 64, 128, 256];
    let images = sizes.iter().map(|s| {
        IcoFrame::as_png(
            image::open(format!("assets/logo_{}.png", s))
                .unwrap()
                .as_bytes(),
            *s,
            *s,
            ColorType::Rgba8,
        )
        .unwrap()
    }).collect::<Vec<IcoFrame<'_>>>();
    let file = std::fs::File::open("assets/logo_256.png").unwrap();
    let encoder = IcoEncoder::new(file);
    encoder.encode_images(&images);
}
