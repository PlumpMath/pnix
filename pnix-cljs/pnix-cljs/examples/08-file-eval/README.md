# 08 — 파일 평가 (`evalFile`)

## 쉽게 말하면 (비유)
지금까지 대부분의 예제는 pnix 소스를 JS **문자열**로 인라인했다. 실전에서는
소스가 `.px` **파일**로 존재하는 경우가 더 흔하다 — 이 예제는 그 경로
(파일 → 평가)를 라이브러리 표면으로 확인한다.

## 무엇을
문자열뿐 아니라 **파일 경로**로 `.px`를 읽는다. Node 라이브러리 export의
`evalFile`/`evalFileValueJson` 표면을 `sample.px`로 확인.

## plain Node의 한계
`fs.readFileSync(path, "utf8")` 자체는 문자열만 준다 — 그걸 pnix 문법으로
파싱/평가하려면 결국 `evalSource`를 다시 호출해야 한다. `evalFile`은 그
"읽기+평가"를 한 호출로 묶어주는 라이브러리 표면일 뿐 별도 파일 I/O 메커니즘이
아니다 — 이 예제는 그 두 스텝이 하나로 묶여 있다는 것 자체를 보여준다.

## pnix-cljs의 방식 (`pnix_cljs_way.js`)
```js
pnix.evalFile(px)          // => { outcome_kind: 'done', value: 42n }
pnix.evalFileValueJson(px) // => 42  (JSON 문자열)
```
`evalFile`은 구조화된 결과 객체를, `evalFileValueJson`은 값만 JSON으로
직렬화해 돌려준다 — 호출자가 필요한 형태를 고를 수 있다.

## 어디에 쓰나
설정/스크립트 파일을 디스크에서 로드해 평가해야 하는 CLI 도구, 빌드
스크립트에서 `.px` 산출물을 읽어 JSON으로 넘길 때.

## 실행
```bash
cd pnix-cljs
./bin/build-cljs
node pnix-cljs/examples/08-file-eval/pnix_cljs_way.js
```
