//! Read-only check of the playback transport using existing local credentials.
use psst_core::{
    cdn::Cdn,
    connection::Credentials,
    item_id::{ItemId, ItemIdType},
    metadata::{Fetch, ToMediaPath},
    session::{SessionConfig, SessionService},
};
use std::{io::Read, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 3 {
        return Err(
            "usage: diagnose_playback <config.json> <track-id> [--authorize] [--play]".into(),
        );
    }
    let config: serde_json::Value =
        serde_json::from_slice(&std::fs::read(PathBuf::from(&args[1]))?)?;
    let mut credentials: Credentials = serde_json::from_value(config["credentials"].clone())?;
    if args.iter().any(|a| a == "--authorize") {
        let (url, verifier) = psst_core::oauth::generate_playback_auth_url();
        let code = psst_core::oauth::get_authcode_listener_with_state_and_ready(
            "127.0.0.1:8898".parse()?,
            std::time::Duration::from_secs(300),
            &url,
            || {
                std::process::Command::new("rundll32.exe")
                    .args(["url.dll,FileProtocolHandler", &url])
                    .spawn()?;
                Ok(())
            },
        )?;
        let token = psst_core::oauth::exchange_webapi_code_for_token(
            psst_core::system_info::CLIENT_ID,
            8898,
            code,
            verifier,
        )?;
        credentials = psst_core::session::SessionConnection::open(SessionConfig {
            login_creds: Credentials::from_access_token(token.access_token),
            proxy_url: None,
        })?
        .credentials;
        let mut updated = config.clone();
        updated["credentials"] = serde_json::to_value(&credentials)?;
        updated["theme"] = serde_json::json!("System");
        std::fs::write(&args[1], serde_json::to_vec_pretty(&updated)?)?;
        println!("native playback authorization: saved");
    }
    let session = SessionService::with_config(SessionConfig {
        login_creds: credentials,
        proxy_url: None,
    });
    session.connected()?;
    println!("session: authenticated");
    let id = ItemId::from_base62(&args[2], ItemIdType::Track).ok_or("invalid track ID")?;
    if args.iter().any(|a| a == "--download") {
        let cache =
            psst_core::cache::Cache::new(std::env::temp_dir().join("xpotify-download-validation"))?;
        psst_core::player::item::PlaybackItem {
            item_id: id,
            norm_level: psst_core::audio::normalize::NormalizationLevel::Track,
        }
        .download(
            &session,
            Cdn::new(session.clone(), None)?,
            cache,
            &psst_core::player::PlaybackConfig::default(),
        )?;
        println!(
            "download: complete encrypted track cached at requested 320 kb/s (no audio output)"
        );
        return Ok(());
    }
    let track = librespot_protocol::metadata::Track::fetch(&session, id)?;
    println!("metadata: received");
    let path = track
        .to_media_path(160)
        .ok_or("track has no playable audio file")?;
    session
        .connected()?
        .get_audio_key(path.item_id, path.file_id)?;
    println!("audio key: received");
    let client_token = psst_core::session::client_token::ClientTokenProvider::new(None);
    client_token.get()?;
    println!("client token: received");
    psst_core::session::login5::Login5::new(None, None).get_access_token(&session)?;
    println!("audio access token: received");
    let cdn = Cdn::new(session.clone(), None)?;
    let uri = cdn.resolve_audio_file_url(path.file_id)?;
    println!("audio location: resolved");
    let (total, mut reader) = cdn.fetch_file_range(&uri.url, 0, 6144)?;
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;
    println!("audio range: {} bytes received of {total}", bytes.len());
    if args.iter().any(|a| a == "--play") {
        use psst_core::{
            audio::{normalize::NormalizationLevel, output::DefaultAudioOutput},
            cache::Cache,
            player::{item::PlaybackItem, PlaybackConfig, Player, PlayerCommand, PlayerEvent},
        };
        let output = DefaultAudioOutput::open()?;
        let cache = Cache::new(std::env::temp_dir().join("xpotify-audio-validation"))?;
        let mut player = Player::new(
            session.clone(),
            cdn,
            cache,
            PlaybackConfig::default(),
            &output,
        );
        player.handle(PlayerEvent::Command(PlayerCommand::SetVolume {
            volume: 0.15,
        }));
        player.handle(PlayerEvent::Command(PlayerCommand::LoadQueue {
            items: vec![PlaybackItem {
                item_id: id,
                norm_level: NormalizationLevel::Track,
            }],
            position: 0,
        }));
        let receiver = player.receiver();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        let mut advanced = false;
        while std::time::Instant::now() < deadline {
            let event = receiver.recv_timeout(std::time::Duration::from_secs(5))?;
            if let PlayerEvent::Playing { .. } = &event {
                println!("player: decoded audio playing through default device");
            }
            if let PlayerEvent::Position { position, .. } = &event {
                if position.as_secs() >= 2 {
                    advanced = true;
                    println!("player: playback advanced beyond two seconds");
                }
            }
            player.handle(event);
            if advanced {
                break;
            }
        }
        player.handle(PlayerEvent::Command(PlayerCommand::Stop));
        if !advanced {
            return Err("player did not advance".into());
        }
    }
    session.shutdown();
    Ok(())
}
