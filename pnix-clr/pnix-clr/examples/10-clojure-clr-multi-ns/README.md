# 10 — ClojureCLR 다중 네임스페이스 (bootstrap)

## 쉽게 말하면 (비유)
지금까지 예제는 **pnix 게스트**(`.px`)를 돌렸다. 이 예제는 다르다 — 여기서
"프로그램"은 **호스트 ClojureCLR 코드 자체**(`demo.lib`, `demo.main` 두
`.clj` 네임스페이스)이고, `clojure-clr` bootstrap 경로로 디스크 위 여러
파일을 `:require`로 로드해 실행한다. pnix-clr 제품 CLI와는 완전히 별개
경로다.

## 무엇을
`demo.lib`(라이브러리 네임스페이스, `add` 함수)와 `demo.main`(entry
네임스페이스, `demo.lib`을 require)이라는 2개 네임스페이스를 호스트
ClojureCLR bootstrap으로 로드·실행한다. 파사드 `clojure-clr -e`가 아니라
멀티파일 **bootstrap** 경로.

## plain .NET의 한계
이건 .NET 자체의 한계라기보다, "여러 파일에 걸친 Clojure 네임스페이스를
`CLOJURE_LOAD_PATH` 기반으로 부트스트랩하는" 것이 net10 Clojure 호스팅에서
당연히 되는 게 아니라 명시적으로 배선해야 하는 경로임을 보여준다 — 이
템플릿(`examples/clojure-clr-project/`)이 그 배선의 최소 예시다.

## pnix-clr의 방식 (실행 결과)
```
$ cd examples/clojure-clr-project
$ ./smoke
clojure-clr-project smoke: PASS (42)
```
`demo.main/-main`이 `demo.lib/add 20 22`를 호출해 `42`를 찍는다 —
`:require`로 네임스페이스 간 의존이 실제로 해석됨을 확인한다.

## 어디에 쓰나
pnix-clr 제품과 무관하게, ClojureCLR 자체를 net10 위에서 멀티파일
프로젝트로 부트스트랩하는 최소 템플릿이 필요할 때(호스트 언어 쪽 day-1
경로).

## 실행
```bash
cd pnix-clr/examples/clojure-clr-project
./smoke
# => PASS (42)
```

`pnix-clr` 제품 CLI와 별개 — 호스트 언어 쪽 day-1 경로.
