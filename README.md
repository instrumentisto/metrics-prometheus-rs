<img align="right" width="200" src="https://raw.githubusercontent.com/instrumentisto/metrics-prometheus-rs/80bcffc2096f9ff213ec84833a9d8dd81a115cd5/logo.png">

[`metrics`] + [`prometheus`] = ❤️
=================================

[![crates.io](https://img.shields.io/crates/v/metrics-prometheus.svg "crates.io")](https://crates.io/crates/metrics-prometheus)
[![Rust 1.85+](https://img.shields.io/badge/rustc-1.85+-lightgray.svg "Rust 1.85+")](https://blog.rust-lang.org/2025/02/20/Rust-1.85.0.html)
[![Unsafe Forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg "Unsafe forbidden")](https://github.com/rust-secure-code/safety-dance)\
[![CI](https://github.com/instrumentisto/metrics-prometheus-rs/actions/workflows/ci.yml/badge.svg?branch=main "CI")](https://github.com/instrumentisto/metrics-prometheus-rs/actions?query=workflow%3ACI+branch%3Amain)
[![Rust docs](https://docs.rs/metrics-prometheus/badge.svg "Rust docs")](https://docs.rs/metrics-prometheus)

[API Docs](https://docs.rs/metrics-prometheus) |
[Changelog](https://github.com/instrumentisto/metrics-prometheus-rs/blob/v0.11.0/CHANGELOG.md)

[`prometheus`] backend for [`metrics`] crate.




## Motivation

[Rust] has at least two ecosystems regarding metrics collection:
- One is based on the [`prometheus`] crate, focusing on delivering metrics to [Prometheus] (or its drop-in replacements like [VictoriaMetrics]). It provides a lot of [Prometheus]-specific capabilities and validates metrics strictly to meet the format used by [Prometheus].
- Another one is based on the [`metrics`] crate, being more generic and targeting a wider scope, rather than [Prometheus] only. It provides a convenient and ergonomic facade, allowing to work with metrics in the very similar way we do work with logs and traces via [`log`]/[`tracing`] ecosystems (and even [supports `tracing::Span`s for metrics labels][`metrics-tracing-context`]).

As the result, some crates use [`prometheus`] crate for providing their metrics, and another crates do use [`metrics`] crate for that. Furthermore, [`prometheus`] and [`metrics`] crates are designed quite differently, making their composition a non-trivial task. This crate aims to mitigate this gap, allowing to combine both [`prometheus`] and [`metrics`] ecosystems in a single project.


### Alternatives

If you're not obligated to deal with [`prometheus`] crate directly or via third-party crates which do use it, consider the [`metrics-exporter-prometheus`] crate, which provides a simple [Prometheus] backend for [`metrics`] facade, without bringing in the whole [`prometheus`] crate's machinery.




## Overview

This crate provides a [`metrics::Recorder`] implementation, allowing to work with a [`prometheus::Registry`] via [`metrics`] facade.

It comes in 3 flavours, allowing to choose the smallest performance overhead depending on a use case:
- Regular [`Recorder`], allowing to create new metrics via [`metrics`] facade anytime, without limits. Provides the same overhead of accessing an already registered metric as a [`metrics::Registry`] does: [`read`-lock] on a sharded [`HashMap`] plus [`Arc`] cloning.
- [`FrozenRecorder`], unable to create new metrics via [`metrics`] facade at all (just no-op in such case). Provides the smallest overhead of accessing an already registered metric: just a regular [`HashMap`] lookup plus [`Arc`] cloning.
- [`FreezableRecorder`], acting the same way as the [`Recorder`] at first, but being able to [`.freeze()`] and so, becoming a [`FrozenRecorder`] at the end. The overhead of accessing an already registered metric is the same as [`Recorder`] and [`FrozenRecorder`] provide, plus [`AtomicBool`] loading to check whether it has been [`.freeze()`]d.

Not any [`prometheus`] metric is supported, because [`metrics`] crate implies only few of them. This is how the [`metrics`] crate's metrics are mapped onto [`prometheus`] ones:
- [`metrics::Counter`]: [`prometheus::IntCounter`] + [`prometheus::IntCounterVec`]
- [`metrics::Gauge`]: [`prometheus::Gauge`] + [`prometheus::GaugeVec`]
- [`metrics::Histogram`]: [`prometheus::Histogram`] + [`prometheus::HistogramVec`]

[`prometheus::MetricVec`] types are used whenever any labels are specified via [`metrics`] facade.

To satisfy the [`metrics::Recorder`]'s requirement of allowing changing metrics description anytime after its registration ([`prometheus`] crate doesn't imply and allow that), the [`Describable`] wrapper is used, allowing to [`arc-swap`] the description.

```rust
// By default `prometheus::default_registry()` is used.
let recorder = metrics_prometheus::install();

// Either use `metrics` crate interfaces.
metrics::counter!("count", "whose" => "mine", "kind" => "owned").increment(1);
metrics::counter!("count", "whose" => "mine", "kind" => "ref").increment(1);
metrics::counter!("count", "kind" => "owned", "whose" => "dummy").increment(1);

// Or construct and provide `prometheus` metrics directly.
recorder.register_metric(prometheus::Gauge::new("value", "help")?);

let report = prometheus::TextEncoder::new()
    .encode_to_string(&prometheus::default_registry().gather())?;
assert_eq!(
    report.trim(),
    r#"
## HELP count count
## TYPE count counter
count{kind="owned",whose="dummy"} 1
count{kind="owned",whose="mine"} 1
count{kind="ref",whose="mine"} 1
## HELP value help
## TYPE value gauge
value 0
    "#
    .trim(),
);

// Metrics can be described anytime after being registered in
// `prometheus::Registry`.
metrics::describe_counter!("count", "Example of counter.");
metrics::describe_gauge!("value", "Example of gauge.");

let report = prometheus::TextEncoder::new()
    .encode_to_string(&recorder.registry().gather())?;
assert_eq!(
    report.trim(),
    r#"
## HELP count Example of counter.
## TYPE count counter
count{kind="owned",whose="dummy"} 1
count{kind="owned",whose="mine"} 1
count{kind="ref",whose="mine"} 1
## HELP value Example of gauge.
## TYPE value gauge
value 0
    "#
    .trim(),
);

// Description can be changed multiple times and anytime.
metrics::describe_counter!("count", "Another description.");

// Even before a metric is registered in `prometheus::Registry`.
metrics::describe_counter!("another", "Yet another counter.");
metrics::counter!("another").increment(1);

let report = prometheus::TextEncoder::new()
    .encode_to_string(&recorder.registry().gather())?;
assert_eq!(
    report.trim(),
    r#"
## HELP another Yet another counter.
## TYPE another counter
another 1
## HELP count Another description.
## TYPE count counter
count{kind="owned",whose="dummy"} 1
count{kind="owned",whose="mine"} 1
count{kind="ref",whose="mine"} 1
## HELP value Example of gauge.
## TYPE value gauge
value 0
    "#
    .trim(),
);
# Ok::<_, prometheus::Error>(())
```


### Limitations

Since [`prometheus`] crate validates the metrics format very strictly, not everything, expressed via [`metrics`] facade, may be put into a [`prometheus::Registry`], ending up with a [`prometheus::Error`] being emitted.

- Metric names cannot be namespaced with dots (and should follow [Prometheus] format).
  ```rust,should_panic
  metrics_prometheus::install();

  // panics: 'queries.count' is not a valid metric name
  metrics::counter!("queries.count").increment(1);
  ```

- The same metric should use always the same set of labels:
  ```rust,should_panic
  metrics_prometheus::install();

  metrics::counter!("count").increment(1);
  // panics: Inconsistent label cardinality, expect 0 label values, but got 1
  metrics::counter!("count", "whose" => "mine").increment(1);
  ```
  ```rust,should_panic
  metrics_prometheus::install();

  metrics::counter!("count", "kind" => "owned").increment(1);
  // panics: label name kind missing in label map
  metrics::counter!("count", "whose" => "mine").increment(1);
  ```
  ```rust,should_panic
  metrics_prometheus::install();

  metrics::counter!("count", "kind" => "owned").increment(1);
  // panics: Inconsistent label cardinality, expect 1 label values, but got 2
  metrics::counter!("count", "kind" => "ref", "whose" => "mine").increment(1);
  ```

- The same name cannot be used for different types of metrics:
  ```rust,should_panic
  metrics_prometheus::install();

  metrics::counter!("count").increment(1);
  // panics: Duplicate metrics collector registration attempted
  metrics::gauge!("count").increment(1.0);
  ```

- Any metric registered in a [`prometheus::Registry`] directly, without using [`metrics`] or this crate interfaces, is not usable via [`metrics`] facade and will cause a [`prometheus::Error`].
  ```rust,should_panic
  metrics_prometheus::install();

  prometheus::default_registry()
      .register(Box::new(prometheus::Gauge::new("value", "help")?))?;

  // panics: Duplicate metrics collector registration attempted
  metrics::gauge!("value").increment(4.5);
  # Ok::<_, prometheus::Error>(())
  ```

- [`metrics::Unit`]s are not supported, as [Prometheus] has no notion of ones. Specifying them via [`metrics`] macros will be no-op.


### [`prometheus::Error`] handling

Since [`metrics::Recorder`] doesn't expose any errors in its API, the emitted [`prometheus::Error`]s can be either turned into a panic, or just silently ignored, returning a no-op metric instead (see [`metrics::Counter::noop()`] for example).

This can be tuned by providing a [`failure::Strategy`] when building a [`Recorder`].

```rust
use metrics_prometheus::failure::strategy;

metrics_prometheus::Recorder::builder()
    .with_failure_strategy(strategy::NoOp)
    .build_and_install();

// `prometheus::Error` is ignored inside.
metrics::counter!("invalid.name").increment(1);

let stats = prometheus::default_registry().gather();
assert_eq!(stats.len(), 0);
```

The default [`failure::Strategy`] is [`PanicInDebugNoOpInRelease`]. See [`failure::strategy`] module for other available [`failure::Strategy`]s,
or provide your own one by implementing the [`failure::Strategy`] trait.




## License

Copyright © 2022-2025 Instrumentisto Team, <https://github.com/instrumentisto>

Licensed under either of [Apache License, Version 2.0][APACHE] or [MIT license][MIT] at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you, as defined in the [Apache-2.0 license][APACHE], shall be dual licensed as above, without any additional terms or conditions.




[`.freeze()`]: https://docs.rs/metrics-prometheus/latest/metrics_prometheus/struct.FreezableRecorder.html#method.freeze
[`Arc`]: https://doc.rust-lang.org/stable/std/sync/struct.Arc.html
[`arc-swap`]: https://docs.rs/arc-swap
[`AtomicBool`]: https://doc.rust-lang.org/stable/std/sync/atomic/struct.AtomicBool.html
[`Describable`]: https://docs.rs/metrics-prometheus/latest/metrics_prometheus/metric/struct.Describable.html
[`failure::strategy`]: https://docs.rs/metrics-prometheus/latest/metrics_prometheus/failure/strategy/index.html
[`failure::Strategy`]: https://docs.rs/metrics-prometheus/latest/metrics_prometheus/failure/trait.Strategy.html
[`FreezableRecorder`]: https://docs.rs/metrics-prometheus/latest/metrics_prometheus/struct.FreezableRecorder.html
[`FrozenRecorder`]: https://docs.rs/metrics-prometheus/latest/metrics_prometheus/struct.FrozenRecorder.html
[`HashMap`]: https://doc.rust-lang.org/stable/std/collections/struct.HashMap.html
[`log`]: https://docs.rs/log
[`metrics`]: https://docs.rs/metrics
[`metrics::Counter`]: https://docs.rs/metrics/latest/metrics/struct.Counter.html
[`metrics::Counter::noop()`]: https://docs.rs/metrics/latest/metrics/struct.Counter.html#method.noop
[`metrics::Gauge`]: https://docs.rs/metrics/latest/metrics/struct.Gauge.html
[`metrics::Histogram`]: https://docs.rs/metrics/latest/metrics/struct.Histogram.html
[`metrics::Recorder`]: https://docs.rs/metrics/latest/metrics/trait.Recorder.html
[`metrics::Registry`]: https://docs.rs/metrics-util/latest/metrics_util/registry/struct.Registry.html
[`metrics::Unit`]: https://docs.rs/metrics/latest/metrics/enum.Unit.html
[`metrics-exporter-prometheus`]: https://docs.rs/metrics-exporter-prometheus
[`metrics-tracing-context`]: https://docs.rs/metrics-tracing-context
[`PanicInDebugNoOpInRelease`]: https://docs.rs/metrics-prometheus/latest/metrics_prometheus/failure/strategy/struct.PanicInDebugNoOpInRelease.html
[`prometheus`]: https://docs.rs/prometheus
[`prometheus::Error`]: https://docs.rs/prometheus/latest/prometheus/enum.Error.html
[`prometheus::Gauge`]: https://docs.rs/prometheus/latest/prometheus/type.Gauge.html
[`prometheus::GaugeVec`]: https://docs.rs/prometheus/latest/prometheus/type.GaugeVec.html
[`prometheus::Histogram`]: https://docs.rs/prometheus/latest/prometheus/struct.Histogram.html
[`prometheus::HistogramVec`]: https://docs.rs/prometheus/latest/prometheus/type.HistogramVec.html
[`prometheus::IntCounter`]: https://docs.rs/prometheus/latest/prometheus/type.IntCounter.html
[`prometheus::IntCounterVec`]: https://docs.rs/prometheus/latest/prometheus/type.IntCounterVec.html
[`prometheus::MetricVec`]: https://docs.rs/prometheus/latest/prometheus/core/struct.MetricVec.html
[`prometheus::Registry`]: https://docs.rs/prometheus/latest/prometheus/struct.Registry.html
[`read`-lock]: https://doc.rust-lang.org/stable/std/sync/struct.RwLock.html#method.read
[`Recorder`]: https://docs.rs/metrics-prometheus/latest/metrics_prometheus/struct.Recorder.html
[`tracing`]: https://docs.rs/tracing
[Prometheus]: https://prometheus.io
[Rust]: https://www.rust-lang.org
[VictoriaMetrics]: https://victoriametrics.com

[APACHE]: https://github.com/instrumentisto/metrics-prometheus-rs/blob/v0.11.0/LICENSE-APACHE
[MIT]: https://github.com/instrumentisto/metrics-prometheus-rs/blob/v0.11.0/LICENSE-MIT
