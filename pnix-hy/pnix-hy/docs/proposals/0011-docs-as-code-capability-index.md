# 0011 — docs-as-code: 생성 capability 인덱스 + doc↔code drift 게이트

- 상태: **SHIPPED 2026-07-02** (accepted "todo/복잡 개발문서를 wiki로 설계↔코드 일치화, 중복개발
  회피, 최적화 위주 관리"). 메커니즘 = **생성 인덱스 + drift 게이트**(별도 손-wiki 없음).
- Scope: pnix-hy **문서/툴링 레인**(+ 얇은 CLI). Additive only. 런타임 의미 무변경,
  `pnix_runtime.py`·4-lane·sacred 무접촉. 두 번째 evaluator/mirror/gate 없음.
- placeholder/out-of-scope 점검: 의도적 placeholder(SCOPE_LOCK §3) 무접촉. 새 "지식관리" 축이므로
  §7대로 이 proposal로 선언. 손으로 유지하는 병렬 wiki는 **명시적 금지**(그게 곧 중복 원천).

## 왜 (문제)

핵심 위험은 "손-wiki 추가"로 **진실의 원천이 하나 더** 생겨 drift·중복개발이 늘어나는 것.
현재 이 프로젝트의 중복방지 척추는 이미 존재한다:
- `_toolkit_reports()` + `*_report()` = 실행되는 스펙(56 self-check).
- `__all__` = 공개 능력 표면. · `INTEROP_ROLE_MATRIX.md` = 기능→소유자→심볼 지도.
- `SCOPE_LOCK.md` = 경계/소유. · `docs/proposals/` = 변경 프로세스.

**결함은 둘**: (1) `todo.md` 3,117줄 단일덩어리가 이력을 누적해 drift; (2) role-matrix가 손-관리라
코드와 어긋날 수 있음.

## 원칙

> 진실의 원천 = **코드**. 문서는 코드에서 **파생/생성**되는 뷰 + **drift 게이트**.
> "설계구조 == 코드구조"를 희망이 아니라 `--check` **게이트**로 만든다.

## 신규 딜리버러블 (전부 additive, reuse-only)

1. **`pnix_hy/capabilities.py` + `capability_index()`** — `_toolkit_reports()` + `__all__` +
   각 심볼 docstring 첫 줄 + `docs/proposals/*.md` 상태에서 **파생**한 구조화 인덱스:
   `{name, kind, owner_lane, module, symbol, report, proposal, status, summary}`.
   코드에서 뽑으므로 drift 불가.
2. **CLI `--capabilities`** (text/json) + 생성 파일 `docs/CAPABILITIES.md`(생성물임을 헤더에 명시,
   손편집 금지). → **개발 전 `pnix-hy-project --capabilities | grep X` = 중복개발 방지 1차 조회.**
3. **`docs_drift_report()`** — 새 `*_report()`를 `--check`에 등록(56 → 57). 검증:
   (a) 문서(role-matrix·proposals·CAPABILITIES·README의 백틱 심볼)가 참조한 심볼이 코드에 실재,
   (b) 모든 `__all__` 공개 심볼이 인덱스로 커버·소유 표기됨(고아 없음),
   (c) `[[...]]` 위키링크가 실재 심볼/문서로 해소됨.
4. **`todo.md` 단일덩어리 해체** — `todo.md`(활성 수락작업만, 작게) + 이력은 git history/
   `docs/archive/`로. 새 작업은 proposal로(기존 규칙). 누적 중단.
5. **`[[<심볼>]]` 위키링크 규약** — 문서 간·문서↔코드 링크를 이중대괄호 `[[<name>]]` 형태로
   (메모리 시스템과 동일). 실제 링크 예는 `docs/CAPABILITIES.md` 상단(예: safe_eval). drift
   게이트(3c)가 식별자-형태 미해소 링크를 잡는다.

## 하지 않는 것 (이번 범위 밖 — 사용자가 "생성 인덱스+게이트"만 선택)

- mkdocs/Docusaurus 정적 사이트, GitHub Wiki(CI 생성), Obsidian 볼트 — **뷰**로서 나중에 원하면
  별도 proposal. 지금은 코드-앵커 인덱스 + 게이트만.
- 손으로 유지하는 병렬 wiki/복제 문서 — 금지.
- 런타임/의미/sacred 변경 — 없음.

## 최적화·중복방지 효과

- "X가 이미 있나?" → `--capabilities`(코드 파생)로 즉답 → 중복개발 차단.
- 문서가 없는 심볼을 참조하거나 심볼이 소유 없이 떠 있으면 `--check`가 **실패** → 설계↔코드 자동 정합.
- 이력 누적 단일 todo 제거 → 관리 표면 축소.

## 남은 한글 번역과의 관계

`todo.md` 해체 + 생성 인덱스 도입 후에는 **거대 단일 todo를 손번역하지 않는다**(그게 중복). 활성
todo(작음)와 curated 문서(proposals/audit/separation)만 한글 유지, 생성물(CAPABILITIES)은 코드 파생
식별자라 번역 대상 아님. → 중복 번역 회피.

## Done-when (구현 시)

- `capability_index()` + `--capabilities`(text/json) 동작, `docs/CAPABILITIES.md` 생성.
- `docs_drift_report()` `--check`에 등록 → **56 → 57**, all_ready, `--gate` PASS(회귀 0).
- `todo.md` 활성-only로 축소 + 이력 아카이브. `[[]]` 링크 해소 검사 통과.
- `pnix_runtime.py`/sacred 무변경, main FF 동기화.
