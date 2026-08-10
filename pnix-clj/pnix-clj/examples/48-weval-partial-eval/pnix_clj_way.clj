;;; pnix-clj의 방식 - IR/residual 경로에서 dispatch-free partial evaluation을 검증한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/48-weval-partial-eval/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.weval :as weval]))

(let [report (weval/report)]
  (println "status:" (:status report))
  (println "supported/unsupported/failed:" (:supported report) (:unsupported report) (:failed report))
  (println "dispatch:" (:dispatch report))
  (println "clj-meta lane:" (:clj-meta-lane report))

  (assert (= :ok (:status report)))
  (assert (pos? (:supported report)))
  (assert (zero? (:failed report)))
  (assert (pos? (get-in report [:dispatch :interpreted-steps])))
  (assert (zero? (get-in report [:dispatch :residual-steps])))
  (assert (= (get-in report [:clj-meta-lane :of])
             (get-in report [:clj-meta-lane :agreeing]))))

(println)
(println "결론: pnix-clj weval report는 interpreted dispatch와 residual dispatch-free 경로를 나란히 검증한다.")
(shutdown-agents)
