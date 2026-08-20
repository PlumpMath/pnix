# pnix-runtime-llvm Examples


> 2026-06-02 update: former client/control runtime material has been absorbed into pnixc-meta mirror primitives. Legacy client/control names below are fixture/schema/path compatibility or historical migration evidence; new implementation work should target pnixc-meta `.px` owners and replacement host adapters.

## Current convergence note (2026-03-13)

This crate document describes one implementation, testing, or audit surface within the current convergence plan.
It does not redefine the repository-wide ontology: the canonical base remains the shared substrate for state, meaning, observation, plan, and evidence.
Read this crate as one adapter/runtime/lowering surface under that substrate, with `pnix` as code/execution projection, `freecat` as spatial/world-model projection, and replacement projection adapters as non-owner control/governance surfaces; former `puck` labels are historical.

실제 사용 예제와 출력 형태를 보여주는 문서입니다.

## 목차

- [JIT 실행 예제](#jit-실행-예제)
- [AOT 컴파일 예제](#aot-컴파일-예제)
- [출력 형태](#출력-형태)
- [LLVM 탐지 팁](#llvm-탐지-팁)

## JIT 실행 예제

### 예제 1: 상수 덧셈

가장 간단한 예제로, 입력 없이 상수만 사용하는 모듈입니다.

**FxCore 모듈** (`minimal_const.json`):
```json
{
  "name": "add_const",
  "inputs": [],
  "morphisms": [
    {
      "name": "add",
      "inputs": [{"name": "x", "ty": "Int"}],
      "outputs": [{"name": "sum", "ty": "Int"}],
      "effect": "pure"
    }
  ],
  "nodes": [
    {"name": "result", "uses": "add", "kind": "normal"}
  ],
  "edges": [
    {"from": "input", "to": "result", "from_input": "2"},
    {"from": "input", "to": "result", "from_input": "3"}
  ]
}
```

**Rust 코드**:
```rust
use pnix_runtime_llvm::JitEngine;
use pnix_runtime_api::EvalConfig;

let mut engine = JitEngine::new();
let fxcore_json = r#"{
  "name": "add_const",
  "inputs": [],
  "morphisms": [{"name": "add", "inputs": [{"name": "x", "ty": "Int"}], "outputs": [{"name": "sum", "ty": "Int"}], "effect": "pure"}],
  "nodes": [{"name": "result", "uses": "add", "kind": "normal"}],
  "edges": [
    {"from": "input", "to": "result", "from_input": "2"},
    {"from": "input", "to": "result", "from_input": "3"}
  ]
}"#;

// 컴파일
let module = engine.compile("add_const", fxcore_json.as_bytes())?;

// 실행 (결정론적 설정)
let config = EvalConfig {
    deterministic: true,
    seed: Some(0),
    now_ms: Some(0),
    clock_step_ms: Some(0),
};
let result = engine.eval(&module, &config)?;

// 결과 파싱
let result_value: i32 = serde_json::from_slice(&result.value.data)?;
assert_eq!(result_value, 5);
```

**출력**:
```json
{
  "ok": true,
  "value": {
    "data": "5"
  },
  "error": null
}
```

### 예제 2: 입력 파라미터 사용

두 개의 정수 입력을 받아 덧셈하는 모듈입니다.

**FxCore 모듈** (`two_inputs.json`):
```json
{
  "name": "add_inputs",
  "inputs": [
    {"name": "a", "ty": "Int"},
    {"name": "b", "ty": "Int"}
  ],
  "morphisms": [
    {
      "name": "add",
      "inputs": [{"name": "x", "ty": "Int"}],
      "outputs": [{"name": "sum", "ty": "Int"}],
      "effect": "pure"
    }
  ],
  "nodes": [
    {"name": "result", "uses": "add", "kind": "normal"}
  ],
  "edges": [
    {"from": "input", "to": "result", "from_input": "a"},
    {"from": "input", "to": "result", "from_input": "b"}
  ]
}
```

**Rust 코드**:
```rust
use pnix_runtime_llvm::JitEngine;
use pnix_runtime_api::EvalConfig;

let mut engine = JitEngine::new();
let fxcore_json = r#"{
  "name": "add_inputs",
  "inputs": [
    {"name": "a", "ty": "Int"},
    {"name": "b", "ty": "Int"}
  ],
  "morphisms": [{"name": "add", "inputs": [{"name": "x", "ty": "Int"}], "outputs": [{"name": "sum", "ty": "Int"}], "effect": "pure"}],
  "nodes": [{"name": "result", "uses": "add", "kind": "normal"}],
  "edges": [
    {"from": "input", "to": "result", "from_input": "a"},
    {"from": "input", "to": "result", "from_input": "b"}
  ]
}"#;

let module = engine.compile("add_inputs", fxcore_json.as_bytes())?;

// 입력값 설정 (실제로는 executor가 처리)
let config = EvalConfig {
    deterministic: true,
    seed: Some(0),
    now_ms: Some(0),
    clock_step_ms: Some(0),
};
let result = engine.eval(&module, &config)?;
```

**참고**: 입력값은 executor가 `EvalConfig`와 함께 제공합니다. JIT 엔진은 컴파일된 함수를 실행하며, 입력값은 함수 파라미터로 전달됩니다.

### 예제 3: 이진 연산 (곱셈)

곱셈 연산을 사용하는 예제입니다.

**FxCore 모듈**:
```json
{
  "name": "multiply",
  "inputs": [
    {"name": "x", "ty": "Int"},
    {"name": "y", "ty": "Int"}
  ],
  "morphisms": [
    {
      "name": "mul",
      "inputs": [{"name": "a", "ty": "Int"}],
      "outputs": [{"name": "product", "ty": "Int"}],
      "effect": "pure"
    }
  ],
  "nodes": [
    {"name": "result", "uses": "mul", "kind": "normal"}
  ],
  "edges": [
    {"from": "input", "to": "result", "from_input": "x"},
    {"from": "input", "to": "result", "from_input": "y"}
  ]
}
```

**지원되는 이진 연산**:
- `add`: 덧셈
- `sub`: 뺄셈
- `mul`: 곱셈
- `div`: 나눗셈 (정수 나눗셈)

## AOT 컴파일 예제

### 예제 1: 기본 AOT 컴파일

호스트 플랫폼용 오브젝트 파일을 생성합니다.

**Rust 코드**:
```rust
use pnix_runtime_llvm::{AotEngine, AotTarget, AotConfig};

let engine = AotEngine::with_config(AotConfig {
    target: AotTarget::LinuxX86_64,
    opt_level: 2,
    ..Default::default()
});

// FxCore 모듈 컴파일
let fxcore_json = r#"{
  "name": "my_module",
  "inputs": [],
  "morphisms": [{"name": "add", "inputs": [{"name": "x", "ty": "Int"}], "outputs": [{"name": "sum", "ty": "Int"}], "effect": "pure"}],
  "nodes": [{"name": "result", "uses": "add", "kind": "normal"}],
  "edges": [
    {"from": "input", "to": "result", "from_input": "2"},
    {"from": "input", "to": "result", "from_input": "3"}
  ]
}"#;

let output = engine.compile("my_module", fxcore_json.as_bytes())?;

// 아티팩트 패키징 (파일 시스템에 쓰지 않음)
let layout = engine.package_artifacts("my_module", &output)?;

// 디스크에 명시적으로 쓰기
engine.write_artifacts_to_disk(&layout, &output, "dist")?;
```

**생성된 디렉토리 구조**:
```
dist/
├── bin/
│   └── my_module          # 실행 파일 (Linux)
├── lib/
│   └── libmy_module.so    # 공유 라이브러리 (Linux)
└── manifest/
    └── my_module.json     # 아티팩트 매니페스트
```

### 예제 2: macOS ARM64 타겟

macOS ARM64용 오브젝트 파일을 생성합니다.

**Rust 코드**:
```rust
use pnix_runtime_llvm::{AotEngine, AotTarget, AotConfig};

let engine = AotEngine::with_config(AotConfig {
    target: AotTarget::MacOSArm64,
    opt_level: 3,  // 최적화 레벨 3
    debug: false,
    ..Default::default()
});

let output = engine.compile("my_app", fxcore_json.as_bytes())?;
let layout = engine.package_artifacts("my_app", &output)?;
engine.write_artifacts_to_disk(&layout, &output, "build")?;
```

**생성된 디렉토리 구조** (macOS):
```
build/
├── bin/
│   └── my_app             # 실행 파일
├── lib/
│   └── libmy_app.dylib    # 동적 라이브러리 (macOS)
└── manifest/
    └── my_app.json        # 매니페스트
```

### 예제 3: 최적화 레벨 설정

다양한 최적화 레벨을 사용하는 예제입니다.

**Rust 코드**:
```rust
// 최적화 없음 (빠른 컴파일, 느린 실행)
let engine_fast = AotEngine::with_config(AotConfig {
    target: AotTarget::LinuxX86_64,
    opt_level: 0,
    ..Default::default()
});

// 균형잡힌 최적화 (권장)
let engine_balanced = AotEngine::with_config(AotConfig {
    target: AotTarget::LinuxX86_64,
    opt_level: 2,
    ..Default::default()
});

// 최대 최적화 (느린 컴파일, 빠른 실행)
let engine_optimized = AotEngine::with_config(AotConfig {
    target: AotTarget::LinuxX86_64,
    opt_level: 3,
    ..Default::default()
});
```

## 출력 형태

### JIT 실행 결과

JIT 실행은 `EvalResult` 구조체를 반환합니다:

```rust
pub struct EvalResult {
    pub ok: bool,
    pub value: Option<EvalValue>,
    pub error: Option<String>,
}

pub struct EvalValue {
    pub data: Vec<u8>,  // JSON 인코딩된 결과
}
```

**성공 케이스**:
```json
{
  "ok": true,
  "value": {
    "data": "5"
  },
  "error": null
}
```

**실패 케이스**:
```json
{
  "ok": false,
  "value": null,
  "error": "LLVM execution error: function returned invalid value"
}
```

**결과 파싱**:
```rust
use serde_json;

let result = engine.eval(&module, &config)?;
if result.ok {
    let result_json: serde_json::Value = serde_json::from_slice(&result.value.unwrap().data)?;
    let result_value: i32 = result_json.as_i64().unwrap() as i32;
    println!("Result: {}", result_value);
} else {
    eprintln!("Error: {}", result.error.unwrap());
}
```

### AOT 매니페스트 출력

AOT 컴파일은 매니페스트 JSON 파일을 생성합니다:

**매니페스트 구조** (`dist/manifest/my_module.json`):
```json
{
  "name": "my_module",
  "target_triple": "x86_64-unknown-linux-gnu",
  "version": "1.0.0",
  "entry_point": "pnix_entry",
  "binary_path": "bin/my_module",
  "library_path": "lib/libmy_module.so",
  "build_timestamp": null,
  "build_config": {
    "opt_level": 2,
    "debug": false,
    "output_format": "object"
  },
  "metadata": {}
}
```

**매니페스트 필드 설명**:
- `name`: 모듈 이름
- `target_triple`: LLVM 타겟 트리플 (예: `"x86_64-unknown-linux-gnu"`)
- `version`: 아티팩트 버전
- `entry_point`: 진입점 함수 이름 (기본값: `"pnix_entry"`)
- `binary_path`: 실행 파일의 상대 경로
- `library_path`: 공유 라이브러리의 상대 경로 (또는 `null`)
- `build_timestamp`: 항상 `null` (결정론적 빌드)
- `build_config`: 빌드 설정 객체
  - `opt_level`: 최적화 레벨 (0-3)
  - `debug`: 디버그 심볼 포함 여부
  - `output_format`: 출력 형식 (예: `"object"`)
- `metadata`: 선택적 메타데이터 맵

**매니페스트 읽기**:
```rust
use std::fs;
use serde_json;

let manifest_path = "dist/manifest/my_module.json";
let manifest_json = fs::read_to_string(manifest_path)?;
let manifest: serde_json::Value = serde_json::from_str(&manifest_json)?;

println!("Module: {}", manifest["name"]);
println!("Target: {}", manifest["target_triple"]);
println!("Binary: {}", manifest["binary_path"]);
```

## LLVM 탐지 팁

LLVM이 설치되어 있지만 `llvm-config`를 찾을 수 없는 경우, 다음 팁을 참고하세요.

### 1. llvm-config 찾기

**macOS (Homebrew)**:
```bash
# llvm-config 위치 확인
which llvm-config
# 또는
find /opt/homebrew -name llvm-config 2>/dev/null
find /usr/local -name llvm-config 2>/dev/null

# 예: /opt/homebrew/opt/llvm@14/bin/llvm-config
```

**Linux**:
```bash
# 버전별 llvm-config 찾기
which llvm-config-14
which llvm-config-14

# 또는
find /usr -name llvm-config* 2>/dev/null
```

### 2. 환경 변수 설정

**LLVM 14.0**:
```bash
# macOS
export LLVM_SYS_140_PREFIX=/opt/homebrew/opt/llvm@14

# Linux
export LLVM_SYS_140_PREFIX=/usr/lib/llvm-14
```

**LLVM 14.0**:
```bash
# macOS
export LLVM_SYS_140_PREFIX=/opt/homebrew/opt/llvm@14

# Linux
export LLVM_SYS_140_PREFIX=/usr/lib/llvm-14
```

**자동 탐지**:
```bash
# llvm-config가 PATH에 있는 경우
export LLVM_SYS_140_PREFIX=$(dirname $(dirname $(which llvm-config)))
```

### 3. 플랫폼별 설치 및 설정

**macOS (Homebrew)**:
```bash
# LLVM 설치
brew install llvm@14

# PATH에 추가
export PATH="/opt/homebrew/opt/llvm@14/bin:$PATH"

# 환경 변수 설정
export LLVM_SYS_140_PREFIX=/opt/homebrew/opt/llvm@14

# 확인
llvm-config --version
llvm-config --libs
```

**Linux (apt)**:
```bash
# LLVM 개발 패키지 설치
sudo apt-get update
sudo apt-get install llvm-14-dev libclang-14-dev

# 확인
llvm-config-14 --version

# 환경 변수 설정 (필요한 경우)
export LLVM_SYS_140_PREFIX=/usr/lib/llvm-14
```

**Linux (수동 탐지)**:
```bash
# 버전별 llvm-config 사용
export LLVM_SYS_140_PREFIX=$(dirname $(dirname $(which llvm-config-14)))
```

### 4. 일반적인 탐지 문제 해결

**문제**: "No suitable version of LLVM was found"

**해결책**:
1. LLVM이 설치되어 있는지 확인:
   ```bash
   llvm-config --version
   ```

2. `LLVM_SYS_<version>_PREFIX` 환경 변수 설정:
   ```bash
   # LLVM 14.0 예시
   export LLVM_SYS_140_PREFIX=/opt/homebrew/opt/llvm@14
   ```

3. 빌드 재시도:
   ```bash
   cargo clean
   cargo build -p pnix-runtime-llvm --features llvm
   ```

**문제**: 버전 불일치

**해결책**:
1. `inkwell`/`llvm-sys`가 요구하는 LLVM 버전 확인:
   ```bash
   # Cargo.toml에서 inkwell 버전 확인
   grep inkwell Cargo.toml
   ```

2. 호환되는 LLVM 버전 설치:
   ```bash
   # 예: LLVM 14.0 필요
   brew install llvm@14
   ```

3. 올바른 환경 변수 설정:
   ```bash
   export LLVM_SYS_140_PREFIX=/opt/homebrew/opt/llvm@14
   ```

**문제**: 여러 LLVM 버전 설치됨

**해결책**:
1. 사용할 LLVM 버전 결정
2. 해당 버전의 `LLVM_SYS_<version>_PREFIX` 설정:
   ```bash
   # LLVM 14.0 사용
   export LLVM_SYS_140_PREFIX=/opt/homebrew/opt/llvm@14
   
   # LLVM 14.0 사용
   export LLVM_SYS_140_PREFIX=/opt/homebrew/opt/llvm@14
   ```

3. PATH에 해당 버전의 `bin` 디렉토리 추가:
   ```bash
   export PATH="/opt/homebrew/opt/llvm@14/bin:$PATH"
   ```

**문제**: 비표준 설치 위치

**해결책**:
1. LLVM 설치 루트 디렉토리 찾기 (bin/, lib/, include/ 포함):
   ```bash
   # 예: /custom/path/to/llvm
   # 구조:
   #   /custom/path/to/llvm/bin/llvm-config
   #   /custom/path/to/llvm/lib/
   #   /custom/path/to/llvm/include/
   ```

2. 환경 변수 설정:
   ```bash
   export LLVM_SYS_140_PREFIX=/custom/path/to/llvm
   ```

### 5. 검증 명령어

LLVM 설치가 올바른지 확인:

```bash
# 버전 확인
llvm-config --version

# 라이브러리 확인
llvm-config --libs

# 포함 디렉토리 확인
llvm-config --includedir

# 타겟 확인
llvm-config --host-target
```

### 6. CI/CD 환경에서의 팁

CI 환경에서는 명시적으로 LLVM 경로를 설정하는 것이 좋습니다:

```yaml
# GitHub Actions 예시
env:
  LLVM_SYS_140_PREFIX: /usr/lib/llvm-14

# 또는
- name: Set LLVM path
  run: |
    export LLVM_SYS_140_PREFIX=$(dirname $(dirname $(which llvm-config-14)))
    echo "LLVM_SYS_140_PREFIX=$LLVM_SYS_140_PREFIX" >> $GITHUB_ENV
```

### 7. 디버깅 팁

빌드 실패 시 상세 정보 확인:

```bash
# 상세 빌드 로그
cargo build -p pnix-runtime-llvm --features llvm -v

# 환경 변수 확인
env | grep LLVM

# llvm-config 경로 확인
which llvm-config
llvm-config --prefix
```

## 추가 리소스

- [README.md](README.md): 전체 문서 및 API 참조
- [fixtures/](fixtures/): 테스트 픽스처 디렉토리
