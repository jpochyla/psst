# Local validation - Windows - 2026-10-05

## Windows 0.2.0 publication checks

- 52 unit tests pass (17 core, 35 GUI), with one live LRCLIB test intentionally excluded; Clippy passes with warnings denied. New coverage verifies library fallback after invalidation without cross-account leakage, playlist snapshot reuse with duplicates/original positions, rejection of stale page data after a changed revision, suppression of repeated transient reads, URL validation and route refresh isolation.
- Windows registered all three native taskbar thumbnail buttons. Using the actual `WM_COMMAND` / `THBN_CLICKED` notifications while minimized resumed and paused native audio at low volume; Next selected The Blacker The Berry and the player was paused at approximately five seconds. Previous reset the paused song to its beginning. These checks exercise the notification path; they do not claim a screenshot of the taskbar flyout.
- Local selector fixtures display own and collaborative playlists, exclude read-only destinations and keep the text input/actions visible at the minimum dialog size. Available cover images use the shared image pipeline, with a playlist icon when absent. Quota failures are explained instead of showing an empty submenu. No remote playlist write was attempted while the actual Spotify quota remained blocked.
- The user's saved Bad News playback, queue and original volume are restored after playback validation. The original Splitify web main remains untouched. Release notes retain the outstanding native Connect/listen-reporting limits rather than claiming the roadmap fully complete.
- Distribution uses an optimized Windows x64 release build with a fixed package allowlist, both licenses and separate EXE/ZIP SHA-256 sidecars. Development preview entry points are excluded from this build; local credentials, Gemini keys and logs are not packaged.

## Quota, queue, lyrics and quality update

- 45 unit tests pass (17 core, 28 GUI); the explicitly live LRCLIB test remains excluded. Clippy for the core library and GUI passes with warnings denied, and the locked Windows build succeeds.
- Actual native queue: clicking the second upcoming row selected Black Skinhead, played for four seconds and reduced the queue from 547 to 545 entries while preserving the remaining order. Playback was paused afterwards. Original saved James Blake playback at 160.298 seconds, queue and 100% volume were restored before launching the final executable.
- Actual encrypted download: `diagnose_playback --download` authenticated the existing session and completed a fresh full-track download at requested 320 kb/s into a separate temporary cache without opening audio output. Downloads are encrypted cache entries, not exported MP3 files.
- Visual fixtures: synchronized lyrics keep the active line visible with fixed follow/refresh controls in dark and light themes, including 900 x 620. A quota fixture visibly counted from nine to six seconds while keeping Retry disabled; the button becomes available at expiry.
- Actual Spotify HTTP 429 deadline persists across restart and token refresh. The final application reopened paused without clearing the deadline; cached queue/artwork and local playback remained available. Cold Web API views display the quota error rather than pretending an empty playlist was loaded. This cannot remove Spotify's existing server cooldown.
- Very High maps to 320 kb/s and is the default. Legacy quality names migrate without changing their old bitrates. This decoder's minimum supported selection is 96 kb/s; Spotify's 24 kb/s Low is not claimed as implemented.
- Native Spotify Connect reception remains unsupported by the inherited player. The device dialog explains the limitation and offers opening the installed official desktop client as a separate receiver. Phone-to-PC control through that client was not live-tested during this update.
- Right-click track menus are shared by queue rows, playback views, synchronized lyric heading and Splitify assignments, alongside existing library/search lists. Actions include playback, encrypted cache download, playlist/library operations and artist navigation.

Reproduce the ordinary checks with `cargo test --locked -p psst-core --lib -p psst-gui --bin psst-gui`. The final Windows archive uses the explicit packaging allowlist and SHA-256 verification; local credentials and diagnostic logs are excluded.

## Roadmap update — 2026-10-05

- 41 native unit tests pass (16 core + 25 GUI); one explicitly live lyrics test is excluded from the ordinary run. Coverage includes connection retries without repeating ambiguous writes, cache isolation/atomic metadata invalidation, playlist reorder indices, persistent folders, original playback positions across pages, source-specific highlighting, exact shuffle/manual queue restoration and rejection of stale/recycled search results.
- Clippy for the core library and native GUI passes with `-D warnings`. Native Windows build and green-icon shortcut are regenerated by the build script.
- Actual account: the full 3,676-track playlist loads in 37 visible pages after removing the inherited 550-track truncation. Cache origin date is visible. The existing session's actual queue is preserved; loading a larger collection does not replace it silently.
- Actual native playback resumed the saved track at 91 seconds, advanced to 94.188 seconds during a low-volume check, and was paused again. Original volume restored to 100%. Completed audio is stored encrypted in cache.
- Local fixture: 750 tracks select page 7 for current track 612; resizing to 900 × 620 keeps the highlighted current track visible. Changing pages renders the new track identities rather than reusing captured metadata from old rows. Search checks cover a result beyond the initial page.
- Windows ZIP packaging uses an explicit file allowlist, includes both licenses, and was checked for ZIP integrity. No local credentials, profile, Gemini key or logs enter the archive. Shell packaging script syntax checked with Bash.
- The extracted Windows package starts using the existing local profile. Closing and restarting preserves the exact engine queue hash, playback position and volume; playback stays paused. The final archive's executable matches the installed build and its SHA-256 sidecar matches the archive.
- Remote playlist writes, physical headphone unplug/reconnect, macOS/Linux builds and native listen reporting are not claimed as live-validated. See `ROADMAP.md` for feature status and limits.

## Earlier integration checks

- Spotify-style queue update: compact right sidebar with current track in green, 48px artwork, artist subtitles, source heading, independent scroll and close control. Visual review passed in dark at 1120 x 800 and light at 900 x 620. Actual account artwork and 549 upcoming tracks were displayed while the existing playlist remained open; closing and reopening from the footer preserved the route and paused playback. Rows load visually in blocks of 100 without modifying the engine queue. GUI tests: 25 pass, one live test excluded; Clippy with warnings denied and the native Windows build pass.

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
