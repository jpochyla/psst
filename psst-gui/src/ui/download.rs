use crate::{
    cmd,
    data::{AppState, Config, Track},
    widget::MyWidgetExt,
};
use druid::{Selector, Widget};
use psst_core::{
    audio::normalize::NormalizationLevel,
    cache::Cache,
    cdn::Cdn,
    player::{item::PlaybackItem, PlaybackConfig},
    session::SessionService,
};
use std::sync::Arc;

type Job = Arc<(Arc<Track>, SessionService, PlaybackConfig)>;
const START: Selector<Job> = Selector::new("app.start-encrypted-download");

pub fn controller(widget: impl Widget<AppState> + 'static) -> impl Widget<AppState> {
    widget
        .on_command(cmd::DOWNLOAD_TRACK, |ctx, track, state| {
            ctx.submit_command(START.with(Arc::new((
                track.clone(),
                state.session.clone(),
                state.config.playback(),
            ))));
        })
        .on_command_async(
            START,
            |job| -> Result<(), String> {
                let (track, session, config) = job.as_ref();
                // Downloads are serialized; a repeated request reuses the completed file.
                static LOCK: parking_lot::Mutex<()> = parking_lot::Mutex::new(());
                let _guard = LOCK.lock();
                let cache =
                    Cache::new(Config::cache_dir().ok_or("No se encontró la carpeta de caché")?)
                        .map_err(|error| error.to_string())?;
                let cdn = Cdn::new(session.clone(), Config::proxy().as_deref())
                    .map_err(|error| error.to_string())?;
                PlaybackItem {
                    item_id: track.id.0,
                    norm_level: NormalizationLevel::Track,
                }
                .download(session, cdn, cache, config)
                .map_err(|error| error.to_string())
            },
            |_, state, job| {
                let track = &job.0;
                state.info_alert(format!("Descargando {} a la caché de audio…", track.name));
            },
            |_, state, (job, result)| {
                let track = &job.0;
                match result {
                    Ok(()) => state.info_alert(format!(
                        "{} descargada. Audio cifrado guardado en la caché.",
                        track.name
                    )),
                    Err(error) => {
                        state.error_alert(format!("No se pudo descargar {}: {error}", track.name))
                    }
                }
            },
        )
}
