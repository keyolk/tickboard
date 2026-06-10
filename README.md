# tickboard

> Rust 기반 실시간 글로벌 주식 모니터 TUI (Terminal User Interface) 앱

미국 50종목 + 한국 50종목 + 주요 ETF/채권의 시세, 차트, 호가, 투자자 동향, AI 분석을 터미널에서 한눈에 확인할 수 있습니다.

---

## Screenshots

### Dashboard — 시장 전체 현황
경제지표, 시장지수, 시장 요약(상승/하락/거래량 Top), 섹터 등락, 종목 테이블, 뉴스 피드를 한 화면에 표시합니다.

![Dashboard](screenshots/dashboard.png)

### Insight — 시장 요약 + 섹터 등락
US/KR 시장별 상승·하락 종목수, Top 상승/하락 종목, 거래량 리더, 섹터별 평균 등락률 바 차트를 제공합니다.

![Insight](screenshots/insight.png)

### Detail — 종목 상세 분석
차트(MA5/MA20 이동평균선 + 골든/데드크로스), 호가, 투자자 동향, 기간별 수익률, 관련 경제지표, AI 종목 분석을 제공합니다.

![Detail](screenshots/detail.png)

### AI Analysis — 뉴스 기사 AI 분석
AWS Bedrock Claude Sonnet을 통해 뉴스 기사 요약, 시장 영향 분석, 투자 인사이트, 관련 종목을 자동 도출합니다. 영어 기사는 한국어로 번역 후 분석합니다.

![AI Analysis](screenshots/ai_analysis.png)

---

## Features

### Dashboard

| 기능 | 설명 |
|------|------|
| **경제지표 바** | WTI, Gold, Silver, Copper, EUR/USD, USD/KRW, USD/JPY, USD/CNY, US 10Y, Bitcoin, Ethereum 표시 |
| **시장 지수** | S&P 500, NASDAQ, DOW / KOSPI, KOSDAQ 카드 표시 |
| **시장 요약** | US/KR 상승·하락 종목수, Top 상승·하락, 거래량 리더 표시 |
| **섹터 등락** | US/KR 시장별 섹터 평균 등락률 바 차트 |
| **주식 테이블** | 하나의 메인 리스트 영역에서 `US / KR / ETF / Bond` 탭 전환, Symbol·Name·Price·Change·%·Vol·MktCap 컬럼 |
| **검색/정렬** | `/` 로 현재 리스트 검색, `s` 로 정렬 기준 순환, `S` 로 정렬 방향 반전 |

### Detail

| 기능 | 설명 |
|------|------|
| **가격 헤더** | 현재가, 등락률, 거래량 표시 |
| **핵심 지표** | Market Cap, PER, EPS, Beta/PBR, Volume, Avg Volume |
| **차트** | 1W/1M/3M/1Y Sparkline + MA5/MA20 + 골든/데드크로스 신호 |
| **호가** | 매도 10단계 + 매수 10단계 호가창 |
| **투자자 동향** | 개인/외국인/기관 최근 10일 순매수·순매도 추이 |
| **기간별 수익률** | 1W, 1M, 3M, 1Y 수익률 계산 |
| **관련 지표** | 종목 섹터 기반 관련 경제지표 자동 매핑 |
| **AI 종목 분석** | `A` 키로 AWS Bedrock Claude 기반 기술적 분석·투자 포인트·리스크 요약 |

### Article

| 기능 | 설명 |
|------|------|
| **기사 전문** | 뉴스 URL에서 기사 본문 자동 추출 |
| **AI 분석** | Claude Sonnet 4.6을 통한 요약·분석·투자 인사이트·관련 종목 도출 |
| **영한 번역** | 영어 기사는 한국어 번역 + 분석 제공 |

---

## Getting Started

### 지원 환경
- Amazon Linux 2023
- macOS
- Linux
- Rust 1.89+ / Cargo

### 설치

```bash
git clone https://github.com/keyolk/tickboard.git
cd tickboard
make setup
```

설치 과정에서 다음을 처리합니다:
1. Rust toolchain 확인
2. release binary 빌드
3. 실행 스크립트 권한 설정

> **AWS Bedrock 미설정 시**: AI 종목 분석, 뉴스 AI 분석, 영한 번역 기능이 비활성화됩니다. 나머지 기능은 정상 동작합니다.

### 실행

```bash
make run
# 또는
./run.sh
```

### Bedrock 설정

```bash
# .env 파일 직접 수정
vi .env
# BEDROCK_API_KEY=your-key
# BEDROCK_REGION=us-east-1
# BEDROCK_MODEL_ID=us.anthropic.claude-sonnet-4-6

# 또는 AWS CLI/IAM credentials 사용
AWS_PROFILE=your-profile make run
```

---

## Makefile Targets

| Target | 설명 |
|--------|------|
| `make run` | release binary가 있으면 실행, 없으면 `cargo run --release` |
| `make build` | debug build |
| `make release` | release build |
| `make test` | unit tests |
| `make lint` | clippy with warnings denied |
| `make fmt` | rustfmt 적용 |
| `make fmt-check` | formatting 검증 |
| `make check` | fmt-check + lint + test |
| `make smoke` | pseudo-terminal 실행/종료 smoke test |
| `make verify` | check + release + smoke |
| `make clean` | target/ 제거 |
| `make setup` | release build + run.sh 권한 설정 |

