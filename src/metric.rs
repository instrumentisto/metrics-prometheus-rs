//! Machinery around [`prometheus`] metrics for making them usable via
//! [`metrics`] crate.

use std::{iter, sync::Arc};

use arc_swap::ArcSwap;
use sealed::sealed;
use smallvec::SmallVec;

use self::bundle::Either;

#[doc(inline)]
pub use self::bundle::Bundle;

/// Wrapper allowing implementing [`metrics::CounterFn`], [`metrics::GaugeFn`]
/// and [`metrics::HistogramFn`] for [`prometheus`] metrics.
#[derive(Clone, Copy, Debug)]
pub struct Metric<M>(M);

impl<M> Metric<M> {
    /// Wraps the provided [`prometheus`] `metric`.
    #[must_use]
    pub const fn wrap(metric: M) -> Self {
        Self(metric)
    }

    /// Unwraps this [`Metric`] returning its inner [`prometheus`] metric
    #[must_use]
    pub fn into_inner(self) -> M {
        self.0
    }
}

impl<M> AsRef<M> for Metric<M> {
    fn as_ref(&self) -> &M {
        &self.0
    }
}

impl<M> AsMut<M> for Metric<M> {
    fn as_mut(&mut self) -> &mut M {
        &mut self.0
    }
}

#[warn(clippy::missing_trait_methods)]
impl metrics::CounterFn for Metric<prometheus::IntCounter> {
    fn increment(&self, value: u64) {
        self.0.inc_by(value);
    }

    fn absolute(&self, value: u64) {
        // `prometheus::IntCounter` doesn't provide any atomic way to set its
        // absolute value, so the implementation below may introduce races when
        // two `.absolute()` operations content, leading to the incorrect value
        // of a sum of two absolute values.
        // However, considering that `.absolute()` operations should be quite
        // rare, and so, rarely content, we do imply this trade-off as
        // acceptable, for a while.
        // TODO: Make a PR to `prometheus` crate allowing setting absolute value
        //       atomically.
        self.0.reset();
        self.0.inc_by(value);
    }
}

#[warn(clippy::missing_trait_methods)]
impl metrics::GaugeFn for Metric<prometheus::Gauge> {
    fn increment(&self, value: f64) {
        self.0.add(value);
    }

    fn decrement(&self, value: f64) {
        self.0.sub(value);
    }

    fn set(&self, value: f64) {
        self.0.set(value);
    }
}

#[warn(clippy::missing_trait_methods)]
impl metrics::HistogramFn for Metric<prometheus::Histogram> {
    fn record(&self, value: f64) {
        self.0.observe(value);
    }
}

/// Fallible [`Metric`] stored in [`metrics::Registry`].
///
/// We're obligated to store [`Fallible`] metrics inside [`metrics::Registry`],
/// because panicking earlier, rather than inside directly called
/// [`metrics::Recorder`] methods, will poison locks the [`metrics::Registry`]
/// is built upon on.
///
/// [`metrics::Registry`]: metrics_util::registry::Registry
#[derive(Debug)]
pub struct Fallible<M>(pub Arc<prometheus::Result<Arc<Metric<M>>>>);

// Manual implementation is required to omit the redundant `M: Clone` trait
// bound imposed by `#[derive(Clone)]`.
impl<M> Clone for Fallible<M> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<M> From<prometheus::Result<Arc<Metric<M>>>> for Fallible<M> {
    fn from(res: prometheus::Result<Arc<Metric<M>>>) -> Self {
        Self(Arc::new(res))
    }
}

impl<M> Fallible<M> {
    /// Mimics [`Result::as_ref()`] method for this [`Fallible`].
    ///
    /// # Errors
    ///
    /// If this [`Fallible`] contains a [`prometheus::Error`].
    pub fn as_ref(&self) -> Result<&Arc<Metric<M>>, &prometheus::Error> {
        (*self.0).as_ref()
    }
}

// Not really used, only implemented to satisfy
// `metrics_util::registry::Storage` requirements for stored items.
#[warn(clippy::missing_trait_methods)]
impl<M> metrics::CounterFn for Fallible<M>
where
    Metric<M>: metrics::CounterFn,
{
    fn increment(&self, value: u64) {
        if let Ok(m) = &*self.0 {
            m.increment(value);
        }
    }

    fn absolute(&self, value: u64) {
        if let Ok(m) = &*self.0 {
            m.absolute(value);
        }
    }
}

