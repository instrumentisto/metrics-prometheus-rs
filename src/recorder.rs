use std::{borrow::Cow, fmt, sync::Arc};

use crate::{
    failure::{self, strategy::PanicInDebugNoOpInRelease},
    metric, storage,
};

#[derive(Clone)]
pub struct Recorder<FailureStrategy = PanicInDebugNoOpInRelease> {
    metrics: Arc<metrics_util::registry::Registry<metrics::Key, storage::Mutable>>,
    storage: storage::Mutable,
    failure_strategy: FailureStrategy,
}

impl<S: fmt::Debug> fmt::Debug for Recorder<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Recorder")
            .field("storage", &self.storage)
            .field("failure_strategy", &self.failure_strategy)
            .finish_non_exhaustive()
    }
}

impl<S> Recorder<S> {
    pub fn new() -> Builder<S>
    where
        S: failure::Strategy + Default,
    {
        Builder {
            storage: storage::Mutable::default(),
            failure_strategy: S::default(),
        }
    }

    pub fn registry(&self) -> &prometheus::Registry {
        &self.storage.prometheus
    }

    pub fn register<M>(&self, metric: M) -> prometheus::Result<()>
    where
        M: metric::Bundled + prometheus::core::Collector,
        <M as metric::Bundled>::Bundle: prometheus::core::Collector + Clone + 'static,
        storage::Mutable: storage::GetCollection<<M as metric::Bundled>::Bundle>,
    {
        self.storage.register_external(metric)
    }

    pub fn must_register<M>(&self, metric: M)
    where
        M: metric::Bundled + prometheus::core::Collector,
        <M as metric::Bundled>::Bundle: prometheus::core::Collector + Clone + 'static,
        storage::Mutable: storage::GetCollection<<M as metric::Bundled>::Bundle>,
    {
        self.register(metric)
            .unwrap_or_else(|e| panic!("failed to register `prometheus` metric: {e}"))
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
        self.storage
            .describe::<prometheus::IntCounter>(name.as_str(), description.into_owned())
    }

    fn describe_gauge(
        &self,
        name: metrics::KeyName,
        _: Option<metrics::Unit>,
        description: metrics::SharedString,
    ) {
        self.storage
            .describe::<prometheus::Gauge>(name.as_str(), description.into_owned())
    }

    fn describe_histogram(
        &self,
        name: metrics::KeyName,
        _: Option<metrics::Unit>,
        description: metrics::SharedString,
    ) {
        self.storage
            .describe::<prometheus::Histogram>(name.as_str(), description.into_owned())
    }

    fn register_counter(&self, key: &metrics::Key) -> metrics::Counter {
        self.metrics
            .get_or_create_counter(key, |counter| {
                counter
                    .as_ref()
                    .map(|c| Arc::clone(&c).into())
                    .or_else(|e| match self.failure_strategy.decide(e) {
                        failure::Action::NoOp => Ok(metrics::Counter::noop()),
                        // PANIC: We cannot panic inside this closure, because
                        //        this may lead to poisoning `RwLock`s inside
                        //        `metrics_util::registry::Registry`.
                        failure::Action::Panic => Err(e.to_string()),
                    })
            })
            .unwrap_or_else(|e| panic!("failed to register `prometheus::IntCounter` metric: {e}"))
    }

    fn register_gauge(&self, key: &metrics::Key) -> metrics::Gauge {
        self.metrics
            .get_or_create_gauge(key, |gauge| {
                gauge.as_ref().map(|c| Arc::clone(&c).into()).or_else(|e| {
                    match self.failure_strategy.decide(e) {
                        failure::Action::NoOp => Ok(metrics::Gauge::noop()),
                        // PANIC: We cannot panic inside this closure, because
                        //        this may lead to poisoning `RwLock`s inside
                        //        `metrics_util::registry::Registry`.
                        failure::Action::Panic => Err(e.to_string()),
                    }
                })
            })
            .unwrap_or_else(|e| panic!("failed to register `prometheus::Gauge` metric: {e}"))
    }

    fn register_histogram(&self, key: &metrics::Key) -> metrics::Histogram {
        self.metrics
            .get_or_create_histogram(key, |histogram| {
                histogram
                    .as_ref()
                    .map(|c| Arc::clone(&c).into())
                    .or_else(|e| match self.failure_strategy.decide(e) {
                        failure::Action::NoOp => Ok(metrics::Histogram::noop()),
                        // PANIC: We cannot panic inside this closure, because
                        //        this may lead to poisoning `RwLock`s inside
                        //        `metrics_util::registry::Registry`.
                        failure::Action::Panic => Err(e.to_string()),
                    })
            })
            .unwrap_or_else(|e| panic!("failed to register `prometheus::Histogram` metric: {e}"))
    }
}

#[derive(Clone)]
pub struct Builder<FailureStrategy = PanicInDebugNoOpInRelease> {
    storage: storage::Mutable,
    failure_strategy: FailureStrategy,
}

impl<S> Builder<S> {
    pub fn with_registry<'r>(mut self, registry: impl Into<Cow<'r, prometheus::Registry>>) -> Self {
        self.storage.prometheus = registry.into().into_owned();
        self
    }

    pub fn with_failure_strategy<F>(self, strategy: F) -> Builder<F>
    where
        F: failure::Strategy,
    {
        Builder {
            storage: self.storage,
            failure_strategy: strategy,
        }
    }

    pub fn with_metric<M>(self, metric: M) -> prometheus::Result<Self>
    where
        M: metric::Bundled + prometheus::core::Collector,
        <M as metric::Bundled>::Bundle: prometheus::core::Collector + Clone + 'static,
        storage::Mutable: storage::GetCollection<<M as metric::Bundled>::Bundle>,
    {
        self.storage.register_external(metric)?;
        Ok(self)
    }

    pub fn with_must_metric<M>(self, metric: M) -> Self
    where
        M: metric::Bundled + prometheus::core::Collector,
        <M as metric::Bundled>::Bundle: prometheus::core::Collector + Clone + 'static,
        storage::Mutable: storage::GetCollection<<M as metric::Bundled>::Bundle>,
    {
        self.with_metric(metric)
            .unwrap_or_else(|e| panic!("failed to register `prometheus` metric: {e}"))
    }

    pub fn register(self) -> Result<Recorder<S>, metrics::SetRecorderError>
    where
        S: failure::Strategy + Clone + 'static,
    {
        let Self {
            storage,
            failure_strategy,
        } = self;
        let rec = Recorder {
            metrics: Arc::new(metrics_util::registry::Registry::new(storage.clone())),
            storage,
            failure_strategy,
        };
        metrics::set_boxed_recorder(Box::new(rec.clone()))?;
        Ok(rec)
    }
}
