# pnix-clj 파운데이션

`pnix-clj/pnix-clj` 에서:

```sh
clojure -M examples/00-foundation/basic.clj
clojure -M examples/00-foundation/interop.clj
clojure -M examples/00-foundation/meta_circular.clj
```

이 명령들은 제품 메커니즘을 보여 준다.

자동 호스트 코드 생성을 수행하는 예제는 없다. `compile-source` 는 기존
clj-meta 메커니즘으로 lower·실행한다는 뜻이며, 새 의미 소유자를 만들지 않는다.

더 넓은 경로: [FOUNDATION_PATH.md](../FOUNDATION_PATH.md).
