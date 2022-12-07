//! [`metrics::Recorder`] implementations.

use std::{borrow::Cow, fmt, sync::Arc};

use crate::{
    failure::{self, strategy::PanicInDebugNoOpInRelease},
    metric, storage,
};

/// [`metrics::Recorder`] registering metrics in a [`prometheus::Registry`] and
/// powered by a [`metrics::Registry`] built on top of a [`storage::Mutable`].
///
/// This [`Recorder`] is capable of registering metrics in its
/// [`prometheus::Registry`] on the fly. By default, the
/// [`prometheus::default_registry()`] is used.
///
/// # Performance
///
/// This [`Recorder`] has the very same performance characteristics of using
/// metrics via [`metrics::Recorder`] interface as the ones provided by a
/// [`metrics::Registry`]: for already registered metrics it's just a
/// [`read`-lock] on a sharded [`HashMap`] plus [`Arc`] cloning.
///
/// # Error handling
///
/// [`prometheus::Registry`] has far more stricter semantics than the ones
/// implied by a [`metrics::Recorder`]. That's why incorrect usage of
/// [`prometheus`] metrics via [`metrics`] crate will inevitably lead to a
/// [`prometheus::Registry`] returning a [`prometheus::Error`] instead of a
/// registering the metric. The returned [`prometheus::Error`] can be either
/// turned into a panic, or just silently ignored, making this [`Recorder`] to
/// return a no-op metric (see [`metrics::Counter::noop()`] for example).
///
/// The desired behavior can be specified with a [`failure::Strategy`]
/// implementation of this [`Recorder`]. By default a
/// [`PanicInDebugNoOpInRelease`] [`failure::Strategy`] is used. See
/// [`failure::strategy`] module for other available [`failure::Strategy`]s, or
/// provide your own one by implementing a [`failure::Strategy`] trait.
///
/// [`HashMap`]: std::collections::HashMap
/// [`metrics::Registry`]: metrics_util::registry::Registry
/// [`read`-lock]: std::sync::RwLock::read()
#[derive(Clone)]
pub struct Recorder<FailureStrategy = PanicInDebugNoOpInRelease> {
    /// [`metrics::Registry`] providing performant access to the stored metrics.
    ///
    /// [`metrics::Registry`]: metrics_util::registry::Registry
    metrics:
        Arc<metrics_util::registry::Registry<metrics::Key, storage::Mutable>>,

    /// [`storage::Mutable`] backing the [`metrics::Registry`] and registering
    /// metrics in its [`prometheus::Registry`].
    ///
    /// [`metrics::Registry`]: metrics_util::registry::Registry
    storage: storage::Mutable,

    /// [`failure::Strategy`] to apply when a [`prometheus::Error`] is
    /// encountered inside [`metrics::Recorder`] methods.
    failure_strategy: FailureStrategy,
}

// TODO: Make a PR with `Debug` impl for `metrics_util::registry::Registry`.
impl<S: fmt::Debug> fmt::Debug for Recorder<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Recorder")
            .field("storage", &self.storage)
            .field("failure_strategy", &self.failure_strategy)
            .finish_non_exhaustive()
    }
}

impl<S> Recorder<S> {
    /// Starts building a new [`Recorder`] on top of the
    /// [`prometheus::default_registry()`].
    pub fn builder() -> Builder<S>
    where
        S: failure::Strategy + Default,
    {
        Builder {
            storage: storage::Mutable::default(),
            failure_strategy: S::default(),
        }
    }

    /// Return the underlying [`prometheus::Registry`] backing this
    /// [`Recorder`].
    ///
    /// # Warning
    ///
    /// Any [`prometheus`] metrics, registered directly in the returned
    /// [`prometheus::Registry`], cannot be used via this [`metrics::Recorder`]
    /// (and, so, [`metrics`] crate interfaces), and trying to use them will
    /// inevitably cause a [`prometheus::Error`] being emitted.
    #[must_use]
    pub const fn registry(&self) -> &prometheus::Registry {
        &self.storage.prometheus
    }

    /// Registers the provided [`prometheus`] `metric` in the underlying
    /// [`prometheus::Registry`] in the way making it usable via this
    /// [`Recorder`] (and, so, [`metrics`] crate interfaces).
    ///
    /// Accepts only the following [`prometheus`] metrics:
    /// - [`prometheus::IntCounter`], [`prometheus::IntCounterVec`]
    /// - [`prometheus::Gauge`], [`prometheus::GaugeVec`]
    /// - [`prometheus::Histogram`], [`prometheus::HistogramVec`]
    ///
    /// # Errors
    ///
    /// If the underlying [`prometheus::Registry`] fails to register the
    /// provided `metric`.
    pub fn register<M>(&self, metric: M) -> prometheus::Result<()>
    where
        M: metric::Bundled + prometheus::core::Collector,
        <M as metric::Bundled>::Bundle:
            prometheus::core::Collector + Clone + 'static,
        storage::Mutable:
            storage::GetCollection<<M as metric::Bundled>::Bundle>,
    {
        self.storage.register_external(metric)
    }

