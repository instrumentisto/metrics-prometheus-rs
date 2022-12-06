use std::{iter, sync::Arc};

use arc_swap::ArcSwap;
use sealed::sealed;
use smallvec::SmallVec;

use self::bundle::Either;

pub use self::bundle::Bundle;

#[derive(Clone, Copy, Debug)]
pub struct Metric<M>(M);

impl<M> Metric<M> {
    pub fn new(metric: M) -> Self {
        Self(metric)
    }

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

impl metrics::CounterFn for Metric<prometheus::IntCounter> {
    fn increment(&self, val: u64) {
        self.0.inc_by(val);
    }

    fn absolute(&self, val: u64) {
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
        self.0.inc_by(val);
    }
}

impl metrics::GaugeFn for Metric<prometheus::Gauge> {
    fn increment(&self, val: f64) {
        self.0.add(val);
    }

    fn decrement(&self, val: f64) {
        self.0.sub(val);
    }

    fn set(&self, val: f64) {
        self.0.set(val);
    }
}

impl metrics::HistogramFn for Metric<prometheus::Histogram> {
    fn record(&self, val: f64) {
        self.0.observe(val);
    }
}

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
    pub fn as_ref(&self) -> Result<&Arc<Metric<M>>, &prometheus::Error> {
        (*self.0).as_ref()
    }
}

impl<M> metrics::CounterFn for Fallible<M>
where
    Metric<M>: metrics::CounterFn,
{
    fn increment(&self, val: u64) {
        if let Ok(m) = &*self.0 {
            m.increment(val)
        }
    }

    fn absolute(&self, val: u64) {
        if let Ok(m) = &*self.0 {
            m.absolute(val)
        }
    }
}

impl<M> metrics::GaugeFn for Fallible<M>
where
    Metric<M>: metrics::GaugeFn,
{
    fn increment(&self, val: f64) {
        if let Ok(m) = &*self.0 {
            m.increment(val)
        }
    }

    fn decrement(&self, val: f64) {
        if let Ok(m) = &*self.0 {
            m.decrement(val)
        }
    }

    fn set(&self, val: f64) {
        if let Ok(m) = &*self.0 {
            m.set(val)
        }
    }
}

impl<M> metrics::HistogramFn for Fallible<M>
where
    Metric<M>: metrics::HistogramFn,
{
    fn record(&self, val: f64) {
        if let Ok(m) = &*self.0 {
            m.record(val)
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Describable<Metric> {
    pub(crate) description: Arc<ArcSwap<String>>,
    pub(crate) metric: Metric,
}

impl<M> Describable<M> {
    pub fn wrap(metric: M) -> Self {
        Self { description: Default::default(), metric }
    }

    pub fn only_description(desc: impl Into<String>) -> Self
    where
        M: Default,
    {
        Self {
            description: Arc::new(ArcSwap::new(Arc::new(desc.into()))),
            metric: M::default(),
        }
    }

    pub fn map<Into>(self, into: impl FnOnce(M) -> Into) -> Describable<Into> {
        Describable { description: self.description, metric: into(self.metric) }
    }
}

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

trait To<T> {
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

#[sealed]
pub trait Bundled {
    type Bundle: Bundle;

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

pub mod bundle {
    use sealed::sealed;
    use smallvec::SmallVec;

    #[derive(Clone, Copy, Debug)]
    pub enum Either<Single, Vec> {
        Single(Single),
        Vec(Vec),
    }

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

    #[sealed]
    pub trait MetricVec {
        type Metric: prometheus::core::Metric;

        fn get_metric_with_label_values(
            &self,
            values: &[&str],
        ) -> prometheus::Result<Self::Metric>;
    }

    #[sealed]
    impl<M, B> MetricVec for prometheus::core::MetricVec<B>
    where
        M: prometheus::core::Metric,
        B: prometheus::core::MetricVecBuilder<M = M>,
    {
        type Metric = M;

        fn get_metric_with_label_values(
            &self,
            vals: &[&str],
        ) -> prometheus::Result<M> {
            self.get_metric_with_label_values(vals)
        }
    }

    #[sealed]
    pub trait Bundle {
        type Single: prometheus::core::Metric;
        type Vec: MetricVec<Metric = Self::Single>;

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
                Self::Single(c) => Ok(c.clone()),
                Self::Vec(v) => {
                    let labels = key
                        .labels()
                        .map(metrics::Label::value)
                        .collect::<SmallVec<[_; 10]>>();
                    v.get_metric_with_label_values(&labels)
                }
            }
        }
    }
}
