use crate::error::Error;
use discord_presence::{models::ActivityType, Client as DiscordClient, DiscordError};

use std::{
    sync::Arc,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crossbeam_channel::{unbounded, Receiver, Sender};

pub struct DiscordRPCClient;

pub enum DiscordRpcCmd {
    Update {
        track: Arc<str>,
        artist: Arc<str>,
        album: Option<String>,
        cover_url: Option<String>,
        duration: Option<Duration>,
        position: Option<Duration>,
    },
    Clear,
}

impl DiscordRPCClient {
    /// Creates a Discord Rich Presence client for Spotify with the provided application ID.
    pub fn create_client(app_id: u64) -> Result<DiscordClient, Error> {
        let mut client = DiscordClient::new(app_id);
        client.start();
        log::info!("Discord RPC client created and started");
        Ok(client)
    }

    pub fn spawn_rpc_worker(app_id: u64) -> Result<Sender<DiscordRpcCmd>, Error> {
        let mut rpc = DiscordRPCClient::create_client(app_id)?;

        let (tx, rx): (Sender<DiscordRpcCmd>, Receiver<DiscordRpcCmd>) = unbounded();

        thread::spawn(move || {
            for cmd in rx {
                match cmd {
                    DiscordRpcCmd::Update {
                        track,
                        artist,
                        album,
                        cover_url,
                        duration,
                        position,
                    } => {
                        let _ = DiscordRPCClient::now_playing_song(
                            &mut rpc,
                            &track,
                            &artist,
                            album.as_deref(),
                            cover_url.as_deref(),
                            duration,
                            position,
                        );
                    }
                    DiscordRpcCmd::Clear => {
                        let _ = DiscordRPCClient::clear_presence(&mut rpc);
                    }
                }
            }
            // when tx is dropped everywhere, rx returns Err -> loop ends, rpc is dropped
        });

        Ok(tx)
    }

    fn get_current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs()
    }

    /// Update the Discord Rich Presence with currently playing Spotify track information.
    pub fn now_playing_song(
        client: &mut DiscordClient,
        track_name: &str,
        artist: &str,
        album: Option<&str>,
        album_cover_url: Option<&str>,
        track_duration: Option<Duration>,
        playback_position: Option<Duration>,
    ) -> Result<(), Error> {
        client
            .set_activity(|act| {
                let mut act = act
                    .details(track_name)
                    .state(artist)
                    ._type(ActivityType::Listening);

                if let Some(cover_url) = album_cover_url {
                    act = act.assets(|assets| {
                        let mut assets = assets.large_image(cover_url);
                        if let Some(album_name) = album {
                            assets = assets.large_text(album_name);
                        }
                        assets
                    });
                }

                if let Some(duration) = track_duration {
                    let now = Self::get_current_timestamp();
                    let position_secs = playback_position
                        .unwrap_or(Duration::from_secs(0))
                        .as_secs();

                    let start_time = now.saturating_sub(position_secs);
                    let end_time = start_time + duration.as_secs();
                    act = act.timestamps(|timestamps| timestamps.start(start_time).end(end_time));
                }

                act
            })
            .map(|_| ()) // Discard the Payload<Activity> and return ()
            .map_err(Error::from)
    }

    /// Stop displaying Rich Presence by clearing the activity.
    pub fn clear_presence(client: &mut DiscordClient) -> Result<(), Error> {
        // Map the payload result to () to match our return type
        client.clear_activity().map(|_| ()).map_err(Error::from)
    }
}

impl From<DiscordError> for Error {
    fn from(value: DiscordError) -> Self {
        Self::DiscordRPCError(Box::new(value))
    }
}
