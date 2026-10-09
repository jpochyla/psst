//! Read-only network smoke test: verify Spotify's signed handshake without login.
use psst_core::connection::Transport;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let access_points = Transport::resolve_ap(None)?;
    println!("Resolved {} Spotify access points.", access_points.len());
    let _transport = Transport::connect(&access_points, None)?;
    println!("Spotify server identity verified; no user credentials sent.");
    Ok(())
}
