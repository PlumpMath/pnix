;;; plain Clojure의 한계: host crossing이 그냥 일어난다.
;;;
;;; 실행:
;;;   cd pnix-clj
;;;   clojure -M examples/04-host-interop-loss-effect/limit_clojure.clj

(ns limit-clojure)

(let [value (eval '(+ 20 22))]
  (println "plain eval result:" value)
  (assert (= 42 value)))

(let [obj (StringBuilder. "pnix")]
  (.append obj "-clj")
  (println "plain Java object:" (str obj))
  (assert (= "pnix-clj" (str obj))))

(let [f (fn [x] (+ x 1))]
  (println "plain function call:" (f 41))
  (assert (= 42 (f 41))))

(println)
(println "결론: plain Clojure는 host crossing을 쉽게 하지만 effect/loss/capability/witness를 기본으로 남기지 않는다.")
