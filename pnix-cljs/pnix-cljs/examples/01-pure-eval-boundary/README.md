# 01 — pure eval boundary (Node)

## 쉽게 말하면

Node의 `eval` / 동적 함수 생성은 게스트 언어 경계가 아니다. 파일·네트워크·전역
오염을 막지 못한다. pnix-cljs는 **문자열 게스트 소스**를 파서/평가기 경계로
넘기고, 결과를 Done/Failed 모양으로 돌려준다.

## plain Node 한계 (`limit_node.js`)

주석으로만 위험 패턴을 표시한다 (실행하지 않음).

## pnix-cljs 방식 (`pnix_cljs_way.js`)

`evalSource`로 순수 산술과 의도적 오류(div0)를 구조화해 본다.

## 실행

```bash
cd pnix-cljs
./bin/build-cljs   # dist 필요 시
node pnix-cljs/examples/01-pure-eval-boundary/limit_node.js
node pnix-cljs/examples/01-pure-eval-boundary/pnix_cljs_way.js
```
