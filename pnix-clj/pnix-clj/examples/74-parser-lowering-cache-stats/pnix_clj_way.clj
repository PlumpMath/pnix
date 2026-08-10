;;; pnix-clj의 방식 - parse/lower cache stats를 단계별로 확인한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/74-parser-lowering-cache-stats/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.lowering :as lowering]
            [pnix-clj.parser :as parser]))

(parser/clear-parse-cache!)
(lowering/clear-lower-cache!)

(let [p1 (parser/parse-source "1 + 2")
      p2 (parser/parse-source "1 + 2")
      l1 (lowering/lower-ast (:ast p1))
      l2 (lowering/lower-ast (:ast p2))
      parse-stats (parser/parse-cache-stats)
      lower-stats (lowering/lower-cache-stats)]
  (println "parse:" (select-keys p1 [:status]))
  (println "lower:" (select-keys l1 [:status :form-hash]))
  (println "parse stats:" parse-stats)
  (println "lower stats:" lower-stats)

  (assert (= :ok (:status p1)))
  (assert (= :ok (:status p2)))
  (assert (= :ok (:status l1)))
  (assert (= (:form-hash l1) (:form-hash l2)))
  (assert (pos? (:hits parse-stats)))
  (assert (pos? (:misses parse-stats)))
  (assert (pos? (:entries parse-stats)))
  (assert (pos? (:hits lower-stats)))
  (assert (pos? (:entries lower-stats))))

(println)
(println "결론: pnix-clj는 parse/lower 단계도 cache stats로 관찰 가능한 evidence surface로 만든다.")
(shutdown-agents)

