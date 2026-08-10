;;; pnix-clj의 방식 - 후보 batch를 witnessed/gated review queue로 만든다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/47-self-improve-review-queue/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.self-improve :as improve]
            [pnix-clj.store :as store]))

(let [log (store/open-store)
      candidates [{:target :inc
                   :new-source "(x: x + 1) 41"
                   :rationale "increment candidate"}
                  {:target :bad
                   :new-source "x: x"
                   :rationale "bare lambda has no value"}]
      round (improve/evaluate-round log candidates)]
  (println "proposals:" (:proposals round))
  (println "ranked:" (:ranked round))
  (println "round hash:" (:round-hash round))
  (println "round events:" (store/events-of log :self-improve/round))

  (assert (= 2 (count (:proposals round))))
  (assert (:all-held? round))
  (assert (string? (:round-hash round)))
  (assert (= 1 (count (store/events-of log :self-improve/round))))
  (assert (= :intact (:status (store/verify-chain log)))))

(println)
(println "결론: pnix-clj self-improve는 후보를 적용하지 않고 owner review queue와 증거 event로 남긴다.")
(shutdown-agents)
