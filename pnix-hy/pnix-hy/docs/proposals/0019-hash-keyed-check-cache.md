# 0019 — 해시-키 검사 캐시 (0013 R2 승격)

- 상태: **SHIPPED 2026-07-02** (승격 수락 동일일). Additive **opt-in** only.
- 근거: Unison(결정적·무I/O 테스트는 의존성 해시 불변이면 재실행 불필요) + Build Systems à la
  Carte(verifying trace) — 3-0 검증.
- Scope: pnix-hy 툴킷/CLI. **기본 동작 무변경** — `--check`는 지금처럼 전수 실행이 기본이고,
  `--check --cached`일 때만 캐시 사용. sacred 레인(`--gate`의 runtime/corpus/mirror)은 캐시 대상
  제외(항상 전수).

## 목표

내용이 안 변했으면 리포트를 재실행하지 않는다 → 로컬 반복 개발에서 `--check` 시간 대폭 단축.
정확성 원칙: **verifying trace** — 캐시 키가 입력 전체를 커버하지 못하면 캐시하지 않는다.

## 구현 (재사용만)

- 신규 `pnix_hy/check_cache.py`:
  - `cache_key(report_name)` = sha256(관련 소스 파일들의 내용해시 결합) — 관련 파일 =
    해당 리포트 함수의 모듈 + 그 모듈이 import하는 `pnix_hy.*` 모듈 전부(보수적: 확실하지 않으면
    **패키지 전체 파일 해시** — 틀린 캐시 히트보다 미스가 낫다) + `PNIX_HY_PYTHON` 버전 문자열.
  - 저장: `~/.cache/pnix-hy/check-cache.json` (report_name → {key, ready, summary, ts}).
  - `cached_run(name, fn)`: 키 일치 + 직전 결과 `ready:True`였을 때만 스킵(FAIL은 캐시 안 함 —
    실패는 항상 재실행). 스킵 시 리포트에 `cached: True` 표기.
- `cli.cmd_check(as_json, cached=False)` + `--cached` 플래그: cached일 때 `_safe_report` 대신
  `cached_run` 경유. 요약 줄에 `(N cached)` 표기.
- 신규 `check_cache.check_cache_report()`: (a) 동일 입력 2회 → 2회째 cached, (b) 모듈 파일에
  바이트 하나 추가(임시 파일 복사본으로 시뮬레이션) → 키 변화 → 재실행 판정, (c) FAIL 리포트는
  캐시되지 않음. `cli`에 `"check_cache"` 등록 → `--check` **+1**.

## 수용 기준 / 시험

- `--check` (기본): 동작·결과 완전 불변. `--check --cached` 2회째: 대부분 cached, all_ready 동일.
- 소스 1바이트 변경 → 해당 리포트 재실행됨(무효화 증명).
- `check_cache_report()["ready"] is True`; `--gate`는 캐시 미사용 경로로 PASS(회귀 0).