// Not really used, only implemented to satisfy
// `metrics_util::registry::Storage` requirements for stored items.
#[warn(clippy::missing_trait_methods)]
impl<M> metrics::GaugeFn for Fallible<M>
where
    Metric<M>: metrics::GaugeFn,
{
    fn increment(&self, value: f64) {
        if let Ok(m) = &*self.0 {
            m.increment(value);
        }
    }

    fn decrement(&self, value: f64) {
        if let Ok(m) = &*self.0 {
            m.decrement(value);
        }
    }

    fn set(&self, value: f64) {
        if let Ok(m) = &*self.0 {
            m.set(value);
        }
    }
}

// Not really used, only implemented to satisfy
// `metrics_util::registry::Storage` requirements for stored items.
#[warn(clippy::missing_trait_methods)]
impl<M> metrics::HistogramFn for Fallible<M>
where
    Metric<M>: metrics::HistogramFn,
{
    fn record(&self, value: f64) {
        if let Ok(m) = &*self.0 {
            m.record(value);
        }
    }
}

/// [`prometheus`] metric with an ability to substitute its [`help` description]
/// after registration in a [`prometheus::Registry`].
///
/// [`help` description]: prometheus::proto::MetricFamily::get_help
#[derive(Clone, Debug, Default)]
pub struct Describable<Metric> {
    /// Swappable [`help` description] of the [`prometheus`] metric.
    ///
    /// [`help` description]: prometheus::proto::MetricFamily::get_help
    pub(crate) description: Arc<ArcSwap<String>>,

    /// [`prometheus`] metric itself.
    pub(crate) metric: Metric,
}

impl<M> Describable<M> {
    /// Wraps the provided [`prometheus`] `metric` into a [`Describable`] one.
    #[must_use]
    pub fn wrap(metric: M) -> Self {
        Self { description: Arc::default(), metric }
    }

    /// Generates a [`Default`] [`prometheus`] metric with the provided
    /// [`help` description].
    ///
    /// [`help` description]: prometheus::proto::MetricFamily::get_help
    #[must_use]
    pub fn only_description(help: impl Into<String>) -> Self
    where
        M: Default,
    {
        Self {
            description: Arc::new(ArcSwap::new(Arc::new(help.into()))),
            metric: M::default(),
        }
    }

    /// Maps the wrapped [`prometheus`] metric `into` another one, preserving
    /// the current overwritten [`help` description] (if any).
    ///
    /// [`help` description]: prometheus::proto::MetricFamily::get_help
    #[must_use]
    pub fn map<Into>(self, into: impl FnOnce(M) -> Into) -> Describable<Into> {
        Describable { description: self.description, metric: into(self.metric) }
    }
}

impl<M> Describable<Option<M>> {
    /// Transposes this [`Describable`] [`Option`]al metric into an [`Option`]
    /// of a [`Describable`] metric.
    #[must_use]
    pub fn transpose(self) -> Option<Describable<M>> {
        self.metric
            .map(|metric| Describable { description: self.description, metric })
    }
}

#[warn(clippy::missing_trait_methods)]
impl<M> prometheus::core::Collector for Describable<M>
where
    M: prometheus::core::Collector,
{
    fn desc(&self) -> Vec<&prometheus::core::Desc> {
        // We could omit changing `help` field here, because `Collector::desc()`
        // method is used by `prometheus::Registry` only for metrics
        // registration and validation in its `.register()` and `.unregister()`
        // methods. When `prometheus::Registry` `.gather()`s metrics, it invokes
        // `Collector::collect()` method, where we do the actual `help` field
        // substitution.
        self.metric.desc()
    }

    fn collect(&self) -> Vec<prometheus::proto::MetricFamily> {
        let mut out = self.metric.collect();
        let new_help = self.description.load_full();
        if !new_help.is_empty() {
            for mf in &mut out {
                mf.set_help((*new_help).clone());
            }
        }
        out
    }
}

/// Custom conversion trait to convert between foreign types.
trait To<T> {
    /// Converts this reference into a `T` value.
    fn to(&self) -> T;
}

impl To<prometheus::Opts> for metrics::Key {
    fn to(&self) -> prometheus::Opts {
        // We use `key.name()` as `help` description here, because `prometheus`
        // crate doesn't allow to make it empty.
        prometheus::Opts::new(self.name(), self.name())
    }
}

impl To<prometheus::HistogramOpts> for metrics::Key {
    fn to(&self) -> prometheus::HistogramOpts {
        // We use `key.name()` as `help` description here, because `prometheus`
        // crate doesn't allow to make it empty.
        prometheus::HistogramOpts::new(self.name(), self.name())
    }
}

/// [`prometheus`] metric being [`Bundle`]d.
#[sealed]
pub trait Bundled {
    /// Type of a [`Bundle`] bundling this [`prometheus`] metric.
    type Bundle: Bundle;

