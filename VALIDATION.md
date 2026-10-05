# Local validation - Windows - 2026-10-04

Integration: existing xpotify repository, branch `splitify-integration`; original Splitify `main` remains the web app.

- Rust stable / Windows C++ Build Tools and SDK installed. Locked native build produces `dist/Xpotify.exe`.
- 26 native tests pass: OAuth callback state, timeout, listener readiness and occupied ports; separate Developer and native playback callback/client identities; DH entropy and server signature; permitted credential destinations; current library endpoint/URI encoding and playlist counts; OS theme default/overrides; AI classification constraints and playlist URL validation.
- Actual local Spotify account: profile and search HTTP 200, Premium confirmed. The obsolete library endpoint returned 403; the current `/me/library/contains` returned 200.
- Actual library mutation: saved one diagnostic track with `PUT /me/library?uris=...`, confirmed its saved state, then removed it with `DELETE` and confirmed the original state was restored. No lasting library changes.
- Actual native playback authorization uses the desktop client with `http://127.0.0.1:8898/login`. Existing local credentials repaired; Developer Web API token preserved.
- Actual playback pipeline: session authenticated, metadata and audio key received, client token and Login5 token received, audio URL resolved, encrypted audio downloaded. The native Player decoded audio, played through this machine's default output device, and advanced beyond two seconds at low volume before stopping.
- Actual queue UI validated using the local Spotify session: a loaded playlist showed 549 upcoming tracks; pressing Next changed the current song and reduced the displayed queue to 548. Playback paused after validation. Light/dark and empty queue fixture views checked at minimum window size. Green icon confirmed in the compiled Windows window; build also generates a local shortcut with the same icon.
- Queue preview tests compare displayed order against actual progression in sequential and shuffle modes, including manual additions and duplicates, repeat modes, starting manually queued tracks while idle, queue resets and preserving the current track when shuffle changes.
- Theme default is System; this Windows profile uses dark application mode. Explicit Light/Dark remain available.
- Polished login, playlist editor and player migrated into xpotify. Editor supports independent scrolling, persistent create controls, selection counters and category editing. Local preview fixtures cover light/dark and minimum window sizes.
- Gemini `gemini-3.5-flash-lite` model lookup and structured classification returned HTTP 200 in the preceding integration checks. No remote playlists were created during validation.
- Original Splitify web main (97d239c): 13 Vitest tests, typecheck, lint and production build passed during the web updates. The web repository is not modified by this native correction.

Reproduce native checks from the repository root:

```powershell
cargo test --locked -p psst-core --features cpal -p psst-gui
cargo clippy --locked -p psst-gui -- -D warnings
powershell -File scripts/Build-Native.ps1
cargo run --locked -p psst-core --features cpal --example diagnose_playback -- "$env:APPDATA/Psst/config.json" 5lfWrciYtohtIMVDVZd0Rf --play
```

The diagnostic prints stages, never credentials. `--play` plays briefly at low volume; `--authorize` opens the native Spotify authorization and updates only local playback credentials/theme. Cargo Audit reports no primary vulnerabilities but retains unsound/unmaintained framework warnings documented in SECURITY-REVIEW.md. No absolute claim of absence of backdoors is made.

- Tooltip update: locked Windows build and Clippy with warnings denied pass. Native dark fixture hover confirmed queue and current/next playback mode hints above the player; moving away removes the hint. Updated local executable and shortcut.

- Lyrics/video update (2026-10-05): 26 native tests pass, plus the explicitly invoked live LRCLIB integration test. Locked build and Clippy with warnings denied pass. Tests cover LRC metadata/multiple timestamps, malformed/overflowing timestamps, rejecting stale song responses and video query injection.
- Actual native session: played See You Again at low volume, opened real LRCLIB lyrics, clicked its first line and verified seek to zero, then pressed Next and verified EARFQUAKE lyrics replaced them automatically. Clicking the video icon opened the correct artist/title YouTube search in Microsoft Edge and paused native audio. No video is embedded or automatically selected. Playback left paused and volume restored.
- Native synthetic lyrics fixtures reviewed in dark and light themes, including 900 x 620. Active line highlighting and tooltips for video and settings render fully above bottom controls. Lyrics are fetched only on demand while the lyrics view is open.

- Library/Connect/startup update (2026-10-05): 29 native tests pass; the live lyrics test remains intentionally excluded from ordinary tests. Locked build and Clippy with warnings denied pass. New tests check paused snapshot round trips, playlist/track/position preservation in remote play requests and immediate quota handling without credentials or network requests.
- Actual restart restored EARFQUAKE from Us at 26 seconds, paused, and loaded the source playlist. Resuming used the saved position; the player log confirmed playback and subsequent pause, with the saved position at 26.123 seconds. The restored queue showed 3,673 upcoming tracks from the current 3,676-track playlist. Playback was left paused and volume restored to 100%.
- Actual global search returned Tyler, The Creator and album results. The library showed real La/Us cover images. Dark/light shell and synthetic news fixtures reviewed at 900 x 620; global device/news tooltips rendered above the top controls.
- Actual Spotify device discovery returned the user's unrestricted iPhone. Selecting it connected while paused, and returning to this computer restored the local track. Spotify initially returned HTTP 500 when transferring an empty session to an already active device; the final code avoids that redundant transfer. Local engine events are ignored while a remote device is selected. Remote audio and transport changes were not exercised because the phone never exposed an active track through the playback API and later disappeared from discovery.
- The actual account follows 199 artists. Artist-release lookups returned HTTP 429 with Retry-After around 84,000 seconds; the final UI completed promptly with an explicit 24-hour quota notice. The cooldown is persisted, cached metadata remains usable, and further release requests are suppressed until the deadline. No live release feed could be confirmed under this quota; populated rows and unread indicators were reviewed with synthetic fixtures.
