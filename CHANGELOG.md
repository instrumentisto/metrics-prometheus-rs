`metrics-prometheus` changelog
==============================

All user visible changes to this project will be documented in this file. This project uses [Semantic Versioning 2.0.0].




## [0.11.0] · 2025-06-23
[0.11.0]: https://github.com/instrumentisto/metrics-prometheus-rs/tree/v0.11.0

[Diff](https://github.com/instrumentisto/metrics-prometheus-rs/compare/v0.10.0...v0.11.0)

### BC Breaks

- Upgraded to 0.20 version of `metrics-util` crate. ([#16])

[#16]: https://github.com/instrumentisto/metrics-prometheus-rs/pull/16




## [0.10.0] · 2025-03-29
[0.10.0]: https://github.com/instrumentisto/metrics-prometheus-rs/tree/v0.10.0

[Diff](https://github.com/instrumentisto/metrics-prometheus-rs/compare/v0.9.0...v0.10.0)

### BC Breaks

- Bumped up [MSRV] to 1.85 because of migration to 2024 edition. ([62fe9630])
- Upgraded to 0.14 version of `prometheus` crate. ([#14])

[#14]: https://github.com/instrumentisto/metrics-prometheus-rs/pull/14
[62fe9630]: https://github.com/instrumentisto/metrics-prometheus-rs/commit/62fe9630da9e42f19b24aeffa317c51fb21a67d2




## [0.9.0] · 2025-01-07
[0.9.0]: https://github.com/instrumentisto/metrics-prometheus-rs/tree/v0.9.0

[Diff](https://github.com/instrumentisto/metrics-prometheus-rs/compare/v0.8.0...v0.9.0)

### BC Breaks

- Upgraded to 0.19 version of `metrics-util` crate. ([#15])

[#15]: https://github.com/instrumentisto/metrics-prometheus-rs/pull/15




## [0.8.0] · 2024-10-21
[0.8.0]: https://github.com/instrumentisto/metrics-prometheus-rs/tree/v0.8.0

[Diff](https://github.com/instrumentisto/metrics-prometheus-rs/compare/v0.7.0...v0.8.0)

### BC Breaks

- Upgraded to 0.24 version of `metrics` crate. ([7398888c])
- Upgraded to 0.18 version of `metrics-util` crate. ([7398888c])
- Bumped up [MSRV] to 1.81 because for `#[expect]` attribute usage. ([a1192b5d])

[7398888c]: https://github.com/instrumentisto/metrics-prometheus-rs/commit/7398888ce269abe305c4cd578df8cc17e81e4d61
[a1192b5d]: https://github.com/instrumentisto/metrics-prometheus-rs/commit/a1192b5d1d7d6069b82d10f71d7fc4e0583897c0




## [0.7.0] · 2024-05-30
[0.7.0]: https://github.com/instrumentisto/metrics-prometheus-rs/tree/v0.7.0

[Diff](https://github.com/instrumentisto/metrics-prometheus-rs/compare/v0.6.0...v0.7.0)

### BC Breaks

- Upgraded to 0.23 version of `metrics` crate. ([#11], [#10])
- Upgraded to 0.17 version of `metrics-util` crate. ([#11], [#10])
- Bumped up [MSRV] to 1.72 because of newer dependencies versions. ([#11], [#10])

[#10]: https://github.com/instrumentisto/metrics-prometheus-rs/issues/10
[#11]: https://github.com/instrumentisto/metrics-prometheus-rs/pull/11




## [0.6.0] · 2023-12-25
[0.6.0]: https://github.com/instrumentisto/metrics-prometheus-rs/tree/v0.6.0

[Diff](https://github.com/instrumentisto/metrics-prometheus-rs/compare/v0.5.0...v0.6.0)

### BC Breaks

- Upgraded to 0.22 version of `metrics` crate. ([#9])
- Upgraded to 0.16 version of `metrics-util` crate. ([#9])

[#9]: https://github.com/instrumentisto/metrics-prometheus-rs/pull/9




## [0.5.0] · 2023-09-06
[0.5.0]: https://github.com/instrumentisto/metrics-prometheus-rs/tree/v0.5.0

[Diff](https://github.com/instrumentisto/metrics-prometheus-rs/compare/v0.4.1...v0.5.0)

### Changed

- Relicensed from "[BlueOak-1.0.0]" as "[MIT] OR [Apache-2.0]". ([f982cbaa], [#8])

[Apache-2.0]: https://github.com/instrumentisto/metrics-prometheus-rs/blob/v0.5.0/LICENSE-APACHE
[BlueOak-1.0.0]: https://github.com/instrumentisto/metrics-prometheus-rs/blob/v0.4.1/LICENSE.md
[MIT]: https://github.com/instrumentisto/metrics-prometheus-rs/blob/v0.5.0/LICENSE
[f982cbaa]: https://github.com/instrumentisto/metrics-prometheus-rs/commit/f982cbaabcefb976e54159a9c758b19712b156ef
[#8]: https://github.com/instrumentisto/metrics-prometheus-rs/pull/8




## [0.4.1] · 2023-04-25
[0.4.1]: https://github.com/instrumentisto/metrics-prometheus-rs/tree/v0.4.1

[Diff](https://github.com/instrumentisto/metrics-prometheus-rs/compare/v0.4.0...v0.4.1)

### Changed

- Updated to 0.5 version of `sealed` crate to fully get rid of `syn` 1.0. ([f923cb69], [#7])

[f923cb69]: https://github.com/instrumentisto/metrics-prometheus-rs/commit/f923cb69553ee624213b7df179c95137134843e3
[#7]: https://github.com/instrumentisto/metrics-prometheus-rs/pull/7




## [0.4.0] · 2023-04-17
[0.4.0]: https://github.com/instrumentisto/metrics-prometheus-rs/tree/v0.4.0

[Diff](https://github.com/instrumentisto/metrics-prometheus-rs/compare/v0.3.1...v0.4.0)

### BC Breaks

- Upgraded to 0.21 version of `metrics` crate. ([#5])
- Upgraded to 0.15 version of `metrics-util` crate. ([#5], [#6])

[#5]: https://github.com/instrumentisto/metrics-prometheus-rs/pull/5
[#6]: https://github.com/instrumentisto/metrics-prometheus-rs/pull/6




## [0.3.1] · 2023-01-24
[0.3.1]: https://github.com/instrumentisto/metrics-prometheus-rs/tree/v0.3.1

[Diff](https://github.com/instrumentisto/metrics-prometheus-rs/compare/v0.3.0...v0.3.1) | [Milestone](https://github.com/instrumentisto/metrics-prometheus-rs/milestone/1)

### Added

- `build()`, `build_freezable()` and `build_frozen()` methods to `recorder::Builder`, allowing to build the resulting `metrics::Recorder` without installing it as `metrics::recorder()`. ([#4])

[#4]: https://github.com/instrumentisto/metrics-prometheus-rs/pull/4




## [0.3.0] · 2022-12-11
[0.3.0]: https://github.com/instrumentisto/metrics-prometheus-rs/tree/v0.3.0

[Diff](https://github.com/instrumentisto/metrics-prometheus-rs/compare/v0.2.0...v0.3.0)

### BC Breaks

- Switched functions naming convention from `must_*` for panicking to `try_*` for fallible. ([#1])

### Added

- `storage::Immutable` allow fast access to already registered metrics. ([#2])
- `FrozenRecorder` implementation of `metrics::Recorder` allowing access already registered metrics fast, but unable to register new ones on the fly. ([#2])
- `FreezableRecorder` implementation of `metrics::Recorder`, uniting both `Recorder` and `FrozenRecorder` ones. ([#2])

[#1]: https://github.com/instrumentisto/metrics-prometheus-rs/pull/1
[#2]: https://github.com/instrumentisto/metrics-prometheus-rs/pull/2




## [0.2.0] · 2022-12-08
[0.2.0]: https://github.com/instrumentisto/metrics-prometheus-rs/tree/v0.2.0

### Initially implemented

- `storage::Mutable` implementation of `metrics_util::registry::Storage` backed by `prometheus::Registry` and allowing to change `help` description of already registered metrics. ([6a6d4eae])
- `Recorder` implementation of `metrics::Recorder` allowing metrics creation on the fly. ([6a6d4eae])
- `NoOp`, `Panic` and `PanicInDebugNoOpInRelease` (default) `failure::Strategy`s to handle possible `prometheus::Error`s. ([6a6d4eae])

[6a6d4eae]: https://github.com/instrumentisto/metrics-prometheus-rs/commit/6a6d4eaefaf6a89a9f26c4d28b440fb671cec75a




[MSRV]: https://doc.rust-lang.org/cargo/reference/manifest.html#the-rust-version-field
[Semantic Versioning 2.0.0]: https://semver.org
