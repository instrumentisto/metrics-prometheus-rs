`metrics-prometheus` changelog
==============================

All user visible changes to this project will be documented in this file. This project uses [Semantic Versioning 2.0.0].




## [0.6.0] · 2023-12-25
[0.6.0]: /../../tree/v0.6.0

[Diff](/../../compare/v0.5.0...v0.6.0)

### BC Breaks

- Upgraded to 0.22 version of `metrics` crate. ([#9])
- Upgraded to 0.16 version of `metrics-util` crate. ([#9])

[#9]: /../../pull/9




## [0.5.0] · 2023-09-06
[0.5.0]: /../../tree/v0.5.0

[Diff](/../../compare/v0.4.1...v0.5.0)

### Changed

- Relicensed from "[BlueOak-1.0.0]" as "[MIT] OR [Apache-2.0]". ([f982cbaa], [#8])

[Apache-2.0]: /../../blob/v0.5.0/LICENSE-APACHE
[BlueOak-1.0.0]: /../../blob/v0.4.1/LICENSE.md
[MIT]: /../../blob/v0.5.0/LICENSE
[f982cbaa]: /../../commit/f982cbaabcefb976e54159a9c758b19712b156ef
[#8]: /../../pull/8




## [0.4.1] · 2023-04-25
[0.4.1]: /../../tree/v0.4.1

[Diff](/../../compare/v0.4.0...v0.4.1)

### Changed

- Updated to 0.5 version of `sealed` crate to fully get rid of `syn` 1.0. ([f923cb69], [#7])

[f923cb69]: /../../commit/f923cb69553ee624213b7df179c95137134843e3
[#7]: /../../pull/7




## [0.4.0] · 2023-04-17
[0.4.0]: /../../tree/v0.4.0

[Diff](/../../compare/v0.3.1...v0.4.0)

### BC Breaks

- Upgraded to 0.21 version of `metrics` crate. ([#5])
- Upgraded to 0.15 version of `metrics-util` crate. ([#5], [#6])

[#5]: /../../pull/5
[#6]: /../../pull/6




## [0.3.1] · 2023-01-24
[0.3.1]: /../../tree/v0.3.1

[Diff](/../../compare/v0.3.0...v0.3.1) | [Milestone](/../../milestone/1)

### Added

- `build()`, `build_freezable()` and `build_frozen()` methods to `recorder::Builder`, allowing to build the resulting `metrics::Recorder` without installing it as `metrics::recorder()`. ([#4])

[#4]: /../../pull/4




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
