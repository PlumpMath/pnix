# 02 — 호스트 라이브러리 import (로컬 피드)

## 무엇을

host-main: C# 프로젝트가 **로컬 export** `Pnix.Clr` 를 참조한다. nuget.org
게시는 제품 게이트가 아니다 (dropped / owner 로컬 전용).

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
