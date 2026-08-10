# BEGINNER PATH

처음 보는 사람이 읽을 순서다.

## 목표

92개 예제를 다 읽지 말고, 가장 현실적인 5개만 먼저 본다.

## 1. AI 설정 검사

읽을 것:

[83-ai-generated-config-gate](83-ai-generated-config-gate/README.md)

상황:

```text
AI가 config를 만들어 왔다.
바로 서버에 적용하면 위험할 수 있다.
```

배울 것:

```text
먼저 pure한지 본다.
몰래 host 정보를 읽는지 본다.
key를 두 번 만든 실수가 있는지 본다.
```

핵심 코드 모양:

```clojure
{:pure? true
 :status :ok
 :reason nil}
```

## 2. 설정표 칸 검사

읽을 것:

[86-service-option-contract](86-service-option-contract/README.md)

상황:

```text
설정에 port가 필요하다.
그런데 AI가 prt라고 잘못 썼다.
```

배울 것:

```text
필수 칸이 빠졌으면 멈춘다.
추가 칸을 허용할지 말지도 코드로 정한다.
```

## 3. CI 영수증

읽을 것:

[84-ci-receipt-matrix](84-ci-receipt-matrix/README.md)

상황:

```text
CI에서 테스트가 통과했다.
그런데 어떤 실행 경로가 맞았는지 모른다.
```

배울 것:

```text
값만 보지 말고 receipt를 남긴다.
나중에 왜 통과했는지 확인할 수 있다.
```

## 4. plugin 권한 막기

읽을 것:

[87-plugin-capability-boundary](87-plugin-capability-boundary/README.md)

상황:

```text
agent나 plugin이 파일을 읽으려고 한다.
그냥 허용하면 위험하다.
```

배울 것:

```text
기본은 막는다.
승인된 권한만 실행한다.
실행하면 witness를 남긴다.
```

## 5. refactor/cache 안정성

읽을 것:

[88-refactor-cache-stability](88-refactor-cache-stability/README.md)

상황:

```text
코드 뜻은 같은데 공백만 바뀌었다.
cache가 매번 miss 나면 낭비다.
```

배울 것:

```text
겉모습 문자열 말고 구조 hash로 같은 프로그램인지 본다.
```

## 그 다음

machine 쪽이 궁금하면:

```text
78 -> 79 -> 81 -> 89 -> 90 -> 91 -> 92
```

감사/재현 쪽이 궁금하면:

```text
14 -> 16 -> 21 -> 29 -> 84
```

compiler/DSL 쪽이 궁금하면:

```text
03 -> 33 -> 48 -> 61 -> 90 -> 92
```

권한/보안 쪽이 궁금하면:

```text
01 -> 23 -> 80 -> 87
```
