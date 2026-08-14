# 08 — pnix-meta 스모크 (형제 트리)

## 무엇을

형제 `pnix-meta` 정규 케이스를 `--pnix-meta-smoke` 로 돌린다.
portable `.px` 의미는 형제 트리에서 로드하고, 호스트는 평가·핀 비교만 한다.

## 비주장

- 이 예제가 monorepo 밖 단독 배포를 보장한다
- 전체 conformance Phase D

## 실행

```bash
cd pnix-clr
./bin/pnix-clr --pnix-meta-smoke
# 개별:
#   ./bin/pnix-clr ../pnix-meta/corpus/conformance/bool-01.px
```
