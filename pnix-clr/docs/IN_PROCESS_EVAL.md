# 프로세스 내 C# 평가기 (실험적 스파이크)

**상태:** 실험적 스파이크 (2026-08-14) — 제품 기본값이 **아님**.  
**지원 기본값:** `Pnix.Clr.Eval.Source` / `Eval.File` — **프로세스 스폰** `pnix-clr`, JSON CLI 계약.  
**옵트인:** `Eval.SourceInProcess` / `FileInProcess` — **net10.0+** 전용.

관련: `csharp/Pnix.Clr/InProcessEval.cs` · 모노레포 `HOST_ENV_P2_P3.md` · `clr-meta/todo.md`.

---

## 프로세스 스폰이 제품 기본값인 이유

| 관심사 | 프로세스 스폰 (현재) | 프로세스 내 (목표) |
|---------|----------------------|-------------------|
| 격리 | 자식 프로세스; 크래시 ≠ 호스트 크래시 | 공유 AppDomain / ALC |
| TFM 혼합 | 호스트 C# net8이 net10 CLI 호출 가능 | TFM / 로드 컨텍스트 정렬 필요 |
| Guest AOT | props를 통한 선택적 Reference | `*.clj.dll` + ClojureCLR 런타임 로드 |
| 배포 크기 | PATH/env에 `pnix-clr` 필요 | 런타임 + artifact 번들 |
| 결정성 | CLI JSON 스키마 이미 게이트됨 | 동일 스키마 + 조용한 드리프트 없음 |

프로세스 스폰은 프로세스 내 경로가 도입된 뒤에도 **기본값**으로 유지됩니다 (옵트인).

---

## 목표

C# host-main 코드가 프로세스를 스폰하지 **않고** `.px` / 인라인 소스를 평가하고, CLI 경로와 **동일한** `EvalResult` 형태(`schema`, `outcome-kind`, `value`/`error`)를 반환하도록 한다.

정직한 범위:

- **포함:** clr limb에서 host-bound pnix의 pure eval (`pnix-clr -e` / 파일과 동일 의미).
- **제외:** 전체 ClojureCLR REPL, 임의 multi-ns Clojure 프로젝트, “모든 배포에서 프로세스 스폰 대체”.

---

## 임베딩 옵션 (정직성 비용 순)

### A. 기존 CLI 프로토콜 위 managed host API (얇은 계층)

평가를 호출마다 `Process.Start` 하지 않고, 장기 생존 helper 프로세스 / named pipe에서 유지.

- **장점:** JSON 계약을 재사용; 진짜 in-proc보다는 약함.
- **단점:** 여전히 프로세스; “프로세스 내”가 아님.
- **판정:** 중간 단계; 스폰 비용이 고통이고 임베딩이 아닐 때만.

### B. AssemblyLoadContext에서 guest AOT + ClojureCLR 로드 (선호 연구 경로)

1. Ship/export가 이미 `runtime-artifact/*.clj.dll` + `Pnix.Clr` multi-TFM을 제공.
2. 호스트가 ClojureCLR substrate(net10 제품 경로 — `TFM_POLICY.md` 참조)를 **격리된** ALC에 로드.
3. CLI가 `-e` / 파일에 쓰는 것과 같은 엔트리(또는 CLI 형태 JSON / EDN을 내는 전용 managed entrypoint)를 호출.
4. 셸 아웃 없이 `EvalResult`로 매핑.

**코드 전에 풀어야 할 블로커:**

1. **Substrate 패키지** — 호스트 옆에 있어야 할 어셈블리(Clojure.Main, deps, 버전 핀 1.12.3-alpha8).
2. **ALC 격리** — 언로드, 중복 타입 identity, 기본 컨텍스트로 누수 없음.
3. **TFM** — 제품 guest AOT는 net10; net8 전용 호스트는 프로세스 스폰만 유지 가능.
4. **스레드 / apartment / statics** — ClojureCLR init은 ALC당 한 번; reentrancy 문서화.
5. **패리티 게이트** — 고정 코퍼스에 대해 프로세스 경로와 바이트 동일 또는 구조적 동등 JSON.

### C. ClojureCLR 없이 CLR에서 pnix의 pure managed 재구현

C# / F#로 평가기 재작성. **현재 거부** — 두 번째 의미 소스; host-bound 제품 교리에 위배.

---

## 수락 스케치 (소유자가 구현을 끌어올 때)

1. 설계 노트(이 파일)가 정확을 유지.
2. 옵트인 API, 예: `Eval.SourceInProcess` / `EvalOptions.Execution = InProcess`, **기본값은 Process**.
3. 게이트 스크립트: 프로세스와 프로세스 내 결과가 `outcome-kind` + value JSON에서 일치하는 N개 fixture (또는 문서화된 held diff만).
4. 부정: substrate 누락 시 actionable 메시지로 fail closed (hang 없음, 조용한 null 없음).
5. 문서: README 표에서 Process = supported, InProcess = 게이트 그린 전까지 experimental.
6. 임베딩만으로 Stage15/N 또는 Trusting-Trust 주장 **없음**.

