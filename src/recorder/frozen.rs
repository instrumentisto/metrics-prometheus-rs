//! Fast and read-only [`metrics::Recorder`].

use std::sync::Arc;

use crate::{
    failure::{self, strategy::PanicInDebugNoOpInRelease},
    storage,
};

use super::Builder;

/// [`metrics::Recorder`] allowing to access already registered metrics in a
/// [`prometheus::Registry`], but not to register new ones, and is built on top
/// of a [`storage::Immutable`].
///
/// Though this [`FrozenRecorder`] is not capable of registering new metrics in
/// its [`prometheus::Registry`] on the fly, it still does allow changing the
/// [`help` description] of already registered ones. By default, the
/// [`prometheus::default_registry()`] is used.
///
/// The only way to register metrics in this [`FrozenRecorder`] is to specify
/// them via [`Builder::with_metric()`]/[`Builder::try_with_metric()`] APIs,
/// before the [`FrozenRecorder`] is built.
///
/// # Example
///
/// ```rust
/// let registry = metrics_prometheus::Recorder::builder()
///     .with_metric(prometheus::IntCounterVec::new(
///         prometheus::opts!("count", "help"),
///         &["whose", "kind"],
///     )?)
///     .with_metric(prometheus::Gauge::new("value", "help")?)
///     .build_frozen_and_install();
///
/// // `metrics` crate interfaces allow to change already registered metrics.
/// metrics::increment_counter!("count", "whose" => "mine", "kind" => "owned");
/// metrics::increment_counter!("count", "whose" => "mine", "kind" => "ref");
/// metrics::increment_counter!("count", "kind" => "owned", "whose" => "dummy");
/// metrics::increment_gauge!("value", 1.0);
///
/// let report = prometheus::TextEncoder::new()
///     .encode_to_string(&registry.gather())?;
/// assert_eq!(
///     report.trim(),
///     r#"
/// ## HELP count help
/// ## TYPE count counter
/// count{kind="owned",whose="dummy"} 1
/// count{kind="owned",whose="mine"} 1
/// count{kind="ref",whose="mine"} 1
/// ## HELP value help
/// ## TYPE value gauge
/// value 1
///     "#
///     .trim(),
/// );
///
/// // However, you cannot register new metrics. This is just no-op.
/// metrics::increment_gauge!("new", 2.0);
///
/// let report = prometheus::TextEncoder::new()
///     .encode_to_string(&registry.gather())?;
/// assert_eq!(
///     report.trim(),
///     r#"
/// ## HELP count help
/// ## TYPE count counter
/// count{kind="owned",whose="dummy"} 1
/// count{kind="owned",whose="mine"} 1
/// count{kind="ref",whose="mine"} 1
/// ## HELP value help
/// ## TYPE value gauge
/// value 1
///     "#
///     .trim(),
/// );
///
/// // Luckily, metrics still can be described anytime after being registered.
/// metrics::describe_counter!("count", "Example of counter.");
/// metrics::describe_gauge!("value", "Example of gauge.");
///
/// let report = prometheus::TextEncoder::new()
///     .encode_to_string(&prometheus::default_registry().gather())?;
/// assert_eq!(
///     report.trim(),
///     r#"
/// ## HELP count Example of counter.
/// ## TYPE count counter
/// count{kind="owned",whose="dummy"} 1
/// count{kind="owned",whose="mine"} 1
/// count{kind="ref",whose="mine"} 1
/// ## HELP value Example of gauge.
/// ## TYPE value gauge
/// value 1
///     "#
///     .trim(),
/// );
/// # Ok::<_, prometheus::Error>(())
/// ```
///
/// # Performance
///
/// This [`FrozenRecorder`] provides the smallest overhead of accessing an
/// already registered metric: just a regular [`HashMap`] lookup plus [`Arc`]
/// cloning.
///
/// # Errors
///
/// [`prometheus::Registry`] has far more stricter semantics than the ones
/// implied by a [`metrics::Recorder`]. That's why incorrect usage of
/// [`prometheus`] metrics via [`metrics`] crate will inevitably lead to a
/// [`prometheus::Registry`] returning a [`prometheus::Error`], which can be
/// either turned into a panic, or just silently ignored, making this
/// [`FrozenRecorder`] to return a no-op metric instead (see
/// [`metrics::Counter::noop()`] for example).
///
/// The desired behavior can be specified with a [`failure::Strategy`]
/// implementation of this [`FrozenRecorder`]. By default a
/// [`PanicInDebugNoOpInRelease`] [`failure::Strategy`] is used. See
/// [`failure::strategy`] module for other available [`failure::Strategy`]s, or
/// provide your own one by implementing the [`failure::Strategy`] trait.
///
/// ```rust,should_panic
/// use metrics_prometheus::failure::strategy;
///
/// metrics_prometheus::Recorder::builder()
///     .with_metric(prometheus::Gauge::new("value", "help")?)
///     .with_failure_strategy(strategy::Panic)
///     .build_and_install();
///
/// metrics::increment_gauge!("value", 1.0);
/// // This panics, as such labeling is not allowed by `prometheus` crate.
/// metrics::increment_gauge!("value", 2.0, "whose" => "mine");
/// # Ok::<_, prometheus::Error>(())
/// ```
///
/// [`FrozenRecorder`]: Recorder`
/// [`HashMap`]: std::collections::HashMap
/// [`help` description]: prometheus::proto::MetricFamily::get_help
#[derive(Debug)]
pub struct Recorder<FailureStrategy = PanicInDebugNoOpInRelease> {
    /// [`storage::Immutable`] providing access to registered metrics in its
    /// [`prometheus::Registry`].
    pub(super) storage: storage::Immutable,

