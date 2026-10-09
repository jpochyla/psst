use librespot_protocol::player::ProvidedTrack;
use std::fmt::{Display, Formatter};

// providers used by spotify
const PROVIDER_CONTEXT: &str = "context";
const PROVIDER_QUEUE: &str = "queue";
const PROVIDER_AUTOPLAY: &str = "autoplay";

// custom providers, used to identify certain states that we can't handle preemptively, yet
/// it seems like spotify just knows that the track isn't available, currently we don't have an
/// option to do the same, so we stay with the old solution for now
const PROVIDER_UNAVAILABLE: &str = "unavailable";

#[derive(Debug, Clone)]
pub enum Provider {
    Context,
    Queue,
    Autoplay,
    Unavailable,
}

impl Display for Provider {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Provider::Context => PROVIDER_CONTEXT,
                Provider::Queue => PROVIDER_QUEUE,
                Provider::Autoplay => PROVIDER_AUTOPLAY,
                Provider::Unavailable => PROVIDER_UNAVAILABLE,
            }
        )
    }
}

pub trait IsProvider {
    fn is_autoplay(&self) -> bool;
    fn is_context(&self) -> bool;
    fn is_queue(&self) -> bool;
    fn is_unavailable(&self) -> bool;

    fn set_provider(&mut self, provider: Provider);
}

impl IsProvider for ProvidedTrack {
    fn is_autoplay(&self) -> bool {
        self.provider == PROVIDER_AUTOPLAY
    }

    fn is_context(&self) -> bool {
        self.provider == PROVIDER_CONTEXT
    }

    fn is_queue(&self) -> bool {
        self.provider == PROVIDER_QUEUE
    }

    fn is_unavailable(&self) -> bool {
        self.provider == PROVIDER_UNAVAILABLE
    }

    fn set_provider(&mut self, provider: Provider) {
        self.provider = provider.to_string()
    }
}
