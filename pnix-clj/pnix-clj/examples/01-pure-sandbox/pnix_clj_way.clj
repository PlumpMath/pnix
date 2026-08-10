;;; pnix-clj의 방식 — 순수성 정적판정 + fuel 한계 = 신뢰 가능한 샌드박스.
;;;
;;; pnix는 순수·지연 언어라 부작용이 '설계상' 없고, safe-eval은 fuel(스텝 예산) 한계를
;;; 강제하며 결코 걸리거나 예외로 새어나오지 않는다(항상 구조화된 판정을 반환).
;;; 실행 '전에' 순수성을 정적으로 판정할 수도 있다.
;;;
;;; 실행:  cd pnix-clj && clojure -M examples/01-pure-sandbox/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.safe-eval :as safe]))

;; 1) 순수 계산은 값으로 안전하게 나온다 (부작용 불가능).
(let [ok (safe/safe-eval "1 + 2 * 3")]
  (println "순수 계산:" (:status ok) (:value ok))
  (assert (and (= :ok (:status ok)) (= 7 (:value ok)))))

;; 2) 부작용(impure)은 :pure-only? true로 '거부'된다 — 실행 전 정적 판정.
(let [impure (safe/safe-eval "builtins.getEnv \"HOME\"" {:pure-only? true})]
  (println "impure 거부:" (:status impure) "| limit:" (:limit-exceeded impure))
  (assert (and (= :held (:status impure))
               (= :impure (:limit-exceeded impure)))))

;; 3) 무한/과도한 계산은 fuel 한계로 '멈춘다' (걸리지 않는다).
(let [bounded (safe/safe-eval "let f = x: f x; in f 1" {:fuel 50000})]
  (println "자원 한계:" (:status bounded) "| reason:" (:reason bounded))
  (assert (= :held (:status bounded))))  ; fuel/재귀 중 하나로 안전하게 종료

;; 4) 실행 '전에' 순수성을 정적으로 알 수 있다.
(let [purity (safe/static-purity-check "builtins.readFile \"/etc/passwd\"")]
  (println "정적 순수성:" (:pure? purity)
           "| 부작용 사용:" (mapv :builtin (:impure-uses purity)))
  (assert (false? (:pure? purity))))

;; 5) 순수한 코드는 정적 판정도 통과한다.
(let [purity (safe/static-purity-check "let x = 40; in x + 2")]
  (println "정적 순수성(순수 코드):" (:pure? purity))
  (assert (true? (:pure? purity))))

(println "\n결론: 신뢰할 수 없는 pnix 입력을 순수·자원제한 하에 안전하게 평가한다.")
