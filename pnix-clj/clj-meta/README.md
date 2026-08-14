# clj-meta

`pnix-clj`를 위한 깨끗한 Clojure 호스트 언어 증명 레인입니다.

이 디렉터리에는 서로 다른 두 레인이 있습니다.

- `stage7-gate.sh`는 stock Clojure 1.12.5용 재현 가능 빌드 레인입니다.
  결정적 호스팅 재빌드를 증명하며, 메타순환 컴파일러 증명이 아닙니다.
- `src/pnix/clj_meta/compiler.clj`와 `selfhost.clj`는 메타 레인입니다.
  Clojure로 작성된 analyzer/ASM 바이트코드 컴파일러와 결정적 self-host
  검사입니다.

소스 스냅샷은 이 디렉터리 바깥에 있습니다.

```sh
clojure-clojure-1.12.5/
```

생성된 stage 트리, 로그, 증명 receipt는 `clj-meta/` 아래에 두고 git에서
무시합니다.

```text
clj-meta/work/
clj-meta/logs/
clj-meta/proof/
```

## 상태 / 주 게이트

[STATUS.md](STATUS.md)를 보세요. 주 게이트: `./bin/clj-meta-gate` (실무 하한:
`./bin/clj-meta-gate selfhost`).

## 명령

```sh
clj-meta/stage7-gate.sh status
clj-meta/stage7-gate.sh stage7-check
clojure -M:compiler-smoke
clojure -M:conformance
clojure -M:selfhost-check
clojure -M:mirror-smoke
clojure -M:audit-self-source
clojure -M:gate
```

`stage7-check`는 전체 호스팅 리플레이를 빌드합니다.

```text
stage1 -> stage2 -> stage3 -> stage4 -> stage5 -> stage6 -> stage7
```

stage 3부터 7까지는 이전 stage의 Java runtime-only Clojure 호스트 jar로 동일한
Clojure 1.12.5 소스 스냅샷을 컴파일한 뒤, 생성된 Clojure jar를 안정적인 zip
entry 이름과 entry 내용 해시로 이전 stage와 비교합니다.

stage 스냅샷은 업스트림 빌드를 JVM 프로세스 간에 결정적으로 만들기 위해
`clj-meta/work/` 안에서만 패치합니다. locals clearing을 끄고, closed-over
locals를 정렬한 뒤 fn/reify 생성자 필드를 방출합니다.

최종 stage는 다음을 컴파일하고 실행합니다.

- `pnix.clj-meta.core`
- `pnix.clj-meta.stm`

## 경계

어느 레인도 JVM 없는 Clojure self-hosting을 주장하지 않습니다. JVM, Clojure
소스 트리 아래의 Java 런타임 클래스, Maven, 로컬 JDK는 영구 기판입니다.
재현 가능 빌드 레인 또한 Clojure로 작성된 컴파일러를 주장하지 않습니다.
호스팅 Java 컴파일러 인프라로 stock Clojure를 다시 빌드합니다.

이 디렉터리는 `pnix-clj` 의미론, brain codec, redb ingest도 소유하지 않습니다.
`pnix-clj`가 올라탈 수 있는 Clojure 호스트/컴파일러 기판을 준비하고 검증합니다.
