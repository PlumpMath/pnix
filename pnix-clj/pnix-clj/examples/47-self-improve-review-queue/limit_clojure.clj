;;; plain Clojure의 한계 - 후보 점수화는 가능하지만 witnessed owner-held queue가 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/47-self-improve-review-queue/limit_clojure.clj

(ns self-improve-review-queue-limit)

(def candidates
  [{:target :inc :source "(fn [x] (+ x 1))" :score 10}
   {:target :bad :source "(fn [x] x)" :score 1}])

(def ranked
  (vec (sort-by (comp - :score) candidates)))

(println "ranked candidates:" ranked)
(println "witness status per candidate?:" false)
(println "owner-held review queue?:" false)

(assert (= :inc (:target (first ranked))))

(println)
(println "결론: plain ranking은 self-improve 후보의 witness/gate/event trail을 만들지 않는다.")

