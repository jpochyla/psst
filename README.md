# Psst

Fast and multi-platform Spotify client with native GUI, without Electron, built in Rust. Very early in development, lacking in features, stability, and general user experience. So far, tested only on Mac.

##### Building

Development build:
```
$ cargo build
```

Release build:
```
$ cargo build --release
```

##### Running and configuration

First we need to setup the client with device username and password.  You can configure these in the [Spotify Settings](https://www.spotify.com/cz/account/set-device-password). 
```
$ cat > psst-gui/config.json
{ "username": "...",
  "password": "..." } 
```

Now we're good to go:
```
$ cd psst-gui
$ cargo run
# Use cargo run --release for the release build.
```

##### Development

Contributions are very welcome! Project structure:

- `/psst-bin` - Example CLI that plays a track.  Credentials need to be configured in the code.
- `/psst-core` - Core library, takes care of Spotify TCP session, audio file retrieval, decoding, audio output, playback queue, etc.
- `/psst-gui` - GUI application built with [Druid](https://github.com/linebender/druid)
- `/psst-protocol` - Internal Protobuf definitions used for Spotify communication.

##### Thanks

This project would not exist without:

- Big thank you to [`librespot`](https://github.com/librespot-org/librespot), the Open Source Spotify client library for Rust.  Most of `psst-core` is directly inspired by the ideas and code of `librespot`, although with a few differences:
    - Spotify Connect (remote control) is not supported yet.
    - We're completely synchronous, without `tokio` or other `async` runtime.  I just don't understand the `async` jungle enough to port `librespot` from pre-async/await `tokio-0.1` code to anything both stable and modern.
    - We're using HTTPS-based CDN audio file retrieval, similar to the official Web client or [`librespot-java`](https://github.com/librespot-org/librespot-java), instead of the older, channel-based approach in `librespot`.
- [`druid`](https://github.com/linebender/druid) native GUI library for Rust.
- [`aspotify`](https://github.com/KaiJewson/aspotify) asynchronous client library for the Spotify Web API.
- [`ncspot`](https://github.com/hrkfdn/ncspot) cross-platform ncurses Spotify client written in Rust, using `librespot`.
- [`soundio`](http://libsound.io) by Andrew Kelley.