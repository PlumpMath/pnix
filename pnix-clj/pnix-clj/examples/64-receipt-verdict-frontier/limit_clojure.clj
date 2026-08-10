;;; plain Clojure의 한계 - 여러 결과를 모아도 verdict/frontier 규칙은 따로 없다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/64-receipt-verdict-frontier/limit_clojure.clj

(ns receipt-verdict-frontier-limit)

(def results
  [{:lane :a :status :ok :value 3}
   {:lane :b :status :ok :value 4}])

(def all-ok?
  (every? #(= :ok (:status %)) results))

(println "all status ok?:" all-ok?)
(println "values agree?:" (apply = (map :value results)))
(println "standard mismatch reason/frontier?:" false)

(assert all-ok?)
(assert (not (apply = (map :value results))))

(println)
(println "결론: plain aggregation은 cross-lane mismatch를 표준 receipt verdict로 바꾸지 않는다.")

