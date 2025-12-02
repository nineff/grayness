# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.0] 2025-12-02

### Changed

- WebP images are no longer converted to PNG since Typst 0.14 supports them natively.

### Fixed

- SVG Filters now use sequential names and can therefore be chained together.
- Various Typos
- SVG Hue-Rotation nolonger incorrectly specifies the amount as "deg"
- Args not being applied correctly for `image-mask()`

## [0.4.1] 2025-08-20

### Fixed

- Output format for `mask()` is now always PNG since it supports an alpha channel.

## [0.4.0] 2025-08-20

### Added

- `mask()` function

## [0.3.0] 2025-03-01

### Added

- All functions are available for both raster and SVG images.
- `invert()` function
- `huerotate()` function
- `brighten()` function (also darkens)

### Removed

- `flip` functions (horizontal/vertical)

## [0.2.0]

### Added

- Initial Ability to work with SVG (currently only Grayscale)
