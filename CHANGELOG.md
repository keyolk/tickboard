# Changelog

All notable changes to tickboard will be documented in this file.

## Unreleased

### Changed
- Rewrote the application as a Rust-only TUI using `ratatui`, `crossterm`, and `tokio`.
- Replaced ad-hoc runtime setup with Makefile-managed build, lint, test, smoke, release, and clean workflows.
- Korean OHLCV quote/detail data now uses an internal KRX-compatible provider in `src/services/krx.rs`.
- Bedrock AI integration uses a Rust AWS CLI adapter and supports `.env`/AWS profile based credentials.
- Dashboard now includes major US ETFs and bond instruments with the same detail/news/AI flow as other US instruments.

### Removed
- Removed the legacy implementation and runtime dependency tree from the repository.

## [1.0.0] - 2026-03-02

첫 정식 릴리스. 미국 50종목 + 한국 50종목 실시간 모니터링 TUI 앱.

### Added
- Dashboard with economic indicators, market indices, market summary, sector performance, US/KR stock tables, news feed, and periodic refresh.
- Detail screen with price metrics, sparkline chart, MA5/MA20 signal, order book, investor trends, period returns, related indicators, and AI stock analysis.
- Article screen with article extraction, Korean analysis, English-to-Korean translation, market impact, investment insights, and related stocks.
- Installer, run script, screenshots, project context, and validation tooling.

### Data Sources
- US stocks/indices/indicators: Yahoo Finance-compatible JSON endpoints.
- KR stocks: internal KRX-compatible OHLCV provider plus Naver Finance fallback data.
- News: RSS feeds from Yahoo Finance, 한국경제, 매일경제, and company-specific feeds.
- AI analysis: AWS Bedrock Claude Sonnet 4.6.
