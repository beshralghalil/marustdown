# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.6](https://github.com/beshralghalil/marustdown/compare/v0.1.5...v0.1.6) - 2026-09-30

### Added

- glide smoothly on page and heading jumps

## [0.1.5](https://github.com/beshralghalil/marustdown/compare/v0.1.4...v0.1.5) - 2026-09-30

### Added

- reload the file when it changes (--watch)

## [0.1.4](https://github.com/beshralghalil/marustdown/compare/v0.1.3...v0.1.4) - 2026-09-27

### Added

- render Mermaid diagrams
- scroll wide blocks horizontally
- render LaTeX math as Unicode

### Other

- show math and diagrams in the README screenshots
- unwind on panic and guard third-party renderers

## [0.1.3](https://github.com/beshralghalil/marustdown/compare/v0.1.2...v0.1.3) - 2026-09-26

### Added

- default to the ansi theme

## [0.1.2](https://github.com/beshralghalil/marustdown/compare/v0.1.1...v0.1.2) - 2026-09-26

### Fixed

- keep the version out of the generated man page

## [0.1.1](https://github.com/beshralghalil/marustdown/compare/v0.1.0...v0.1.1) - 2026-09-26

### Fixed

- show the cursor line in reverse video when colors are off

## [0.1.0](https://github.com/beshralghalil/marustdown/releases/tag/v0.1.0) - 2026-09-26

### Added

- ship a man page and shell completions

### Other

- format cli.rs
- document installation, contributing and the release process
- release with release-plz and dist
- Prepare the crate for publishing
- Add a configuration reference
- Remove the sample document
- Add VHS demo tape and screenshots to the README
- Write plain text when output isn't a terminal
- Highlight code with syntect
- Follow links from the keyboard
- Rewrite sample as a tour of every markdown feature
- License under MIT
- Add README
- Implement terminal markdown viewer
- Add gitignore and sample document
