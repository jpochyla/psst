fn main() {
    #[cfg(windows)]
    {
        let ico_path = "assets/logo.ico";
        if std::fs::metadata(ico_path).is_err() {
            let ico_frames = load_images();
            save_ico(&ico_frames, ico_path);
        }
        let mut res = winres::WindowsResource::new();
        res.set_icon(ico_path);
        res.compile().expect("Could not attach exe icon");
    }
}

#[cfg(windows)]
use image::{
    codecs::ico::{IcoEncoder, IcoFrame},
    ColorType,
};

#[cfg(windows)]
fn load_images() -> Vec<IcoFrame<'static>> {
    let sizes = vec![32, 64, 128, 256];
    sizes
        .iter()
        .map(|s| {
            IcoFrame::as_png(
                image::open(format!("assets/logo_{}.png", s))
                    .unwrap()
                    .as_bytes(),
                *s,
                *s,
                ColorType::Rgba8,
            )
            .unwrap()
        })
        .collect()
}

#[cfg(windows)]
fn save_ico(images: &[IcoFrame<'_>], ico_path: &str) {
    let file = std::fs::File::create(ico_path).unwrap();
    let encoder = IcoEncoder::new(file);
    encoder.encode_images(&images).unwrap();
}
