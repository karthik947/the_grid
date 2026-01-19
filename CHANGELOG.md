# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.1] - 2026-01-19

### Added

- Intro page for **The Grid** shown on startup.
- Dedicated layout panel; settings panel now focuses on preset-related configuration.
- `scripts/run_dev` helper to launch the app during development.

### Changed

- Replaced egui/eframe UI with a terminal UI using `ratatui` to reduce CPU and memory usage.
- Switched logging from `env_logger` to `flexi_logger`; logs now write to files with a max history of 3.
- Engine/indicator modules now emit a full snapshot once, then only deltas; indicator values are quantized (volatility to 1 decimal, RSI to 0).

### Removed

- `diagrams/flow.puml` (redundant with README and changelog).

## [0.1.0] - 2025-12-24

### Added

- First beta release of **The Grid**: realtime Binance websocket ingestion with reconnects and reboot signaling.
- Binance REST adapter with rate limiting, plus `KlineStore` and `HistoryService` to warm indicators from history.
- Async indicator engine (tokio multi-thread) with RSI + volatility state, batched UI updates, and warmup gating.
- eframe/egui dashboard with presets, size presets, indicator toggles, thresholds, and live table rendering via `IndexLookup`.
- Typed message buses (EngineBus, HistoryBus, WsBus, UiBus) to isolate runtime services.
- Initial logging and error model scaffolding.

[Unreleased]: https://github.com/your-repo/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/your-repo/releases/tag/v0.1.0
