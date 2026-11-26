# Xpotify queue patch

Source: the published crates.io `librespot-connect` 0.8.0 sources, under the
upstream MIT license included as `LICENSE`. Other librespot crates remain
registry dependencies pinned by the root lockfile.
The crate's VCS metadata identifies upstream commit
`d36f9f1907e8cc9d68a93f8ebc6b627b1bf7267d`, directory `connect`.

Changes are confined to `src/spirc.rs`:

- `Spirc::queue_state()` exposes a latest-value Tokio watch channel containing
  the active local player's actual current/next tracks, shuffle options and
  context. Progress-only updates do not produce queue notifications. This lets
  the GUI display engine shuffle order without polling Spotify's Web API or
  relying on delayed echoes of its own Connect publications.
- `Spirc::add_to_queue()` validates a Spotify URI and dispatches a local command
  through the existing Connect task and `ConnectState::add_to_queue`. Manual
  additions retain FIFO order ahead of context tracks, without reloading audio.
  The upstream next-track window holds 80 entries; a full manual queue is
  rejected rather than silently discarding a previously queued song.
- One upstream `matches!` expression uses parentheses to satisfy current Clippy.
- `Spirc::play_queued()` selects a queue occurrence by its unique ID and URI,
  rejects stale identities before changing state, advances within the existing
  context and loads audio once. It does not reload or reshuffle the playlist.
- An offline regression test checks shuffle, advancement, ignored progress and
  latest-value delivery.

No authentication, destinations, encryption, audio decoder or telemetry code
was replaced. This patch does not add listen reporting. Compare this directory
against the published 0.8.0 crate when upgrading; preserve these three APIs or
remove their GUI callers deliberately.
