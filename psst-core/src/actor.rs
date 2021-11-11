use std::{
    fmt::Display,
    thread::{self, JoinHandle},
    time::Duration,
};

use crossbeam_channel::{
    bounded, unbounded, Receiver, RecvTimeoutError, SendError, Sender, TrySendError,
};

pub enum Act<T: Actor> {
    Continue,
    WaitOr {
        timeout: Duration,
        timeout_msg: T::Message,
    },
    Shutdown,
}

pub trait Actor: Sized {
    type Message: Send + 'static;
    type Error: Display;

    fn handle(&mut self, msg: Self::Message) -> Result<Act<Self>, Self::Error>;

    fn process(mut self, recv: Receiver<Self::Message>) {
        let mut act = Act::Continue;
        loop {
            let msg = match act {
                Act::Continue => match recv.recv() {
                    Ok(msg) => msg,
                    Err(_) => {
                        break;
                    }
                },
                Act::WaitOr {
                    timeout,
                    timeout_msg,
                } => match recv.recv_timeout(timeout) {
                    Ok(msg) => msg,
                    Err(RecvTimeoutError::Timeout) => timeout_msg,
                    Err(RecvTimeoutError::Disconnected) => {
                        break;
                    }
                },
                Act::Shutdown => {
                    break;
                }
            };
            act = match self.handle(msg) {
                Ok(act) => act,
                Err(err) => {
                    log::error!("error: {}", err);
                    break;
                }
            };
        }
    }

    fn spawn<F>(cap: Capacity, factory: F) -> ActorHandle<Self::Message>
    where
        F: FnOnce(Sender<Self::Message>) -> Self + Send + 'static,
    {
        let (send, recv) = cap.to_channel();
        ActorHandle {
            sender: send.clone(),
            thread: thread::spawn(move || {
                factory(send).process(recv);
            }),
        }
    }

    fn spawn_default<F>(factory: F) -> ActorHandle<Self::Message>
    where
        F: FnOnce(Sender<Self::Message>) -> Self + Send + 'static,
    {
        Self::spawn(Capacity::Bounded(128), factory)
    }
}

pub struct ActorHandle<M> {
    thread: JoinHandle<()>,
    sender: Sender<M>,
}

impl<M> ActorHandle<M> {
    pub fn sender(&self) -> Sender<M> {
        self.sender.clone()
    }

    pub fn join(self) {
        let _ = self.thread.join();
    }

    pub fn send(&self, msg: M) -> Result<(), SendError<M>> {
        self.sender.send(msg)
    }

    pub fn try_send(&self, msg: M) -> Result<(), TrySendError<M>> {
        self.sender.try_send(msg)
    }
}

pub enum Capacity {
    Sync,
    Bounded(usize),
    Unbounded,
}

impl Capacity {
    pub fn to_channel<T>(&self) -> (Sender<T>, Receiver<T>) {
        match self {
            Capacity::Sync => bounded(0),
            Capacity::Bounded(cap) => bounded(*cap),
            Capacity::Unbounded => unbounded(),
        }
    }
}
