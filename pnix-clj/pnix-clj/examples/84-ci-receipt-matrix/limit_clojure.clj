;;; plain Clojure의 한계 - CI에서 eval pass/fail만 보면 lane/provenance가 없다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/84-ci-receipt-matrix/limit_clojure.clj

(ns ci-receipt-matrix-limit)

(def forms
  {'arith '(+ 1 2)
   'list-len '(count [1 2 3])
   'bad '(/ 1 0)})

(defn run-form
  [[id form]]
  (try
    {:id id :status :pass :value (eval form)}
    (catch Throwable t
      {:id id :status :fail :error (.getName (class t))})))

(let [rows (mapv run-form forms)]
  (doseq [row rows]
    (println row))
  (println "multi-lane receipt?:" false)
  (println "held reason taxonomy?:" false)
  (assert (= 2 (count (filter #(= :pass (:status %)) rows))))
  (assert (= 1 (count (filter #(= :fail (:status %)) rows)))))

(println)
(println "결론: plain CI eval은 pass/fail은 만들지만 evaluator/clj-meta/px receipt나 stable held reason을 남기지 않는다.")
