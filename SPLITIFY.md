# Xpotify with Splitify

## Windows 0.4.5

The library defaults to a 72 px compact rail with playlist artwork, navigation icons, tooltips and Preferences. Its top library button expands the full 260 px sidebar; the same button collapses it. This preference persists across restarts. Both views retain playlist context menus, active navigation and the same cached playlist/profile data; their request handlers stay mounted while switching modes. Playlist folders remain available in the expanded view.

Home carousels use one horizontal scroll per section and reserve 16 px below their cards. Card labels have an 80 px clipped area so long descriptions cannot paint over the scrollbar or following section. The source icon beside the current track opens its album/playlist page, with an explicit tooltip and left-click navigation.

If local Connect is still registering or its command channel is full, Xpotify retains the latest validated playback selection and sends it once the receiver is ready. Pause/resume changes that pending intent; Stop, switching to a remote device, disabling Connect or closing the app cancels it. An explicit selection may trigger reconnection, while automatic network failures retain the existing retry backoff. A closed command channel is distinguished from a full one. Startup still restores in pause and does not activate this PC without a playback request.

Both the current-playback panel and full queue show 50 upcoming songs per page. Use **‹ 50** / **50 ›** to move backward or forward; the range label shows the current entries and total. Scroll within each page to see all its rows. The final page contains the remaining songs, and unavailable navigation buttons are disabled. Pagination keeps absolute queue positions, including duplicate tracks, and returns to the top of the queue rows when changing pages. Only visible rows create widgets and request artwork; paging makes no additional Spotify Web API calls. Native Connect currently supplies an 80-track upcoming window, so its second page can contain 30 songs; this UI does not infer additional songs from the unshuffled playlist or change the engine's wire limit.

The audio shadow spans the complete footer, including volume, with mirrored bands that bring bass pulses to both ends. The seek line is now 2 px tall while retaining its 8 px mouse target.

The playback footer in 0.4.2 has a subtle green square-cell audio shadow driven by eight local PCM bands. It fades out on pause, silence, mute or stale output. The animation uses a local 30 Hz timer and never requests Spotify audio analysis or updates queue data. Remote devices do not expose their PCM to this PC.

Playlist pages now have a persistent **Reproducir playlist** button, which starts all loaded tracks from the beginning using the current playback mode. It is disabled while tracks are unavailable. Scrollable library views reserve a wider right gutter, and the playback panel places **Letras**, **Cola** and **Videoclip** in one row. Preferences → About → Source points to `https://github.com/angelopol/xpotify`.

This update adapts Spotifast's compact colors and divider spacing. The native Connect queue observes actual engine shuffle/advancement directly, without Web API polling. Selecting an active queue row retains its context and remaining order. Large queue panels render only visible rows; equivalent cached reads share a per-key lock, while at most two HTTP responses are in flight. Transient JSON reads share a three-attempt budget. See `SPOTIFAST-INTEGRATION.md` in the source repository for details and limits.

