# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

## [2.3.2] - 2022-07-27

### Added
* highlight newly added question ([PR #28](https://github.com/liveask/liveask/pull/28))
* animation on question that changed like count ([PR #30](https://github.com/liveask/liveask/pull/30))

### Fixes
* use shared validation for `/addquestion` ([PR #29](https://github.com/liveask/liveask/pull/29))

## [2.3.0/2.3.1] - 2022-07-25

### Added
* premium feature: queestion screening/reviewing/whitelisting ([PR #26](https://github.com/liveask/liveask/pull/26))
* server side question validation (min/max length...)
* admin view exposes mod link in event view

### Fixes
* continous re-renders due to data change detection in `question-age-timer`
* scrolling to newly added question was broken
* remove `/api/addevent` (see `2.2.1`)

## [2.2.1] - 2022-07-20

### Fixes
* minor style improvements

### Changed
* add duplicate route `/api/event/add` for `/api/addevent` (to deprecate)

