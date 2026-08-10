;;; pnix-clj의 방식 - admitted witness도 기본 정책에서는 자동 promote되지 않는다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/46-self-mod-gate/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.self-mod-gate :as gate]
            [pnix-clj.store :as store]))

(let [log (store/open-store)
      proposal {:target :example-inc
                :new-source "(x: x + 1) 41"
                :rationale "example self modification proposal"}
      decision (gate/propose-and-gate log proposal)
      proposed-events (store/events-of log :self-mod/proposed)
      held-events (store/events-of log :self-mod/held)]
  (println "decision:" decision)
  (println "proposed events:" proposed-events)
  (println "held events:" held-events)

  (assert (= :held (:decision decision)))
  (assert (= :no-auto-promotion-owner-required (:reason decision)))
  (assert (= 1 (count proposed-events)))
  (assert (= 1 (count held-events)))
  (assert (= :intact (:status (store/verify-chain log)))))

(println)
(println "결론: pnix-clj self-mod gate는 제안과 적용을 분리하고 기본값을 owner-held로 둔다.")
(shutdown-agents)
