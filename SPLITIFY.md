# Xpotify with Splitify

The native integration lives in the existing xpotify repository, branch `splitify-integration`. The original Splitify repository stays on `main` with its Next.js web application. No nested Rust copy or separate native repository is required.

## Run on this machine

Double-click `Xpotify.lnk` for the green app shortcut, or `Start-Xpotify.cmd`. The launcher starts `dist/Xpotify.exe` from this repository, loading `.env.local` from the project directory. To rebuild: `powershell -File scripts/Build-Native.ps1`. Rust stable and Visual Studio C++ Build Tools / Windows SDK are required for compilation. The running application requires neither Node nor a web server.

The default theme is **System**. On Windows it follows AppsUseLightTheme and updates when that setting changes. Preferences can override it with Light or Dark. The system option falls back to light on other platforms.

## Playback queue and app icon

Open **Cola** in the sidebar or the queue icon beside the playback controls. The view shows the current song and upcoming tracks with artist and duration. It follows the actual native player's shuffle order, manually added tracks, duplicates, track changes and repeat mode. When stopped, manually queued tracks can be started with **Reproducir cola**. Stop or starting a new source resets the queue.

The Windows executable has a green waveform icon, generated from the matching vector design in `psst-gui/assets/logo.svg`. To regenerate its PNG sizes, run `powershell -File scripts/Generate-AppIcon.ps1`, then rebuild. The build always regenerates the ICO resource in Cargo's output directory, avoiding a stale cached icon in the source tree.

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
