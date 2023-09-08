# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

## [2.3.5] - 2022-09-08

### Fixes
* fix wordcloud not working due to dependency bug ([#31](https://github.com/liveask/liveask/pull/31))

### Changed
* backend: remove panic testing route 
* backend: cleanup some todos around event validation 
* backend: update, simplify and auto-format dependencies
* frontend: merge multiple bool flags into bitflag (optimization and clippy cleanuo)

## [2.3.4] - 2022-08-22

### Changed
* backend: reduce sentry performance sampling in release builds
* backend: some common non-critical errors downgraded to warnings
* backend: cargo updates
* frontend: better messaging "your questions in review by host"

## [2.3.3] - 2022-08-04

### Changed
* aws sdk update
* cargo updates

## [2.3.2] - 2022-07-27

### Added
* highlight newly added question ([PR #28](https://github.com/liveask/liveask/pull/28))
* animation on question that changed like count ([PR #30](https://github.com/liveask/liveask/pull/30))

### Fixes
* use shared validation for `/addquestion` ([PR #29](https://github.com/liveask/liveask/pull/29))

## [2.3.0/2.3.1] - 2022-07-25

### Added
* premium feature: question screening/reviewing/whitelisting ([PR #26](https://github.com/liveask/liveask/pull/26))
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

