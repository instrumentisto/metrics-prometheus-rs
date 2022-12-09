//! [`metrics::Recorder`] implementations.

pub mod layer;

use std::{fmt, sync::Arc};

use crate::{
    failure::{self, strategy::PanicInDebugNoOpInRelease},
    metric, storage, IntoCow,
};

pub use metrics_util::layers::Layer;

/// [`metrics::Recorder`] registering metrics in a [`prometheus::Registry`] and
/// powered by a [`metrics::Registry`] built on top of a [`storage::Mutable`].
///
/// This [`Recorder`] is capable of registering metrics in its
/// [`prometheus::Registry`] on the fly. By default, the
/// [`prometheus::default_registry()`] is used.
///
/// # Example
///
/// ```rust
/// let recorder = metrics_prometheus::install();
///
/// // Either use `metrics` crate interfaces.
/// metrics::increment_counter!("count", "whose" => "mine", "kind" => "owned");
/// metrics::increment_counter!("count", "whose" => "mine", "kind" => "ref");
/// metrics::increment_counter!("count", "kind" => "owned", "whose" => "dummy");
///
/// // Or construct and provide `prometheus` metrics directly.
/// recorder.try_register_metric(prometheus::Gauge::new("value", "help")?)?;
///
/// let report = prometheus::TextEncoder::new()
///     .encode_to_string(&prometheus::default_registry().gather())?;
/// assert_eq!(
///     report.trim(),
///     r#"
/// ## HELP count count
/// ## TYPE count counter
/// count{kind="owned",whose="dummy"} 1
/// count{kind="owned",whose="mine"} 1
/// count{kind="ref",whose="mine"} 1
/// ## HELP value help
/// ## TYPE value gauge
/// value 0
///     "#
///     .trim(),
/// );
///
/// // Metrics can be described anytime after being registered in
/// // `prometheus::Registry`.
/// metrics::describe_counter!("count", "Example of counter.");
/// metrics::describe_gauge!("value", "Example of gauge.");
///
/// let report = prometheus::TextEncoder::new()
///     .encode_to_string(&recorder.registry().gather())?;
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
/// value 0
///     "#
///     .trim(),
/// );
///
/// // Description can be changed multiple times and anytime:
/// metrics::describe_counter!("count", "Another description.");
///
/// // Even before a metric is registered in `prometheus::Registry`.
/// metrics::describe_counter!("another", "Yet another counter.");
/// metrics::increment_counter!("another");
///
/// let report = prometheus::TextEncoder::new()
///     .encode_to_string(&recorder.registry().gather())?;
/// assert_eq!(
///     report.trim(),
///     r#"
/// ## HELP another Yet another counter.
/// ## TYPE another counter
/// another 1
/// ## HELP count Another description.
/// ## TYPE count counter
/// count{kind="owned",whose="dummy"} 1
/// count{kind="owned",whose="mine"} 1
/// count{kind="ref",whose="mine"} 1
/// ## HELP value Example of gauge.
/// ## TYPE value gauge
/// value 0
///     "#
///     .trim(),
/// );
/// # Ok::<_, prometheus::Error>(())
/// ```
///
/// # Performance
///
/// This [`Recorder`] has the very same performance characteristics of using
/// metrics via [`metrics::Recorder`] interface as the ones provided by a
/// [`metrics::Registry`]: for already registered metrics it's just a
/// [`read`-lock] on a sharded [`HashMap`] plus [`Arc`] cloning.
///
/// # Errors
///
/// [`prometheus::Registry`] has far more stricter semantics than the ones
/// implied by a [`metrics::Recorder`]. That's why incorrect usage of
/// [`prometheus`] metrics via [`metrics`] crate will inevitably lead to a
/// [`prometheus::Registry`] returning a [`prometheus::Error`] instead of a
/// registering the metric. The returned [`prometheus::Error`] can be either
/// turned into a panic, or just silently ignored, making this [`Recorder`] to
/// return a no-op metric instead (see [`metrics::Counter::noop()`] for
/// example).
///
/// The desired behavior can be specified with a [`failure::Strategy`]
/// implementation of this [`Recorder`]. By default a
/// [`PanicInDebugNoOpInRelease`] [`failure::Strategy`] is used. See
/// [`failure::strategy`] module for other available [`failure::Strategy`]s, or
/// provide your own one by implementing the [`failure::Strategy`] trait.
///
/// ```rust,should_panic
/// use metrics_prometheus::failure::strategy;
///
/// metrics_prometheus::Recorder::builder()
///     .with_failure_strategy(strategy::Panic)
///     .build_and_install();
///
/// metrics::increment_counter!("count", "kind" => "owned");
/// // This panics, as such labeling is not allowed by `prometheus` crate.
/// metrics::increment_counter!("count", "whose" => "mine");
/// ```
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

