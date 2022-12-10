//! [`metrics::registry::Storage`] implementations.
//!
//! [`metrics::registry::Storage`]: metrics_util::registry::Storage

pub mod immutable;
pub mod mutable;

use sealed::sealed;

#[doc(inline)]
pub use self::{immutable::Storage as Immutable, mutable::Storage as Mutable};

/// Name identifying a [`metric::Bundle`] stored in a storage.
///
/// [`metric::Bundle`]: crate::metric::Bundle
pub type KeyName = String;

/// Retrieving a `Collection` of [`metric::Bundle`]s from a storage.
///
/// [`metric::Bundle`]: crate::metric::Bundle
#[sealed]
pub trait Get<Collection> {
    /// Returns a `Collection` of [`metric::Bundle`]s stored in this storage.
    ///
    /// [`metric::Bundle`]: crate::metric::Bundle
    #[must_use]
    fn collection(&self) -> &Collection;
}
