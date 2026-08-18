# 93. Live oracle differential test

## 무엇을 보여주나

`live-oracle`은 실제 `nix-instantiate`(설치돼 있을 때)와 pnix-clj evaluator에
**같은 소스**를 돌려 값을 직접 비교한다. `52-static-oracle-corpus`가 미리
캡처해 둔 fixture와 비교한다면, 이건 **지금 이 머신의 실제 Nix**와 실시간으로
비교한다.

## plain Clojure의 한계

plain Clojure로 이걸 하려면 `nix-instantiate`를 손으로 셸아웃하고, 없을 때는
그냥 건너뛰거나 예외를 던지고, stdout JSON을 손으로 파싱해야 한다 —
matched/mismatched 소스가 몇 개인지, 어떤 소스에서 갈렸는지 구조화된 표가
자동으로 안 생긴다.

## pnix-clj 방식

`oracle/report`는 grammar-fuzzer로 생성한 positive 소스 각각을 pnix-clj와
`nix-instantiate --eval --strict --json`에 동시에 돌려 값을 비교한다.
`nix-instantiate`가 없으면 실패가 아니라 구조화된 `:skipped`로 빠진다
(lane-classification: `external-authority :comparison-only`,
`default-when-missing :skipped` — 이 오라클이 없어도 코어 게이트는 안 막힌다).

## 어디에 쓰나

- evaluator 변경이 real Nix 의미론에서 벗어났는지 CI에서 실시간 확인
- `52`의 정적 캡처 fixture와 상호보완: 정적 fixture는 재현 가능·오프라인,
  live oracle은 최신 Nix와의 drift를 잡음

## 코드 비교

`limit_clojure.clj` 핵심 발췌:

```clojure
(defn pretend-checked-against-nix? [source]
  (boolean (seq source)))
```

`pnix_clj_way.clj` 핵심 발췌:

```clojure
(let [report (oracle/report {:positive-count 5 :seed 0})]
  (assert (= :ok (:status report)))
  (assert (zero? (:mismatched report)))
  (assert (pos? (:matched report))))
```

비교하면, limit 파일은 값을 흉내만 낼 뿐 실제 Nix를 부르지 않는다. pnix-clj
파일은 실제 `nix-instantiate`를 부르고, 없으면 `:skipped`로 명시적으로
갈린다.

## 코드 해설

`oracle/report`가 돌아가는 순서:

```clojure
;; 1. nix-instantiate 실행파일을 PATH/기본 경로에서 찾는다.
(oracle/discover-command)

;; 2. 없으면 바로 구조화된 skip.
;; 있으면 grammar-fuzzer positive 소스마다:
;;   - pnix-clj로 평가
;;   - nix-instantiate --eval --strict --json 로 평가
;;   - 두 값을 비교해 :matched/:mismatched/:pnix-held/:oracle-held 분류
```

## 산업/실무 적용

CI에서 이렇게 판단에 붙일 수 있다.

```clojure
{:job :live-oracle-differential
 :status (:status report)
 :mismatched (:mismatched report)
 :first-mismatched (:first-mismatched report)
 :decision (if (zero? (:mismatched report)) :allow-merge :block-merge)}
```

Nix 버전이 올라가거나 evaluator를 고친 PR에서, 정적 fixture만으로는 못 잡는
"지금 설치된 실제 Nix와 미묘하게 달라진" 회귀를 잡는다.

## 초딩 설명

이전 설명: live-oracle은 지금 이 컴퓨터에 깔린 진짜 Nix와 pnix-clj를 같은
문제로 비교한다.

초딩 설명: 우리 반 선생님(pnix-clj)과 옆반 선생님(진짜 Nix)한테 같은 문제를
내고 답을 맞혀 보는 것과 같다. 두 선생님 답이 다르면 어디서 갈렸는지 표로
남긴다. 옆반 선생님이 없는 날(설치 안 됨)엔 "오늘은 비교 못 함"이라고만
적고 넘어간다 — 실패로 안 친다.

기억할 것: `52`는 예전에 받아 적어 둔 정답지고, `93`은 오늘 옆반 선생님한테
직접 다시 물어보는 것이다.
