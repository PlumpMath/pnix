# 77. Dynamic attr key strictness

## 무엇을 보여주나

D20에서 잡힌 dynamic attr key 의미를 보여준다. eval-time key collision은 silent overwrite가 아니라 `:duplicate-attr`이고, dynamic key 값은 string이어야 한다. 단, attrset이 강제되지 않으면 construction error도 lazily 드러나지 않는다.

## plain Clojure의 한계

Clojure map은 같은 key를 `assoc`하면 덮어쓰고, 숫자 같은 임의 key도 허용한다. 그래서 real Nix의 dynamic attr key collision/type rule과 `or` default의 catch boundary를 기본으로 표현하지 않는다.

## pnix-clj 방식

`pnix/eval-source`와 `pnix/run-source`가 dynamic attr key를 평가하면서 collision/type error는 held reason으로, 정상 string key와 lazy unused attrset은 accepted receipt로 분리한다.

## 어디에 쓰나

AI가 생성한 attrset code, string interpolation key, config merge logic에서 “덮어쓰기처럼 보이지만 real Nix에서는 error”인 경우를 잡을 때 쓴다.

## 코드 비교

`limit_clojure.clj` 핵심 발췌:

```clojure
(def overwritten
  (assoc {"a" 1} "a" 2))

(def numeric-key
  (assoc {} 1 "one"))

(assert (= 2 (get overwritten "a")))
(assert (= "one" (get numeric-key 1)))
```

`pnix_clj_way.clj` 핵심 발췌:

```clojure
(doseq [{:keys [source reason]} held-cases]
  (let [r (pnix/eval-source source)]
    (assert (= :held (:status r)))
    (assert (= reason (:reason r)))))

(doseq [{:keys [source expected]} accepted-cases]
  (let [receipt (pnix/run-source source)]
    (assert (= :accepted (:status receipt)))
    (assert (= expected (get-in receipt [:eval-result :value])))))
```

비교하면, limit 파일은 map update 규칙으로 overwrite/임의 key를 허용한다. pnix-clj 파일은 D20 real Nix semantics를 `:duplicate-attr`, `:dynamic-attr-key-not-string`, accepted receipt로 분리한다.


## 코드 해설

이 README의 두 파일은 같은 문제를 일부러 다른 태도로 푼다. `limit_clojure.clj`는 plain Clojure로 가능한 최소 구현을 보여주고, `pnix_clj_way.clj`는 같은 문제를 pnix-clj의 gate/receipt/witness/lane API에 태운다.

읽을 때는 아래 주석처럼 보면 된다.

```clojure
;; limit_clojure.clj
;; - plain Clojure에서 77. Dynamic attr key strictness 문제를 어떻게 흉내 내는지 본다.
;; - 핵심은 '값은 만들 수 있지만, 그 값이 안전한지/재현 가능한지/같은 의미인지
;;   증거가 자동으로 남지 않는다'는 점이다.
;; - 실무에서는 이 부분이 버그 리포트, 감사 로그, CI verdict로 바로 이어지기 어렵다.

;; pnix_clj_way.clj
;; - source나 fixture를 pnix-clj API에 넣고, result map을 받는다.
;; - (:status result), (:reason result), (:value result), hash/receipt/witness field를 assert 한다.
;; - 이 assert는 예제에서는 교육용이지만, 실제로는 CI gate, PR comment,
;;   deployment approval, audit event row로 바꿔 붙이면 된다.
```

이 예제에서 특히 봐야 할 점은 다음이다.

- plain 쪽 한계: plain Clojure 쪽은 map merge, destructuring, try/catch처럼 익숙한 도구를 쓰지만, typo·중복·권한요구가 실행 결과 뒤에 숨는다.
- pnix-clj 쪽 핵심: pnix-clj 쪽은 `:status`, `:reason`, `:value`를 assert 해서 적용 전 gate row를 만든다. 특히 D19/D20 계열은 real Nix와 같은 pattern/dynamic-key semantics를 문서화된 reason으로 남긴다.
- 판단 기준: AI가 만든 설정/함수 인자를 적용하기 전에, required/default/duplicate/uncatchable error를 사람이 리뷰 가능한 verdict로 바꾼다.

## 산업/실무 적용

적용 가능한 개발 도메인 예시는 다음과 같다.

- SRE/platform 설정 검토
- Kubernetes/Nix module generator
- SaaS tenant config
- feature-flag rollout
- AI coding-agent PR review

실무 흐름으로 바꾸면 이렇게 쓴다. 생성된 source를 바로 merge하지 말고 review row로 만든 뒤, `:ok`만 자동 적용하고 `:held`는 PR comment나 human review queue로 보낸다.

