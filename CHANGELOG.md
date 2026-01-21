# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

As of January 2026 and until the 1.0.0 version is released, the authors will only make minor version changes (incrementing the `x` in `0.x.0`) if breaking changes are made (including changing the minimum supported Rust version). Features will now result in a patch version change (incrementing the `y` in `0.x.y`). This brings us into closer compliance with typical SemVer practice (and follows the default behavior of [`release-plz`](https://release-plz.dev/docs/config#the-features_always_increment_minor-field).

## [0.4.1](https://github.com/scouten-adobe/jumbf-rs/compare/v0.4.0...v0.4.1)
_28 September 2024_

### Fixed

* Only test `mod debug` with feature `parser`
* Only compile `mod debug` on feature `parser`
* Elided lifetimes must be explicit in Rust nightly
* Fix benchmark invocations ([#12](https://github.com/scouten-adobe/jumbf-rs/pull/12))

### Other

* Add two parsing benchmarks ([#9](https://github.com/scouten-adobe/jumbf-rs/pull/9))
* Numerous changes to build infrastructure
  * Start using [release-plz](https://release-plz.ieni.dev) for release management
  * Start using [commitlint-rs](https://keisukeyamashita.github.io/commitlint-rs/) for PR title validation
  * Start using Dependabot to track GitHub Actions upgrades
  * Update to latest version of cargo-deny, actions/checkout, codecov/codecov-action, CodSpeedHQ/action
  * Remove deprecated actions-rs/clippy-check action
  * Remove nightly build task

## 0.4.0
_27 March 2024_

* Add `ChildBox.as_super_box()` and `.as_data_box()` methods ([#7](https://github.com/scouten-adobe/jumbf-rs/pull/7))

## 0.3.0
_22 March 2024_

* Add an example for 1offset_within_superbox` ([#6](https://github.com/scouten-adobe/jumbf-rs/pull/6))
* `DataBox`: Add new function `offset_within_superbox` ([#5](https://github.com/scouten-adobe/jumbf-rs/pull/5))

## 0.2.2
_13 March 2024_

* Update to reflect 2023 version of JUMBF standard

## 0.2.1
_13 March 2024_

* Fix incorrect changelog link

## 0.2.0
_13 March 2024_

* Add ability to limit recursion when parsing superboxes ([#3](https://github.com/scouten-adobe/jumbf-rs/pull/3))
* Change `SuperBox::from_box` to `SuperBox::from_data_box` ([#4](https://github.com/scouten-adobe/jumbf-rs/pull/4))
* Add more examples to readme
* Rename `Box` to `DataBox` ([#1](https://github.com/scouten-adobe/jumbf-rs/pull/1))

## 0.1.0
_12 March 2024_

* First public release
