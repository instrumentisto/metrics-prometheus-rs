pub use self::strategy::Strategy;

#[derive(Clone, Copy, Debug)]
pub enum Action {
    NoOp,
    Panic,
}

pub mod strategy {
    use super::Action;

    pub trait Strategy {
        fn decide(&self, res: &prometheus::Error) -> Action;
    }

    #[derive(Clone, Copy, Debug, Default)]
    pub struct NoOp;

    impl Strategy for NoOp {
        fn decide(&self, _: &prometheus::Error) -> Action {
            Action::NoOp
        }
    }

    #[derive(Clone, Copy, Debug, Default)]
    pub struct Panic;

    impl Strategy for Panic {
        fn decide(&self, _: &prometheus::Error) -> Action {
            Action::Panic
        }
    }

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
                Action::Panic
            }
        }
    }
}
