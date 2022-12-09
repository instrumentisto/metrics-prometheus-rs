//! [`metrics::registry::Storage`] implementations.
//!
//! [`metrics::registry::Storage`]: metrics_util::registry::Storage

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use sealed::sealed;

use crate::{metric, Metric};

/// Thread-safe [`HashMap`] a [`Collection`] is built upon.
pub type Map<K, V> = Arc<RwLock<HashMap<K, V>>>;

/// Name identifying a [`metric::Bundle`] stored in a [`Mutable`] storage.
pub type KeyName = String;

/// [`Collection`] of [`Describable`] [`metric::Bundle`]s, stored in a
/// [`Mutable`] storage.
///
/// [`Describable`]: metric::Describable
pub type Collection<M> = Map<KeyName, metric::Describable<Option<M>>>;

/// Retrieving a [`Collection`] of `M`etrics from a [`Mutable`] storage.
#[sealed]
pub trait GetCollection<M> {
    /// Returns a [`Collection`] of `M`etrics stored in a [`Mutable`] storage.
    #[must_use]
    fn collection(&self) -> &Collection<M>;
}

/// [`metrics::registry::Storage`] backed by a [`prometheus::Registry`] and
/// allowing to change a [`help` description] of the registered [`prometheus`]
/// metrics in runtime.
///
/// This [`metrics::registry::Storage`] is capable of registering metrics in its
/// [`prometheus::Registry`] on the fly. By default, the
/// [`prometheus::default_registry()`] is used.
///
/// # Errors
///
/// This [`Mutable`] storage returns [`metric::Fallible`] in its
/// [`metrics::registry::Storage`] interface, because it cannot panic, as is
/// called inside [`metrics::Registry`] and, so, may poison its inner locks.
/// That's why possible errors are passed through up to the
/// [`metrics::Recorder`] using this [`Mutable`] storage, and should be resolved
/// there.
///
/// [`metrics::Registry`]: metrics_util::registry::Registry
/// [`metrics::registry::Storage`]: metrics_util::registry::Storage
/// [`help` description]: prometheus::proto::MetricFamily::get_help
#[derive(Clone, Debug)]
pub struct Mutable {
    /// [`prometheus::Registry`] backing this [`Mutable`] storage.
    pub(crate) prometheus: prometheus::Registry,

    /// [`Collection`] of [`prometheus::IntCounter`] metrics registered in this
    /// [`Mutable`] storage.
    counters: Collection<metric::PrometheusIntCounter>,

    /// [`Collection`] of [`prometheus::Gauge`] metrics registered in this
    /// [`Mutable`] storage.
    gauges: Collection<metric::PrometheusGauge>,

    /// [`Collection`] of [`prometheus::Histogram`] metrics registered in this
    /// [`Mutable`] storage.
    histograms: Collection<metric::PrometheusHistogram>,
}

#[sealed]
impl GetCollection<metric::PrometheusIntCounter> for Mutable {
    fn collection(&self) -> &Collection<metric::PrometheusIntCounter> {
        &self.counters
    }
}

#[sealed]
impl GetCollection<metric::PrometheusGauge> for Mutable {
    fn collection(&self) -> &Collection<metric::PrometheusGauge> {
        &self.gauges
    }
}

#[sealed]
impl GetCollection<metric::PrometheusHistogram> for Mutable {
    fn collection(&self) -> &Collection<metric::PrometheusHistogram> {
        &self.histograms
    }
}

impl Default for Mutable {
    fn default() -> Self {
        Self {
            prometheus: prometheus::default_registry().clone(),
            counters: Collection::default(),
            gauges: Collection::default(),
            histograms: Collection::default(),
        }
    }
}

