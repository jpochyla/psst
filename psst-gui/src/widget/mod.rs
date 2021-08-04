mod dispatcher;
mod empty;
mod ex_click;
pub mod icons;
mod link;
mod maybe;
mod promise;
pub mod remote_image;
mod theme;
mod utils;

use druid::{widget::ControllerHost, Data, Env, EventCtx, MouseEvent, Widget};

pub use dispatcher::ViewDispatcher;
pub use empty::Empty;
pub use ex_click::ExClick;
pub use icons::Icon;
pub use link::Link;
pub use maybe::Maybe;
pub use promise::{Async, AsyncAction};
pub use remote_image::RemoteImage;
pub use theme::ThemeScope;
pub use utils::{Border, Clip, Logger};

pub trait MyWidgetExt<T: Data>: Widget<T> + Sized + 'static {
    fn link(self) -> Link<T> {
        Link::new(self)
    }

    fn on_ex_click(
        self,
        f: impl Fn(&mut EventCtx, &MouseEvent, &mut T, &Env) + 'static,
    ) -> ControllerHost<Self, ExClick<T>> {
        ControllerHost::new(self, ExClick::new(f))
    }
}

impl<T: Data, W: Widget<T> + 'static> MyWidgetExt<T> for W {}
