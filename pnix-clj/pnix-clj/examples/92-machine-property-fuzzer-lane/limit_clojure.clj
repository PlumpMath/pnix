;;; plain Clojure의 한계 - sample 몇 개를 손으로 돌리는 것은
;;; machine/evaluator generative agreement check가 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/92-machine-property-fuzzer-lane/limit_clojure.clj

(ns machine-property-fuzzer-lane-limit)

(def samples
  ["1 + 2"
   "if true then 1 else 2"
   "let v = 3; in v * 2"])

(defn pretend-same?
  [source]
  ;; 실제로 machine/evaluator 두 레인을 돌리지 않는다.
  ;; 그냥 샘플 문자열이 비어 있지 않으면 통과로 친다.
  (boolean (seq source)))

(def result
  {:status :ok
   :sample-count (count samples)
   :machine-pass? true
   :shrinks-on-failure? false
   :generated-random-sources? false})

(doseq [source samples]
  (println "sample:" source "=>" (pretend-same? source)))

(println "plain result:" result)

(assert (= :ok (:status result)))
(assert (= true (:machine-pass? result)))

(println)
(println "결론: hand-picked sample 통과는 랜덤 생성 source에서 machine⇄evaluator가 계속 같은지 보여주지 못한다.")
