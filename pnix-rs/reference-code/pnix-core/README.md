# pnix-core


> 2026-06-02 update: former client/control runtime material has been absorbed into pnixc-meta mirror primitives. Legacy client/control names below are fixture/schema/path compatibility or historical migration evidence; new implementation work should target pnixc-meta `.px` owners and replacement host adapters.

## Current convergence note (2026-03-13)

This crate document describes one implementation, testing, or audit surface within the current convergence plan.
It does not redefine the repository-wide ontology: the canonical base remains the shared substrate for state, meaning, observation, plan, and evidence.
Read this crate as one adapter/runtime/lowering surface under that substrate, with `pnix` as code/execution projection, `freecat` as spatial/world-model projection, and replacement projection adapters as non-owner control/governance surfaces; former `puck` labels are historical.

> **pnix-core는 실행하지 않는다.**
> **pnix-core는 의미를 해석하여 IR을 생성·검증하는 크로스플랫폼 컴파일러 코어다.**

---

## Quick Start

```bash
# 빌드
cargo build

# 테스트 (구조 테스트 + 금지 심볼 검사)
cargo test

# 의존성 정책 검사
cargo deny check
```

---

## API

```rust
use pnix_core::{parse, check, compile, inspect_ir, CompileOptions, SourceUnit};

let src = SourceUnit {
    name: "demo".into(),
    text: "...".into(),
};

// 문법 검증
let parsed = parse(&src)?;

// 의미 완성 판정 (Meaning Closure)
let checked = check(&src, &CompileOptions::default())?;

// IR/Artifact 생성
let compiled = compile(&src, &CompileOptions::default())?;

// 구조 조회
let ir_dump = inspect_ir(&src, &CompileOptions::default())?;
```

---

## 모듈 구조

```
src/
├─ diagnostics/    # Span, SourceMap, Error
├─ ast/            # Language-specific AST
├─ surface/        # Unified surface IR
├─ core/           # FxCore (Meaning IR)
├─ ssa/            # SSA IR
├─ passes/         # Lowering, Optimize, SSA-Opt
├─ contracts/      # Effect, Purity, Determinism
├─ build_ir/       # Build graph
├─ codegen/        # Text generation (TS, Python, Clojure, Nix)
└─ meta/           # MetaFx (self-description IR)
```

---

## 금지 사항

* `main.rs` 없음 (library crate)
* 값 계산 없음
* IO/시간/상태 없음
* 툴체인 호출 없음

## Guardrail

* 실행 경계 검사: `scripts/check-core-boundary.sh`
* 원칙/설계 근거: `docs/architecture.md`, `prd.md` (Core boundary/정책 항목)

---

## 테스트 정책

* 구조 테스트만 허용
* 값 assert 금지

```rust
// OK
assert!(ir.has_node("solve-linear"));

// NOT OK
// assert_eq!(compute("(+ 1 2)"), 3);
```
