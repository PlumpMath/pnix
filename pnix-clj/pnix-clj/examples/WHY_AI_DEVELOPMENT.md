# WHY AI DEVELOPMENT

pnix-clj가 AI 개발 쪽에서 왜 의미가 있는지 짧게 정리한다.

## 핵심 생각

AI는 코드를 빠르게 만든다.

그런데 빠르게 만든 코드에는 이런 문제가 생긴다.

```text
틀린 key를 만들 수 있다.
중복 설정을 만들 수 있다.
몰래 파일이나 환경변수를 읽는 코드를 만들 수 있다.
테스트는 통과하지만 다른 실행 경로에서는 깨질 수 있다.
나중에 왜 그 결과가 나왔는지 설명하기 어려울 수 있다.
```

pnix-clj는 이 문제를 “AI를 믿자”가 아니라 “AI 결과를 검사 가능한 표로 만들자”로 푼다.

## AI coding agent에 필요한 것

AI agent가 코드를 만들면 바로 merge하지 말고 이런 질문을 해야 한다.

```text
이 코드는 순수한가?
파일/환경변수/네트워크 권한이 필요한가?
같은 key를 두 번 만들었나?
다른 실행 경로에서도 같은 답인가?
나중에 같은 결과를 다시 만들 수 있나?
```

pnix-clj examples는 이 질문들을 작은 코드로 보여준다.

## 가장 직접적인 예제

AI config:

```text
83-ai-generated-config-gate
85-generated-config-merge-collision
86-service-option-contract
```

AI agent 권한:

```text
87-plugin-capability-boundary
23-capability-gate
80-interop-opaque-host-ref
```

AI refactor/CI:

```text
84-ci-receipt-matrix
88-refactor-cache-stability
90-machine-report-fuel-witness
91-machine-report-artifact-gate
92-machine-property-fuzzer-lane
```

## AI 개발에서의 좋은 흐름

```text
1. AI가 후보 코드를 만든다.
2. pnix-clj가 먼저 검사한다.
3. 결과가 :ok면 자동 진행할 수 있다.
4. 결과가 :held면 reason을 보고 사람이 결정한다.
5. receipt/witness/hash를 남겨 나중에 다시 확인한다.
```

코드 모양:

```clojure
{:ai-output generated-source
 :checks {:purity purity-result
          :eval eval-result
          :receipt receipt}
 :decision (case (:status eval-result)
             :ok :auto-approve
             :held :human-review)
 :reason (:reason eval-result)}
```

## pnix-clj의 포지션

pnix-clj는 “AI가 코드를 더 잘 만들게 하는 도구”라기보다, “AI가 만든 코드를 더 안전하게 받아들이는 도구”에 가깝다.

AI 시대에는 코드 생성보다 더 중요한 문제가 생긴다.

```text
생성된 코드를 믿을 수 있는가?
어떤 권한을 썼는가?
나중에 같은 결과를 다시 만들 수 있는가?
실패하면 왜 실패했는가?
```

pnix-clj examples는 이 질문에 답하는 작은 패턴 모음이다.

## 한 줄 결론

AI가 코드를 만들수록, 그냥 실행하는 시스템보다 증거를 남기며 실행하는 시스템이 더 중요해진다.
