;;; pnix-clj의 방식 - fixture corpus 실행 중 evaluator coverage를 측정한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/41-coverage-report/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.coverage :as coverage]))

(defn metric-ok?
  [m]
  (and (pos? (:total m))
       (<= 0 (:covered m) (:total m))
       (vector? (:missing m))))

(let [report (coverage/report {:include-runtime? false})
      summary (:summary report)]
  (println "sources:" (:source-count report))
  (println "op coverage:" (:op summary))
  (println "builtin coverage:" (:builtin summary))
  (println "branch coverage:" (:branch summary))

  (assert (pos? (:source-count report)))
  (assert (every? metric-ok? (vals summary)))
  (assert (pos? (get-in summary [:op :covered])))
  (assert (pos? (get-in summary [:builtin :covered])))
  (assert (pos? (get-in summary [:branch :covered]))))

(println)
(println "결론: pnix-clj는 corpus 실행을 coverage evidence로 바꿔 빠진 evaluator 표면을 찾게 한다.")
(shutdown-agents)