impl Recorder {
    /// Starts building a new [`Recorder`] on top of the
    /// [`prometheus::default_registry()`].
    pub fn builder() -> Builder {
        Builder {
            storage: storage::Mutable::default(),
            failure_strategy: PanicInDebugNoOpInRelease,
            layers: layer::Stack::identity(),
        }
    }
}

impl<S> Recorder<S> {
    /// Return the underlying [`prometheus::Registry`] backing this
    /// [`Recorder`].
    ///
    /// # Warning
    ///
    /// Any [`prometheus`] metrics, registered directly in the returned
    /// [`prometheus::Registry`], cannot be used via this [`metrics::Recorder`]
    /// (and, so, [`metrics`] crate interfaces), and trying to use them will
    /// inevitably cause a [`prometheus::Error`] being emitted.
    ///
    /// ```rust,should_panic
    /// use metrics_prometheus::failure::strategy;
    ///
    /// let recorder = metrics_prometheus::Recorder::builder()
    ///     .with_failure_strategy(strategy::Panic)
    ///     .build_and_install();
    ///
    /// let counter = prometheus::IntCounter::new("value", "help")?;
    /// recorder.registry().register(Box::new(counter))?;
    ///
    /// // panics: Duplicate metrics collector registration attempted
    /// metrics::increment_counter!("value");
    /// # Ok::<_, prometheus::Error>(())
    /// ```
    #[must_use]
    pub const fn registry(&self) -> &prometheus::Registry {
        &self.storage.prometheus
    }

