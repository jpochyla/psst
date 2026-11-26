Based on librespot-playback 0.8.0 (MIT).

Adds Player::set_bitrate, processed on the player thread. Quality changes invalidate preloaded audio but preserve the active decoder, playback position, session, and queue. Subsequent loads use the new bitrate.
