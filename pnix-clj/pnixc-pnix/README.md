# pnixc-pnix (data-only 모델)


> 2026-06-02 갱신: 이전 client/control 런타임 재료는 pnixc-meta mirror 프리미티브로
> 흡수되었습니다. 아래 레거시 client/control 이름은 fixture/schema/path 호환 또는
> 역사적 마이그레이션 증거입니다. 새 구현 작업은 pnixc-meta `.px` 소유자와 대체
> 호스트 어댑터를 대상으로 해야 합니다.

## 현재 수렴 메모 (2026-03-13)

이 루트 수준 문서는 개념·운영·거버넌스·역사 지원 표면입니다.
정본 저장소 방향은 `prd.md`, `todo.md`, `todo-3d.md`에 있습니다.
현재 수렴 기반은 상태, 의미, 관찰, 계획, 증거의 공유 기판으로 남습니다.
`pnix`, `freecat`, pnixc-meta closed-action/receipt 레인은 별도 최종 온톨로지가
아니라 그 기판 위의 투영으로 읽어야 합니다. 역사적 client/control 레인은
pnixc-meta mirror 프리미티브로 흡수되었습니다.

이 디렉터리는 pnixc 컴파일러 파이프라인의 **data-only** pnix 모델을 담습니다.
메타순환 러너에서 Stage0 부분 집합 검사와 IR 방출에 쓰이지만, 아직은 **실행
가능한 컴파일러가 아닙니다**.

설계 목표:
- 엄격한 pnix-subset-v1 준수
- IO/import/builtin 부수 효과 없음
- 결정적·재현 가능 메타데이터

파일:
- pnixc.px: 컴파일러 파이프라인 개요
- driver.px: CLI 모델 (flags/modes)
- exec/plan.px: 실행 계획 (비실행)
- exec/runtime.px: 런타임 (expr + module data-only parsing/lowering)
- ast/pnix_ast.px: pnix AST 스키마 (data-only)
- ast/unified_ast.px: 통합 AST 스키마 (data-only)
- lower/parse.px, lower/lower.px: 프론트엔드 파이프라인 단계
- emit/*.px: 방출 대상 (ir/ssa/aot)
- version.px: 버전 스탬프

pnixc-in-pnix이 실행 가능해지면 이 파일들은 증명 로그와 회귀 테스트가 쓰는
안정적이고 감사된 메타데이터로 남아야 합니다.

data-only 모델은 `pnixc_model_runner`로 검증되며, 메타순환 실행 중
`tmp/pnixc/proof/pnixc-model.json`으로 직렬화됩니다.
