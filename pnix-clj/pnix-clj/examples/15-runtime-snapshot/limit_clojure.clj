;;; plain Clojure의 한계 - 캐시된 값이 어느 runtime/version에서 나온 것인지 pin이 없다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/15-runtime-snapshot/limit_clojure.clj

(ns runtime-snapshot-limit)

(def cache (atom {}))

(defn cached-eval [source]
  (if-let [hit (get @cache source)]
    hit
    (let [value (eval (read-string source))]
      (swap! cache assoc source value)
      value)))

(println "value:" (cached-eval "(+ 1 2)"))
(println "cache:" @cache)
(println "missing: evaluator-version, JVM/classpath pin, fail-closed mismatch gate")

(assert (= 3 (cached-eval "(+ 1 2)")))

(println)
(println "결론: plain cache는 runtime이 바뀌어도 stale result를 거부할 근거가 없다.")
