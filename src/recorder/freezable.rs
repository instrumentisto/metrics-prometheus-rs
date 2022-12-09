use std::sync::Arc;

use once_cell::sync::OnceCell;

use crate::{
    failure::{self, strategy::PanicInDebugNoOpInRelease},
    storage,
};

#[derive(Clone)]
pub struct Recorder<FailureStrategy = PanicInDebugNoOpInRelease> {
    usual: super::Recorder<FailureStrategy>,
    frozen: Arc<OnceCell<storage::Immutable>>,
}

impl<S> Recorder<S> {
    #[must_use]
    pub(super) fn wrap(usual: super::Recorder<S>) -> Self {
        Self { usual, frozen: Arc::default() }
    }
}

impl<S> metrics::Recorder for Recorder<S>
where
    S: failure::Strategy,
{
    fn describe_counter(
        &self,
        name: metrics::KeyName,
        unit: Option<metrics::Unit>,
        description: metrics::SharedString,
    ) {
        if self.frozen.get().is_none() {
            self.usual.describe_counter(name, unit, description)
        }
    }

    fn describe_gauge(
        &self,
        name: metrics::KeyName,
        unit: Option<metrics::Unit>,
        description: metrics::SharedString,
    ) {
        if self.frozen.get().is_none() {
            self.usual.describe_gauge(name, unit, description)
        }
    }

    fn describe_histogram(
        &self,
        name: metrics::KeyName,
        unit: Option<metrics::Unit>,
        description: metrics::SharedString,
    ) {
        if self.frozen.get().is_none() {
            self.usual.describe_histogram(name, unit, description)
        }
    }

    fn register_counter(&self, key: &metrics::Key) -> metrics::Counter {
        if let Some(frozen) = self.frozen.get() {
            frozen
                .get_metric::<prometheus::IntCounter>(key)
                .and_then(|res| {
                    res.map_err(|e| {
                        match self.usual.failure_strategy.decide(&*e) {
                            failure::Action::NoOp => (),
                            failure::Action::Panic => panic!(
                                "failed to register `prometheus::IntCounter` \
                                 metric: {}",
                                *e,
                            ),
                        }
                    })
                    .ok()
                })
                .map_or_else(metrics::Counter::noop, |m| {
                    // TODO: Eliminate this `Arc` allocation via `metrics` PR.
                    metrics::Counter::from_arc(Arc::new(m))
                })
        } else {
            self.usual.register_counter(key)
        }
    }

    fn register_gauge(&self, key: &metrics::Key) -> metrics::Gauge {
        if let Some(frozen) = self.frozen.get() {
            frozen
                .get_metric::<prometheus::Gauge>(key)
                .and_then(|res| {
                    res.map_err(|e| {
                        match self.usual.failure_strategy.decide(&*e) {
                            failure::Action::NoOp => (),
                            failure::Action::Panic => panic!(
                                "failed to register `prometheus::Gauge` \
                                 metric: {}",
                                *e,
                            ),
                        }
                    })
                    .ok()
                })
                .map_or_else(metrics::Gauge::noop, |m| {
                    // TODO: Eliminate this `Arc` allocation via `metrics` PR.
                    metrics::Gauge::from_arc(Arc::new(m))
                })
        } else {
            self.usual.register_gauge(key)
        }
    }

    fn register_histogram(&self, key: &metrics::Key) -> metrics::Histogram {
        if let Some(frozen) = self.frozen.get() {
            frozen
                .get_metric::<prometheus::Histogram>(key)
                .and_then(|res| {
                    res.map_err(|e| {
                        match self.usual.failure_strategy.decide(&*e) {
                            failure::Action::NoOp => (),
                            failure::Action::Panic => panic!(
                                "failed to register `prometheus::Histogram` \
                                 metric: {}",
                                *e,
                            ),
                        }
                    })
                    .ok()
                })
                .map_or_else(metrics::Histogram::noop, |m| {
                    // TODO: Eliminate this `Arc` allocation via `metrics` PR.
                    metrics::Histogram::from_arc(Arc::new(m))
                })
        } else {
            self.usual.register_histogram(key)
        }
    }
}
