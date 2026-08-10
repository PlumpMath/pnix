;;; pnix-clj의 방식 — cached-eval = 정본 내용주소 캐시(공백/괄호 변형이 한 엔트리).
;;;
;;; 캐시 키를 표현(문자열)이 아니라 위치 제거 AST 해시 + epoch 로 잡는다. 공백/괄호가 달라도
;;; 같은 프로그램이면 같은 엔트리. bypass는 항상 fresh 평가라 캐시가 답을 바꿀 수 없다(miss==hit).
;;;
;;; 실행:  cd pnix-clj && clojure -M examples/12-content-addressed-cache/pnix_clj_way.clj

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
