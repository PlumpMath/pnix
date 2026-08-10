;;; pnix-clj의 방식 - 반복 실행 determinism을 snapshot에 묶인 event로 남긴다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/44-purity-event-spine/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.purity :as purity]
            [pnix-clj.snapshot :as snapshot]
            [pnix-clj.store :as store]))

(let [log (store/open-store)
      snap (snapshot/make-snapshot)
      result (purity/purity-check! "1 + 2" {:runs 3 :store log :snapshot snap})
      events (store/events-of log :purity/run)
      first-event (first events)]
  (println "purity result:" result)
  (println "events:" events)
  (println "chain:" (store/verify-chain log))

  (assert (= :ok (:status result)))
  (assert (= 1 (count events)))
  (assert (= (:snapshot/id snap)
             (get-in first-event [:payload :snapshot/id])))
  (assert (= :intact (:status (store/verify-chain log)))))

(println)
(println "결론: pnix-clj purity check는 재실행 증거를 snapshot-pinned event chain으로 만든다.")
(shutdown-agents)