---

## 비목표

- 첫 스파이크에 nuget.org 요구 (로컬 export 레이아웃으로 충분).
- `clojure-clr` facade 또는 bootstrap multi-ns 스토리 교체.
- 임의 사용자 Clojure 프로젝트를 프로세스 내에 로드.

---

## 착륙한 스파이크 (2026-08-14)

| 조각 | 위치 |
|-------|----------|
| 구현 | `csharp/Pnix.Clr/InProcessEval.cs` (net10 `#if`) |
| API | `Eval.SourceInProcess` / `FileInProcess` |
| 패리티 예제 | `csharp/examples/InProcessParity/` |
| 게이트 | `bin/pnix-clr-inprocess-eval-gate` (옵트인; 아직 `pnix-clr-gate`에 없음) |

### 동작 방식

1. **substrate** (`PNIX_CLR_SUBSTRATE` 또는 checkout `clojure-clr-…/net10.0/publish`)와 **artifact** (`PNIX_CLR_ARTIFACT`)를 resolve.
2. `AssemblyLoadContext.Default.Resolving`을 훅하여 guest AOT DLL이 `Clojure.dll`을 찾도록 함.
3. substrate 어셈블리 preload; `pnix-clr.evaluator` / `main` / `json`을 `require`.
4. reflection으로 `eval-source`(또는 `eval-file`) + `projection` + `write-json` 호출 — `-main`의 `Environment.Exit` **없음**.
5. 프로세스 경로와 같은 `EvalResult` 형태로 파싱.

### Env 계약

| 변수 | 역할 |
|----------|------|
| `PNIX_CLR_ARTIFACT` | Guest AOT 디렉터리 (`manifest.json` + `*.clj.dll`) |
| `PNIX_CLR_SUBSTRATE` | ClojureCLR net10 publish 디렉터리 (`Clojure.dll`) |
| `PNIX_CLR_ROOT` | 호스트 루트 (import confinement) |
| `PNIX_CLR` | 패리티 비교에 여전히 쓰이는 프로세스 경로 |

### 검증된 코퍼스 (게이트)

- `1 + 2` → 3  
- `true && !false` → true  
- `if true then 40 + 2 else 0` → 42  
- `1 / 0` → failed / division-by-zero (패리티)  
- Substrate 누락 → `NotSupportedException` (fail closed)

### “admitted” 전에 여전히 열린 항목

- [x] 더 넓은 패리티 코퍼스 (14 source 케이스 + file + 2 negatives) — 게이트 2026-08-14
- [ ] Collectible isolated ALC — **현재 blocked**: ClojureCLR guest AOT가
  `Assembly.Load`로 **기본** 컨텍스트에 초기화됨; collectible ALC는 이미
  로드된 substrate 타입을 dual Resolving 없이 볼 수 없고, dual Resolving은
  Default로 붕괴. 문서화된 tradeoff; ALC-aware load를 지원하는 substrate에서만
  재검토.
- [x] substrate+artifact 있을 때 `pnix-clr-gate`에 연결 (`PNIX_CLR_INPROCESS_GATE=0`이면 스킵)
- [x] Reentrancy 정책 — **직렬화**: eval-source 주변 global lock
  (ClojureCLR RT는 process-wide). 동시 호출자는 대기; `*Async`
  헬퍼는 존재하나 같은 lock을 공유. multi-threaded RT 아님.
- [ ] net8 호스트 스토리 (프로세스 스폰 유지)
- [ ] Unload / collectible ALC (blocked — 위 참조)
- [ ] 임베딩으로 Stage15/N 주장 없음

### 실행

```bash
export PNIX_CLR_ROOT=$PWD
export PNIX_CLR_ARTIFACT=$PWD/pnix-clr/target/runtime-artifact
export PNIX_CLR_SUBSTRATE=$PWD/clojure-clr-clojure-1.12.3-alpha8/Clojure/Clojure.Main/bin/Release/net10.0/publish
export PNIX_CLR=$PWD/bin/pnix-clr
./bin/pnix-clr-inprocess-eval-gate

# Product aggregate: auto-runs when substrate+artifact exist.
# Skip: PNIX_CLR_INPROCESS_GATE=0 ./bin/pnix-clr-gate
./bin/pnix-clr-gate

# HelloPnix demo (net10, same env):
dotnet run --project csharp/examples/HelloPnix -c Release -f net10.0 -- --inprocess '1 + 2'
```

### nuget.org

**제품 목표 아님** — 로컬 `pack-pnix-clr-nupkg` / file feed만 (소유자).
