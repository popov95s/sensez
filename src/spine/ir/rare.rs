//! A collection that is empty for most values, held behind a pointer until first written.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::ops::{Deref, DerefMut};

pub trait Sparse: Default + Sync + 'static {
    fn shared_empty() -> &'static Self;
    fn has_items(&self) -> bool;
    fn shrink(&mut self);
}

#[derive(Debug, Clone)]
pub struct Rare<T: Sparse>(Option<Box<T>>);

impl<T: Sparse> Rare<T> {
    /// Frees the allocation when nothing was stored, else drops spare capacity.
    pub fn compact(&mut self) {
        match self.0.as_deref_mut() {
            Some(items) if items.has_items() => items.shrink(),
            Some(_) => self.0 = None,
            None => {}
        }
    }
}

impl<T: Sparse> From<T> for Rare<T> {
    fn from(items: T) -> Self {
        Self(Some(Box::new(items)))
    }
}

impl<T: Sparse> Default for Rare<T> {
    fn default() -> Self {
        Self(None)
    }
}

impl<T: Sparse> Deref for Rare<T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.0.as_deref().unwrap_or_else(|| T::shared_empty())
    }
}

impl<T: Sparse> DerefMut for Rare<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.0.get_or_insert_with(Default::default)
    }
}

impl<T: Sparse + Serialize> Serialize for Rare<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        (**self).serialize(serializer)
    }
}

impl<'de, T: Sparse + Deserialize<'de>> Deserialize<'de> for Rare<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut rare = Self(Some(Box::new(T::deserialize(deserializer)?)));
        rare.compact();
        Ok(rare)
    }
}

macro_rules! sparse {
    ($($ty:ty),+ $(,)?) => {$(
        impl $crate::spine::ir::Sparse for $ty {
            fn shared_empty() -> &'static Self {
                static EMPTY: ::std::sync::LazyLock<$ty> = ::std::sync::LazyLock::new(Default::default);
                &EMPTY
            }

            fn has_items(&self) -> bool {
                !self.is_empty()
            }

            fn shrink(&mut self) {
                self.shrink_to_fit();
            }
        }
    )+};
}

pub(super) use sparse;

#[cfg(test)]
#[path = "rare_tests.rs"]
mod tests;
