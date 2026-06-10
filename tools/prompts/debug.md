# Prompt: 디버깅

## Template
버그를 진단할 때 다음 절차를 따르세요:

1. **재현 경로 확인**: Dashboard / Detail / Article 중 어느 화면인지 확인
2. **빌드 검증**: `make check`
3. **런타임 확인**: `make smoke` 또는 `RUST_LOG=debug make run`
4. **데이터 흐름 추적**:
   - services/ → tasks/AppMsg → app state → ui/screens → ui/widgets 순서로 확인
   - generation ID가 최신 메시지만 적용하는지 확인
5. **외부 API 응답 확인**: 해당 service 모듈 단위로 endpoint/파싱 로직 확인
6. **터미널 이벤트 확인**: key handling이 현재 screen stack top에 적용되는지 확인

## Common Issues
- 빈 테이블 → 외부 금융 endpoint 실패 또는 parser schema 변경
- stale 데이터 표시 → generation ID mismatch 확인
- AI 비활성 → `.env`, `AWS_PROFILE`, AWS CLI Bedrock 권한 확인
- 터미널 깨짐 → panic/early return 후 terminal restore 경로 확인
