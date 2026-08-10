;;; pnix-clj의 방식 - mirror-chain!이 같은 source를 반복 실행하고,
;;; run event와 chain convergence verdict를 남긴다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/20-mirror-chain/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.mirror-chain :as mc]
            [pnix-clj.store :as store]))

(let [log (store/open-store)
      chain (mc/mirror-chain! "let x = 40; in x + 2" {:runs 3 :store log})
      events (store/events-of log :mirror/run)
      drifts (store/events-of log :mirror/chain-drift)
      verify (store/verify-chain log)]
  (println "chain:" chain)
  (println "run events:" events)
  (println "drift events:" drifts)
  (println "log verify:" verify)

  (assert (= :ok (:status chain)))
  (assert (= true (:chain-converged? chain)))
  (assert (= 1 (count events)))
  (assert (zero? (count drifts)))
  (assert (= :intact (:status verify))))

(println)
(println "결론: pnix-clj mirror-chain은 시간축 반복 실행 안정성을 event log로 고정한다.")
