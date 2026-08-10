# 0012 — CI로 강제되는 관리 게이트 (proposal 0011을 자동 강제)

- 상태: **SHIPPED 2026-07-02** (accepted "프로젝트관리가 제대로되기만하면됨"). Additive only.
- Scope: CI/로컬 툴링. 코드/런타임/문서 의미 무변경. 기존 게이트(`--check`/`--gate`/`--capabilities`)를
  **실행만** 한다 — 새 검사 로직 없음.

## 왜

proposal 0011의 관리 게이트(capability 인덱스 + docs drift)는 **사람이 `--check`를 기억해서 돌릴
때만** 작동했다. CI에는 hy-meta smoke만 있고 pnix-hy 툴킷 게이트가 없었다. "관리가 제대로 된다"는
= 사람 기억이 아니라 **자동 강제**. 이 proposal이 그 마지막 조각.

## 무엇을 추가 (전부 기존 게이트 실행)

- **`.github/workflows/pnix-hy-gate.yml`** — PR + push(main, `codex/**`)에서:
  1. `hy==1.3.0` 설치, `PNIX_HY_PYTHON` = CI python.
  2. `--check` (57 toolkit self-check, `docs_drift` 포함) — 문서↔코드 drift·미해소 위키링크·
     stale 생성물이 있으면 **CI 실패**.
  3. `docs/CAPABILITIES.md` 재생성 후 `git diff --exit-code` — 커밋된 생성물이 코드와 어긋나면 실패.
  4. `--gate` — sacred 레인(runtime self-test / rust corpus / 4-lane mirror / closure) + toolkit,
     회귀 0 보장.
- **`pnix-hy/Makefile`** — 로컬 편의: `make check` / `make gate` / `make capabilities`(재생성) /
  `make verify`. CI와 같은 명령을 손으로도.

## 효과 (관리가 "제대로")

- 중복개발·문서 drift·심볼 소멸·생성물 stale·sacred 회귀가 **모든 push에서 자동 차단**.
- 진실의 원천(코드)·파생 뷰(`--capabilities`)·게이트(`--check`/`--gate`)가 사람 개입 없이 정합 유지.

## Forbidden (지킴)

- 새 검사 로직 없음(기존 게이트 실행만). 런타임/sacred/문서 의미 무변경. `pnix_runtime.py` 무접촉.
