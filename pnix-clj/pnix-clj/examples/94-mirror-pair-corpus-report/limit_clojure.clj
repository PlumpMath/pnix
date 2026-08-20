;;; 한계 - plain Clojure로 "여러 실행 경로(직접 evaluator, clj-meta 바이트코드,
;;; .px 런타임, pnix mirror)가 코퍼스 전체에서 하나로 수렴한다"를 확인하려면
;;; 204개 소스를 손으로 하나씩 돌리고 4-레인 값을 눈으로 비교해야 한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/94-mirror-pair-corpus-report/limit_clojure.clj

(ns mirror-pair-corpus-limit)

(defn pretend-four-lanes-agree?
  [source]
  ;; 실제로 4개 레인을 돌리지 않는다 - 그냥 소스가 비어있지 않으면 "동의"한다 침.
  (boolean (seq source)))

(println "204개 소스 각각의 ready? 를 손으로 셀 수 있나?:" false)
(println "pretend-four-lanes-agree? \"1 + 2\":" (pretend-four-lanes-agree? "1 + 2"))
(println)
(println "결론: plain Clojure는 코퍼스 규모의 4-레인 수렴을 집계 report로 안 준다.")
