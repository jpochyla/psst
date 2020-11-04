use psst_core::{
    audio_output::AudioOutput,
    audio_player::{PlaybackConfig, PlaybackItem, Player, PlayerCommand, PlayerEvent},
    cache::{Cache, CacheHandle},
    cdn::{Cdn, CdnHandle},
    connection::Credentials,
    error::Error,
    item_id::{ItemId, ItemIdType},
    session::SessionHandle,
};
use std::{io, io::BufRead, path::PathBuf, thread};

fn main() {
    env_logger::init();

    let login_creds = Credentials::from_username_and_password("...".into(), "...".into());
    let session = SessionHandle::new();

    let connection = session.connect(login_creds).unwrap();
    let processing = thread::spawn({
        move || {
            connection.service().unwrap();
        }
    });

    start(session).unwrap();
    processing.join().unwrap();
}

fn start(session: SessionHandle) -> Result<(), Error> {
    let cdn = Cdn::connect(session.clone());
    let cache = Cache::new(PathBuf::from("cache"))?;
    let item_id = ItemId::from_base62("6UCFZ9ZOFRxK8oak7MdPZu", ItemIdType::Track).unwrap();
    play_item(session, cdn, cache, PlaybackItem { item_id })
}

fn play_item(
    session: SessionHandle,
    cdn: CdnHandle,
    cache: CacheHandle,
    item: PlaybackItem,
) -> Result<(), Error> {
    let output = AudioOutput::open()?;
    let output_ctrl = output.controller();
    let config = PlaybackConfig::default();

    let (mut player, player_receiver) =
        Player::new(session, cdn, cache, config, output.controller());

    let output_thread = thread::spawn({
        let player_source = player.audio_source();
        move || {
            output
                .start_playback(player_source)
                .expect("Playback failed");
        }
    });

    let _ui_thread = thread::spawn({
        let player_sender = player.event_sender();

        player_sender
            .send(PlayerEvent::Command(PlayerCommand::LoadQueue {
                items: vec![item, item, item],
                position: 0,
            }))
            .unwrap();

        move || {
            for line in io::stdin().lock().lines() {
                match line.as_ref().map(|s| s.as_str()) {
                    Ok("p") => {
                        player_sender
                            .send(PlayerEvent::Command(PlayerCommand::Pause))
                            .unwrap();
                    }
                    Ok("r") => {
                        player_sender
                            .send(PlayerEvent::Command(PlayerCommand::Resume))
                            .unwrap();
                    }
                    Ok("s") => {
                        player_sender
                            .send(PlayerEvent::Command(PlayerCommand::Stop))
                            .unwrap();
                    }
                    Ok("<") => {
                        player_sender
                            .send(PlayerEvent::Command(PlayerCommand::Previous))
                            .unwrap();
                    }
                    Ok(">") => {
                        player_sender
                            .send(PlayerEvent::Command(PlayerCommand::Next))
                            .unwrap();
                    }
                    _ => log::warn!("unknown command"),
                }
            }
        }
    });

    for event in player_receiver {
        player.handle(event);
    }

    output_ctrl.close();
    output_thread.join().unwrap();

    Ok(())
}
