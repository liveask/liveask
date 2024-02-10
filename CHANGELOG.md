# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

## [2.7.2] - 2024-02-??

### Changed
* remove wordcloud, value did not justify effort and can be manuall generated from export data
* allow promo codes for discounts
* add mod url as stripe checkout metadata
* allow tracking non-string data

### Fixes
* show my local unscreened questions if not other questions in event

## [2.7.1] - 2024-02-05

### Changed
* use stripe as payment processor to help premium customers in more countries to be able to upgrade
* send event age in seconds into tracking
* tracking `event-tag` to analyze tag usage
* do not show wordcloud when event has no questions

### Fixes
* ok button was still enabled after wrong password input

## [2.6.2] - 2024-01-26

### Added
* set safari theme color

## [2.6.1] - 2024-01-19

### Fixes
* fix safari animation issue (since `2.3.5`) due to wordcloud always updateing when like happened

### Changed
* use `serde_dynamo` for tags and contexts as a test (potentially rolling it out to everything at some point but certainly for new data added in the future)

## [2.6.0] - 2024-01-15

### Added
* add service status page & link
* admin-only: allow setting event context link (eg. to point to meetup page)
* premium: moderator can tag questions (*use case: multiple parts/talks/topics in an event - the moderator can change the tag during the event and new questions will be tagged with the currently active tag*)

### Changed
* show footer on event page

## [2.5.0] - 2023-11-07

### Added
* event password support ([PR #50](https://github.com/liveask/liveask/pull/50))

### Changed
* make dropbown/checkbox match round button design

## [2.4.3] - 2023-10-21

### Changed
* use aws ses for mail sending instead of mailjet

## [2.4.2] - 2023-10-15

### Changed
* add tracking event for survey open button
* fix text alignment of 'ask question'

## [2.4.1] - 2023-10-09

### Changed
* improved SEO via meta tag
* better readability of social icons
* less standard warnings to lower pressure on sentry

## [2.4.0] - 2023-10-08

### Changed
* do not use local non-worker yew agents for GlobalEvents and Websocket (prepare for yew `0.20` upgrade)
* autosize textareas for question text and event desc
* upgrade to yew `0.20`
* fix sharing on twitter

## [2.3.11] - 2023-10-03

### Changed
* added a survey feedback request button

### Fixes
* live event reload was broken in `2.3.10`

## [2.3.10] - 2023-10-03

### Changed
* rename x/twitter profile to @liveaskapp

## [2.3.9] - 2023-10-03

### Changed
* premium upgrade window improvements
* show if an event was deleted vs. never existed
* BE: fix event upgrade tracking
* FE: better free event timeout explanation and CTA

## [2.3.8] - 2023-09-28

### Changed
* BE: use posthog for some event tracking
* FE: remove [metrical](https://metrical.xyz)
* FE: fix most yew warnings about img tags missing alt text
* FE: easier debuggable paypal credentials
* FE: more social links in footer of home page

## [2.3.7] - 2023-09-15

### Fixes
* fix changed twitter/x link
* add producthunt and linkedin social links

## [2.3.6] - 2023-09-08

### Fixes
* fix worker caching issues via renaming

## [2.3.5] - 2023-09-08

### Fixes
* fix wordcloud not working due to dependency bug ([#31](https://github.com/liveask/liveask/pull/31))
* simplify going to the right event from paypal purchase receipt

### Changed
* backend: remove panic testing route
* backend: cleanup some todos around event validation
* backend: update, simplify and auto-format dependencies
* frontend: merge multiple bool flags into bitflag (optimization and clippy cleanup)
* frontend: clicking version leads to changelog

## [2.3.4] - 2023-08-22

### Changed
* backend: reduce sentry performance sampling in release builds
* backend: some common non-critical errors downgraded to warnings
* backend: cargo updates
* frontend: better messaging "your questions in review by host"

## [2.3.3] - 2023-08-04

### Changed
* aws sdk update
* cargo updates

## [2.3.2] - 2023-07-27

### Added
* highlight newly added question ([PR #28](https://github.com/liveask/liveask/pull/28))
* animation on question that changed like count ([PR #30](https://github.com/liveask/liveask/pull/30))

### Fixes
* use shared validation for `/addquestion` ([PR #29](https://github.com/liveask/liveask/pull/29))

## [2.3.0/2.3.1] - 2023-07-25

### Added
* premium feature: question screening/reviewing/whitelisting ([PR #26](https://github.com/liveask/liveask/pull/26))
* server side question validation (min/max length...)
* admin view exposes mod link in event view

### Fixes
* continuous re-renders due to data change detection in `question-age-timer`
* scrolling to newly added question was broken
* remove `/api/addevent` (see `2.2.1`)

## [2.2.1] - 2023-07-20

### Fixes
* minor style improvements

### Changed
* add duplicate route `/api/event/add` for `/api/addevent` (to deprecate)
