use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use sealed::sealed;

use crate::{metric, Metric};

pub type Map<K, V> = Arc<RwLock<HashMap<K, V>>>;

pub type KeyName = String;

type Collection<M> = Map<KeyName, metric::Describable<Option<M>>>;

#[sealed]
pub trait GetCollection<M> {
    fn collection(&self) -> &Collection<M>;
}

#[derive(Clone, Debug)]
pub struct Mutable {
    pub(crate) prometheus: prometheus::Registry,
    counters: Collection<metric::PrometheusIntCounter>,
    gauges: Collection<metric::PrometheusGauge>,
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
            prometheus: prometheus::default_registry().to_owned(),
            counters: Collection::default(),
            gauges: Collection::default(),
            histograms: Collection::default(),
        }
    }
}

impl Mutable {
    pub fn describe<M>(&self, name: &str, description: String)
    where
        M: metric::Bundled,
        Self: GetCollection<<M as metric::Bundled>::Bundle>,
    {
        if let Some(metric) = self.collection().read().unwrap().get(name) {
            metric.description.store(Arc::new(description));
        } else {
            let mut storage = self.collection().write().unwrap();

            if let Some(metric) = storage.get(name) {
                metric.description.store(Arc::new(description));
            } else {
                storage.insert(
                    name.into(),
                    metric::Describable::only_description(description),
                );
            }
        }
    }

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
        use metric::Bundle as _;

        let name = key.name();

        let bundle_opt = self
            .collection()
            .read()
            .unwrap()
            .get(name)
            .and_then(|m| m.metric.clone());

        let bundle = if let Some(bundle) = bundle_opt {
            bundle
        } else {
            let mut storage = self.collection().write().unwrap();

            let bundle_opt = storage.get(name).and_then(|m| m.metric.clone());
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
        Ok(Arc::new(Metric::new(bundle.get_single_metric(key)?)))
    }

    pub fn register_external<M>(&self, metric: M) -> prometheus::Result<()>
    where
        M: metric::Bundled + prometheus::core::Collector,
        <M as metric::Bundled>::Bundle:
            prometheus::core::Collector + Clone + 'static,
        Self: GetCollection<<M as metric::Bundled>::Bundle>,
    {
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
        let _ = storage.insert(name, entry);

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