    /// Wraps this [`prometheus`] metric into its [`Bundle`].
    fn into_bundle(self) -> Self::Bundle;
}

#[sealed]
impl Bundled for prometheus::IntCounter {
    type Bundle = PrometheusIntCounter;

    fn into_bundle(self) -> Self::Bundle {
        PrometheusIntCounter::Single(self)
    }
}

#[sealed]
impl Bundled for prometheus::IntCounterVec {
    type Bundle = PrometheusIntCounter;

    fn into_bundle(self) -> Self::Bundle {
        PrometheusIntCounter::Vec(self)
    }
}

#[sealed]
impl Bundled for prometheus::Gauge {
    type Bundle = PrometheusGauge;

    fn into_bundle(self) -> Self::Bundle {
        PrometheusGauge::Single(self)
    }
}

#[sealed]
impl Bundled for prometheus::GaugeVec {
    type Bundle = PrometheusGauge;

    fn into_bundle(self) -> Self::Bundle {
        PrometheusGauge::Vec(self)
    }
}

#[sealed]
impl Bundled for prometheus::Histogram {
    type Bundle = PrometheusHistogram;

    fn into_bundle(self) -> Self::Bundle {
        PrometheusHistogram::Single(self)
    }
}

#[sealed]
impl Bundled for prometheus::HistogramVec {
    type Bundle = PrometheusHistogram;

    fn into_bundle(self) -> Self::Bundle {
        PrometheusHistogram::Vec(self)
    }
}

/// [`Bundle`] of [`prometheus::IntCounter`] metrics.
pub type PrometheusIntCounter =
    Either<prometheus::IntCounter, prometheus::IntCounterVec>;

impl TryFrom<&metrics::Key> for PrometheusIntCounter {
    type Error = prometheus::Error;

    fn try_from(key: &metrics::Key) -> Result<Self, Self::Error> {
        let mut labels_iter = key.labels();
        Ok(if let Some(first_label) = labels_iter.next() {
            let label_names = iter::once(first_label)
                .chain(labels_iter)
                .map(metrics::Label::key)
                .collect::<SmallVec<[_; 10]>>();
            Self::Vec(prometheus::IntCounterVec::new(key.to(), &label_names)?)
        } else {
            Self::Single(prometheus::IntCounter::with_opts(key.to())?)
        })
    }
}

/// [`Bundle`] of [`prometheus::Gauge`] metrics.
pub type PrometheusGauge = Either<prometheus::Gauge, prometheus::GaugeVec>;

impl TryFrom<&metrics::Key> for PrometheusGauge {
    type Error = prometheus::Error;

    fn try_from(key: &metrics::Key) -> Result<Self, Self::Error> {
        let mut labels_iter = key.labels();
        Ok(if let Some(first_label) = labels_iter.next() {
            let label_names = iter::once(first_label)
                .chain(labels_iter)
                .map(metrics::Label::key)
                .collect::<SmallVec<[_; 10]>>();
            Self::Vec(prometheus::GaugeVec::new(key.to(), &label_names)?)
        } else {
            Self::Single(prometheus::Gauge::with_opts(key.to())?)
        })
    }
}

/// [`Bundle`] of [`prometheus::Histogram`] metrics.
pub type PrometheusHistogram =
    Either<prometheus::Histogram, prometheus::HistogramVec>;

impl TryFrom<&metrics::Key> for PrometheusHistogram {
    type Error = prometheus::Error;

    fn try_from(key: &metrics::Key) -> Result<Self, Self::Error> {
        let mut labels_iter = key.labels();
        Ok(if let Some(first_label) = labels_iter.next() {
            let label_names = iter::once(first_label)
                .chain(labels_iter)
                .map(metrics::Label::key)
                .collect::<SmallVec<[_; 10]>>();
            Self::Vec(prometheus::HistogramVec::new(key.to(), &label_names)?)
        } else {
            Self::Single(prometheus::Histogram::with_opts(key.to())?)
        })
    }
}

/// Definitions of [`Bundle`] machinery.
pub mod bundle {
    use std::collections::HashMap;

    use sealed::sealed;

    /// Either a single [`prometheus::Metric`] or a [`prometheus::MetricVec`] of
    /// them, forming a [`Bundle`].
    ///
    /// [`prometheus::Metric`]: prometheus::core::Metric
    /// [`prometheus::MetricVec`]: prometheus::core::MetricVec
    #[derive(Clone, Copy, Debug)]
    pub enum Either<Single, Vec> {
        /// Single [`prometheus::Metric`].
        ///
        /// [`prometheus::Metric`]: prometheus::core::Metric
        Single(Single),

