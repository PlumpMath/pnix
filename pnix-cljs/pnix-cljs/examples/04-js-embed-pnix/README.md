# 04 — JS에 pnix 임베드 (host-main)

## 쉽게 말하면 (비유)
`00`~`03`은 pnix 소스를 JS **문자열 리터럴**로 넘겼다. 이건 그 소스가
**별도 `.px` 파일**에 살면서, 호스트 JS 프로그램이 그걸 읽어 평가하는
드라이버 역할을 하는 모양이다 — 다른 언어는 `.px`를 read-time에 그냥
문자열로 취급하지만, 여기서는 파일 자체가 pnix 게스트 표현으로 승격된다.

## 무엇을
호스트 프로그램(JS)이 드라이버이고, `snippet.px`(`let scale = n: n * 10; in scale 4`)
가 게스트 표현이다. `fs.readFileSync`로 읽은 텍스트를 `evalSource`에
넘겨 `40`을 얻는다. 반대 축(pnix-main REPL)은 모노레포 flake
`.#pnix-cljs-pnix` 계열 — `17-repl-session` 참고.

## plain Node의 한계
`.px` 파일을 그냥 `fs.readFileSync`로 읽으면 **문자열**일 뿐이다 — pnix
문법으로 파싱하거나 평가할 표준 JS 메커니즘이 없다. 별도 언어 파일을
호스트 프로그램에 "임베드"한다는 개념 자체가 plain Node에는 없다.

## pnix-cljs의 방식 (`host_main.js`)
```js
const source = fs.readFileSync(path.join(__dirname, "snippet.px"), "utf8");
pnix.evalSource(source);
// => { outcome_kind: 'done', value: 40n }
```
호스트가 `.px` 파일을 텍스트로 읽어 `evalSource`에 넘기기만 하면, 그 순간
pnix 문법/의미로 평가된 값을 돌려받는다 — read-time에 죽은 문자열이 아니라
평가 가능한 게스트 표현이 된다.

## 어디에 쓰나
설정 파일/스크립트를 `.px`로 분리해 두고 Node 애플리케이션에서 로드해
평가하고 싶을 때(빌드 스크립트, 설정 DSL 등).

## 실행
```bash
node pnix-cljs/examples/04-js-embed-pnix/host_main.js
```
