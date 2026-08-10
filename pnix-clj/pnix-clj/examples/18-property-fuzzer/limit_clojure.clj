;;; plain Clojure의 한계 - 몇 개 sample eval은 cross-lane property나 shrink 증거가 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/18-property-fuzzer/limit_clojure.clj

(ns property-fuzzer-limit)

(def samples ['(+ 1 2) '(* 2 3) '(if true 1 2)])

(doseq [form samples]
  (println form "=>" (eval form)))

(println "missing: generated pnix corpus, shrink, cross-lane/cache/specializer/machine properties")

(assert (= [3 6 1] (mapv eval samples)))

(println)
(println "결론: hand-picked sample은 counterexample search/shrink가 붙은 property check가 아니다.")
