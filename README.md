# Psst

Fast Spotify client with native GUI, without Electron, built in Rust. Very early in development, lacking in features, stability, and general user experience. It is being tested only on Mac so far, but aims for full Windows and Linux support. Contributions welcome!

**Note:** Spotify Premium account is required.

[![Build](https://github.com/jpochyla/psst/actions/workflows/build.yml/badge.svg)](https://github.com/jpochyla/psst/actions)

![Screenshot](./psst-gui/assets/screenshot.png)

##### Pre-built binaries

GitHub Actions automatically runs when new commits are pushed to `master`. To download prebuilt binaries for x86_64 macOS, Windows or Ubuntu, [go to the newest successfully built action](https://nightly.link/jpochyla/psst/workflows/build/master).

##### Linux

Our user-interface library, Druid, has two possible backends to choose from on Linux: GTK and pure X11, with Wayland backend in the works. The default linux backend is GTK. Before building on Linux, make sure the required dependencies are installed. 

Debian/Ubuntu:

```shell
sudo apt-get install libssl-dev libgtk-3-dev libcairo2-dev libasound2-dev
```

RHEL/Fedora:

```shell
sudo dnf install openssl-devel gtk3-devel cairo-devel alsa-lib-devel
```

##### Building

On all platforms, the **latest Rust stable** (at least 1.54.0) is needed.

Development build:
```shell
git submodule update --recursive --init
cargo build
```

Release build:
```shell
git submodule update --recursive --init
cargo build --release
# Use `cargo bundle --release` for building the installation bundle (i.e. macOS .app)
```

##### Running and configuration

```shell
cd psst-gui
cargo run
# Use `cargo run --release` for the release build.
```

##### Roadmap

- [x] Vorbis track playback
- [x] Browsing saved albums and tracks
- [x] Save / unsave albums and tracks
- [x] Browsing followed playlists
- [x] Search for artist, albums, and tracks
- [x] Podcast support
- [x] Media keys control
- [x] Open Spotify links through search bar
- [x] Audio volume control
- [x] Audio loudness normalization
- [x] Genre playlists and "For You" content
- [x] Dark theme
- [ ] Resilience to network errors (automatically retry timed-out requests)
- [ ] Managing playlists
    - Follow / unfollow
    - Add / remove track
    - Reorder tracks
    - Rename playlist
    - Playlist folders
- [ ] Playback queue
- [ ] React to audio output device events
    - Pause after disconnecting headphones
    - Transfer playback after connecting headphones
- [ ] Better caching
    - Cache as much as possibly of WebAPI responses
    - Visualize cache utilization
        - Total cache usage in the config dialog
        - Show time origin of cached data, allow to refresh
- [ ] Trivia on the artist page, Wikipedia links
- [ ] Downloading encrypted tracks
- [ ] Reporting played tracks to Spotify servers
- [ ] OS-specific application bundles
- UI
    - [ ] Rethink current design, consider a two-pane layout
        - Left pane for browsing
        - Right pane for current playback
    - [ ] Detect light/dark OS theme
    - [ ] Robust error states, ideally with retry button
    - [ ] Correct playback highlight
        - Highlight now-playing track only in the correct album / playlist
        - Keep highlighted track in viewport
    - [ ] Paging or virtualized lists for albums and tracks
    - [ ] Grid for albums and artists
    - [ ] Robust active/inactive menu visualization
    - [ ] Save last route, volume, playback state

##### Development

Contributions are very welcome! Project structure:

- `/psst-core` - Core library, takes care of Spotify TCP session, audio file retrieval, decoding, audio output, playback queue, etc.
- `/psst-gui` - GUI application built with [Druid](https://github.com/linebender/druid)
- `/psst-cli` - Example CLI that plays a track.  Credentials need to be configured in the code.
- `/psst-protocol` - Internal Protobuf definitions used for Spotify communication.

##### Privacy Policy

Psst connects only to the official Spotify servers, and does not call home. Cache of various things is stored locally, and can be deleted at any time. User credentials are not stored at all (re-usable authentication token from Spotify is used instead).

##### Thanks

This project would not exist without:

- Big thank you to [`librespot`](https://github.com/librespot-org/librespot), the Open Source Spotify client library for Rust.  Most of `psst-core` is directly inspired by the ideas and code of `librespot`, although with a few differences:
    - Spotify Connect (remote control) is not supported yet.
    - Psst is completely synchronous, without `tokio` or other `async` runtime, although it will probably change in the future.
    - Psst is using HTTPS-based CDN audio file retrieval, similar to the official Web client or [`librespot-java`](https://github.com/librespot-org/librespot-java), instead of the channel-based approach in `librespot`.
- [`druid`](https://github.com/linebender/druid) native GUI library for Rust.
- [`ncspot`](https://github.com/hrkfdn/ncspot) cross-platform ncurses Spotify client written in Rust, using `librespot`.
- ...and of course other libraries and projects.
