;;; plain Clojure의 한계 - Clojure form을 pnix source로 안전하게 되돌리는 whitelist가 없다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/10-reverse-synthesis/limit_clojure.clj

(ns reverse-synthesis-limit)

(def form '(+ 1 (* 2 3)))

(let [value (eval form)]
  (println "clojure form:" form)
  (println "host value:" value)
  (println "manual translation would be unchecked: 1 + 2 * 3")
  (println "missing: whitelist, held reason, pnix tower verification")
  (assert (= 7 value)))

(println)
(println "결론: plain Clojure에는 form -> pnix projection의 의미보존 gate가 없다.")
