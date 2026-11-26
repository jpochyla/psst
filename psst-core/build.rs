use std::{env, fs, io::Write};
use time::OffsetDateTime;

fn main() {
    let outdir = env::var("OUT_DIR").unwrap();
    let outfile = format!("{outdir}/build-time.txt");

    let mut fh = fs::File::create(outfile).unwrap();
    let now = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
    write!(fh, r#""{now}""#).ok();

    // Do not require a .git directory or embed an arbitrary remote in Rust source.
    let remote_url = "https://github.com/angelopol/xpotify";

    let outfile = format!("{outdir}/remote-url.txt");
    let mut file = fs::File::create(outfile).unwrap();
    write!(file, r#""{remote_url}""#).ok();
}
