use std::{
    fmt::Display,
    thread::{self, JoinHandle},
};

use crossbeam_channel::{bounded, unbounded, Receiver, Sender};

pub enum ActorOp {
    Continue,
    Shutdown,
}

pub trait Actor: Sized {
    type Message: Send + 'static;
    type Error: Display;

    fn handle(&mut self, msg: Self::Message) -> Result<ActorOp, Self::Error>;

    fn process(mut self, recv: Receiver<Self::Message>) {
        for msg in recv {
            match self.handle(msg) {
                Ok(ActorOp::Continue) => {
                    continue;
                }
                Ok(ActorOp::Shutdown) => {
                    break;
                }
                Err(err) => {
                    log::error!("error: {}", err);
                    break;
                }
            }
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
