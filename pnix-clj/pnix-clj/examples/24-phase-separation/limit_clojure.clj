;;; plain Clojure의 한계 — read/eval/host effect가 쉽게 한 덩어리로 섞인다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/24-phase-separation/limit_clojure.clj

(ns phase-separation-limit)

(def source
  "(let [x 40] (+ x 2))")

(def host-effect-source
  "(System/getenv \"HOME\")")

(let [form (read-string source)
      value (eval form)]
  (println "source:" source)
  (println "read form:" form)
  (println "eval value:" value)
  (println "parse verdict:" nil)
  (println "purity verdict:" nil)
  (println "lowering verdict:" nil)
  (println "compile receipt:" nil)
  (assert (= 42 value)))

(let [form (read-string host-effect-source)
      value (eval form)]
  (println)
  (println "host-effect source:" host-effect-source)
  (println "host-effect value exists?:" (boolean value))
  (println "capability gate verdict:" nil)
  (assert (or (nil? value) (string? value))))

(println)
(println "결론: plain Clojure는 read/eval/host-effect가 가능하지만 phase별 verdict/receipt를 기본으로 분리하지 않는다.")
