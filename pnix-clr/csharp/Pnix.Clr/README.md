# Pnix.Clr — pnix-clr용 C# 호스트 라이브러리

**C#** 프로젝트가 pnix (`.px`)를 평가하고 캐시 경로를 손으로 복사하지 않고
**CLR guest AOT DLL**을 연결할 수 있게 하는 호스트 언어 임포트 표면.

**교리 (전 호스트):** monorepo [`../../../HOST_DEV_ENV.md`](../../../HOST_DEV_ENV.md).  
이 패키지는 clr 호스트 전용 **host-main C#** 표면이며 — 이식 가능한 멀티호스트
`.px` 라이브러리가 아니다.

## 이것이 / 아닌 것

| 표면 | 역할 |
|------|------|
| `Pnix.Clr.Eval.Source` / `File` | **지원:** 프로세스 스폰 `pnix-clr`, JSON CLI 결과 파싱 |
| `Eval.SourceInProcess` / `FileInProcess` | **실험 (net10+)** — ALC/substrate 임베드; gate `bin/pnix-clr-inprocess-eval-gate`; [`docs/IN_PROCESS_EVAL.md`](../../docs/IN_PROCESS_EVAL.md) 참고 |
| `lib/net10.0/runtime-artifact/*.clj.dll` | Guest AOT (ClojureCLR 바인딩) |
| `build/Pnix.Clr.props` + `.targets` (export 레이아웃; 소스는 `msbuild/`) | MSBuild HintPath / Reference 배선 |

이식 가능한 멀티호스트 `.px` 바이트코드 패키지가 **아니다**. 아티팩트는
pnix의 CLR 림에 **호스트 바인딩**된다.

## 빠른 시작 (`export-pnix-clr-library` 또는 HM `pnix-clr-library` 이후)

```csharp
using Pnix.Clr;

// 인라인
var r = Eval.Source("1 + 2").EnsureDone();
Console.WriteLine(r.Value); // 3

// 파일 임포트 (.px)
var f = Eval.File("examples/hello.px").EnsureDone();
```

환경 (`dot-nix` / export가 설정):

- `PNIX_CLR` — `pnix-clr` 실행 파일 경로  
- `PNIX_CLR_ROOT` — 체크아웃 또는 캐시 트리 루트  
- `PNIX_CLR_ARTIFACT` — runtime-artifact 디렉터리  
- `PNIX_CLR_LIBRARY` — export된 라이브러리 루트 (이 레이아웃)

## MSBuild

```xml
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
    <!-- 선택: guest AOT DLL도 Reference -->
    <!-- <PnixClrImportGuestDlls>true</PnixClrImportGuestDlls> -->
  </PropertyGroup>
  <Import Project="$(PNIX_CLR_LIBRARY)/build/Pnix.Clr.props"
          Condition="'$(PNIX_CLR_LIBRARY)' != ''" />
  <Import Project="$(PNIX_CLR_LIBRARY)/build/Pnix.Clr.targets"
          Condition="'$(PNIX_CLR_LIBRARY)' != ''" />
</Project>
```

또는 `pnix-clr-refs` / `pnix-clr-library`로 절대 경로 출력.

## 빌드 / export (pnix-clr 체크아웃에서)

```bash
./bin/build-pnix-clr-artifact          # 없으면 guest AOT
./bin/export-pnix-clr-library          # → target/pnix-clr-library
# 또는: nix run .#pnix-clr-library
./bin/export-pnix-clr-library          # dual-TFM DLL + props 재빌드
./bin/pnix-clr-library-smoke           # export + API 검사 + nupkg + HelloPnix
./bin/pack-pnix-clr-nupkg              # 로컬 .nupkg만
# MSBuild 샘플: ../Directory.Build.props.sample
# 인프로세스: ../../docs/IN_PROCESS_EVAL.md  ·  HelloPnix --inprocess
# HelloPnix 기본은 ProjectReference (최신 API); PnixClrUseExport=true 로
# PNIX_CLR_LIBRARY DLL Reference 강제.
```
## 관련 CLI

- `pnix-clr` / `pnix-clr-pnix` — eval / REPL  
- `clojure-clr` — clr-meta 위 focused `-e` / 단일 파일 파사드  
- `pnix-clr-refs` — artifact DLL 경로 출력 (dot-nix 헬퍼)
