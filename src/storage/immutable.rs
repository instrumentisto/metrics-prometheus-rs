//! Immutable storage of [`metric::Describable`].

use std::{collections::HashMap, sync::Arc};

use sealed::sealed;

use crate::{metric, Metric};

use super::KeyName;

/// Collection of [`Describable`] [`metric::Bundle`]s, stored in an immutable
/// [`Storage`].
///
/// [`Describable`]: metric::Describable
pub type Collection<M> = HashMap<KeyName, metric::Describable<M>>;

/// Snapshot of a [`mutable::Storage`], that is not capable of registering
/// metrics in a [`prometheus::Registry`] on the fly, but still allowing to
/// change their [`help` description] on the fly, once they were registered in
/// the [`mutable::Storage`] before snapshot.
///
/// In comparison with a [`metrics::Registry`], this immutable [`Storage`]
/// provides much less overhead of accessing an already registered metric (just
/// a simple [`HashMap`] lookup), however, is not capable of registering new
/// metrics on the fly.
///
/// [`metrics::Registry`]: metrics_util::registry::Registry
/// [`mutable::Storage`]: super::Mutable
/// [`help` description]: prometheus::proto::MetricFamily::get_help
#[derive(Debug)]
pub struct Storage {
    /// [`Collection`] of [`prometheus::IntCounter`] metrics registered in this
    /// immutable [`Storage`].
    counters: Collection<metric::PrometheusIntCounter>,

    /// [`Collection`] of [`prometheus::Gauge`] metrics registered in this
    /// immutable [`Storage`].
    gauges: Collection<metric::PrometheusGauge>,

    /// [`Collection`] of [`prometheus::Histogram`] metrics registered in this
    /// immutable [`Storage`].
    histograms: Collection<metric::PrometheusHistogram>,
}

#[sealed]
impl super::Get<Collection<metric::PrometheusIntCounter>> for Storage {
    fn collection(&self) -> &Collection<metric::PrometheusIntCounter> {
        &self.counters
    }
}

#[sealed]
impl super::Get<Collection<metric::PrometheusGauge>> for Storage {
    fn collection(&self) -> &Collection<metric::PrometheusGauge> {
        &self.gauges
    }
}

#[sealed]
impl super::Get<Collection<metric::PrometheusHistogram>> for Storage {
    fn collection(&self) -> &Collection<metric::PrometheusHistogram> {
        &self.histograms
    }
}

impl Storage {
    /// Changes the [`help` description] of the [`prometheus`] `M`etric
    /// identified by its `name`. No-op if this immutable [`Storage`] doesn't
    /// contain it.
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
        <M as metric::Bundled>::Bundle: metric::Bundle,
        Self: super::Get<Collection<<M as metric::Bundled>::Bundle>>,
    {
        use super::Get as _;

        if let Some(bundle) = self.collection().get(name) {
            bundle.description.store(Arc::new(description));
        };
    }

    /// Returns a [`prometheus`] `M`etric stored in this immutable [`Storage`]
    /// and identified by the provided [`metrics::Key`].
    ///
    /// Accepts only the following [`prometheus`] metrics:
    /// - [`prometheus::IntCounter`]
    /// - [`prometheus::Gauge`]
    /// - [`prometheus::Histogram`]
    ///
    /// Intended to be used in [`metrics::Recorder::register_counter()`],
    /// [`metrics::Recorder::register_gauge()`] and
    /// [`metrics::Recorder::register_histogram()`] implementations.
    ///
    /// # Errors
    ///
    /// If the identified [`prometheus`] `M`etric doesn't comply with the
    /// labeling of the provided [`metrics::Key`].
    pub fn get_metric<M>(
        &self,
        key: &metrics::Key,
    ) -> Option<Result<Metric<M>, prometheus::Error>>
    where
        M: metric::Bundled,
        <M as metric::Bundled>::Bundle: metric::Bundle<Single = M>,
        Self: super::Get<Collection<<M as metric::Bundled>::Bundle>>,
    {
        use super::Get as _;
        use metric::Bundle as _;

        self.collection().get(key.name()).map(|bundle| {
            bundle.metric.get_single_metric(key).map(Metric::wrap)
        })
    }
}

#[expect( // intentional
    clippy::fallible_impl_from,
    reason = "`RwLock` usage is fully panic-safe inside, so the `impl` is \
              infallible, in fact"
)]
impl From<&super::mutable::Storage> for Storage {
    /// Creates a new immutable [`Storage`] by [draining] the referred
    /// [`mutable::Storage`] and leaving it empty.
    ///
    /// [`mutable::Storage`]: super::mutable::Storage
    /// [draining]: HashMap::drain
    #[expect( // intentional
        clippy::unwrap_used,
        reason = "`RwLock` usage is fully panic-safe here"
    )]
    fn from(mutable: &super::mutable::Storage) -> Self {
        Self {
            counters: mutable
                .counters
                .write()
                .unwrap()
                .drain()
                .filter_map(|(name, bundle)| Some((name, bundle.transpose()?)))
                .collect(),
            gauges: mutable
                .gauges
                .write()
                .unwrap()
                .drain()
                .filter_map(|(name, bundle)| Some((name, bundle.transpose()?)))
                .collect(),
            histograms: mutable
                .histograms
                .write()
                .unwrap()
                .drain()
                .filter_map(|(name, bundle)| Some((name, bundle.transpose()?)))
                .collect(),
        }
    }
}
