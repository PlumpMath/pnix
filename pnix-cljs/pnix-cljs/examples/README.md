# examples — pnix-cljs (ClojureScript / Node)

> **성숙도:** experimental seed. 여기서는 **인정된 표면**만 다룬다
> (parse/eval, Node 라이브러리 import, Done/Failed 결과 모양).  
> 다섯 호스트 패리티, Stage15/N, 완전한 meta-circular 타워를 **주장하지 않는다**.

## 형제 호스트와 규모

| 호스트 | 카탈로그 크기 (대략) |
|--------|----------------------|
| clj / hy | 연구용 dense 카탈로그 |
| rs | 중간 pillar 카탈로그 |
| **cljs** | **코어 00–15** (이 트리) — 실표면이 있을 때만 확장 |

공유 테마 표: 모노레포 [`examples/EXAMPLES_BALANCE.md`](../../../examples/EXAMPLES_BALANCE.md).

## 패턴

번호 슬라이스마다 보통:

- `README.md` — 무엇 / 왜 / 실행 방법
- 필요 시 호스트 네이티브 limit 또는 way 파일 (JS / `.px`)

## 카탈로그

| 디렉터리 | 테마 |
|----------|------|
| [`00-foundation`](00-foundation/) | dist 모듈로 seed eval |
| [`01-pure-eval-boundary`](01-pure-eval-boundary/) | plain Node `eval` vs pnix 평가 경계 |
| [`02-host-library-import`](02-host-library-import/) | 로컬 라이브러리 export + `evalFile` |
| [`03-outcome-projection`](03-outcome-projection/) | Done / Failed (조용한 throw만 아님) |
| [`04-js-embed-pnix`](04-js-embed-pnix/) | host-main: JS가 `.px` 를 돌림 |
| [`05-experimental-honesty`](05-experimental-honesty/) | 이 호스트가 **주장하지 않는 것** |
| [`06-meta-pair-boundary`](06-meta-pair-boundary/) | pnix-cljs vs cljs-meta 역할 |
| [`07-builtins-surface`](07-builtins-surface/) | typeOf / attrNames / getAttr seed |
| [`08-file-eval`](08-file-eval/) | `evalFile` 로 디스크 `.px` |
| [`09-rec-let-select`](09-rec-let-select/) | rec · let · select 문법 |
| [`10-value-json-projection`](10-value-json-projection/) | 관측용 JSON 투영 (타입 권위 아님) |
| [`11-list-higher-order`](11-list-higher-order/) | map · filter · genList · concatLists |
| [`12-with-assert-merge`](12-with-assert-merge/) | with · assert · ++ · // |
| [`13-pattern-lambda`](13-pattern-lambda/) | attrset formal · 기본값 · 커리 |
| [`14-tryEval`](14-tryEval/) | tryEval 성공/throw |
| [`15-string-and-version`](15-string-and-version/) | substring · concatStringsSep · splitVersion |

## 실행 (대표)

```bash
cd pnix-cljs
./bin/build-cljs   # dist 가 없거나 오래됐을 때
node pnix-cljs/examples/00-foundation/node.js
# 라이브러리 import 스모크 (모노레포):
#   ./bin/pnix-cljs-library-smoke
```

모노레포 `examples/host-import/cljs/` 도 참고.
