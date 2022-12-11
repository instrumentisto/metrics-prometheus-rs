use std::sync::Arc;

use once_cell::sync::OnceCell;

use crate::failure::strategy::PanicInDebugNoOpInRelease;

#[derive(Clone, Debug)]
pub struct Recorder<FailureStrategy = PanicInDebugNoOpInRelease> {
    usual: super::Recorder<FailureStrategy>,
    frozen: Arc<OnceCell<super::Frozen<FailureStrategy>>>,
}

impl<S> Recorder<S> {
    #[must_use]
    pub(super) fn wrap(usual: super::Recorder<S>) -> Self {
        Self { usual, frozen: Arc::default() }
    }

    pub fn freeze(&self)
    where
        S: Clone,
    {
        let _ = self.frozen.get_or_init(|| super::Frozen {
            storage: (&self.usual.storage).into(),
            failure_strategy: self.usual.failure_strategy.clone(),
        });
    }
}

impl<S> metrics::Recorder for Recorder<S>
where
    super::Recorder<S>: metrics::Recorder,
    super::Frozen<S>: metrics::Recorder,
{
    fn describe_counter(
        &self,
        name: metrics::KeyName,
        unit: Option<metrics::Unit>,
        description: metrics::SharedString,
    ) {
        if let Some(frozen) = self.frozen.get() {
            frozen.describe_counter(name, unit, description);
        } else {
            self.usual.describe_counter(name, unit, description);
        }
    }

    fn describe_gauge(
        &self,
        name: metrics::KeyName,
        unit: Option<metrics::Unit>,
        description: metrics::SharedString,
    ) {
        if let Some(frozen) = self.frozen.get() {
            frozen.describe_gauge(name, unit, description);
        } else {
            self.usual.describe_gauge(name, unit, description);
        }
    }

    fn describe_histogram(
        &self,
        name: metrics::KeyName,
        unit: Option<metrics::Unit>,
        description: metrics::SharedString,
    ) {
        if let Some(frozen) = self.frozen.get() {
            frozen.describe_histogram(name, unit, description);
        } else {
            self.usual.describe_histogram(name, unit, description);
        }
    }

    fn register_counter(&self, key: &metrics::Key) -> metrics::Counter {
        self.frozen.get().map_or_else(
            || self.usual.register_counter(key),
            |frozen| frozen.register_counter(key),
        )
    }

    fn register_gauge(&self, key: &metrics::Key) -> metrics::Gauge {
        self.frozen.get().map_or_else(
            || self.usual.register_gauge(key),
            |frozen| frozen.register_gauge(key),
        )
    }

    fn register_histogram(&self, key: &metrics::Key) -> metrics::Histogram {
        self.frozen.get().map_or_else(
            || self.usual.register_histogram(key),
            |frozen| frozen.register_histogram(key),
        )
    }
}
