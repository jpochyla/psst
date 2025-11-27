use druid::Data;

use crate::error::Error;

#[derive(Clone, Debug, Data)]
pub enum Promise<T: Data, D: Data = (), E: Data = Error> {
    Empty,
    Deferred { def: D },
    Resolved { def: D, val: T },
    Rejected { def: D, err: E },
}

#[derive(Eq, PartialEq, Debug)]
pub enum PromiseState {
    Empty,
    Deferred,
    Resolved,
    Rejected,
}

impl<T: Data, D: Data, E: Data> Promise<T, D, E> {
    pub fn state(&self) -> PromiseState {
        match self {
            Self::Empty => PromiseState::Empty,
            Self::Deferred { .. } => PromiseState::Deferred,
            Self::Resolved { .. } => PromiseState::Resolved,
            Self::Rejected { .. } => PromiseState::Rejected,
        }
    }

    pub fn is_resolved(&self) -> bool {
        self.state() == PromiseState::Resolved
    }

    pub fn is_deferred(&self, d: &D) -> bool
    where
        D: PartialEq,
    {
        matches!(self, Self::Deferred { def } if def == d)
    }

    pub fn contains(&self, d: &D) -> bool
    where
        D: PartialEq,
    {
        matches!(self, Self::Resolved { def, .. } if def == d)
    }

    pub fn deferred(&self) -> Option<&D> {
        match self {
            Promise::Deferred { def }
            | Promise::Resolved { def, .. }
            | Promise::Rejected { def, .. } => Some(def),
            Promise::Empty => None,
        }
    }

    pub fn resolved(&self) -> Option<&T> {
        if let Promise::Resolved { val, .. } = self {
            Some(val)
        } else {
            None
        }
    }

    pub fn resolved_mut(&mut self) -> Option<&mut T> {
        if let Promise::Resolved { val, .. } = self {
            Some(val)
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        *self = Self::Empty;
    }

    pub fn defer(&mut self, def: D) {
        *self = Self::Deferred { def };
    }

    pub fn resolve(&mut self, def: D, val: T) {
        *self = Self::Resolved { def, val };
    }

    pub fn reject(&mut self, def: D, err: E) {
        *self = Self::Rejected { def, err };
    }

    pub fn resolve_or_reject(&mut self, def: D, res: Result<T, E>) {
        match res {
            Ok(val) => self.resolve(def, val),
            Err(err) => self.reject(def, err),
        }
    }

    pub fn update(&mut self, (def, res): (D, Result<T, E>))
    where
        D: PartialEq,
    {
        if self.is_deferred(&def) {
            self.resolve_or_reject(def, res);
        } else {
            // Ignore.
        }
    }
}

impl<D: Data + Default, T: Data, E: Data> Promise<T, D, E> {
    pub fn defer_default(&mut self) {
        *self = Self::Deferred { def: D::default() };
    }
}

impl<T: Data, D: Data, E: Data> Default for Promise<T, D, E> {
    fn default() -> Self {
        Self::Empty
    }
}
