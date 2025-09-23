use crate::{data::Config, webapi::WebApi};
use psst_core::session::SessionService;

/// Token utilities to keep Session, WebApi, and Config in sync with minimal
/// duplication.
///
/// Conventions:
/// - Treat access tokens as short-lived, refresh tokens as durable.
/// - Passing `None` clears that token; passing `Some` sets/overwrites it.
pub struct TokenUtils;

impl TokenUtils {
    #[inline]
    fn set_bearer(session: &SessionService, token: Option<&str>) {
        match token {
            Some(t) => {
                let s = t.to_string();
                session.set_oauth_bearer(Some(s.clone()));
                WebApi::global().set_oauth_bearer(Some(s));
            }
            None => {
                session.set_oauth_bearer(None);
                WebApi::global().set_oauth_bearer(None);
            }
        }
    }

    #[inline]
    fn set_refresh(session: &SessionService, token: Option<&str>) {
        match token {
            Some(t) => {
                let s = t.to_string();
                session.set_oauth_refresh_token(Some(s.clone()));
                WebApi::global().set_oauth_refresh_token(Some(s));
            }
            None => {
                session.set_oauth_refresh_token(None);
                WebApi::global().set_oauth_refresh_token(None);
            }
        }
    }

    /// Apply tokens to runtime holders (Session and WebApi).
    /// - If `access_token` is Some, set it on both Session and WebApi. If None,
    ///   clear on both.
    /// - If `refresh_token` is Some, set it on both. If None, do not change
    ///   existing refresh tokens unless `clear_refresh_if_none` is true (then
    ///   it will be cleared).
    pub fn apply_runtime_tokens(
        session: &SessionService,
        access_token: Option<&str>,
        refresh_token: Option<&str>,
        clear_refresh_if_none: bool,
    ) {
        // Summarize action in a single log line
        let access_state = if access_token.is_some() {
            "set"
        } else {
            "cleared"
        };
        let refresh_state = if refresh_token.is_some() {
            "set"
        } else if clear_refresh_if_none {
            "cleared"
        } else {
            "unchanged"
        };
        log::info!("TokenUtils: runtime access={access_state}, refresh={refresh_state}");

        // Apply access token on both holders
        Self::set_bearer(session, access_token);

        // Apply or clear refresh token as requested
        if let Some(rt) = refresh_token {
            Self::set_refresh(session, Some(rt));
        } else if clear_refresh_if_none {
            Self::set_refresh(session, None);
        }
    }

    /// Persist tokens into Config (and optionally save).
    /// - If `access_token` is Some, store it. If None, clear it.
    /// - If `refresh_token` is Some, store it. If None, do not change the
    ///   persisted refresh token unless `clear_refresh_if_none` is true (then
    ///   it will be cleared).
    pub fn persist_tokens(
        config: &mut Config,
        access_token: Option<&str>,
        refresh_token: Option<&str>,
        clear_refresh_if_none: bool,
        save: bool,
    ) {
        log::info!("TokenUtils: persist save={save} clear_refresh_if_none={clear_refresh_if_none}");
        // Access token
        config.oauth_bearer = access_token.map(|s| s.to_string());

        // Refresh token
        if let Some(rt) = refresh_token {
            config.oauth_refresh_token = Some(rt.to_string());
        } else if clear_refresh_if_none {
            config.oauth_refresh_token = None;
        }

        if save {
            config.save();
        }
    }

    /// Apply and persist tokens atomically (runtime first, then config).
    /// Pass `clear_refresh_if_none = true` to force clearing refresh token when
    /// `refresh_token` is None. The `save` flag controls whether to write
    /// the updated config to disk.
    pub fn apply_and_persist(
        session: &SessionService,
        config: &mut Config,
        access_token: Option<String>,
        refresh_token: Option<String>,
        clear_refresh_if_none: bool,
        save: bool,
    ) {
        log::info!("TokenUtils: apply_and_persist save={save} clear_refresh_if_none={clear_refresh_if_none}");
        Self::apply_runtime_tokens(
            session,
            access_token.as_deref(),
            refresh_token.as_deref(),
            clear_refresh_if_none,
        );
        Self::persist_tokens(
            config,
            access_token.as_deref(),
            refresh_token.as_deref(),
            clear_refresh_if_none,
            save,
        );
    }

    /// Install tokens from config into runtime holders. Does not modify or save
    /// the config.
    /// - Access token is applied as-is (cleared if None).
    /// - Refresh token is applied as-is (cleared if None).
    pub fn install_from_config(session: &SessionService, config: &Config) {
        let access_state = if config.oauth_bearer.is_some() {
            "set"
        } else {
            "none"
        };
        let refresh_state = if config.oauth_refresh_token.is_some() {
            "set"
        } else {
            "none"
        };
        log::info!(
            "TokenUtils: install_from_config access={access_state}, refresh={refresh_state}"
        );
        Self::apply_runtime_tokens(
            session,
            config.oauth_bearer.as_deref(),
            config.oauth_refresh_token.as_deref(),
            true,
        );
    }

    /// Handle a refresh result (new access token and optional rotated refresh
    /// token).
    /// - Always installs the new access token.
    /// - If `maybe_rotated_refresh` is Some, replace refresh token with it;
    ///   otherwise retain the existing refresh token.
    /// - Persists the tokens and saves the config if `save` is true.
    pub fn apply_refresh_result(
        session: &SessionService,
        config: &mut Config,
        new_access: String,
        maybe_rotated_refresh: Option<String>,
        save: bool,
    ) {
        log::info!(
            "TokenUtils: apply_refresh_result(rotated_refresh={})",
            maybe_rotated_refresh.is_some()
        );
        let refresh = maybe_rotated_refresh.or_else(|| config.oauth_refresh_token.clone());
        Self::apply_and_persist(
            session,
            config,
            Some(new_access),
            refresh,
            /* clear_refresh_if_none = */ false,
            save,
        );
    }

    /// Clear both access and refresh tokens across Session, WebApi, and Config.
    pub fn clear_all(session: &SessionService, config: &mut Config, save: bool) {
        log::warn!("TokenUtils: clear_all(save={})", save);
        Self::apply_runtime_tokens(session, None, None, true);
        Self::persist_tokens(config, None, None, true, save);
    }
}