---

## Key Bindings

### Dashboard
| Key | Action | 설명 |
|-----|--------|------|
| `R` | Refresh | 전체 데이터 새로고침 |
| `?` | Shortcuts | 단축키 모달 열기/닫기 |
| `Tab` | Toggle Pane | 시장 리스트와 뉴스 포커스 전환 |
| `Shift+Tab` | Toggle Pane | 반대 방향 포커스 전환 |
| `H` / `L` / `←` / `→` | Switch Tab | 시장 탭 전환 (US / KR / ETF / Bond) |
| `↑` / `↓` / `J` / `K` | Navigate | 현재 포커스 리스트에서 선택 이동 |
| `/` | Search | 현재 포커스 리스트 검색/필터 |
| `S` | Sort Field | 정렬 기준 순환 |
| `Shift+S` | Sort Direction | 정렬 방향 반전 |
| `Enter` | Open | 종목 상세 또는 뉴스 기사 화면으로 이동 |
| `Q` | Quit | 앱 종료 |

### Detail
| Key | Action | 설명 |
|-----|--------|------|
| `?` | Shortcuts | 단축키 모달 열기/닫기 |
| `1` `2` `3` `4` | Period | 차트 기간 전환 (1W/1M/3M/1Y) |
| `Left` / `Right` | Cycle Period | 차트 기간 순환 전환 |
| `P` | Next Period | 다음 차트 기간 |
| `A` | AI Analysis | AI 종목 분석 토글 |
| `/` | News Search | 회사 뉴스 리스트 검색/필터 |
| `Enter` | News AI | 선택한 뉴스 기사 AI 분석 |
| `R` | Refresh | 종목 데이터 새로고침 |
| `B` / `Escape` | Back | 대시보드로 복귀 |

### Article
| Key | Action | 설명 |
|-----|--------|------|
| `?` | Shortcuts | 단축키 모달 열기/닫기 |
| `R` | Reload | 기사/AI 분석 재실행 |
| `B` / `Escape` | Back | 이전 화면으로 복귀 |

---

## Architecture

### Screen Flow
```
Dashboard ── Enter on stock ──> Detail ── Enter on news ──> Article
    ^                              |                          |
    └──────────── B/Esc ───────────┴──────── B/Esc ───────────┘
```

### Data Flow
```
Dashboard refresh
  ├─ Wave 1: US/KR indices
  ├─ Wave 2: economic indicators
  ├─ Wave 3: US/KR stock quotes
  └─ Wave 4: market caps

Detail refresh
  ├─ fast OHLCV/detail
  ├─ fundamentals retry/backoff
  ├─ company news
  ├─ order book simulation
  └─ investor trend simulation
```

### Project Structure
```
tickboard/
├── Cargo.toml
├── Cargo.lock
├── Makefile
├── run.sh
├── install.sh
├── src/
│   ├── main.rs
│   ├── app.rs
│   ├── config.rs
│   ├── models/
│   ├── services/
│   │   ├── yahoo.rs       # US/index/indicator Yahoo JSON endpoints
│   │   ├── krx.rs         # internal KR OHLCV provider
│   │   ├── kr.rs          # KR provider orchestration/fallbacks
│   │   ├── news.rs        # RSS/news/article extraction
│   │   ├── bedrock.rs     # AWS Bedrock CLI adapter
│   │   └── simulation.rs  # order book / investor trend simulation
│   ├── tasks/
│   └── ui/
└── screenshots/
```

---

## Dependencies

| Crate | 용도 |
|-------|------|
| `ratatui` / `crossterm` | TUI 렌더링, 키보드 이벤트, 터미널 제어 |
| `tokio` | 비동기 작업, 주기적 새로고침, 백그라운드 로딩 |
| `reqwest` / `serde_json` | Yahoo Finance, Naver Finance, RSS HTTP/JSON 처리 |
| `regex` | RSS/HTML 기사 추출 및 Naver/Yahoo 보조 파싱 |
| `chrono` | 시간, 날짜, 갱신 시각 표시 |
| `aws` CLI | AWS Bedrock Claude 호출 (선택사항) |

---

## Troubleshooting

| 증상 | 원인 | 해결 |
|------|------|------|
| 주식 데이터 안 나옴 | 네트워크 또는 외부 금융 데이터 endpoint 장애 | `R` 키로 새로고침, 인터넷 연결 확인 |
| KR 주식 데이터 지연 | 장 마감/휴장 또는 Naver/KRX-compatible endpoint 지연 | 다음 거래일 장중 갱신 확인 |
| AI 분석 안 됨 | AWS Bedrock 미설정 | `.env` 또는 `AWS_PROFILE` 설정 |
| 터미널 깨짐 | 유니코드 미지원 터미널 | iTerm2, Windows Terminal, Kitty 사용 |

---

## License

MIT