    /// Registers the provided [`prometheus`] `metric` in the underlying
    /// [`prometheus::Registry`] in the way making it usable via this
    /// [`Recorder`] (and, so, [`metrics`] crate interfaces).
    ///
    /// Accepts only the following [`prometheus`] metrics:
    /// - [`prometheus::IntCounter`], [`prometheus::IntCounterVec`]
    /// - [`prometheus::Gauge`], [`prometheus::GaugeVec`]
    /// - [`prometheus::Histogram`], [`prometheus::HistogramVec`]
    ///
    /// # Panics
    ///
    /// If the underlying [`prometheus::Registry`] fails to register the
    /// provided `metric`.
    pub fn must_register<M>(&self, metric: M)
    where
        M: metric::Bundled + prometheus::core::Collector,
        <M as metric::Bundled>::Bundle:
            prometheus::core::Collector + Clone + 'static,
        storage::Mutable:
            storage::GetCollection<<M as metric::Bundled>::Bundle>,
    {
        self.register(metric).unwrap_or_else(|e| {
            panic!("failed to register `prometheus` metric: {e}")
        });
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
        self.metrics
            .get_or_create_counter(key, |counter| {
                counter.as_ref().map(|c| Arc::clone(c).into()).or_else(|e| {
                    match self.failure_strategy.decide(e) {
                        failure::Action::NoOp => Ok(metrics::Counter::noop()),
                        // PANIC: We cannot panic inside this closure, because
                        //        this may lead to poisoning `RwLock`s inside
                        //        `metrics_util::registry::Registry`.
                        failure::Action::Panic => Err(e.to_string()),
                    }
                })
            })
            .unwrap_or_else(|e| {
                panic!(
                    "failed to register `prometheus::IntCounter` metric: {e}"
                )
            })
    }

    fn register_gauge(&self, key: &metrics::Key) -> metrics::Gauge {
        self.metrics
            .get_or_create_gauge(key, |gauge| {
                gauge.as_ref().map(|c| Arc::clone(c).into()).or_else(|e| {
                    match self.failure_strategy.decide(e) {
                        failure::Action::NoOp => Ok(metrics::Gauge::noop()),
                        // PANIC: We cannot panic inside this closure, because
                        //        this may lead to poisoning `RwLock`s inside
                        //        `metrics_util::registry::Registry`.
                        failure::Action::Panic => Err(e.to_string()),
                    }
                })
            })
            .unwrap_or_else(|e| {
                panic!("failed to register `prometheus::Gauge` metric: {e}")
            })
    }

    fn register_histogram(&self, key: &metrics::Key) -> metrics::Histogram {
        self.metrics
            .get_or_create_histogram(key, |histogram| {
                histogram.as_ref().map(|c| Arc::clone(c).into()).or_else(|e| {
                    match self.failure_strategy.decide(e) {
                        failure::Action::NoOp => Ok(metrics::Histogram::noop()),
                        // PANIC: We cannot panic inside this closure, because
                        //        this may lead to poisoning `RwLock`s inside
                        //        `metrics_util::registry::Registry`.
                        failure::Action::Panic => Err(e.to_string()),
                    }
                })
            })
            .unwrap_or_else(|e| {
                panic!("failed to register `prometheus::Histogram` metric: {e}")
            })
    }
}

/// Builder for building a [`Recorder`].
#[derive(Debug)]
#[must_use]
pub struct Builder<FailureStrategy = PanicInDebugNoOpInRelease> {
    /// [`storage::Mutable`] registering metrics in its
    /// [`prometheus::Registry`].
    storage: storage::Mutable,

    /// [`failure::Strategy`] of the built [`Recorder`] to apply when a
    /// [`prometheus::Error`] is encountered inside its [`metrics::Recorder`]
    /// methods.
    failure_strategy: FailureStrategy,
}

