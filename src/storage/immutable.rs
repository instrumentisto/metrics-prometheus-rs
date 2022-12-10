use std::{borrow::Borrow, collections::HashMap, ops::Deref, sync::Arc};

use sealed::sealed;

use crate::{metric, Metric};

use super::KeyName;

pub type Collection<M> =
    HashMap<KeyName, prometheus::Result<metric::Describable<M>>>;

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

        if let Some(Ok(bundle)) = self.collection().get(name) {
            bundle.description.store(Arc::new(description));
        };
    }

    pub fn get_metric<'s, M>(
        &'s self,
        key: &metrics::Key,
    ) -> Option<Result<Metric<M>, MaybeOwned<'s, prometheus::Error>>>
    where
        M: metric::Bundled,
        <M as metric::Bundled>::Bundle: metric::Bundle<Single = M> + 's,
        Self: super::Get<Collection<<M as metric::Bundled>::Bundle>>,
    {
        use super::Get as _;
        use metric::Bundle as _;

        self.collection().get(key.name()).map(|res| {
            res.as_ref()
                .map_err(MaybeOwned::Borrowed)
                .and_then(|bundle| {
                    bundle
                        .metric
                        .get_single_metric(key)
                        .map_err(MaybeOwned::Owned)
                })
                .map(Metric::wrap)
        })
    }
}

pub enum MaybeOwned<'b, B> {
    Borrowed(&'b B),
    Owned(B),
}

impl<'b, B> Deref for MaybeOwned<'b, B> {
    type Target = B;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Borrowed(r) => *r,
            Self::Owned(v) => v,
        }
    }
}

impl<'b, B> AsRef<B> for MaybeOwned<'b, B> {
    fn as_ref(&self) -> &B {
        &**self
    }
}

impl<'b, B> Borrow<B> for MaybeOwned<'b, B> {
    fn borrow(&self) -> &B {
        &**self
    }
}
