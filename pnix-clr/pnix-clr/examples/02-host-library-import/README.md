# 02 — host library import (local feed)

## 무엇을

host-main: C# 프로젝트가 **로컬 export** `Pnix.Clr` 를 참조한다. nuget.org
게시는 제품 게이트가 아니다 (dropped / owner local-only).

## 실행

```bash
cd pnix-clr
./bin/pnix-clr-library-smoke
# monorepo host-import:
#   examples/host-import/clr/smoke
```

## 관련

- `csharp/examples/HelloPnix/`
- monorepo `HOST_IMPORT.md` § clr
