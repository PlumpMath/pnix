# pnix-cljs 스코프 잠금

## 제품 소유

`pnix-cljs`는 ClojureScript/JavaScript 프로젝션 메커니즘만 소유한다:

- PNIX 소스 토큰화 및 파싱
- 네이티브 ClojureScript 값 위 평가
- 명목상 machine outcome 값
- Node 및 CommonJS interop

## 의미 소유

이 저장소에는 이식 가능한 언어 의미를 소유하는 별도 저장소-수준 트리가
없다. 이 호스트는 복사된 Clojure/JVM 런타임 트리를 유지하지 않고 자체
네이티브 seed로 파싱/평가한다. 네이티브 seed는 공유 적합 코퍼스가 연결되고
all-host gate로 비교되기 전까지 정규 크로스호스트 패리티를 주장할 수 없다.

## 이 seed에서 제외

- service policy 및 admission status
- evaluator fallback
- proof-receipt-gated execution
- JVM/Java/ASM 구현 코드
- retained effects 및 filesystem execution
- automatic application code generation
- authoritative string-encoded types
- 복사된 `stdlib`, `pnixc-pnix`, `pnix-mirror-runtime`, 또는 domain-content roots
