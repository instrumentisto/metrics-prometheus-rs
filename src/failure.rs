//! Definitions for dealing with a [`prometheus::Error`].

#[doc(inline)]
pub use self::strategy::Strategy;

/// Possible actions on an encountered [`prometheus::Error`] inside
/// [`metrics::Recorder`] methods.
#[derive(Clone, Copy, Debug)]
pub enum Action {
    /// Return a no-op metric implementation (see [`metrics::Counter::noop()`]
    /// for example).
    NoOp,

    /// Panic with the encountered [`prometheus::Error`].
    Panic,
}

/// Strategies for dealing with a [`prometheus::Error`].
pub mod strategy {
    use super::Action;

    /// Strategy deciding which [`Action`] should be performed on an encountered
    /// [`prometheus::Error`] inside [`metrics::Recorder`] methods.
    pub trait Strategy {
        /// Inspects the encountered [`prometheus::Error`] and returns the
        /// [`Action`] to be performed.
        fn decide(&self, res: &prometheus::Error) -> Action;
    }

    /// [`Strategy`] returning always [`Action::NoOp`].
    #[derive(Clone, Copy, Debug, Default)]
    pub struct NoOp;

    impl Strategy for NoOp {
        fn decide(&self, _: &prometheus::Error) -> Action {
            Action::NoOp
        }
    }

    /// [`Strategy`] returning always [`Action::Panic`].
    #[derive(Clone, Copy, Debug, Default)]
    pub struct Panic;

    impl Strategy for Panic {
        fn decide(&self, _: &prometheus::Error) -> Action {
            Action::Panic
        }
    }

    /// [`Strategy`] returning an [`Action::Panic`] in debug mode, and
    /// [`Action::NoOp`] in release mode.
    #[derive(Clone, Copy, Debug, Default)]
    pub struct PanicInDebugNoOpInRelease;

    impl Strategy for PanicInDebugNoOpInRelease {
        fn decide(&self, _: &prometheus::Error) -> Action {
            #[cfg(debug_assertions)]
            {
                Action::Panic
            }
            #[cfg(not(debug_assertions))]
            {
                Action::NoOp
            }
        }
    }
}
