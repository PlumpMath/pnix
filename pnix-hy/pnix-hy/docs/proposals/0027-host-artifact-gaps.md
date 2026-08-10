# 0027 — host artifact 잔여 gap: form_sha256 + env diff (감사 G2+G5 승격)

- 상태: **SHIPPED 2026-07-02** (동일일 승격·구현). Additive, **hy-meta 호스트 레인**(host_exec.py /
  clean_replay.py) — bootstrap.py 본체 무접촉, 기존 필드/시그니처 보존(새 필드·기본값 인자만).
- 근거: 감사 검증 진짜 gap G2(read-단계 form 해시가 artifact 레코드에 부재), G5(env drift가 해시
  단위 감지뿐 — 변수 단위 diff 부재). G3(cache-key 의존성 필드)는 0023에서 pnix-hy 측 해소.

## 딜리버러블
1. **G2**: host artifact 레코드에 `form_sha256`(reader가 낸 폼 표현의 내용 해시) 추가 — 기존
   source/ast/python/code/pyc 해시 옆의 read-단계 정체성.
2. **G5**: `env_diff(a, b)` — env 스냅샷/manifest의 **변수 단위** {added, removed, changed} 리포트
   (기존 해시 비교는 유지, 설명력만 추가).

## 수용: 직접 프로브로 검증(artifact에 form_sha256 존재+결정적; env_diff가 심은 변경을 변수명으로
지목). hy-meta smoke + pnix-hy `--gate`(stage 검사 경유) 회귀 0.
