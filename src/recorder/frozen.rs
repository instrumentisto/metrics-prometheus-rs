use std::sync::Arc;

use crate::{
    failure::{self, strategy::PanicInDebugNoOpInRelease},
    storage,
};

#[derive(Clone)]
pub struct Recorder<FailureStrategy = PanicInDebugNoOpInRelease> {
    pub(super) storage: Arc<storage::Immutable>,

    /// [`failure::Strategy`] to apply when a [`prometheus::Error`] is
    /// encountered inside [`metrics::Recorder`] methods.
    pub(super) failure_strategy: FailureStrategy,
}

impl<S> metrics::Recorder for Recorder<S>
where
    S: failure::Strategy,
{
    fn describe_counter(
        &self,
        _: metrics::KeyName,
        _: Option<metrics::Unit>,
        _: metrics::SharedString,
    ) {
        // No-op
    }

    fn describe_gauge(
        &self,
        _: metrics::KeyName,
        _: Option<metrics::Unit>,
        _: metrics::SharedString,
    ) {
        // No-op
    }

    fn describe_histogram(
        &self,
        _: metrics::KeyName,
        _: Option<metrics::Unit>,
        _: metrics::SharedString,
    ) {
        // No-op
    }

    fn register_counter(&self, key: &metrics::Key) -> metrics::Counter {
        self.storage
            .get_metric::<prometheus::IntCounter>(key)
            .and_then(|res| {
                res.map_err(|e| match self.failure_strategy.decide(&*e) {
                    failure::Action::NoOp => (),
                    failure::Action::Panic => panic!(
                        "failed to register `prometheus::IntCounter` metric: \
                         {}",
                        *e,
                    ),
                })
                .ok()
            })
            .map_or_else(metrics::Counter::noop, |m| {
                // TODO: Eliminate this `Arc` allocation via `metrics` PR.
                metrics::Counter::from_arc(Arc::new(m))
            })
    }

    fn register_gauge(&self, key: &metrics::Key) -> metrics::Gauge {
        self.storage
            .get_metric::<prometheus::Gauge>(key)
            .and_then(|res| {
                res.map_err(|e| match self.failure_strategy.decide(&*e) {
                    failure::Action::NoOp => (),
                    failure::Action::Panic => panic!(
                        "failed to register `prometheus::Gauge` metric: {}",
                        *e,
                    ),
                })
                .ok()
            })
            .map_or_else(metrics::Gauge::noop, |m| {
                // TODO: Eliminate this `Arc` allocation via `metrics` PR.
                metrics::Gauge::from_arc(Arc::new(m))
            })
    }

    fn register_histogram(&self, key: &metrics::Key) -> metrics::Histogram {
        self.storage
            .get_metric::<prometheus::Histogram>(key)
            .and_then(|res| {
                res.map_err(|e| match self.failure_strategy.decide(&*e) {
                    failure::Action::NoOp => (),
                    failure::Action::Panic => panic!(
                        "failed to register `prometheus::Histogram` metric: {}",
                        *e,
                    ),
                })
                .ok()
            })
            .map_or_else(metrics::Histogram::noop, |m| {
                // TODO: Eliminate this `Arc` allocation via `metrics` PR.
                metrics::Histogram::from_arc(Arc::new(m))
            })
    }
}
