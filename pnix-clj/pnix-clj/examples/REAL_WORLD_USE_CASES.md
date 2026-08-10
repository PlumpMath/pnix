# REAL WORLD USE CASES

실무에서 어디에 붙일 수 있는지 짧게 정리한 문서다.

## 1. AI가 만든 config 자동 검토

도메인:

```text
SaaS platform
SRE
Kubernetes/Nix module generator
feature flag system
```

문제:

```text
AI가 설정을 만들었다.
바로 적용하면 빠진 값, 중복 key, 몰래 host 읽기가 섞일 수 있다.
```

볼 예제:

```text
83, 85, 86, 77, 81
```

적용 코드 모양:

```clojure
{:source generated-config
 :checks {:pure? pure?
          :status status
          :reason reason}
 :decision (if (= :ok status)
             :auto-approve
             :manual-review)}
```

## 2. CI semantic smoke

도메인:

```text
compiler team
DevTools
runtime migration
build/release engineering
```

문제:

```text
테스트는 통과했는데, evaluator/lowering/runtime이 같은 뜻인지 모른다.
```

볼 예제:

```text
84, 22, 57, 71, 90, 91, 92
```

적용 코드 모양:

```clojure
{:case source
 :direct direct-result
 :compiled compiled-result
 :machine machine-result
 :same? (= direct-result compiled-result machine-result)}
```

machine report를 CI artifact로 남길 때는 91번처럼 쓴다.

```clojure
{:job :machine-report
 :artifact path
 :hash hash
 :rows (:row-count report)
 :decision (if (= :ok (:status report))
             :allow-merge
             :block-merge)}
```

AI가 evaluator나 machine 쪽을 고쳤다면 92번처럼 random source sweep도 같이 붙인다.

```clojure
{:job :machine-property-fuzzer
 :seed seed
 :machine-pass? (:machine-pass? report)
 :smallest-failing-source (:smallest-failing-source report)}
```

## 3. agent/plugin 권한 관리

도메인:

```text
AI coding agent
enterprise plugin system
internal developer portal
data platform UDF
```

문제:

```text
plugin이 파일, 환경변수, Java 객체, process에 접근하려고 한다.
권한 없이 실행하면 위험하다.
```

볼 예제:

```text
01, 23, 25, 80, 87
```

적용 코드 모양:

```clojure
{:tool :read-file
 :requested-effect :file-read
 :granted #{:pure}
 :status :held
 :reason :capability-denied}
```

## 4. 감사 로그와 재현성

도메인:

```text
fintech
regtech
compliance
ML experiment tracking
supply-chain build evidence
```

문제:

```text
값은 저장했는데 나중에 왜 그 값이 나왔는지 설명하기 어렵다.
```

볼 예제:

```text
14, 16, 21, 29, 49, 84
```

적용 코드 모양:

```clojure
{:rule-id rule-id
 :value value
 :receipt receipt
 :hash hash
 :replayable? true}
```

## 5. DSL/compiler 최적화 검증

도메인:

```text
internal DSL
policy engine
rule engine
build optimizer
low-code platform
```

문제:

```text
코드를 최적화하거나 다른 실행기로 바꾸면 뜻이 바뀔 수 있다.
```

볼 예제:

```text
03, 33, 48, 61, 78, 79, 90
```

적용 코드 모양:

```clojure
{:source source
 :before old-result
 :after new-result
 :same? (= old-result new-result)
 :ship? same?}
```

## 6. module/import resolver 검증

도메인:

```text
build system
package manager
monorepo config
Nix-like module loader
```

문제:

```text
import가 어디서 값을 가져오는지 분명해야 한다.
resolver가 없는데 몰래 실행되면 안 된다.
```

볼 예제:

```text
51, 56, 89
```

적용 코드 모양:

```clojure
{:import "./m"
 :resolver-present? true
 :status :ok
 :value imported-value}
```

## 7. 문서와 구현 drift 줄이기

도메인:

```text
developer portal
release dashboard
internal docs
capability inventory
```

문제:

```text
문서는 손으로 고치고, 코드는 따로 바뀌면 둘이 어긋난다.
```

볼 예제:

```text
58, 59, 73
```

적용 코드 모양:

```clojure
{:report-kind kind
 :status status
 :count count
 :hash report-hash}
```