```clojure
;; 실제 서비스 코드에서는 아래 map을 DB row, CI artifact, PR comment,
;; deployment approval payload 같은 형태로 저장하면 된다.
{:domain :platform-config
 :generated-source source
 :gate [:pure? :status :reason]
 :approve? (= :ok (:status verdict))
 :manual-review? (= :held (:status verdict))}
```

업체나 팀 관점에서 보면, 이 예제는 라이브러리 기능 하나를 보여주는 것이 아니라 "자동화가 결정을 내리기 전에 어떤 증거를 요구할 것인가"를 정하는 작은 패턴이다.


## 초딩 설명

### 이 예제가 말하는 것

이전 설명: AI가 만든 설정/함수 인자를 적용하기 전에, required/default/duplicate/uncatchable error를 사람이 리뷰 가능한 verdict로 바꾼다.

초딩 설명: AI가 숙제처럼 설정표를 만들어 왔을 때, 바로 회사 서버에 붙이지 말고 선생님처럼 빨간펜 검사를 하는 것이다. 빠진 칸이 있는지, 같은 칸을 두 번 썼는지, 몰래 집 주소 같은 정보를 읽으려는지 먼저 본다.

한 문장으로 줄이면, 이 예제는 `그냥 믿고 실행하기` 대신 `먼저 확인하고, 이유를 적고, 나중에 다시 볼 수 있게 남기기`를 보여준다.

### 코드 쉽게 읽기

이전 설명: `limit_clojure.clj`와 `pnix_clj_way.clj`를 비교해서 plain Clojure의 한계와 pnix-clj 방식을 본다.

초딩 설명: 두 파일은 같은 문제를 두 가지 방식으로 푼다.

```clojure
;; limit_clojure.clj
;; 그냥 해 본다. 답이 나올 수도 있지만, 위험했는지, 왜 멈췄는지,
;; 나중에 다시 확인할 영수증이 있는지는 잘 모른다.

;; pnix_clj_way.clj
;; 먼저 검사하고, 결과를 표처럼 받는다.
;; :ok    = 초록불, 해도 됨
;; :held  = 잠깐 멈춤, 이유를 봐야 함
;; :reason = 왜 멈췄는지 적힌 쪽지
;; :value  = 진짜 답
;; assert  = 예상한 답이 맞는지 확인하는 선생님
```

이 README의 `코드 비교`에서 `assert`가 보이면, 어렵게 생각하지 말고 `이 줄은 약속한 결과가 맞는지 확인한다`고 읽으면 된다. `hash`는 물건의 지문, `receipt`는 영수증, `witness`는 증인 도장, `gate`는 문지기라고 생각하면 된다.

### plain 쪽을 쉽게 말하면

이전 설명: plain Clojure는 종이에 새 값을 그냥 덮어쓰는 것과 비슷하다. 이름이 두 번 나오면 마지막 이름이 이기고, 틀린 칸 이름도 모르고 지나갈 수 있다.

초딩 설명: plain Clojure는 장난감을 바로 움직여 보는 것과 같다. 빠르고 쉽지만, 장난감이 어디를 건드렸는지, 같은 놀이를 내일 다시 해도 같은 결과가 나오는지, 누가 허락했는지 적어 두지 않는다.

### pnix-clj 쪽을 쉽게 말하면

이전 설명: pnix-clj는 문 앞의 검사 선생님처럼 `:ok`면 통과, `:held`면 멈춤, `:reason`에는 왜 멈췄는지 쪽지를 붙인다.

초딩 설명: pnix-clj는 놀이 전에 체크리스트를 읽고, 놀이가 끝나면 영수증을 붙인다. 성공하면 초록불, 위험하면 멈춤, 멈춘 이유는 쪽지로 남긴다. 그래서 사람이 다시 보거나 CI가 자동으로 판단하기 쉽다.

### 실무 응용을 쉽게 말하면

이전 설명: 플랫폼/SRE 팀은 tenant 설정, feature flag, Kubernetes/Nix module, AI가 만든 PR 설정을 배포 전에 자동 검사할 수 있다.

초딩 설명: 회사에서는 사람이 모든 코드를 매번 눈으로 확인하기 어렵다. 그래서 이 예제처럼 작은 검사표를 만들어 두면, AI나 자동화가 만든 결과를 바로 믿지 않고 `통과`, `사람이 봐야 함`, `막아야 함`으로 나눌 수 있다.

```clojure
;; 77. Dynamic attr key strictness를 실제 서비스에 붙이면 이런 모양의 기록을 남긴다.
{:thing "AI가 만든 설정"
 :check [:빠진칸? :중복칸? :몰래읽기?]
 :result (if safe? :자동통과 :사람검토)}
```

기억할 것: 어려운 이름을 다 외울 필요는 없다. `pnix_clj_way.clj`가 하는 일은 대부분 `먼저 검사한다`, `결과와 이유를 표로 받는다`, `나중에 다시 확인할 증거를 남긴다` 세 가지다.
