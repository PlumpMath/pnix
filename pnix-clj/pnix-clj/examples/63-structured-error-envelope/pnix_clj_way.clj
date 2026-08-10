;;; pnix-clj의 방식 - held 결과에 machine-readable error envelope를 붙인다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/63-structured-error-envelope/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.error :as err]))

(let [held (err/held :eval :example-held {:message "blocked" :source-id :demo})
      thrown (try
               (/ 1 0)
               (catch Throwable t
                 (err/held-throwable :eval :divide-by-zero t {:source-id :demo})))]
  (println "held:" held)
  (println "thrown:" thrown)

  (assert (= :held (:status held)))
  (assert (= :pnix-clj.error.v0 (get-in held [:error :schema])))
  (assert (= :eval (get-in held [:error :phase])))
  (assert (= :example-held (:reason held)))
  (assert (= :divide-by-zero (:reason thrown)))
  (assert (string? (get-in thrown [:error :class]))))

(println)
(println "결론: pnix-clj error helper는 실패를 phase/reason/schema가 있는 held evidence로 만든다.")
(shutdown-agents)

