;;; pnix-clj의 방식 - capability index를 코드에서 결정적으로 만든다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/58-capability-index/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.capabilities :as capabilities]))

(let [index (capabilities/index)]
  (println "index:" (select-keys index [:kind :schema :builtin-count]))
  (println "report artifacts:" (count (:report-artifacts index)))
  (println "public API namespaces:" (count (:public-api index)))
  (println "sample reports:" (take 8 (:report-artifacts index)))

  (assert (= :pnix-capability-index (:kind index)))
  (assert (pos? (:builtin-count index)))
  (assert (some #{"weval"} (:report-artifacts index)))
  (assert (some #{"stage15-exec"} (:report-artifacts index)))
  (assert (contains? (:public-api index) "pnix-clj.parser")))

(println)
(println "결론: pnix-clj capability index는 구현된 API/report surface를 코드에서 직접 읽는다.")
(shutdown-agents)