    /// Tries to register the provided [`prometheus`] `metric` in the underlying
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
    ///
    /// # Example
    ///
    /// ```rust
    /// let recorder = metrics_prometheus::install();
    ///
    /// let counter = prometheus::IntCounterVec::new(
    ///     prometheus::opts!("value", "help"),
    ///     &["whose", "kind"],
    /// )?;
    ///
    /// recorder.try_register_metric(counter.clone())?;
    ///
    /// counter.with_label_values(&["mine", "owned"]).inc();
    /// counter.with_label_values(&["foreign", "ref"]).inc_by(2);
    /// counter.with_label_values(&["foreign", "owned"]).inc_by(3);
    ///
    /// let report = prometheus::TextEncoder::new()
    ///     .encode_to_string(&prometheus::default_registry().gather())?;
    /// assert_eq!(
    ///     report.trim(),
    ///     r#"
    /// ## HELP value help
    /// ## TYPE value counter
    /// value{kind="owned",whose="foreign"} 3
    /// value{kind="owned",whose="mine"} 1
    /// value{kind="ref",whose="foreign"} 2
    ///     "#
    ///     .trim(),
    /// );
    ///
    /// metrics::increment_counter!(
    ///     "value", "whose" => "mine", "kind" => "owned",
    /// );
    /// metrics::increment_counter!(
    ///     "value", "whose" => "mine", "kind" => "ref",
    /// );
    /// metrics::increment_counter!(
    ///     "value", "kind" => "owned", "whose" => "foreign",
    /// );
    ///
    /// let report = prometheus::TextEncoder::new()
    ///     .encode_to_string(&prometheus::default_registry().gather())?;
    /// assert_eq!(
    ///     report.trim(),
    ///     r#"
    /// ## HELP value help
    /// ## TYPE value counter
    /// value{kind="owned",whose="foreign"} 4
    /// value{kind="owned",whose="mine"} 2
    /// value{kind="ref",whose="foreign"} 2
    /// value{kind="ref",whose="mine"} 1
    ///     "#
    ///     .trim(),
    /// );
    /// # Ok::<_, prometheus::Error>(())
    /// ```
    pub fn try_register_metric<M>(&self, metric: M) -> prometheus::Result<()>
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
    ///
    /// # Example
    ///
    /// ```rust
    /// let recorder = metrics_prometheus::install();
    ///
    /// let gauge = prometheus::GaugeVec::new(
    ///     prometheus::opts!("value", "help"),
    ///     &["whose", "kind"],
    /// )?;
    ///
    /// recorder.register_metric(gauge.clone());
    ///
    /// gauge.with_label_values(&["mine", "owned"]).inc();
    /// gauge.with_label_values(&["foreign", "ref"]).set(2.0);
    /// gauge.with_label_values(&["foreign", "owned"]).set(3.0);
    ///
    /// let report = prometheus::TextEncoder::new()
    ///     .encode_to_string(&prometheus::default_registry().gather())?;
    /// assert_eq!(
    ///     report.trim(),
    ///     r#"
    /// ## HELP value help
    /// ## TYPE value gauge
    /// value{kind="owned",whose="foreign"} 3
    /// value{kind="owned",whose="mine"} 1
    /// value{kind="ref",whose="foreign"} 2
    ///     "#
    ///     .trim(),
    /// );
    ///
    /// metrics::increment_gauge!(
    ///     "value", 2.0, "whose" => "mine", "kind" => "owned",
    /// );
    /// metrics::decrement_gauge!(
    ///     "value", 2.0, "whose" => "mine", "kind" => "ref",
    /// );
    /// metrics::increment_gauge!(
    ///     "value", 2.0, "kind" => "owned", "whose" => "foreign",
    /// );
    ///
    /// let report = prometheus::TextEncoder::new()
    ///     .encode_to_string(&prometheus::default_registry().gather())?;
    /// assert_eq!(
    ///     report.trim(),
    ///     r#"
    /// ## HELP value help
    /// ## TYPE value gauge
    /// value{kind="owned",whose="foreign"} 5
    /// value{kind="owned",whose="mine"} 3
    /// value{kind="ref",whose="foreign"} 2
    /// value{kind="ref",whose="mine"} -2
    ///     "#
    ///     .trim(),
    /// );
    /// # Ok::<_, prometheus::Error>(())
    /// ```
    pub fn register_metric<M>(&self, metric: M)
    where
        M: metric::Bundled + prometheus::core::Collector,
        <M as metric::Bundled>::Bundle:
            prometheus::core::Collector + Clone + 'static,
        storage::Mutable:
            storage::GetCollection<<M as metric::Bundled>::Bundle>,
    {
        self.try_register_metric(metric).unwrap_or_else(|e| {
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
pub struct Builder<
    FailureStrategy = PanicInDebugNoOpInRelease,
    Layers = layer::Stack,
> {
    /// [`storage::Mutable`] registering metrics in its
    /// [`prometheus::Registry`].
    storage: storage::Mutable,

    /// [`failure::Strategy`] of the built [`Recorder`] to apply when a
    /// [`prometheus::Error`] is encountered inside its [`metrics::Recorder`]
    /// methods.
    failure_strategy: FailureStrategy,

    /// [`metrics::Layer`]s to wrap the built [`Recorder`] with upon its
    /// installation as [`metrics::recorder()`].
    ///
    /// [`metrics::Layer`]: Layer
    layers: Layers,
}

impl<S, L> Builder<S, L> {
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
    ///
    /// # Example
    ///
    /// ```rust
    /// let custom = prometheus::Registry::new_custom(Some("my".into()), None)?;
    ///
    /// metrics_prometheus::Recorder::builder()
    ///     .with_registry(&custom)
    ///     .build_and_install();
    ///
    /// metrics::increment_counter!("count");
    ///
    /// let report =
    ///     prometheus::TextEncoder::new().encode_to_string(&custom.gather())?;
    /// assert_eq!(
    ///     report.trim(),
    ///     r#"
    /// ## HELP my_count count
    /// ## TYPE my_count counter
    /// my_count 1
    ///     "#
    ///     .trim(),
    /// );
    /// # Ok::<_, prometheus::Error>(())
    /// ```
    pub fn with_registry<'r>(
        mut self,
        registry: impl IntoCow<'r, prometheus::Registry>,
    ) -> Self {
        self.storage.prometheus = registry.into_cow().into_owned();
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
    /// to return a no-op metric instead (see [`metrics::Counter::noop()`] for
    /// example).
    ///
    /// The default [`failure::Strategy`] is [`PanicInDebugNoOpInRelease`]. See
    /// [`failure::strategy`] module for other available [`failure::Strategy`]s,
    /// or provide your own one by implementing the [`failure::Strategy`] trait.
    ///
    /// # Example
    ///
    /// ```rust
    /// use metrics_prometheus::failure::strategy;
    ///
    /// metrics_prometheus::Recorder::builder()
    ///     .with_failure_strategy(strategy::NoOp)
    ///     .build_and_install();
    ///
    /// metrics::increment_counter!("invalid.name");
    ///
    /// let stats = prometheus::default_registry().gather();
    /// assert_eq!(stats.len(), 0);
    /// ```
    #[allow(clippy::missing_const_for_fn)] // false positive: drop
    pub fn with_failure_strategy<F>(self, strategy: F) -> Builder<F, L>
    where
        F: failure::Strategy,
    {
        Builder {
            storage: self.storage,
            failure_strategy: strategy,
            layers: self.layers,
        }
    }

    /// Tries to register the provided [`prometheus`] `metric` in the underlying
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
    ///
    /// # Example
    ///
    /// ```rust
    /// let gauge = prometheus::Gauge::new("value", "help")?;
    ///
    /// metrics_prometheus::Recorder::builder()
    ///     .try_with_metric(gauge.clone())?
    ///     .build_and_install();
    ///
    /// gauge.inc();
    ///
    /// let report = prometheus::TextEncoder::new()
    ///     .encode_to_string(&prometheus::default_registry().gather())?;
    /// assert_eq!(
    ///     report.trim(),
    ///     r#"
    /// ## HELP value help
    /// ## TYPE value gauge
    /// value 1
    ///     "#
    ///     .trim(),
    /// );
    ///
    /// metrics::increment_gauge!("value", 1.0);
    ///
    /// let report = prometheus::TextEncoder::new()
    ///     .encode_to_string(&prometheus::default_registry().gather())?;
    /// assert_eq!(
    ///     report.trim(),
    ///     r#"
    /// ## HELP value help
    /// ## TYPE value gauge
    /// value 2
    ///     "#
    ///     .trim(),
    /// );
    /// # Ok::<_, prometheus::Error>(())
    /// ```
    pub fn try_with_metric<M>(self, metric: M) -> prometheus::Result<Self>
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
    ///
    /// # Example
    ///
    /// ```rust
    /// let counter = prometheus::IntCounter::new("value", "help")?;
    ///
    /// metrics_prometheus::Recorder::builder()
    ///     .with_metric(counter.clone())
    ///     .build_and_install();
    ///
    /// counter.inc();
    ///
    /// let report = prometheus::TextEncoder::new()
    ///     .encode_to_string(&prometheus::default_registry().gather())?;
    /// assert_eq!(
    ///     report.trim(),
    ///     r#"
    /// ## HELP value help
    /// ## TYPE value counter
    /// value 1
    ///     "#
    ///     .trim(),
    /// );
    ///
    /// metrics::increment_counter!("value");
    ///
    /// let report = prometheus::TextEncoder::new()
    ///     .encode_to_string(&prometheus::default_registry().gather())?;
    /// assert_eq!(
    ///     report.trim(),
    ///     r#"
    /// ## HELP value help
    /// ## TYPE value counter
    /// value 2
    ///     "#
    ///     .trim(),
    /// );
    /// # Ok::<_, prometheus::Error>(())
    /// ```
    pub fn with_metric<M>(self, metric: M) -> Self
    where
        M: metric::Bundled + prometheus::core::Collector,
        <M as metric::Bundled>::Bundle:
            prometheus::core::Collector + Clone + 'static,
        storage::Mutable:
            storage::GetCollection<<M as metric::Bundled>::Bundle>,
    {
        self.try_with_metric(metric).unwrap_or_else(|e| {
            panic!("failed to register `prometheus` metric: {e}")
        })
    }

    /// Builds a [`Recorder`] out of this [`Builder`] and tries to install it as
    /// [`metrics::recorder()`].
    ///
    /// # Errors
    ///
    /// If the built [`Recorder`] fails to be installed as
    /// [`metrics::recorder()`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use metrics_prometheus::{failure::strategy, recorder};
    /// use metrics_util::layers::FilterLayer;
    ///
    /// let custom = prometheus::Registry::new_custom(Some("my".into()), None)?;
    ///
    /// let res = metrics_prometheus::Recorder::builder()
    ///     .with_registry(&custom)
    ///     .try_with_metric(prometheus::IntCounter::new("count", "help")?)?
    ///     .try_with_metric(prometheus::Gauge::new("value", "help")?)?
    ///     .with_failure_strategy(strategy::Panic)
    ///     .with_layer(FilterLayer::from_patterns(["ignored"]))
    ///     .try_build_and_install();
    /// assert!(res.is_ok(), "cannot install `Recorder`: {}", res.unwrap_err());
    ///
    /// metrics::increment_counter!("count");
    /// metrics::increment_gauge!("value", 3.0);
    /// metrics::histogram!("histo", 38.0);
    /// metrics::histogram!("ignored_histo", 1.0);
    ///
    /// let report =
    ///     prometheus::TextEncoder::new().encode_to_string(&custom.gather())?;
    /// assert_eq!(
    ///     report.trim(),
    ///     r#"
    /// ## HELP my_count help
    /// ## TYPE my_count counter
    /// my_count 1
    /// ## HELP my_histo histo
    /// ## TYPE my_histo histogram
    /// my_histo_bucket{le="0.005"} 0
    /// my_histo_bucket{le="0.01"} 0
    /// my_histo_bucket{le="0.025"} 0
    /// my_histo_bucket{le="0.05"} 0
    /// my_histo_bucket{le="0.1"} 0
    /// my_histo_bucket{le="0.25"} 0
    /// my_histo_bucket{le="0.5"} 0
    /// my_histo_bucket{le="1"} 0
    /// my_histo_bucket{le="2.5"} 0
    /// my_histo_bucket{le="5"} 0
    /// my_histo_bucket{le="10"} 0
    /// my_histo_bucket{le="+Inf"} 1
    /// my_histo_sum 38
    /// my_histo_count 1
    /// ## HELP my_value help
    /// ## TYPE my_value gauge
    /// my_value 3
    ///     "#
    ///     .trim(),
    /// );
    /// # Ok::<_, prometheus::Error>(())
    /// ```
    pub fn try_build_and_install(
        self,
    ) -> Result<Recorder<S>, metrics::SetRecorderError>
    where
        S: failure::Strategy + Clone,
        L: Layer<Recorder<S>>,
        <L as Layer<Recorder<S>>>::Output: metrics::Recorder + 'static,
    {
        let Self { storage, failure_strategy, layers } = self;
        let rec = Recorder {
            metrics: Arc::new(metrics_util::registry::Registry::new(
                storage.clone(),
            )),
            storage,
            failure_strategy,
        };
        metrics::set_boxed_recorder(Box::new(layers.layer(rec.clone())))?;
        Ok(rec)
    }

    /// Builds a [`Recorder`] out of this [`Builder`] and installs it as
    /// [`metrics::recorder()`].
    ///
    /// # Panics
    ///
    /// If the built [`Recorder`] fails to be installed as
    /// [`metrics::recorder()`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use metrics_prometheus::{failure::strategy, recorder};
    /// use metrics_util::layers::FilterLayer;
    ///
    /// let custom = prometheus::Registry::new_custom(Some("my".into()), None)?;
    ///
    /// let recorder = metrics_prometheus::Recorder::builder()
    ///     .with_registry(custom)
    ///     .try_with_metric(prometheus::IntCounter::new("count", "help")?)?
    ///     .try_with_metric(prometheus::Gauge::new("value", "help")?)?
    ///     .with_failure_strategy(strategy::Panic)
    ///     .with_layer(FilterLayer::from_patterns(["ignored"]))
    ///     .build_and_install();
    ///
    /// metrics::increment_counter!("count");
    /// metrics::increment_gauge!("value", 3.0);
    /// metrics::histogram!("histo", 38.0);
    /// metrics::histogram!("ignored_histo", 1.0);
    ///
    /// let report = prometheus::TextEncoder::new()
    ///     .encode_to_string(&recorder.registry().gather())?;
    /// assert_eq!(
    ///     report.trim(),
    ///     r#"
    /// ## HELP my_count help
    /// ## TYPE my_count counter
    /// my_count 1
    /// ## HELP my_histo histo
    /// ## TYPE my_histo histogram
    /// my_histo_bucket{le="0.005"} 0
    /// my_histo_bucket{le="0.01"} 0
    /// my_histo_bucket{le="0.025"} 0
    /// my_histo_bucket{le="0.05"} 0
    /// my_histo_bucket{le="0.1"} 0
    /// my_histo_bucket{le="0.25"} 0
    /// my_histo_bucket{le="0.5"} 0
    /// my_histo_bucket{le="1"} 0
    /// my_histo_bucket{le="2.5"} 0
    /// my_histo_bucket{le="5"} 0
    /// my_histo_bucket{le="10"} 0
    /// my_histo_bucket{le="+Inf"} 1
    /// my_histo_sum 38
    /// my_histo_count 1
    /// ## HELP my_value help
    /// ## TYPE my_value gauge
    /// my_value 3
    ///     "#
    ///     .trim(),
    /// );
    /// # Ok::<_, prometheus::Error>(())
    /// ```
    pub fn build_and_install(self) -> Recorder<S>
    where
        S: failure::Strategy + Clone,
        L: Layer<Recorder<S>>,
        <L as Layer<Recorder<S>>>::Output: metrics::Recorder + 'static,
    {
        self.try_build_and_install().unwrap_or_else(|e| {
            panic!(
                "failed to install `metrics_prometheus::Recorder` as \
                 `metrics::recorder()`: {e}",
            )
        })
    }
}

impl<S, H, T> Builder<S, layer::Stack<H, T>> {
    /// Adds the provided [`metrics::Layer`] to wrap the built [`Recorder`] upon
    /// its installation as [`metrics::recorder()`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use metrics_util::layers::FilterLayer;
    ///
    /// metrics_prometheus::Recorder::builder()
    ///     .with_layer(FilterLayer::from_patterns(["ignored"]))
    ///     .with_layer(FilterLayer::from_patterns(["skipped"]))
    ///     .build_and_install();
    ///
    /// metrics::increment_counter!("ignored_counter");
    /// metrics::increment_counter!("reported_counter");
    /// metrics::increment_counter!("skipped_counter");
    ///
    /// let report = prometheus::TextEncoder::new()
    ///     .encode_to_string(&prometheus::default_registry().gather())?;
    /// assert_eq!(
    ///     report.trim(),
    ///     r#"
    /// ## HELP reported_counter reported_counter
    /// ## TYPE reported_counter counter
    /// reported_counter 1
    ///     "#
    ///     .trim(),
    /// );
    /// # Ok::<_, prometheus::Error>(())
    /// ```
    ///
    /// [`metrics::Layer`]: Layer
    #[allow(clippy::missing_const_for_fn)] // false positive: drop
    pub fn with_layer<L>(
        self,
        layer: L,
    ) -> Builder<S, layer::Stack<L, layer::Stack<H, T>>>
    where
        L: Layer<<layer::Stack<H, T> as Layer<Recorder<S>>>::Output>,
        layer::Stack<H, T>: Layer<Recorder<S>>,
    {
        Builder {
            storage: self.storage,
            failure_strategy: self.failure_strategy,
            layers: self.layers.push(layer),
        }
    }
}
