;;; pnix-clj의 방식 - nREPL middleware를 fake transport로 dry-run한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/67-nrepl-middleware-dry-run/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [nrepl.transport :as transport]
            [pnix-clj.nrepl :as pnix-nrepl]))

(let [sent (atom [])
      fake-transport (reify transport/Transport
                       (recv [_] nil)
                       (recv [_ _] nil)
                       (send [_ msg]
                         (swap! sent conj msg)))
      handler (pnix-nrepl/wrap-pnix-eval
               (fn [msg]
                 (swap! sent conj {:delegated (:op msg)})))]
  (handler {:op "eval"
            :code "1 + 2"
            :transport fake-transport})
  (println "sent messages:" @sent)

  (assert (= "3" (:value (first @sent))))
  (assert (= "pnix" (:ns (first @sent))))
  (assert (= #{:done} (:status (second @sent))))
  (assert (= 2 (count @sent))))

(println)
(println "결론: pnix-clj nREPL middleware는 eval op를 Clojure fallback 없이 pnix evaluator로 라우팅한다.")
(shutdown-agents)

