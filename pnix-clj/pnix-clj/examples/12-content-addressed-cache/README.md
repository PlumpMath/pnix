# 12 · content-addressed cache — 정본 내용주소 캐시

## 쉽게 말하면 (비유)
`memoize`는 **글자 그대로** 기억한다 — `"1 + 2"`와 `"1 +  2"`(공백 하나 차이)를 **다른 것**으로 본다.
`cached-eval`은 **뜻으로** 기억한다 — 공백/괄호가 달라도 같은 프로그램이면 **한 칸**에 넣는다.
```clojure
(cached-eval "1 + 2")    ; => cache miss
(cached-eval "1 +   2")  ; => cache HIT  (같은 내용주소)
```
직관: 캐시 키를 **표현(문자열)이 아니라 정본 내용(위치 제거 AST 해시)**으로 잡는다.

## 무엇을
평가 결과를 **내용주소**(위치 제거 AST 해시 + epoch)로 캐시한다. 공백/괄호 변형은 같은 엔트리에
매핑되고, 캐시는 답을 **바꿀 수 없다**(bypass는 항상 fresh 평가라 miss==hit 값 보장).

## plain의 한계 (`limit_clojure.clj`)
`clojure.core/memoize`는 인자(문자열)를 그대로 키로 쓴다 → 같은 뜻이라도 **포맷이 다르면 미스**.
또 결과가 캐시 때문에 달라질 위험(무효화/버전 관리)을 직접 챙겨야 한다.

## pnix-clj의 방식 (`pnix_clj_way.clj`)
- `cache-key src` — 위치 제거 AST 해시 기반 **정본 키**(공백/괄호 무관).
- `cached-eval src` — miss면 평가·저장, hit면 저장값 반환. **miss==hit 값**(캐시가 답을 못 바꿈).
- epoch로 버전 격리; `clear-eval-cache!`/`eval-cache-stats`로 통제.

## 어디에 쓰나
- 같은 계산을 포맷만 바꿔 반복 요청하는 **평가 서비스/빌드 캐시**
- 캐시가 **정확성을 해치지 않아야** 하는(내용주소·재현성) 계산 레이어
- alpha/포맷 변형에 강건한 캐시가 필요한 곳

## 실행
```sh
cd pnix-clj
clojure -M examples/12-content-addressed-cache/limit_clojure.clj
clojure -M examples/12-content-addressed-cache/pnix_clj_way.clj
```

## 코드 비교

`limit_clojure.clj` 핵심 발췌:

```clojure
(ns limit-clojure)
(def calls (atom 0))
(def evaluate
  (memoize (fn [src]
             (swap! calls inc)
             ;; (실제 평가 대신) 계산 횟수만 센다
             (count src))))
(evaluate "1 + 2")     ; miss -> 계산
(evaluate "1 + 2")     ; hit  (같은 문자열)
(evaluate "1 +  2")    ; MISS! 뜻은 같지만 공백이 달라 다른 키
(println "memoize 계산 횟수:" @calls
         "  <- '1 + 2' 와 '1 +  2' 는 같은 뜻인데도 2번 계산됨")
(assert (= 2 @calls))
(println "표현 기반 키라 alpha/포맷 변형에 취약하고, 정본 내용주소가 아니다.")
(println "\n결론: plain memoize는 '같은 뜻이면 한 칸'을 보장하지 못한다.")
```

`pnix_clj_way.clj` 핵심 발췌:

```clojure
(ns pnix-clj-way
  (:require [pnix-clj.cached-eval :as ce]))
(ce/clear-eval-cache!)
;; 1) 첫 평가는 miss, 저장된다.
(let [a (ce/cached-eval "1 + 2")]
  (println "첫 평가:" (:value a) "| cache:" (get-in a [:cache :status]))
  (assert (and (= 3 (:value a)) (= :miss (get-in a [:cache :status])))))
;; 2) 공백만 다른 같은 프로그램은 HIT (정본 내용주소).
(let [b (ce/cached-eval "1 +   2")]
  (println "공백 변형:" (:value b) "| cache:" (get-in b [:cache :status])
           "  <- 같은 뜻이라 HIT")
  (assert (and (= 3 (:value b)) (= :hit (get-in b [:cache :status])))))
;; 3) 괄호 변형도 같은 내용주소 -> HIT.
(let [c (ce/cached-eval "(1 + 2)")]
  (println "괄호 변형:" (:value c) "| cache:" (get-in c [:cache :status]))
  (assert (= 3 (:value c))))
;; 4) 캐시가 답을 바꾸지 않는다: miss 값 == hit 값 (내용주소 불변).
(println "miss==hit 값 보장: cached-eval은 캐시 히트여도 fresh 평가와 같은 값.")
;; 5) 통계로 통제 (hit/miss/bypass 관측).
(println "cache stats:" (ce/eval-cache-stats))
(println "\n결론: 표현이 아니라 '뜻(정본 내용)'으로 캐시 — 포맷 변형에 강건하고 정확성 불변.")
```

