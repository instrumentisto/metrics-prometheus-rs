use std::{collections::HashMap, sync::Arc};

use sealed::sealed;

use crate::{metric, Metric};

use super::KeyName;

pub type Collection<M> = HashMap<KeyName, metric::Describable<M>>;

#[derive(Debug)]
pub struct Storage {
    counters: Collection<metric::PrometheusIntCounter>,
    gauges: Collection<metric::PrometheusGauge>,
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
    pub fn describe<M>(&self, name: &str, description: String)
    where
        M: metric::Bundled,
        <M as metric::Bundled>::Bundle: metric::Bundle<Single = M>,
        Self: super::Get<Collection<<M as metric::Bundled>::Bundle>>,
    {
        use super::Get as _;

        if let Some(bundle) = self.collection().get(name) {
            bundle.description.store(Arc::new(description));
        };
    }

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

impl From<&super::mutable::Storage> for Storage {
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
