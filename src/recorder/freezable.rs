//! [`metrics::Recorder`] being able to stop registering new metrics in the
//! benefit of providing fast access to already registered ones.

use std::sync::Arc;

use once_cell::sync::OnceCell;

use crate::{failure::strategy::PanicInDebugNoOpInRelease, metric, storage};

use super::Builder;

/// [`metrics::Recorder`] being essential a usual [`Recorder`], which is able to
/// become a [`Frozen`] one at some point after creation.
///
/// This [`FreezableRecorder`] is capable of registering metrics in its
/// [`prometheus::Registry`] on the fly, but only before being [`.freeze()`]d.
/// By default, the [`prometheus::default_registry()`] is used.
///
/// # Example
///
/// ```rust
/// let recorder = metrics_prometheus::install_freezable();
///
/// // Either use `metrics` crate interfaces.
/// metrics::increment_counter!("count", "whose" => "mine", "kind" => "owned");
/// metrics::increment_counter!("count", "whose" => "mine", "kind" => "ref");
/// metrics::increment_counter!("count", "kind" => "owned", "whose" => "dummy");
///
/// // Or construct and provide `prometheus` metrics directly.
/// recorder.register_metric(prometheus::Gauge::new("value", "help")?);
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
/// recorder.freeze();
///
/// // However, you cannot register new metrics after freezing.
/// // This is just no-op.
/// metrics::increment_gauge!("new", 2.0);
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
/// // Luckily, metrics still can be described anytime after being registered,
/// // even after freezing.
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
/// # Ok::<_, prometheus::Error>(())
/// ```
///
/// # Performance
///
/// This [`FreezableRecorder`] provides the same overhead of accessing an
/// already registered metric as a usual [`Recorder`] or a [`FrozenRecorder`],
/// depending on whether it has been [`.freeze()`]d, plus an [`AtomicBool`]
/// loading to check whether it has been actually [`.freeze()`]d.
///
/// So, before [`.freeze()`] it's: [`AtomicBool`] loading plus [`read`-lock] on
/// a sharded [`HashMap`] plus [`Arc`] cloning.
///
/// And after [`.freeze()`]: [`AtomicBool`] loading plus regular [`HashMap`]
/// lookup plus [`Arc`] cloning.
///
/// # Errors
///
/// [`prometheus::Registry`] has far more stricter semantics than the ones
/// implied by a [`metrics::Recorder`]. That's why incorrect usage of
/// [`prometheus`] metrics via [`metrics`] crate will inevitably lead to a
/// [`prometheus::Registry`] returning a [`prometheus::Error`] instead of
/// registering the metric. The returned [`prometheus::Error`] can be either
/// turned into a panic, or just silently ignored, making this
/// [`FreezableRecorder`] to return a no-op metric instead (see
/// [`metrics::Counter::noop()`] for example).
///
/// The desired behavior can be specified with a [`failure::Strategy`]
/// implementation of this [`FreezableRecorder`]. By default a
/// [`PanicInDebugNoOpInRelease`] [`failure::Strategy`] is used. See
/// [`failure::strategy`] module for other available [`failure::Strategy`]s, or
/// provide your own one by implementing the [`failure::Strategy`] trait.
///
/// ```rust,should_panic
/// use metrics_prometheus::failure::strategy;
///
/// let recoder = metrics_prometheus::Recorder::builder()
///     .with_failure_strategy(strategy::Panic)
///     .build_freezable_and_install();
///
/// metrics::increment_counter!("count", "kind" => "owned");
///
/// recoder.freeze();
///
/// // This panics, as such labeling is not allowed by `prometheus` crate.
/// metrics::increment_counter!("count", "whose" => "mine");
/// ```
///
/// [`AtomicBool`]: std::sync::atomic::AtomicBool
/// [`failure::Strategy`]: crate::failure::Strategy
/// [`FreezableRecorder`]: Recorder
/// [`Frozen`]: super::Frozen
/// [`FrozenRecorder`]: super::Frozen
/// [`HashMap`]: std::collections::HashMap
/// [`Recorder`]: super::Recorder
/// [`.freeze()`]: Self::freeze()
/// [`read`-lock]: std::sync::RwLock::read()
#[derive(Clone, Debug)]
pub struct Recorder<FailureStrategy = PanicInDebugNoOpInRelease> {
    /// Usual [`Recorder`] for registering metrics on the fly.
    ///
    /// [`Recorder`]: super::Recorder
    usual: super::Recorder<FailureStrategy>,

    /// [`FrozenRecorder`] for fast access to already registered metrics.
    ///
    /// This one is built by draining the [`Recorder::usual`].
    ///
    /// [`FrozenRecorder`]: super::Frozen
    frozen: Arc<OnceCell<super::Frozen<FailureStrategy>>>,
}

impl Recorder {
    /// Starts building a new [`FreezableRecorder`] on top of the
    /// [`prometheus::default_registry()`].
    ///
    /// [`FreezableRecorder`]: Recorder
    pub fn builder() -> Builder {
        super::Recorder::builder()
    }
}

impl<S> Recorder<S> {
    /// Wraps the provided `usual` [`Recorder`] into a [`Freezable`] one.
    ///
    /// [`Freezable`]: Recorder
    /// [`Recorder`]: super::Recorder
    pub(super) fn wrap(usual: super::Recorder<S>) -> Self {
        Self { usual, frozen: Arc::default() }
    }

