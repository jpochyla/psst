fn main() {
    #[cfg(windows)]
    {
        build_logo_ico();
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/logo.ico");
        res.compile().expect("Could not attach exe icon");
    }
}

fn build_logo_ico() {
    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);
    // Read a PNG file from disk and add it to the collection:
    let file = std::fs::File::open("assets/logo_256.png").unwrap();
    let image = ico::IconImage::read_png(file).unwrap();
    icon_dir.add_entry(ico::IconDirEntry::encode(&image).unwrap());
    // Finally, write the ICO file to disk:
    let file = std::fs::File::create("assets/logo.ico").unwrap();
    icon_dir.write(file).unwrap();
}