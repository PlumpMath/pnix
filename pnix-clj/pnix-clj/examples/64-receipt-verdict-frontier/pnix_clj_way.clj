;;; pnix-clj의 방식 - lane 결과에서 verdict와 frontier summary를 계산한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/64-receipt-verdict-frontier/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.receipt :as receipt]))

(def base
  {:parse-result {:status :ok}
   :eval-result {:status :ok :value 3}
   :lowering-result {:status :ok}
   :clj-meta-result {:status :ok :value 3}
   :px-runtime {:status :ok :value 3}
   :pnix-mirror {:status :ok :value 3}})

(let [accepted (receipt/verdict base)
      rejected (receipt/verdict (assoc base :clj-meta-result {:status :ok :value 4}))
      summary (receipt/summarize [{:source-id :ok
                                   :status (:status accepted)
                                   :reason (:reason accepted)
                                   :lane-summary []}
                                  {:source-id :bad
                                   :status (:status rejected)
                                   :reason (:reason rejected)
                                   :lane-summary [{:lane :pnix-clj-lowering-clj-meta
                                                   :status :ok
                                                   :frontier :clj-meta}]}])]
  (println "accepted verdict:" accepted)
  (println "rejected verdict:" rejected)
  (println "summary:" summary)

  (assert (= {:status :accepted :reason :all-lanes-agree} accepted))
  (assert (= :rejected (:status rejected)))
  (assert (= :evaluator-clj-meta-mismatch (:reason rejected)))
  (assert (= 2 (:total summary)))
  (assert (= 1 (:accepted summary)))
  (assert (= 1 (:rejected summary)))
  (assert (= :bad (get-in summary [:first-frontier :source-id]))))

(println)
(println "결론: pnix-clj receipt는 lane evidence를 deterministic verdict/frontier로 줄인다.")
(shutdown-agents)

