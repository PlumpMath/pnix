;;; plain Clojure의 한계 - host object를 그대로 넘기면 opaque/capability/witness 경계가 없다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/80-interop-opaque-host-ref/limit_clojure.clj

(ns interop-opaque-host-ref-limit)

(def host-object
  (java.util.Date. 0))

(def crossed
  {:value host-object})

(println "raw host class:" (.getName (class (:value crossed))))
(println "raw object identity preserved directly?:" (identical? host-object (:value crossed)))
(println "opaque handle?:" false)
(println "release gate?:" false)
(println "capability/witness?:" false)

(assert (instance? java.util.Date (:value crossed)))
(assert (identical? host-object (:value crossed)))

(println)
(println "결론: plain Clojure는 host object를 직접 넘기며, capability gate나 opaque reference lifecycle receipt를 남기지 않는다.")