Download the optimized Windows x64 executable or portable ZIP from [GitHub Releases](https://github.com/angelopol/xpotify/releases/latest). The package includes the launcher, documentation, the Psst, librespot and Spotifast licenses and SHA-256 verification. Build locally with `powershell -File scripts/Build-Native.ps1 -Release`; `Package-Native.ps1` now uses the release build by default. The release executable excludes development preview modes.

Windows taskbar thumbnails include **Anterior**, **Reproducir/Pausar** and **Siguiente**, including while the main window is minimized. The middle icon follows playback state. Controls are disabled when playback is unavailable, and reinstalled when Explorer recreates the taskbar.

Right-click **Add to Playlist** lists your own and collaborative playlists. When the library or profile is unavailable it displays an explanation, plus **Buscar playlist o pegar enlace...**. The selector filters locally, shows available covers, explains quota/loading errors and accepts an editable playlist's Spotify URL/URI. Hovering does not make network requests. Spotify permissions and quotas still apply when adding a song.

Startup loads the requested view and the sidebar/profile rather than eagerly downloading saved tracks and home sections. Navigation deduplicates pending loads. Unchanged playlist snapshots reuse cached complete tracks, including duplicate occurrences and original positions; a changed snapshot invalidates old page data before reading it. Refreshing a playlist or saved collection invalidates only its corresponding response pages, and route refresh preserves other views in memory. Transient failed reads are held for 30 seconds to prevent immediate repeated failures. Last successful profile/playlist-list responses can survive metadata refresh for temporary-error fallback for up to 14 days, scoped to the OAuth session. Image requests share one download per URL with at most eight workers.

Native Spotify Connect reception is integrated with librespot 0.8.0 and the fork's native audio output. Enable it in Devices (enabled by default), then select `Xpotify · [PC name]` from Spotify on your phone. Playback starts inactive and preserves the restored song in pause. Playback, seek, volume and queue events use the Connect session, without Web API polling. The original local engine remains available by disabling native Connect, including local files. Audio output changes restart the receiver in pause. The Connect audio cache is capped at 512 MiB and contributes to the cache usage shown in Preferences.

The user explicitly chose native playback over the official player. Native listen reporting remains unavailable: Connect state updates do not guarantee Spotify history, Wrapped or stream counts. Last.fm is separate. Other limitations are recorded in [ROADMAP.md](ROADMAP.md), including 24 kb/s HE-AAC Low and untested macOS/Linux builds. The executable is not Authenticode signed. The lockfile pins vergen 9.0.6 because librespot 0.8.0's build metadata dependency is incompatible with vergen 9.1.0.

The native integration lives in the existing xpotify repository, branch `splitify-integration`. The original Splitify repository stays on `main` with its Next.js web application. No nested Rust copy or separate native repository is required.

The fork roadmap and its remaining limits are tracked in [ROADMAP.md](ROADMAP.md). The native app now includes paged track lists, album/artist grids, local playlist folders, safe playlist reordering, a resizable current-playback pane, network retries and metadata cache dates/refresh. Playback state preserves the engine's actual shuffle order and manually queued tracks and restores in pause. Windows distribution: `powershell -File scripts/Package-Native.ps1`; the ZIP contains no account credentials or Gemini key. macOS/Linux packaging scripts and a manual workflow are provided but have not been tested on this Windows machine.

## Run on this machine

Double-click `Xpotify.lnk` for the green app shortcut, or `Start-Xpotify.cmd`. The launcher starts `dist/Xpotify.exe` from this repository, loading `.env.local` from the project directory. To rebuild: `powershell -File scripts/Build-Native.ps1`. Rust stable and Visual Studio C++ Build Tools / Windows SDK are required for compilation. The running application requires neither Node nor a web server.

The default theme is **System**. On Windows it follows AppsUseLightTheme and updates when that setting changes. Preferences can override it with Light or Dark. The system option falls back to light on other platforms.

## Playback queue and app icon

Open **Cola** in the sidebar or the queue icon beside the playback controls. The view shows the current song and upcoming tracks with artist and duration. It follows the actual native player's shuffle order, manually added tracks, duplicates, track changes and repeat mode. When stopped, manually queued tracks can be started with **Reproducir cola**. Stop or starting a new source resets the queue.

The Windows executable has a green waveform icon, generated from the matching vector design in `psst-gui/assets/logo.svg`. To regenerate its PNG sizes, run `powershell -File scripts/Generate-AppIcon.ps1`, then rebuild. The build always regenerates the ICO resource in Cargo's output directory, avoiding a stale cached icon in the source tree.

Windows uses the embedded **Xpotify** name and explicit window relaunch properties for the taskbar menu, independently of Cargo's internal `psst-gui` target name. The relaunch command points to the running executable, so pinning also works for the portable download.

## Spotify login

Set your Spotify Developer application's redirect URI to exactly `http://127.0.0.1:8888/login`. Keep the web application's redirect URIs too. The first browser authorization uses your configured Client ID to access the library and playlists. The second uses the native playback client, the `streaming` scope, and its registered `http://127.0.0.1:8898/login` callback. You do not add the second callback to your Developer application.

Both flows use PKCE, random state, bounded loopback listeners, and no client secret. The listeners bind before the browser opens. Native playback credentials and Developer API tokens must have separate client identities: using the Developer client's stored credentials with desktop Login5 rejects playback. Premium is required. This machine's existing session has already been repaired and successfully tested.

The configuration remains in the existing Psst user profile so the Spotify session and preferences survive migration. Tokens are local configuration data, not embedded in the executable.

## Organize playlists

Open **Organizar con IA** in the sidebar or the home screen, or right-click a playlist and choose **Dividir con Splitify IA**. Choose or paste a playlist, describe the grouping, optionally provide comma-separated subcategories, and generate a preview. Listen to tracks, move them between categories, rename or merge categories, and deselect tracks before creating private playlists. The source playlist is preserved. Partial failures list playlists already created and prevent immediate duplicate creation.

The fixed model is `gemini-3.5-flash-lite`. Add local values in the ignored `.env.local`:

```dotenv
SPOTIFY_CLIENT_ID="your-client-id"
AI_AGENT_API_KEY="your-gemini-key"
AI_AGENT_MODEL="gemini-3.5-flash-lite"
TZ="America/Caracas"
```

Only musical metadata is sent to Gemini. Classification handles batches of 150 tracks and rejects invented IDs and missing tracks. Creation uses batches of 100 tracks. The web editor's SQLite history, text import and Last.fm enrichment have not been ported.

Debug builds include local UI previews: `dist/Xpotify.exe --preview-ui=login`, `--preview-ui=editor`, `--preview-ui=editor-empty`, `--preview-ui=player`, `--preview-ui=queue` and `--preview-ui=queue-empty`; add `--light` or `--dark` for dark fixtures. Preview commands cannot issue remote writes and never replace normal startup data.

See `SECURITY-REVIEW.md` for the security review and remaining dependency risks; `VALIDATION.md` records actual checks. Upstream MIT licensing is preserved in `LICENSE.md`.

Icon controls show Spanish tooltips after 500 ms of hover. Playback hints describe the current shuffle/repeat mode and the next action; save hints distinguish adding from removing. Hints appear above controls when possible and disappear on exit or click.

## Lyrics and music videos

The music-note button opens lyrics for the current song and refreshes them as the song changes. LRCLIB is queried by title, primary artist, album and duration using HTTPS without Spotify credentials; Spotify is a fallback. Available timed lyrics highlight the current line and support click-to-seek, including the first line at zero seconds. Plain lyrics and instrumental tracks have distinct messages; unavailable lyrics can be retried. Successful results are kept in a bounded in-memory cache. Availability depends on provider coverage.

The video icon and each song's context menu offer **Buscar videoclip en YouTube**. This opens a browser search for the artist and title and pauses native music to avoid overlapping audio. It does not select an unverified video automatically or embed video in the native window. Spotify's track API does not provide a music-video playback URL.

Development UI review: `--preview-ui=lyrics --dark` or `--light` uses synthetic lyrics and blocks remote actions.

## Library, startup and Spotify Connect

The main window now has a global search field and Home button at the top, playlist cover thumbnails in the library, and a full-width player with volume at the bottom. Device and news icons remain available across routes. System remains the default theme.

Closing the app saves the current track, source playlist/album, queue and position in the local profile. Startup restores them paused and loads the source view. Audio is loaded only after pressing Play. Snapshots contain up to 5,000 entries and preserve the engine's exact shuffle order, repeat mode and manual queue additions. Older profiles gain this state when saved. The default library loading limit is now 5,000, with 100 visible track rows per page.

Open the device icon to select a Spotify Connect device or **Este equipo · Xpotify**. Remote controls use Spotify's official playback API, including play/pause, previous/next, seek, queue additions, shuffle/repeat and volume. Remote queue entries come from Spotify. Open Spotify on the other device with the same account so it can be discovered. Returning to this computer keeps the song paused, ready to continue. Device support and account permissions can limit available controls.

The native fork is not a Connect receiver. To control this PC from a phone, use **Abrir Spotify en esta PC** in the device view and select the official desktop client's device. This requires Spotify desktop; it does not remotely control Xpotify's native engine. Remote status polling now runs every 15 seconds playing or 60 seconds paused, with local progress interpolation.

HTTP 429 is a server quota, not a timeout. Xpotify persists Spotify's retry deadline, suppresses further requests during the cooldown, displays a countdown and disables retry until expiry. Metadata refresh preserves existing cache during the block. Ordinary cached responses last 15 minutes; timed-out reads retain their bounded retries. The existing Spotify-imposed wait cannot be removed by the app.

Click an upcoming queue song to play it while preserving the native queue traversal. Right-click songs in browsing lists, the queue, current playback or the AI editor for playback, artist/album navigation, playlist/library actions and **Descargar para la caché de audio**. Downloads store complete encrypted audio; no MP3 export or bulk playlist download is provided.

Timed lyrics automatically follow the active line, with a fixed **Seguir letra** switch; plain lyrics have no fabricated synchronization. Preferences now expose **Very high (320 kb/s)** separately from High (160) and Normal (96). Low uses the engine's supported 96 kb/s minimum. Existing profiles retain their bitrate when the old quality names migrate; the choice applies to subsequent loads, subject to available track formats.

The bell opens releases from followed artists: recent albums/singles from the last 90 days, cover images, unread indicators and a local **Marcar como leídas** action. It scans each artist's latest 20 releases and displays up to 300 unique releases. Spotify announcements and its complete official notifications inbox are not available in this view. Responses are cached for six hours; a Spotify release quota response preserves the cooldown across restarts and displays a clear notice instead of waiting indefinitely. Cached releases remain usable when available.

For local visual review, `--preview-ui=news --dark` or `--light` uses synthetic releases and blocks remote actions.

Library save/remove commands are attached to the main shell, so they work from every route. A successful Spotify response updates the locally known saved state even when the full saved collection was never prefetched. Failed writes leave that state unchanged. Successful writes invalidate only their saved-collection response pages; manual collection refresh clears the local overrides.
