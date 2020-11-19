use crate::error::Error;
use druid::Data;

#[derive(Eq, PartialEq, Debug)]
pub enum PromiseState {
    Empty,
    Deferred,
    Resolved,
    Rejected,
}

#[derive(Clone, Debug, Data)]
pub enum Promise<T: Data, D: Data = (), E: Data = Error> {
    Empty,
    Deferred(D),
    Resolved(T),
    Rejected(E),
}

impl<T: Data, D: Data, E: Data> Promise<T, D, E> {
    pub fn state(&self) -> PromiseState {
        match self {
            Self::Empty => PromiseState::Empty,
            Self::Deferred(_) => PromiseState::Deferred,
            Self::Resolved(_) => PromiseState::Resolved,
            Self::Rejected(_) => PromiseState::Rejected,
        }
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    pub fn is_rejected(&self) -> bool {
        matches!(self, Self::Rejected(_))
    }

    pub fn is_deferred(&self, def: &D) -> bool
    where
        D: PartialEq,
    {
        matches!(self, Self::Deferred(d) if d == def)
    }

    pub fn defer(&mut self, def: D) {
        *self = Self::Deferred(def);
    }

    pub fn resolve(&mut self, val: T) {
        *self = Self::Resolved(val);
    }

    pub fn reject(&mut self, err: E) {
        *self = Self::Rejected(err);
    }

    pub fn resolve_or_reject(&mut self, res: Result<T, E>) {
        *self = match res {
            Ok(ok) => Self::Resolved(ok),
            Err(err) => Self::Rejected(err),
        };
    }
}

impl<D: Data + Default, T: Data, E: Data> Promise<T, D, E> {
    pub fn defer_default(&mut self) {
        *self = Self::Deferred(D::default())
    }
}

impl<T: Data, D: Data, E: Data> Default for Promise<T, D, E> {
    fn default() -> Self {
        Self::Empty
    }
}
