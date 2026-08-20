# LLVM Runtime Unsafe Code Audit


> 2026-06-02 update: former client/control runtime material has been absorbed into pnixc-meta mirror primitives. Legacy client/control names below are fixture/schema/path compatibility or historical migration evidence; new implementation work should target pnixc-meta `.px` owners and replacement host adapters.

## Current convergence note (2026-03-13)

This crate document describes one implementation, testing, or audit surface within the current convergence plan.
It does not redefine the repository-wide ontology: the canonical base remains the shared substrate for state, meaning, observation, plan, and evidence.
Read this crate as one adapter/runtime/lowering surface under that substrate, with `pnix` as code/execution projection, `freecat` as spatial/world-model projection, and replacement projection adapters as non-owner control/governance surfaces; former `puck` labels are historical.

## Unsafe 코드 사용 현황 및 안전성 검증

### 1. jit.rs

#### StringPtrGuard (14-39줄)
**위치**: `struct StringPtrGuard` 및 `impl Drop`
**안전성**: ✅ 안전
- **이유**: 
  - FFI 경계에서 반환된 포인터를 안전하게 관리
  - Drop에서 null 체크 후 해제
  - free_fn가 제공되면 사용, 없으면 libc::free 사용
- **검증**: 포인터는 JIT 코드에서 반환된 유효한 메모리 주소이며, 호출자가 해제 책임을 가짐

#### read_c_string_bytes (43-61줄)
**위치**: `unsafe fn read_c_string_bytes`
**안전성**: ✅ 안전 (경계 검증 완료)
- **이유**:
  - null 포인터 체크
  - CStr::from_ptr 사용 (표준 라이브러리)
  - 최대 길이 제한 (1MB)으로 경계 검증
- **검증**: 
  - MAX_STRING_LENGTH = 1MB로 제한하여 보안 취약점 방지
  - CStr::from_ptr은 NUL-terminated 문자열을 안전하게 읽음

#### FFI 호출 (1157-1159줄, 1582-1584줄)
**위치**: `entry_fn_ptr.call()`
**안전성**: ✅ 안전 (패닉 캐치 완료)
- **이유**:
  - `std::panic::catch_unwind`로 패닉 처리
  - FFI 경계에서 undefined behavior 방지
- **검증**: JIT 코드 패닉 시 안전하게 처리됨

### 2. lib.rs

#### GEP 연산 (3048줄, 3157줄, 3245줄)
**위치**: `builder.build_gep()`
**안전성**: ✅ 안전 (인덱스 검증 완료)
- **3048줄**: `current_ptr` 증가
  - 인덱스는 항상 유효한 범위 내 (concat 문자열 길이)
- **3157줄**: 빈 문자열 배열 GEP
  - [0, 0] 인덱스는 항상 유효 (빈 배열의 첫 바이트)
- **3245줄**: 입력 배열 GEP
  - 인덱스는 enumerate로 생성되어 항상 유효 범위 내
- **검증**: 모든 GEP 연산은 유효한 인덱스 범위 내에서 수행됨

#### 포인터 역참조 (1285줄)
**위치**: 문자열 포인터 읽기
**안전성**: ✅ 안전 (null 체크 및 경계 검증)
- **검증**: null 체크 및 유효성 검증 완료

### 3. ffi.rs

#### 동적 라이브러리 로딩 (211줄, 230줄, 301줄, 323줄, 345줄)
**위치**: `DynamicLibrary::load()`, `library.get()`
**안전성**: ✅ 안전 (심볼 검증 완료)
- **이유**:
  - 심볼 이름 검증
  - null 포인터 체크
  - CStr::from_ptr 사용
- **검증**: 모든 동적 라이브러리 로딩은 검증된 심볼만 사용

## 결론

모든 unsafe 코드는 적절한 검증과 안전 장치를 가지고 있습니다:
- ✅ null 포인터 체크
- ✅ 경계 검증 (최대 길이 제한)
- ✅ 패닉 처리 (FFI 경계)
- ✅ 인덱스 범위 검증 (GEP 연산)
- ✅ 메모리 해제 보장 (RAII 패턴)

**추가 개선 사항**:
- 문서화 개선 (각 unsafe 블록에 주석 추가)
- 테스트 커버리지 향상
- 정적 분석 도구 활용
