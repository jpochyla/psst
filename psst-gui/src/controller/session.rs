use std::thread::{self, JoinHandle};

use druid::{
    widget::{prelude::*, Controller},
    ExtEventSink, Target,
};
use psst_core::session::{SessionConfig, SessionHandle};

use crate::{cmd, data::AppState};

pub struct SessionController {
    thread: Option<JoinHandle<()>>,
}

impl SessionController {
    pub fn new() -> Self {
        Self { thread: None }
    }

    fn start_connection_thread(
        &mut self,
        handle: SessionHandle,
        config: SessionConfig,
        event_sink: ExtEventSink,
    ) {
        self.thread.replace(thread::spawn(move || {
            Self::connect_and_service(handle, config, event_sink);
        }));
    }

    fn connect_and_service(handle: SessionHandle, config: SessionConfig, event_sink: ExtEventSink) {
        let try_connect_and_service = || {
            let session = handle.connect(config)?;
            event_sink
                .submit_command(cmd::SESSION_CONNECTED, (), Target::Auto)
                .unwrap();
            session.service()
        };
        match try_connect_and_service() {
            Ok(_) => {
                log::info!("connection shutdown");
            }
            Err(err) => {
                log::error!("connection error: {:?}", err);
            }
        };
        event_sink
            .submit_command(cmd::SESSION_DISCONNECTED, (), Target::Auto)
            .unwrap();
    }
}

impl<W> Controller<AppState, W> for SessionController
where
    W: Widget<AppState>,
{
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut AppState,
        env: &Env,
    ) {
        match event {
            Event::Command(cmd) if cmd.is(cmd::SESSION_CONNECT) => {
                if data.config.has_credentials() {
                    self.start_connection_thread(
                        data.session.clone(),
                        data.config.session(),
                        ctx.get_external_handle(),
                    );
                }
                ctx.set_handled();
            }
            _ => {
                child.event(ctx, event, data, env);
            }
        }
    }

    fn lifecycle(
        &mut self,
        child: &mut W,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &AppState,
        env: &Env,
    ) {
        if let LifeCycle::WidgetAdded = event {
            if data.config.has_credentials() {
                self.start_connection_thread(
                    data.session.clone(),
                    data.config.session(),
                    ctx.get_external_handle(),
                );
            }
        }
        child.lifecycle(ctx, event, data, env)
    }
}