        /// [`prometheus::MetricVec`] of [`prometheus::Metric`]s.
        ///
        /// [`prometheus::Metric`]: prometheus::core::Metric
        /// [`prometheus::MetricVec`]: prometheus::core::MetricVec
        Vec(Vec),
    }

    #[warn(clippy::missing_trait_methods)]
    impl<S, V> prometheus::core::Collector for Either<S, V>
    where
        S: prometheus::core::Collector,
        V: prometheus::core::Collector,
    {
        fn desc(&self) -> Vec<&prometheus::core::Desc> {
            match self {
                Self::Single(m) => m.desc(),
                Self::Vec(v) => v.desc(),
            }
        }

        fn collect(&self) -> Vec<prometheus::proto::MetricFamily> {
            match self {
                Self::Single(m) => m.collect(),
                Self::Vec(v) => v.collect(),
            }
        }
    }

    /// [`prometheus::MetricVec`] of [`prometheus::Metric`]s.
    ///
    /// [`prometheus::Metric`]: prometheus::core::Metric
    /// [`prometheus::MetricVec`]: prometheus::core::MetricVec
    #[sealed]
    pub trait MetricVec {
        /// Type of [`prometheus::Metric`]s forming this [`MetricVec`].
        ///
        /// [`prometheus::Metric`]: prometheus::core::Metric
        type Metric: prometheus::core::Metric;

        /// Calls [`prometheus::MetricVec::get_metric_with()`][0] method of this
        /// [`MetricVec`].
        ///
        /// # Errors
        ///
        /// If a [`prometheus::Metric`] cannot be identified or created for the
        /// provided label `values`.
        ///
        /// [`prometheus::Metric`]: prometheus::core::Metric
        /// [0]: prometheus::core::MetricVec::get_metric_with()
        fn get_metric_with(
            &self,
            labels: &HashMap<&str, &str>,
        ) -> prometheus::Result<Self::Metric>;
    }

    #[sealed]
    impl<M, B> MetricVec for prometheus::core::MetricVec<B>
    where
        M: prometheus::core::Metric,
        B: prometheus::core::MetricVecBuilder<M = M>,
    {
        type Metric = M;

        fn get_metric_with(
            &self,
            labels: &HashMap<&str, &str>,
        ) -> prometheus::Result<M> {
            self.get_metric_with(labels)
        }
    }

    /// Bundle of a [`prometheus::Metric`]s family.
    ///
    /// [`Either`] a single [`prometheus::Metric`] or a
    /// [`prometheus::MetricVec`] of them.
    ///
    /// [`prometheus::Metric`]: prometheus::core::Metric
    /// [`prometheus::MetricVec`]: prometheus::core::MetricVec
    #[sealed]
    pub trait Bundle {
        /// Type of a single [`prometheus::Metric`] that may be stored in this
        /// [`Bundle`].
        ///
        /// [`prometheus::Metric`]: prometheus::core::Metric
        type Single: prometheus::core::Metric;

        /// Type of a [`prometheus::MetricVec`] that may be stored in this
        /// [`Bundle`].
        ///
        /// [`prometheus::MetricVec`]: prometheus::core::MetricVec
        type Vec: MetricVec<Metric = Self::Single>;

        /// Returns a single [`prometheus::Metric`] of this [`Bundle`],
        /// identified by the provided [`metrics::Key`].
        ///
        /// # Errors
        ///
        /// If the provided [`metrics::Key`] cannot identify any
        /// [`prometheus::Metric`] in this [`Bundle`].
        ///
        /// [`prometheus::Metric`]: prometheus::core::Metric
        fn get_single_metric(
            &self,
            key: &metrics::Key,
        ) -> prometheus::Result<Self::Single>;
    }

    #[sealed]
    impl<M, B> Bundle for Either<M, prometheus::core::MetricVec<B>>
    where
        M: prometheus::core::Metric + Clone,
        B: prometheus::core::MetricVecBuilder<M = M>,
    {
        type Single = M;
        type Vec = prometheus::core::MetricVec<B>;

        fn get_single_metric(
            &self,
            key: &metrics::Key,
        ) -> prometheus::Result<M> {
            match self {
                Self::Single(c) => {
                    if key.labels().next().is_some() {
                        return Err(
                            prometheus::Error::InconsistentCardinality {
                                expect: 0,
                                got: key.labels().count(),
                            },
                        );
                    }
                    Ok(c.clone())
                }
                Self::Vec(v) => {
                    let labels =
                        key.labels().map(|l| (l.key(), l.value())).collect();
                    v.get_metric_with(&labels)
                }
            }
        }
    }
}
