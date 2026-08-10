;;; pnix-clj의 방식 — interop/run-crossing으로 capability gate와 typed witness를 함께 남긴다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/25-typed-attestation/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.interop :as interop]))

(defn host-add-two
  [x]
  (+ x 2))

(defn crossing-meta
  [effect loss]
  (interop/interop-meta {:direction :clojure->host-value
                         :effect-class effect
                         :loss-status loss}))

(defn typed-attestation
  [result]
  (let [w (:witness result)]
    {:status (:status result)
     :reason (:reason result)
     :value (:value result)
     :capability (:capability result)
     :interop (:interop result)
     :witness-schema (:schema w)
     :witness-kind (:kind w)
     :witness-direction (:direction w)
     :witness-effect-class (:effect-class w)
     :witness-loss-status (:loss-status w)
     :input-hash (:input-hash w)
     :output-hash (:output-hash w)
     :witness-hash (:witness-hash w)
     :witness? (interop/witness? w)}))

(let [input 40

      pure-meta (crossing-meta :pure :lossless)
      pure-result
      (interop/run-crossing
       :typed-attestation-pure-crossing
       pure-meta
       {:input input}
       #{:pure}
       (fn []
         {:status :ok
          :value (host-add-two input)}))

      denied-meta (crossing-meta :host-eval :opaque)
      denied-result
      (interop/run-crossing
       :typed-attestation-denied-crossing
       denied-meta
       {:form '(+ 40 2)}
       #{:pure}
       (fn []
         {:status :ok
          :value 42}))

      pure-attestation (typed-attestation pure-result)
      denied-attestation (typed-attestation denied-result)]

  (println "pure crossing attestation:")
  (println pure-attestation)

  (println)
  (println "denied crossing attestation:")
  (println denied-attestation)

  ;; accepted crossing
  (assert (= :ok (:status pure-attestation)))
  (assert (= 42 (:value pure-attestation)))
  (assert (= {:status :ok :effect :pure :granted true}
             (:capability pure-attestation)))
  (assert (= :pnix-clj.interop.v0
             (:schema (:interop pure-attestation))))
  (assert (= :pnix-clj.interop.witness.v0
             (:witness-schema pure-attestation)))
  (assert (= :typed-attestation-pure-crossing
             (:witness-kind pure-attestation)))
  (assert (= :clojure->host-value
             (:witness-direction pure-attestation)))
  (assert (= :pure (:witness-effect-class pure-attestation)))
  (assert (= :lossless (:witness-loss-status pure-attestation)))
  (assert (string? (:input-hash pure-attestation)))
  (assert (string? (:output-hash pure-attestation)))
  (assert (string? (:witness-hash pure-attestation)))
  (assert (= true (:witness? pure-attestation)))

  ;; denied crossing still has typed witness
  (assert (= :held (:status denied-attestation)))
  (assert (= :capability-denied (:reason denied-attestation)))
  (assert (= :host-eval (:effect (:capability denied-attestation))))
  (assert (nil? (:granted (:capability denied-attestation))))
  (assert (= :pnix-clj.interop.witness.v0
             (:witness-schema denied-attestation)))
  (assert (= :typed-attestation-denied-crossing
             (:witness-kind denied-attestation)))
  (assert (= :host-eval (:witness-effect-class denied-attestation)))
  (assert (= :opaque (:witness-loss-status denied-attestation)))
  (assert (string? (:input-hash denied-attestation)))
  (assert (string? (:output-hash denied-attestation)))
  (assert (string? (:witness-hash denied-attestation)))
  (assert (= true (:witness? denied-attestation))))

(println)
(println "결론: pnix-clj는 capability crossing의 허용/거부 모두에 typed interop witness attestation을 남긴다.")