비교하면, limit 파일은 plain Clojure 값/예외/수동 상태를 직접 만든다. pnix-clj 파일은 같은 문제를 pnix-clj API에 태우고 `assert`로 `:status`, hash, receipt, gate verdict 같은 증거를 확인한다. 전체 실행 코드는 같은 디렉터리의 두 `.clj` 파일을 보면 된다.


## 코드 해설

이 README의 두 파일은 같은 문제를 일부러 다른 태도로 푼다. `limit_clojure.clj`는 plain Clojure로 가능한 최소 구현을 보여주고, `pnix_clj_way.clj`는 같은 문제를 pnix-clj의 gate/receipt/witness/lane API에 태운다.

읽을 때는 아래 주석처럼 보면 된다.

```clojure
;; limit_clojure.clj
;; - plain Clojure에서 12 · content-addressed cache — 정본 내용주소 캐시 문제를 어떻게 흉내 내는지 본다.
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

- plain 쪽 한계: plain Clojure 쪽은 atom, memoize, 파일 저장, pr-str 같은 로컬 편의를 쓰지만, 나중에 같은 결과인지 증명할 구조가 부족하다.
- pnix-clj 쪽 핵심: pnix-clj 쪽은 content hash, snapshot id, replay verdict, cache status, receipt summary를 assert 해서 audit 가능한 row를 만든다.
- 판단 기준: 값 하나가 아니라, 그 값이 어떤 runtime/snapshot/hash/event-chain에서 나왔는지 재현 가능한 증거로 남긴다.

## 산업/실무 적용

적용 가능한 개발 도메인 예시는 다음과 같다.

- fintech/regtech 감사 로그
- ML experiment provenance
- supply-chain build receipt
- compliance evidence store
- CI artifact retention

실무 흐름으로 바꾸면 이렇게 쓴다. 실행 결과를 저장할 때 value만 저장하지 말고 source hash, runtime snapshot, verdict, replay key를 함께 저장한다.

```clojure
;; 실제 서비스 코드에서는 아래 map을 DB row, CI artifact, PR comment,
;; deployment approval payload 같은 형태로 저장하면 된다.
{:domain :audit-ledger
 :source-hash source-hash
 :runtime snapshot-id
 :verdict (:status receipt)
 :replayable? true}
```

업체나 팀 관점에서 보면, 이 예제는 라이브러리 기능 하나를 보여주는 것이 아니라 "자동화가 결정을 내리기 전에 어떤 증거를 요구할 것인가"를 정하는 작은 패턴이다.


## 초딩 설명

### 이 예제가 말하는 것

이전 설명: 값 하나가 아니라, 그 값이 어떤 runtime/snapshot/hash/event-chain에서 나왔는지 재현 가능한 증거로 남긴다.

초딩 설명: 답만 적는 게 아니라, 풀이 과정과 도장을 같이 남기는 것이다. 나중에 누가 '정말 그 답이 맞아?'라고 물으면 같은 문제를 다시 풀어 보고 같은 답이 나오는지 확인할 수 있다.

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

이전 설명: plain Clojure는 답을 메모장에 적어 두는 것과 비슷하다. 나중에 누가, 언제, 어떤 규칙으로 만든 답인지 찾기 어렵다.

초딩 설명: plain Clojure는 장난감을 바로 움직여 보는 것과 같다. 빠르고 쉽지만, 장난감이 어디를 건드렸는지, 같은 놀이를 내일 다시 해도 같은 결과가 나오는지, 누가 허락했는지 적어 두지 않는다.

### pnix-clj 쪽을 쉽게 말하면

이전 설명: pnix-clj는 영수증처럼 `receipt`, 지문처럼 `hash`, 다시 풀기처럼 `replay`를 붙인다. 그래서 나중에 다시 확인할 수 있다.

초딩 설명: pnix-clj는 놀이 전에 체크리스트를 읽고, 놀이가 끝나면 영수증을 붙인다. 성공하면 초록불, 위험하면 멈춤, 멈춘 이유는 쪽지로 남긴다. 그래서 사람이 다시 보거나 CI가 자동으로 판단하기 쉽다.

### 실무 응용을 쉽게 말하면

이전 설명: 핀테크 계산 기록, 규제 감사 로그, ML 실험 기록, supply-chain build 증거, CI artifact 보관에 쓴다.

초딩 설명: 회사에서는 사람이 모든 코드를 매번 눈으로 확인하기 어렵다. 그래서 이 예제처럼 작은 검사표를 만들어 두면, AI나 자동화가 만든 결과를 바로 믿지 않고 `통과`, `사람이 봐야 함`, `막아야 함`으로 나눌 수 있다.

```clojure
;; 12 · content-addressed cache — 정본 내용주소 캐시를 실제 서비스에 붙이면 이런 모양의 기록을 남긴다.
{:answer 42
 :receipt "어떤 길로 계산했는지"
 :hash "답의 지문"
 :replay "나중에 다시 확인 가능"}
```

기억할 것: 어려운 이름을 다 외울 필요는 없다. `pnix_clj_way.clj`가 하는 일은 대부분 `먼저 검사한다`, `결과와 이유를 표로 받는다`, `나중에 다시 확인할 증거를 남긴다` 세 가지다.