    /// Returns the underlying [`prometheus::Registry`] backing this
    /// [`FreezableRecorder`].
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
    ///     .build_freezable_and_install();
    ///
    /// let counter = prometheus::IntCounter::new("value", "help")?;
    /// recorder.registry().register(Box::new(counter))?;
    ///
    /// // panics: Duplicate metrics collector registration attempted
    /// metrics::increment_counter!("value");
    /// # Ok::<_, prometheus::Error>(())
    /// ```
    ///
    /// [`FreezableRecorder`]: Recorder
    #[must_use]
    pub const fn registry(&self) -> &prometheus::Registry {
        &self.usual.storage.prometheus
    }

    /// Tries to register the provided [`prometheus`] `metric` in the underlying
    /// [`prometheus::Registry`] in the way making it usable via this
    /// [`FreezableRecorder`] (and, so, [`metrics`] crate interfaces).
    ///
    /// No-op, if this [`FreezableRecorder`] has been [`.freeze()`]d.
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
    /// let recorder = metrics_prometheus::install_freezable();
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
    /// recorder.freeze();
    /// // No-op, as the `Recorder` has been frozen.
    /// recorder.try_register_metric(prometheus::Gauge::new("new", "help")?)?;
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
    ///     .encode_to_string(&recorder.registry().gather())?;
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
    ///
    /// [`FreezableRecorder`]: Recorder
    /// [`.freeze()`]: Recorder::freeze()
    pub fn try_register_metric<M>(&self, metric: M) -> prometheus::Result<()>
    where
        M: metric::Bundled + prometheus::core::Collector,
        <M as metric::Bundled>::Bundle:
            prometheus::core::Collector + Clone + 'static,
        storage::Mutable: storage::Get<
            storage::mutable::Collection<<M as metric::Bundled>::Bundle>,
        >,
    {
        if self.frozen.get().is_none() {
            self.usual.try_register_metric(metric)?;
        }
        Ok(())
    }

    /// Registers the provided [`prometheus`] `metric` in the underlying
    /// [`prometheus::Registry`] in the way making it usable via this
    /// [`FreezableRecorder`] (and, so, [`metrics`] crate interfaces).
    ///
    /// No-op, if this [`FreezableRecorder`] has been [`.freeze()`]d.
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
    /// let recorder = metrics_prometheus::install_freezable();
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
    /// recorder.freeze();
    /// // No-op, as the `Recorder` has been frozen.
    /// recorder.register_metric(prometheus::Gauge::new("new", "help")?);
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
    ///     .encode_to_string(&recorder.registry().gather())?;
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
    ///
    /// [`FreezableRecorder`]: Recorder
    /// [`.freeze()`]: Recorder::freeze()
    pub fn register_metric<M>(&self, metric: M)
    where
        M: metric::Bundled + prometheus::core::Collector,
        <M as metric::Bundled>::Bundle:
            prometheus::core::Collector + Clone + 'static,
        storage::Mutable: storage::Get<
            storage::mutable::Collection<<M as metric::Bundled>::Bundle>,
        >,
    {
        if self.frozen.get().is_none() {
            self.usual.register_metric(metric);
        }
    }

    /// Freezes this [`FreezableRecorder`], making it unable to register new
    /// [`prometheus`] metrics in the benefit of providing faster access to the
    /// already registered ones.
    ///
    /// No-op, if this [`FreezableRecorder`] has been [`.freeze()`]d already.
    ///
    /// # Performance
    ///
    /// This [`FreezableRecorder`] provides the same overhead of accessing an
    /// already registered metric as a usual [`Recorder`] or a
    /// [`FrozenRecorder`], depending on whether it has been [`.freeze()`]d,
    /// plus an [`AtomicBool`] loading to check whether it has been actually
    /// [`.freeze()`]d.
    ///
    /// So, before [`.freeze()`] it's: [`AtomicBool`] loading plus [`read`-lock]
    /// on a sharded [`HashMap`] plus [`Arc`] cloning.
    ///
    /// And after [`.freeze()`]: [`AtomicBool`] loading plus regular [`HashMap`]
    /// lookup plus [`Arc`] cloning.
    ///
    /// # Example
    ///
    /// ```rust
    /// let recorder = metrics_prometheus::install_freezable();
    ///
    /// metrics::increment_counter!("count");
    ///
    /// let report = prometheus::TextEncoder::new()
    ///     .encode_to_string(&recorder.registry().gather())?;
    /// assert_eq!(
    ///     report.trim(),
    ///     r#"
    /// ## HELP count count
    /// ## TYPE count counter
    /// count 1
    ///     "#
    ///     .trim(),
    /// );
    ///
    /// recorder.freeze();
    ///
    /// metrics::increment_counter!("count");
    /// // This is no-op.
    /// metrics::increment_counter!("new");
    ///
    /// let report = prometheus::TextEncoder::new()
    ///     .encode_to_string(&recorder.registry().gather())?;
    /// assert_eq!(
    ///     report.trim(),
    ///     r#"
    /// ## HELP count count
    /// ## TYPE count counter
    /// count 2
    ///     "#
    ///     .trim(),
    /// );
    /// # Ok::<_, prometheus::Error>(())
    /// ```
    ///
    /// [`AtomicBool`]: std::sync::atomic::AtomicBool
    /// [`FreezableRecorder`]: Recorder
    /// [`FrozenRecorder`]: super::Frozen
    /// [`HashMap`]: std::collections::HashMap
    /// [`.freeze()`]: Recorder::freeze()
    pub fn freeze(&self)
    where
        S: Clone,
    {
        _ = self.frozen.get_or_init(|| super::Frozen {
            storage: (&self.usual.storage).into(),
            failure_strategy: self.usual.failure_strategy.clone(),
        });
    }
}

#[warn(clippy::missing_trait_methods)]
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
