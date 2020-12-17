pub fn main() {
    cc::Build::new()
        .file("./minivorbis.c")
        .include("./minivorbis")
        .compile("libminivorbis");

    println!("cargo:rerun-if-changed=./minivorbis/minivorbis.h");
    println!("cargo:rerun-if-changed=./minivorbis.c");
    println!("cargo:rerun-if-env-changed=CC");
}
