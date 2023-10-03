use std::{env, fs, io::Write};

fn main() {
    let outdir = env::var("OUT_DIR").unwrap();
    let outfile = format!("{}/build-time.txt", outdir);

    let mut fh = fs::File::create(outfile).unwrap();
    write!(fh, r#""{}""#, chrono::Local::now()).ok();

    let git_config = gix_config::File::from_git_dir("../.git/".into()).expect("Git Config not found!");
    // Get Git's 'Origin' URL
    let mut remote_url = git_config
        .raw_value("remote", Some("origin".as_ref()), "url")
        .expect("Couldn't extract origin url!")
        .to_string();
    // Check whether origin is accessed via ssh
    if remote_url.contains('@') {
        // If yes, strip the `git@` prefix and split the domain and path
        let mut split = remote_url
            .strip_prefix("git@")
            .unwrap_or(&remote_url)
            .split(':');
        let domain = split
            .next()
            .expect("Couldn't extract domain from ssh-style origin");
        let path = split
            .next()
            .expect("Couldn't expect path from ssh-style origin");

        // And construct the http-style url
        remote_url = format!("https://{domain}/{path}");
    }
    remote_url = remote_url.trim_end_matches(".git").to_owned();
    let outfile = format!("{}/remote-url.txt", outdir);
    let mut file = fs::File::create(outfile).unwrap();
    write!(file, r#""{}""#, remote_url).ok();
}
