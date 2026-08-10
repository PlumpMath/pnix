;;; pnix-clj의 방식 - WIKI registry와 roadmap integrity를 data로 확인한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/73-wiki-integrity-index/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.wiki :as wiki]))

(let [registry (wiki/capability-registry)
      roadmap (wiki/roadmap)
      integrity (wiki/integrity)]
  (println "capability count:" (count registry))
  (println "first capability:" (select-keys (first registry) [:kind :module :run]))
  (println "roadmap items:" (count (:items roadmap)))
  (println "integrity:" integrity)

  (assert (pos? (count registry)))
  (assert (some #(= "weval" (:kind %)) registry))
  (assert (pos? (count (:items roadmap))))
  (assert (= :ok (:status integrity))))

(println)
(println "결론: pnix-clj wiki data는 docs가 code registry를 앞서가지 않도록 integrity gate를 제공한다.")
(shutdown-agents)

