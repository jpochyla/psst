use std::{env, fs, io::Write};

fn main() {
    let outdir = env::var("OUT_DIR").unwrap();
    let outfile = format!("{}/last-build.txt", outdir);

    let mut fh = fs::File::create(&outfile).unwrap();
    write!(fh, r#""{}""#, chrono::Local::now()).ok();
}
