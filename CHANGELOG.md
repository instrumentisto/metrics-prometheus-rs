`metrics-prometheus` changelog
==============================

All user visible changes to this project will be documented in this file. This project uses [Semantic Versioning 2.0.0].




## [0.2.0] · 2022-12-??
[0.2.0]: /../../tree/v0.2.0

### Initially implemented

- `storage::Mutable` implementation of `metrics_util::registry::Storage` backed by `prometheus::Registry` and allowing to change `help` description of already registered metrics. ([6a6d4eae])
- `Recorder` implementation of `metrics::Recorder` allowing metrics creation on the fly. ([6a6d4eae])
- `NoOp`, `Panic` and `PanicInDebugNoOpInRelease` (default) `failure::Strategy`s to handle possible `prometheus::Error`s. ([6a6d4eae])

[6a6d4eae]: /../../commit/6a6d4eaefaf6a89a9f26c4d28b440fb671cec75a




[Semantic Versioning 2.0.0]: https://semver.org