;;; plain Clojure의 한계 - 값은 얻지만 lane별 receipt와 collapse verdict가 없다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/02-four-lane-receipt/limit_clojure.clj

(ns four-lane-receipt-limit)

(def source "(+ 40 2)")

(let [form (read-string source)
      value (eval form)]
  (println "source:" source)
  (println "plain value:" value)
  (println "missing: parse/eval/lowering/clj-meta/px-runtime/pnix-mirror receipt")
  (println "missing: cross-lane collapse verdict")
  (assert (= 42 value)))

(println)
(println "결론: plain Clojure eval은 값만 주고, 여러 substrate가 같은 값에 동의했다는 증거는 남기지 않는다.")
