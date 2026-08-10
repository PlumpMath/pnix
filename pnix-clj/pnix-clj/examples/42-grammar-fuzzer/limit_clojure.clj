;;; plain Clojure의 한계 - 임의 sample eval은 grammar fuzzer gate가 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/42-grammar-fuzzer/limit_clojure.clj

(ns grammar-fuzzer-limit)

(def generated-clj-forms
  ['(+ 1 2)
   '(if true 1 2)
   '(/ 1 0)])

(def results
  (mapv (fn [form]
          (try
            {:status :ok :value (eval form)}
            (catch Throwable t
              {:status :error :class (.getName (class t))})))
        generated-clj-forms))

(println "plain generated results:" results)
(println "has pnix expected accepted/held gate?:" false)

(assert (= [:ok :ok :error] (mapv :status results)))

(println)
(println "결론: plain generated eval은 seed, pnix grammar class, lane summary를 묶은 fuzzer report가 아니다.")

