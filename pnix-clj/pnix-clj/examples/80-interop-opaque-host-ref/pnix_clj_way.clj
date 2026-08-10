;;; pnix-clj의 방식 - interop boundary가 host object를 opaque ref와 capability witness로 다룬다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/80-interop-opaque-host-ref/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.interop :as interop]))

(def crossing-meta
  (interop/interop-meta {:direction :clojure->pnix
                         :effect-class :host-eval
                         :loss-status :opaque}))

(let [denied (interop/run-crossing :example-host-eval
                                   crossing-meta
                                   {:form '(+ 1 2)}
                                   #{:pure}
                                   (fn [] {:status :ok :value 3}))
      allowed (interop/run-crossing :example-host-eval
                                    crossing-meta
                                    {:form '(+ 1 2)}
                                    #{:pure :host-eval}
                                    (fn [] {:status :ok :value 3}))
      ref (interop/from-host (java.util.Date. 0))
      deref-before (interop/opaque-ref-deref ref)
      host-value (interop/to-host ref)]
  (println "denied:" (select-keys denied [:status :reason :capability :witness]))
  (println "allowed:" (select-keys allowed [:status :value :capability :witness]))
  (println "opaque ref:" ref)
  (println "deref before release:" (select-keys deref-before [:status]))
  (println "host class:" (.getName (class host-value)))

  (assert (= :held (:status denied)))
  (assert (= :capability-denied (:reason denied)))
  (assert (= :ok (:status allowed)))
  (assert (= 3 (:value allowed)))
  (assert (= :host-eval (get-in allowed [:witness :effect-class])))
  (assert (string? (get-in allowed [:witness :witness-hash])))
  (assert (interop/opaque-host-ref? ref))
  (assert (= :ok (:status deref-before)))
  (assert (instance? java.util.Date host-value))

  (interop/release-opaque-ref! ref)
  (let [deref-after (interop/opaque-ref-deref ref)]
    (println "deref after release:" (select-keys deref-after [:status :reason]))
    (assert (= :held (:status deref-after)))
    (assert (= :opaque-ref-released (:reason deref-after)))))

(println)
(println "결론: pnix-clj interop은 host effect를 deny-by-default로 gate하고, 허용 crossing과 opaque host object lifecycle에 witness를 붙인다.")
(shutdown-agents)
