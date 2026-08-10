;;; pnix-clj의 방식: host crossing에 effect/loss/capability/witness를 붙인다.
;;;
;;; 실행:
;;;   cd pnix-clj
;;;   clojure -M examples/04-host-interop-loss-effect/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.interop :as interop]))

(let [meta (interop/interop-meta {:direction :clj->pnix
                                  :effect-class :pure
                                  :loss-status :lossless})]
  (println "interop meta:" meta)
  (assert (= :pure (:effect-class meta)))
  (assert (= :lossless (:loss-status meta))))

(let [denied (interop/check-capability :host-eval #{:pure})]
  (println "host-eval without grant:" (:status denied) (:reason denied))
  (assert (= :held (:status denied))))

(let [granted (interop/check-capability :host-eval interop/host-eval-capabilities)]
  (println "host-eval with grant:" (:status granted))
  (assert (= :ok (:status granted))))

(let [r (interop/host-eval-form :demo-host-eval
                                '(+ 20 22)
                                interop/host-eval-capabilities)]
  (println "host eval result:" (:status r) (:value r))
  (println "witness hash:" (get-in r [:witness :witness-hash]))
  (assert (= :ok (:status r)))
  (assert (= 42 (:value r)))
  (assert (string? (get-in r [:witness :witness-hash]))))

(let [ref (interop/make-opaque-host-ref (StringBuilder. "pnix"))]
  (println "opaque host ref:" (select-keys ref [:kind :id :class]))
  (assert (= :opaque-host-ref (:kind ref)))
  (assert (integer? (:id ref)))
  (assert (interop/opaque-host-ref? ref))
  (interop/release-opaque-ref! ref))

(println)
(println "결론: pnix-clj interop는 crossing을 값만이 아니라 effect/loss/capability/witness까지 함께 다룬다.")
