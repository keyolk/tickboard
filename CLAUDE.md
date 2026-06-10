# tickboard - Claude Code Project Context

## Project Overview
Rust 기반 실시간 글로벌 주식 모니터 TUI 앱.
미국(50종목) + 한국(50종목) 시장을 대시보드, 상세, 뉴스 AI 분석 화면으로 제공.

## Tech Stack
- **Framework**: ratatui + crossterm
- **Runtime**: tokio
- **Data**: Yahoo Finance JSON endpoints, Naver Finance/KRX-compatible internal provider, RSS scraping via reqwest/regex
- **AI**: AWS Bedrock Claude Sonnet 4.6 via AWS CLI
- **Language**: Rust 2021

## Project Structure
```
Cargo.toml           # Rust package manifest
Makefile             # build/test/lint/run workflow
src/main.rs          # app entrypoint
src/app.rs           # terminal setup, event loop, screen stack
src/config.rs        # symbols, sectors, indicators, RSS feeds, mappings
src/models/          # StockQuote, StockDetail, MarketIndex, EconomicIndicator, NewsItem
src/services/        # Yahoo, internal KRX/Naver, indicators, news, Bedrock, simulation
src/tasks/           # async worker messages and schedulers
src/ui/              # theme, layout, screens, widgets
screenshots/         # UI screenshots
```

## Key Patterns
- **Message-driven async**: background tokio tasks send `AppMsg` to the TUI event loop.
- **Generation IDs**: each refresh group increments generation; stale messages are ignored.
- **Wave Loading**: Dashboard 4단계 순차 로딩 (indices → indicators → stocks → market caps).
- **Screen Stack**: Dashboard → Detail → Article; `b`/Esc pops the stack.
- **Data Models**: Rust structs/enums in `src/models/`; `Market`, `Currency`, `ChartPeriod` are typed enums.
- **색상 규칙**: 상승 `#F04452` (빨강), 하락 `#3182F6` (파랑).

## Coding Conventions
- Rust-only repository; do not add Python runtime dependencies.
- Use `Makefile` targets for build/test/lint/run/clean workflows.
- 한국어 UI, 영어 code identifiers.
- Prefer explicit app state and small render functions over global mutable state.
- External API failures must render partial data gracefully, not crash the TUI.

## Commands
```bash
make run       # run TUI
make check     # fmt check + clippy + tests
make build     # debug build
make release   # release build
make clean     # remove target/
```

## Important Notes
- 한국 OHLCV는 `src/services/krx.rs` 내부 provider가 Naver/KRX-compatible daily JSON endpoint를 직접 호출한다.
- Yahoo/Naver external endpoints can rate limit or change; keep parsers isolated and fallbacks graceful.
- Bedrock uses `aws bedrock-runtime invoke-model`; `BEDROCK_API_KEY` is mapped to AWS CLI env credentials for compatibility.
- 호가/투자자동향은 현재가/OHLCV 기반 시뮬레이션 데이터.