impl<S> Builder<S> {
    /// Sets the provided [`prometheus::Registry`] to be used by the built
    /// [`Recorder`].
    ///
    /// When not specified, the [`prometheus::default_registry()`] is used by
    /// default.
    ///
    /// # Warning
    ///
    /// Any [`prometheus`] metrics, already registered in the provided
    /// [`prometheus::Registry`], cannot be used via the built
    /// [`metrics::Recorder`] (and, so, [`metrics`] crate interfaces), and
    /// trying to use them will inevitably cause a [`prometheus::Error`] being
    /// emitted.
    pub fn with_registry<'r>(
        mut self,
        registry: impl Into<Cow<'r, prometheus::Registry>>,
    ) -> Self {
        self.storage.prometheus = registry.into().into_owned();
        self
    }

    /// Sets the provided [`failure::Strategy`] to be used by the built
    /// [`Recorder`].
    ///
    /// [`prometheus::Registry`] has far more stricter semantics than the ones
    /// implied by a [`metrics::Recorder`]. That's why incorrect usage of
    /// [`prometheus`] metrics via [`metrics`] crate will inevitably lead to a
    /// [`prometheus::Registry`] returning a [`prometheus::Error`] instead of a
    /// registering the metric. The returned [`prometheus::Error`] can be either
    /// turned into a panic, or just silently ignored, making the [`Recorder`]
    /// to return a no-op metric (see [`metrics::Counter::noop()`] for example).
    ///
    /// The default [`failure::Strategy`] is [`PanicInDebugNoOpInRelease`]. See
    /// [`failure::strategy`] module for other available [`failure::Strategy`]s,
    /// or provide your own one by implementing a [`failure::Strategy`] trait.
    #[allow(clippy::missing_const_for_fn)] // false positive: drop
    pub fn with_failure_strategy<F>(self, strategy: F) -> Builder<F>
    where
        F: failure::Strategy,
    {
        Builder { storage: self.storage, failure_strategy: strategy }
    }

    /// Registers the provided [`prometheus`] `metric` in the underlying
    /// [`prometheus::Registry`] in the way making it usable via the created
    /// [`Recorder`] (and, so, [`metrics`] crate interfaces).
    ///
    /// Accepts only the following [`prometheus`] metrics:
    /// - [`prometheus::IntCounter`], [`prometheus::IntCounterVec`]
    /// - [`prometheus::Gauge`], [`prometheus::GaugeVec`]
    /// - [`prometheus::Histogram`], [`prometheus::HistogramVec`]
    ///
    /// # Errors
    ///
    /// If the underlying [`prometheus::Registry`] fails to register the
    /// provided `metric`.
    pub fn with_metric<M>(self, metric: M) -> prometheus::Result<Self>
    where
        M: metric::Bundled + prometheus::core::Collector,
        <M as metric::Bundled>::Bundle:
            prometheus::core::Collector + Clone + 'static,
        storage::Mutable:
            storage::GetCollection<<M as metric::Bundled>::Bundle>,
    {
        self.storage.register_external(metric)?;
        Ok(self)
    }

    /// Registers the provided [`prometheus`] `metric` in the underlying
    /// [`prometheus::Registry`] in the way making it usable via the created
    /// [`Recorder`] (and, so, [`metrics`] crate interfaces).
    ///
    /// Accepts only the following [`prometheus`] metrics:
    /// - [`prometheus::IntCounter`], [`prometheus::IntCounterVec`]
    /// - [`prometheus::Gauge`], [`prometheus::GaugeVec`]
    /// - [`prometheus::Histogram`], [`prometheus::HistogramVec`]
    ///
    /// # Panics
    ///
    /// If the underlying [`prometheus::Registry`] fails to register the
    /// provided `metric`.
    pub fn with_must_metric<M>(self, metric: M) -> Self
    where
        M: metric::Bundled + prometheus::core::Collector,
        <M as metric::Bundled>::Bundle:
            prometheus::core::Collector + Clone + 'static,
        storage::Mutable:
            storage::GetCollection<<M as metric::Bundled>::Bundle>,
    {
        self.with_metric(metric).unwrap_or_else(|e| {
            panic!("failed to register `prometheus` metric: {e}")
        })
    }

    /// Builds a [`Recorder`] out of this [`Builder`] and registers it as
    /// [`metrics::recorder()`].
    ///
    /// # Errors
    ///
    /// If the built [`Recorder`] fails to be registered as
    /// [`metrics::recorder()`].
    pub fn register(self) -> Result<Recorder<S>, metrics::SetRecorderError>
    where
        S: failure::Strategy + Clone + 'static,
    {
        let Self { storage, failure_strategy } = self;
        let rec = Recorder {
            metrics: Arc::new(metrics_util::registry::Registry::new(
                storage.clone(),
            )),
            storage,
            failure_strategy,
        };
        metrics::set_boxed_recorder(Box::new(rec.clone()))?;
        Ok(rec)
    }

    /// Builds a [`Recorder`] out of this [`Builder`] and registers it as
    /// [`metrics::recorder()`].
    ///
    /// # Panics
    ///
    /// If the built [`Recorder`] fails to be registered as
    /// [`metrics::recorder()`].
    pub fn must_register(self) -> Recorder<S>
    where
        S: failure::Strategy + Clone + 'static,
    {
        self.register().unwrap_or_else(|e| {
            panic!(
                "failed to register `metrics_prometheus::Recorder` as \
                 `metrics::recorder()`: {e}",
            )
        })
    }
}