impl Mutable {
    /// Changes the [`help` description] of the [`prometheus`] `M`etric
    /// identified by its `name`.
    ///
    /// Accepts only the following [`prometheus`] `M`etrics:
    /// - [`prometheus::IntCounter`], [`prometheus::IntCounterVec`]
    /// - [`prometheus::Gauge`], [`prometheus::GaugeVec`]
    /// - [`prometheus::Histogram`], [`prometheus::HistogramVec`]
    ///
    /// Intended to be used in [`metrics::Recorder::describe_counter()`],
    /// [`metrics::Recorder::describe_gauge()`] and
    /// [`metrics::Recorder::describe_histogram()`] implementations.
    ///
    /// [`help` description]: prometheus::proto::MetricFamily::get_help
    pub fn describe<M>(&self, name: &str, description: String)
    where
        M: metric::Bundled,
        <M as metric::Bundled>::Bundle: Clone,
        Self: GetCollection<<M as metric::Bundled>::Bundle>,
    {
        // PANIC: `RwLock` usage is fully panic-safe here.
        #![allow(
            clippy::missing_panics_doc,
            clippy::unwrap_in_result,
            clippy::unwrap_used
        )]

        // TODO: Just drop the guard, you idiot.
        // We do `.clone()` here intentionally to release `.read()` lock.
        let metric_opt = self.collection().read().unwrap().get(name).cloned();

        if let Some(metric) = metric_opt {
            metric.description.store(Arc::new(description));
        } else {
            let mut storage = self.collection().write().unwrap();

            if let Some(metric) = storage.get(name) {
                metric.description.store(Arc::new(description));
            } else {
                drop(storage.insert(
                    name.into(),
                    metric::Describable::only_description(description),
                ));
            }
        }
    }

    /// Initializes a new [`prometheus`] `M`etric (or reuses the existing one)
    /// in the underlying [`prometheus::Registry`], satisfying the labeling of
    /// the provided [`metrics::Key`] according to
    /// [`metrics::registry::Storage`] interface semantics, and returns it for
    /// use in a [`metrics::Registry`].
    ///
    /// # Errors
    ///
    /// If the underlying [`prometheus::Registry`] fails to register the newly
    /// initialized [`prometheus`] `M`etric according to the provided
    /// [`metrics::Key`].
    ///
    /// [`metrics::Registry`]: metrics_util::registry::Registry
    /// [`metrics::registry::Storage`]: metrics_util::registry::Storage
    fn register<'k, M>(
        &self,
        key: &'k metrics::Key,
    ) -> prometheus::Result<Arc<Metric<M>>>
    where
        M: metric::Bundled + prometheus::core::Metric + Clone,
        <M as metric::Bundled>::Bundle: metric::Bundle<Single = M>
            + prometheus::core::Collector
            + Clone
            + TryFrom<&'k metrics::Key, Error = prometheus::Error>
            + 'static,
        Self: GetCollection<<M as metric::Bundled>::Bundle>,
    {
        // PANIC: `RwLock` usage is panic-safe here (considering the
        //        `prometheus::Registry::register()` does not).
        #![allow(
            clippy::missing_panics_doc,
            clippy::unwrap_in_result,
            clippy::unwrap_used
        )]

        use metric::Bundle as _;

        let name = key.name();

        let mut bundle_opt = self
            .collection()
            .read()
            .unwrap()
            .get(name)
            .and_then(|m| m.metric.clone());

        let bundle = if let Some(bundle) = bundle_opt {
            bundle
        } else {
            let mut storage = self.collection().write().unwrap();

            bundle_opt = storage.get(name).and_then(|m| m.metric.clone());
            if let Some(bundle) = bundle_opt {
                bundle
            } else {
                let bundle: <M as metric::Bundled>::Bundle = key.try_into()?;

                // This way we reuse existing `description` if it has been set
                // before metric registration.
                let mut entry = storage.entry(name.into()).or_default();
                // We should register in `prometheus::Registry` before storing
                // in our `Collection`. This way `metrics::Recorder`
                // implementations using this `storage::Mutable` will be able to
                // retry registration in `prometheus::Registry`.
                // TODO: Re-register?
                self.prometheus.register(Box::new(
                    entry.clone().map(|_| bundle.clone()),
                ))?;
                entry.metric = Some(bundle.clone());

                bundle
            }
        };
        Ok(Arc::new(Metric::wrap(bundle.get_single_metric(key)?)))
    }

    /// Registers the provided [`prometheus`] `metric` in the underlying
    /// [`prometheus::Registry`] in the way making it usable via this
    /// [`metrics::registry::Storage`] (and, so, [`metrics`] crate interfaces).
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
    /// [`metrics::registry::Storage`]: metrics_util::registry::Storage
    pub fn register_external<M>(&self, metric: M) -> prometheus::Result<()>
    where
        M: metric::Bundled + prometheus::core::Collector,
        <M as metric::Bundled>::Bundle:
            prometheus::core::Collector + Clone + 'static,
        Self: GetCollection<<M as metric::Bundled>::Bundle>,
    {
        // PANIC: `RwLock` usage is panic-safe here (considering the
        //        `prometheus::Registry::register()` does not).
        #![allow(
            clippy::missing_panics_doc,
            clippy::unwrap_in_result,
            clippy::unwrap_used
        )]

        let name = metric
            .desc()
            .first()
            .map(|d| d.fq_name.clone())
            .unwrap_or_default();
        let entry = metric::Describable::wrap(Some(metric.into_bundle()));

        let mut storage = self.collection().write().unwrap();
        // We should register in `prometheus::Registry` before storing in our
        // `Collection`. This way `metrics::Recorder` implementations using this
        // `storage::Mutable` will be able to retry registration in
        // `prometheus::Registry`.
        // TODO: Re-register?
        self.prometheus
            .register(Box::new(entry.clone().map(Option::unwrap)))?;
        drop(storage.insert(name, entry));

        Ok(())
    }
}

impl metrics_util::registry::Storage<metrics::Key> for Mutable {
    // PANIC: We cannot panic inside `metrics_util::registry::Storage`
    //        implementation, because it will poison locks used inside
    //        `metrics_util::registry::Registry`. That's why we should pass
    //        possible errors through it and deal with them inside
    //        `metrics::Recorder` implementation.
    type Counter = metric::Fallible<prometheus::IntCounter>;
    type Gauge = metric::Fallible<prometheus::Gauge>;
    type Histogram = metric::Fallible<prometheus::Histogram>;

    fn counter(&self, key: &metrics::Key) -> Self::Counter {
        self.register::<prometheus::IntCounter>(key).into()
    }

    fn gauge(&self, key: &metrics::Key) -> Self::Gauge {
        self.register::<prometheus::Gauge>(key).into()
    }

    fn histogram(&self, key: &metrics::Key) -> Self::Histogram {
        self.register::<prometheus::Histogram>(key).into()
    }
}

pub struct Immutable {
    counters:
        HashMap<KeyName, prometheus::Result<metric::PrometheusIntCounter>>,
    //gauges: HashMap<KeyName, metric::PrometheusGauge>,
    //histogram: HashMap<KeyName, metric::PrometheusHistogram>,
}

impl Immutable {
    pub fn get_metric<M>(
        &self,
        key: &metrics::Key,
    ) -> Result<&Metric<<M as metric::Bundled>::Bundle>, &prometheus::Error>
    where
        M: metric::Bundled,
    {
        todo!()
    }
}
