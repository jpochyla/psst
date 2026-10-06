fn main() {
    #[cfg(windows)]
    add_windows_icon();
}

#[cfg(windows)]
fn add_windows_icon() {
    use image::{
        codecs::ico::{IcoEncoder, IcoFrame},
        ColorType,
    };

    let ico_path = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("logo.ico");
    let ico_frames = load_images();
    save_ico(&ico_frames, &ico_path);

    let mut res = winres::WindowsResource::new();
    res.set_icon(ico_path.to_str().unwrap());
    res.set("ProductName", "Xpotify");
    res.set("FileDescription", "Xpotify");
    res.set("InternalName", "Xpotify");
    res.set("OriginalFilename", "Xpotify.exe");
    // The taskbar resolves its application label from this executable resource.
    res.append_rc_content("STRINGTABLE\nBEGIN\n    101 \"Xpotify\"\nEND\n");
    res.compile().expect("Could not attach exe icon");

    fn load_images() -> Vec<IcoFrame<'static>> {
        let sizes = [16, 32, 64, 128, 256];
        sizes
            .iter()
            .map(|s| {
                println!("cargo:rerun-if-changed=assets/logo_{s}.png");
                IcoFrame::as_png(
                    image::open(format!("assets/logo_{s}.png"))
                        .unwrap()
                        .as_bytes(),
                    *s,
                    *s,
                    ColorType::Rgba8.into(),
                )
                .unwrap()
            })
            .collect()
    }

    fn save_ico(images: &[IcoFrame<'_>], ico_path: &std::path::Path) {
        let file = std::fs::File::create(ico_path).unwrap();
        let encoder = IcoEncoder::new(file);
        encoder.encode_images(images).unwrap();
    }
}
