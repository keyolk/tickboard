# Prompt: 새 기능 추가

## Template
새 기능을 추가할 때 다음 절차를 따르세요:

1. **src/config.rs 확인** — 필요한 설정/매핑이 있는지
2. **src/models/ 확인** — 필요한 데이터 필드/enum이 있는지
3. **src/services/ 확인** — 데이터 소스가 있는지, 새 async service 함수 필요 여부
4. **src/tasks/ 확인** — 백그라운드 작업 메시지/스케줄러가 필요한지
5. **src/ui/widgets/ 구현** — 재사용 가능한 렌더링 함수 추가/수정
6. **src/ui/screens/ 통합** — 화면 state, key handling, render 연결
7. **검증** — `make check` 및 필요한 경우 `make smoke`

## Checklist
- [ ] 추가 API 호출 최소화 (기존 데이터 활용 우선)
- [ ] generation ID로 stale async result 방지
- [ ] 외부 API 실패 시 partial UI 유지
- [ ] 색상 규칙 준수 (상승 빨강, 하락 파랑)
- [ ] `cargo clippy -- -D warnings` 통과
