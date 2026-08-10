;;; plain Clojure의 한계 - reflection 값은 읽을 수 있지만 stable host-lane snapshot은 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/45-reflection-host-lane/limit_clojure.clj

(ns reflection-host-lane-limit)

(def ns-count
  (count (all-ns)))

(def raw-var
  #'clojure.core/map)

(println "namespace count:" ns-count)
(println "raw Var class:" (.getName (class raw-var)))
(println "classpath string length:" (count (or (System/getProperty "java.class.path") "")))
(println "stable host-lane id?:" false)

(assert (var? raw-var))
(assert (pos? ns-count))

(println)
(println "결론: plain reflection은 host 정보를 읽지만 stable EDN snapshot/hash로 pin하지 않는다.")

