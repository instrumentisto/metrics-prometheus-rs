`metrics-prometheus` changelog
==============================

All user visible changes to this project will be documented in this file. This project uses [Semantic Versioning 2.0.0].




## [0.3.0] · 2022-12-11
[0.3.0]: /../../tree/v0.3.0

[Diff](/../../compare/v0.2.0...v0.3.0)

### BC Breaks

- Switched functions naming convention from `must_*` for panicking to `try_*` for fallible. ([#1])

### Added

- `storage::Immutable` allow fast access to already registered metrics. ([#2])
- `FrozenRecorder` implementation of `metrics::Recorder` allowing access already registered metrics fast, but unable to register new ones on the fly. ([#2])
- `FreezableRecorder` implementation of `metrics::Recorder`, uniting both `Recorder` and `FrozenRecorder` ones. ([#2])

[#1]: /../../pull/1
[#2]: /../../pull/2




## [0.2.0] · 2022-12-08
[0.2.0]: /../../tree/v0.2.0

### Initially implemented

- `storage::Mutable` implementation of `metrics_util::registry::Storage` backed by `prometheus::Registry` and allowing to change `help` description of already registered metrics. ([6a6d4eae])
- `Recorder` implementation of `metrics::Recorder` allowing metrics creation on the fly. ([6a6d4eae])
- `NoOp`, `Panic` and `PanicInDebugNoOpInRelease` (default) `failure::Strategy`s to handle possible `prometheus::Error`s. ([6a6d4eae])

[6a6d4eae]: /../../commit/6a6d4eaefaf6a89a9f26c4d28b440fb671cec75a




[Semantic Versioning 2.0.0]: https://semver.org
