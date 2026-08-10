;;; plain Clojure의 한계 - host 값은 만들 수 있지만 projection 검증 report는 없다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/36-clojure-projection-report/limit_clojure.clj

(ns clojure-projection-report-limit)

(def host-value
  (eval '(java.util.Date. 0)))

(println "host value class:" (.getName (class host-value)))
(println "printed value:" (pr-str host-value))
(println "has projection runtime receipt?:" false)

(assert (instance? java.util.Date host-value))

(println)
(println "결론: plain eval은 host 값을 만들지만 pnix term projection, runtime self-test, host crossing count를 남기지 않는다.")