    /// [`failure::Strategy`] to apply when a [`prometheus::Error`] is
    /// encountered inside [`metrics::Recorder`] methods.
    pub(super) failure_strategy: FailureStrategy,
}

impl Recorder {
    /// Starts building a new [`FrozenRecorder`] on top of the
    /// [`prometheus::default_registry()`].
    ///
    /// [`FrozenRecorder`]: Recorder
    pub fn builder() -> Builder {
        super::Recorder::builder()
    }
}

impl<S> metrics::Recorder for Recorder<S>
where
    S: failure::Strategy,
{
    fn describe_counter(
        &self,
        name: metrics::KeyName,
        _: Option<metrics::Unit>,
        description: metrics::SharedString,
    ) {
        self.storage.describe::<prometheus::IntCounter>(
            name.as_str(),
            description.into_owned(),
        );
    }

    fn describe_gauge(
        &self,
        name: metrics::KeyName,
        _: Option<metrics::Unit>,
        description: metrics::SharedString,
    ) {
        self.storage.describe::<prometheus::Gauge>(
            name.as_str(),
            description.into_owned(),
        );
    }

    fn describe_histogram(
        &self,
        name: metrics::KeyName,
        _: Option<metrics::Unit>,
        description: metrics::SharedString,
    ) {
        self.storage.describe::<prometheus::Histogram>(
            name.as_str(),
            description.into_owned(),
        );
    }

    fn register_counter(&self, key: &metrics::Key) -> metrics::Counter {
        self.storage
            .get_metric::<prometheus::IntCounter>(key)
            .and_then(|res| {
                res.map_err(|e| match self.failure_strategy.decide(&e) {
                    failure::Action::NoOp => (),
                    failure::Action::Panic => panic!(
                        "failed to register `prometheus::IntCounter` metric: \
                         {e}",
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
                res.map_err(|e| match self.failure_strategy.decide(&e) {
                    failure::Action::NoOp => (),
                    failure::Action::Panic => panic!(
                        "failed to register `prometheus::Gauge` metric: {e}",
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
                res.map_err(|e| match self.failure_strategy.decide(&e) {
                    failure::Action::NoOp => (),
                    failure::Action::Panic => panic!(
                        "failed to register `prometheus::Histogram` metric: \
                         {e}",
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
