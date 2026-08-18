# 02 — 호스트 라이브러리 import (로컬 피드)

## 쉽게 말하면 (비유)
`npm install`/`pip install` 대신 로컬 폴더를 그대로 참조하는 것과 같은
감각이다 — `Pnix.Clr`는 **로컬로 export된 dual-TFM 라이브러리**로,
C# 프로젝트가 project reference나 로컬 nupkg 피드로 붙여 쓴다. nuget.org
공개 배포는 이 제품의 게이트가 아니다.

## 무엇을
host-main: C# 프로젝트가 로컬 export `Pnix.Clr`를 참조해 pnix를 API로
호출한다. 라이브러리 export 자체(dual-TFM 레이아웃, props/targets)와
그 위의 HelloPnix process-spawn 스모크를 확인한다.

## plain .NET의 한계
표준 NuGet 배포 흐름은 공개 레지스트리를 전제한다 — 아직 공개 배포하지
않는 experimental 라이브러리를 "제대로 된 패키지처럼" 참조하려면 로컬
피드/project reference 배선을 직접 해야 한다. 이 예제가 보여주는 건 그
배선이 실제로 dual-TFM(net8/net10) 레이아웃으로 동작한다는 것.

## pnix-clr의 방식 (실행 결과)
```
$ ./bin/pnix-clr-library-smoke
== pnix-clr-library-smoke (local feed only) ==
  OK  export layout (dual TFM + props)
  OK  net10 API includes SourceInProcess / FileInProcess
== pnix-clr-nupkg-smoke (local feed only) ==
  OK  library layout (net8 + net10 + props/targets)
  OK  packed Pnix.Clr.0.1.0.nupkg
  OK  nupkg contains dual-TFM DLLs + build props + nuspec
== summary: PASS (local nupkg only; not nuget.org) ==
  OK  HelloPnix process-spawn => 3
== summary: PASS (local clr library only; not nuget.org) ==
```

## 어디에 쓰나
.NET 애플리케이션에 pnix 평가를 라이브러리로 내장하고 싶을 때(공개 NuGet
배포 전, 로컬 project reference 단계에서).

## 실행
```bash
cd pnix-clr
./bin/pnix-clr-library-smoke
# 모노레포 host-import:
#   examples/host-import/clr/smoke
```

## 관련
- `csharp/examples/HelloPnix/`
- 모노레포 `HOST_IMPORT.md` § clr
