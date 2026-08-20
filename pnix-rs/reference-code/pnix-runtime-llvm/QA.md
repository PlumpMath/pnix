# pnix-runtime-llvm QA Guide


> 2026-06-02 update: former client/control runtime material has been absorbed into pnixc-meta mirror primitives. Legacy client/control names below are fixture/schema/path compatibility or historical migration evidence; new implementation work should target pnixc-meta `.px` owners and replacement host adapters.

## Current convergence note (2026-03-13)

This crate document describes one implementation, testing, or audit surface within the current convergence plan.
It does not redefine the repository-wide ontology: the canonical base remains the shared substrate for state, meaning, observation, plan, and evidence.
Read this crate as one adapter/runtime/lowering surface under that substrate, with `pnix` as code/execution projection, `freecat` as spatial/world-model projection, and replacement projection adapters as non-owner control/governance surfaces; former `puck` labels are historical.

테스트 명령어와 예상 결과, 문제 해결 방법을 정리한 문서입니다.

## 목차

- [빠른 시작](#빠른-시작)
- [테스트 명령어](#테스트-명령어)
- [예상 결과](#예상-결과)
- [실행 환경](#실행-환경)
- [LLVM 탐지 및 설정](#llvm-탐지-및-설정)
- [문제 해결](#문제-해결)

## 빠른 시작

### LLVM Feature 없이 테스트

```bash
cargo test -p pnix-runtime-llvm --lib
```

**예상 결과**:
- 33개 테스트 통과
- 2개 테스트 무시됨 (feature-gated)
- 0개 실패

### LLVM Feature와 함께 테스트

```bash
cargo test -p pnix-runtime-llvm --lib --features llvm
```

**예상 결과** (LLVM이 설치된 경우):
- 모든 테스트 통과 (feature-gated 테스트 포함)
- JIT 실행 성공
- AOT 컴파일 성공

**예상 결과** (LLVM이 없는 경우):
- 빌드 실패 또는 feature-gated 테스트만 무시됨

## 테스트 명령어

### 기본 단위 테스트

```bash
# LLVM feature 없이
cargo test -p pnix-runtime-llvm --lib

# LLVM feature와 함께
cargo test -p pnix-runtime-llvm --lib --features llvm
```

### 통합 테스트

```bash
# AOT manifest 안정성 테스트
cargo test -p pnix-runtime-llvm --test aot_manifest_stability
```

### Doc 테스트

```bash
# 문서 예제 코드 테스트
cargo test -p pnix-runtime-llvm --doc
```

**참고**: 일부 doc 테스트는 문서 예제 코드 문제로 실패할 수 있습니다 (Unicode 문자, `?` 연산자 사용 등).

### 특정 테스트 실행

```bash
# 특정 테스트만 실행
cargo test -p pnix-runtime-llvm --lib test_jit_engine_creation

# 패턴 매칭
cargo test -p pnix-runtime-llvm --lib aot
```

## 예상 결과

### LLVM Feature 없이 (기본)

**Behavior (llvm=off)**:
- JIT/AOT compile/eval 경로는 **stub을 반환하지 않고** 항상 `RuntimeError::unimplemented(...)`로 명시적으로 실패합니다.
- 따라서 “그럴듯한 성공(stub compile ok)”로 사용자가 오해하는 케이스가 없습니다.

**단위 테스트** (`cargo test -p pnix-runtime-llvm --lib`):
```
running 35 tests
test tests::test_aot_compile_to_target ... ok
test tests::test_aot_compile_stub ... ok
...
test tests::test_jit_with_llvm_feature ... ignored
test tests::test_aot_with_llvm_feature ... ignored
...

test result: ok. 33 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out
```

**통합 테스트** (`cargo test -p pnix-runtime-llvm --test aot_manifest_stability`):
```
running 3 tests
test test_aot_layout_stable_paths ... ok
test test_aot_manifest_stable_ordering ... ok
test test_aot_manifest_all_targets_stable ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

**Doc 테스트** (`cargo test -p pnix-runtime-llvm --doc`):
```
running 5 tests
test crates/pnix-runtime-llvm/src/lib.rs - (line 178) ... ok
test crates/pnix-runtime-llvm/src/lib.rs - (line 143) ... ok
test crates/pnix-runtime-llvm/src/lib.rs - (line 62) ... FAILED
test crates/pnix-runtime-llvm/src/lib.rs - (line 86) ... FAILED
test crates/pnix-runtime-llvm/src/lib.rs - (line 15) ... FAILED

failures:
---- crates/pnix-runtime-llvm/src/lib.rs - (line 62) stdout ----
error: unknown start of token: \u{251c}
...
```

**참고**: Doc 테스트 실패는 문서 예제 코드의 문제이며, 런타임 기능과는 무관합니다.

### LLVM Feature와 함께

**LLVM이 설치된 경우**:
- 모든 테스트 통과 (feature-gated 테스트 포함)
- JIT 실행 성공
- AOT 컴파일 성공

**LLVM이 없는 경우**:
- 빌드 실패 또는 feature-gated 테스트만 무시됨
- 에러 메시지: "No suitable version of LLVM was found"

## 실행 환경

### 현재 테스트 환경 (2025-12-21)

- **OS**: macOS 26.2 (Darwin 25.2.0, x86_64)
- **Rust**: 1.92.0 (ded5c06cf 2025-12-08)
- **Cargo**: 1.92.0 (344c4567c 2025-10-21)
- **LLVM**: 미설치 (llvm-config 없음)
- **LLVM 환경 변수**: 미설정

### 지원되는 환경

- **macOS**: x86_64, ARM64 (M1/M2)
- **Linux**: x86_64
- **Windows**: x86_64 (MSVC)

### 최소 요구사항

- Rust 1.70 이상
- Cargo (Rust와 함께 설치됨)
- LLVM (JIT/AOT 기능 사용 시)

## LLVM 탐지 및 설정

### llvm-env-check.sh 빠른 점검

저장소 루트에서 다음 스크립트를 실행하면 `llvm-config` 존재 여부와
버전/프리픽스를 빠르게 확인할 수 있습니다.

```bash
./scripts/llvm-env-check.sh
```

실행 권한이 없으면:
```bash
bash scripts/llvm-env-check.sh
```

**스킵 기준**:
- `--features llvm` 없이 테스트할 때
- CI 환경에 LLVM을 설치하지 않는 경우
- `llvm-config`가 없는 환경에서 기능 확인이 필요 없을 때

### LLVM 설치 확인

```bash
# llvm-config 확인
which llvm-config

# 버전 확인 (설치된 경우)
llvm-config --version
```

### macOS에서 LLVM 설치

**Homebrew 사용**:
```bash
# LLVM 14 설치
brew install llvm@14

# PATH에 추가
export PATH="/opt/homebrew/opt/llvm@14/bin:$PATH"

# 환경 변수 설정
export LLVM_SYS_140_PREFIX=/opt/homebrew/opt/llvm@14
```

**설치 확인**:
```bash
which llvm-config
llvm-config --version
llvm-config --libs
```

### Linux에서 LLVM 설치

**apt 사용** (Ubuntu/Debian):
```bash
sudo apt-get update
sudo apt-get install llvm-14-dev libclang-14-dev

# 환경 변수 설정 (필요한 경우)
export LLVM_SYS_140_PREFIX=/usr/lib/llvm-14
```

**설치 확인**:
```bash
llvm-config-14 --version
llvm-config-14 --libs
```

### LLVM 환경 변수 설정

**LLVM_SYS_*_PREFIX 설정**:
```bash
# LLVM 14.0 예시
export LLVM_SYS_140_PREFIX=/opt/homebrew/opt/llvm@14

# LLVM 14.0 예시
export LLVM_SYS_140_PREFIX=/opt/homebrew/opt/llvm@14

# 자동 탐지 (llvm-config가 PATH에 있는 경우)
export LLVM_SYS_140_PREFIX=$(dirname $(dirname $(which llvm-config)))
```

**확인**:
```bash
env | grep LLVM_SYS
```

### llvm-config 찾기

**macOS**:
```bash
# Homebrew 설치 위치 확인
find /opt/homebrew -name llvm-config 2>/dev/null
find /usr/local -name llvm-config 2>/dev/null

# which 사용 (PATH에 있는 경우)
which llvm-config
```

**Linux**:
```bash
# 버전별 llvm-config 찾기
which llvm-config-14
which llvm-config-14

# 시스템 전체 검색
find /usr -name llvm-config* 2>/dev/null
```

### LLVM 버전 확인

**inkwell/llvm-sys 요구사항 확인**:
```bash
# Cargo.toml에서 inkwell 버전 확인
grep inkwell crates/pnix-runtime-llvm/Cargo.toml

# llvm-config 버전 확인
llvm-config --version
```

**버전 불일치 해결**:
- `Cargo.toml`의 inkwell 버전과 호환되는 LLVM 버전 설치
- 또는 호환되는 LLVM 버전으로 업데이트

## 문제 해결

### 문제 1: llvm-config를 찾을 수 없음

**증상**:
```
error: No suitable version of LLVM was found
```

**해결 방법**:
1. LLVM 설치 확인:
   ```bash
   which llvm-config
   ```

2. LLVM 설치 (macOS):
   ```bash
   brew install llvm@14
   export PATH="/opt/homebrew/opt/llvm@14/bin:$PATH"
   ```

3. 환경 변수 설정:
   ```bash
   export LLVM_SYS_140_PREFIX=/opt/homebrew/opt/llvm@14
   ```

4. 빌드 재시도:
   ```bash
   cargo clean
   cargo build -p pnix-runtime-llvm --features llvm
   ```

### 문제 2: 버전 불일치

**증상**:
```
error: LLVM version mismatch
```

**해결 방법**:
1. `Cargo.toml`에서 요구되는 LLVM 버전 확인:
   ```bash
   grep inkwell crates/pnix-runtime-llvm/Cargo.toml
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

### 문제 3: libclang 오류

**증상**:
```
error: libclang not found
```

**해결 방법**:
1. libclang 설치 (macOS):
   ```bash
   brew install llvm@14  # libclang 포함
   ```

2. LIBCLANG_PATH 설정:
   ```bash
   export LIBCLANG_PATH=/opt/homebrew/opt/llvm@14/lib
   ```

### 문제 4: 여러 LLVM 버전 설치됨

**증상**:
- 빌드 시 잘못된 LLVM 버전 사용

**해결 방법**:
1. 사용할 LLVM 버전 결정
2. 해당 버전의 `LLVM_SYS_*_PREFIX` 설정:
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

### 문제 5: 비표준 설치 위치

**증상**:
- LLVM이 표준 위치가 아닌 곳에 설치됨

**해결 방법**:
1. LLVM 설치 루트 디렉토리 찾기:
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

### 문제 6: Doc 테스트 실패

**증상**:
```
test crates/pnix-runtime-llvm/src/lib.rs - (line 62) ... FAILED
error: unknown start of token: \u{251c}
```

**원인**:
- 문서 예제 코드에 Unicode 문자 (Box Drawing) 사용
- 문서 예제 코드에 `?` 연산자 사용 (함수 반환 타입 문제)

**해결 방법**:
- 문서 예제 코드 수정 필요 (런타임 기능과는 무관)
- 단위/통합 테스트는 정상 작동

### 문제 7: 테스트가 느림

**증상**:
- 테스트 실행 시간이 오래 걸림

**해결 방법**:
1. 병렬 실행 확인:
   ```bash
   cargo test -p pnix-runtime-llvm --lib -- --test-threads=4
   ```

2. 특정 테스트만 실행:
   ```bash
   cargo test -p pnix-runtime-llvm --lib test_jit_engine_creation
   ```

### 문제 8: CI 환경에서 LLVM 탐지 실패

**증상**:
- CI 환경에서 LLVM을 찾을 수 없음

**해결 방법**:
1. CI 설정 파일에 환경 변수 추가:
   ```yaml
   # GitHub Actions 예시
   env:
     LLVM_SYS_140_PREFIX: /usr/lib/llvm-14
   ```

2. 또는 자동 탐지 스크립트 사용:
   ```bash
   export LLVM_SYS_140_PREFIX=$(dirname $(dirname $(which llvm-config-14)))
   ```

## 검증 명령어

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

# 환경 변수 확인
env | grep LLVM
```

## 추가 리소스

- [README.md](README.md): 전체 문서 및 API 참조
- [EXAMPLES.md](EXAMPLES.md): JIT/AOT 실행 예제 및 LLVM 탐지 팁
- [fixtures/](fixtures/): 테스트 픽스처 디렉토리

## 테스트 결과 기록

### 2025-12-21 테스트 결과

**환경**:
- OS: macOS 26.2 (Darwin 25.2.0, x86_64)
- Rust: 1.92.0
- Cargo: 1.92.0
- LLVM: 미설치

**결과**:
- 단위 테스트: 33개 통과, 2개 무시됨
- 통합 테스트: 3개 통과
- Doc 테스트: 5개 중 3개 실패 (문서 예제 코드 문제)

**결론**: LLVM feature 없이도 모든 단위/통합 테스트 통과. Doc 테스트는 문서 예제 코드 수정 필요.

### 2025-12-22 테스트 결과

**환경**:
- OS: macOS (로컬)
- LLVM: 미설치

**결과**:
- 단위 테스트: 44개 통과, 2개 무시됨 (feature-gated)
- 통합 테스트: 3개 통과
- Doc 테스트: 4개 통과

**결론**: LLVM feature 없이도 모든 단위/통합/Doc 테스트 통과. (llvm feature가 필요한 테스트는 의도적으로 ignored)
