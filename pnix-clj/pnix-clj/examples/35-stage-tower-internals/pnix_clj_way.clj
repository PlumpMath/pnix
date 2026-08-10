;;; pnix-clj의 방식 — tower/run-tower의 내부 layers, adjacent pairs,
;;; collapse witness, held frontier를 전시한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/35-stage-tower-internals/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.tower :as tower]))

(def source
  "let x = 40; in x + 2")

(def held-input
  "builtins.getEnv \"HOME\"")

(def expected-layers
  [:read
   :emit-roundtrip
   :direct-eval
   :specialize-residual
   :lowering
   :clj-meta-host
   :px-runtime
   :pnix-mirror])

(def expected-pairs
  [[:read :emit-roundtrip]
   [:direct-eval :specialize-residual]
   [:direct-eval :clj-meta-host]
   [:direct-eval :px-runtime]
   [:px-runtime :pnix-mirror]])

(defn layer-summary
  [t]
  (mapv (fn [{:keys [layer status value reason bytecode-determinism]}]
          {:layer layer
           :status status
           :value value
           :reason reason
           :bytecode-determinism bytecode-determinism})
        (:layers t)))

(defn pair-summary
  [t]
  (mapv (fn [{:keys [pair ok? evidence]}]
          {:pair pair
           :ok? ok?
           :evidence evidence})
        (:pairs t)))

(let [t (tower/run-tower source)
      layers (layer-summary t)
      pairs (pair-summary t)
      collapse (:collapse t)
      witness (:witness collapse)
      layer-names (mapv :layer layers)
      pair-names (mapv :pair pairs)
      all-layers-ok? (every? #(= :ok (:status %)) layers)
      all-pairs-ok? (every? :ok? pairs)]

  (println "source:" source)
  (println "collapse:" (:status collapse) "value=" (:value collapse))
  (println "witness:" witness)

  (println)
  (println "layers:")
  (doseq [{:keys [layer status value reason bytecode-determinism]} layers]
    (println " -" layer
             "status=" status
             "value=" value
             "reason=" reason
             "bytecode-determinism=" bytecode-determinism))

  (println)
  (println "pairs:")
  (doseq [{:keys [pair ok? evidence]} pairs]
    (println " -" pair
             "ok?=" ok?
             "evidence=" evidence))

  (println)
  (println "all layers ok?:" all-layers-ok?)
  (println "all pairs ok?:" all-pairs-ok?)

  (assert (= expected-layers layer-names))
  (assert (= expected-pairs pair-names))
  (assert (= true all-layers-ok?))
  (assert (= true all-pairs-ok?))
  (assert (= :collapsed (:status collapse)))
  (assert (= 42 (:value collapse)))
  (assert (= expected-layers (:agreeing-layers collapse)))
  (assert (string? (:source-hash witness)))
  (assert (string? (:ast-hash witness)))
  (assert (= :ok (:cross-mirror witness))))

(let [held (tower/run-tower held-input)
      collapse (:collapse held)
      blocking (:blocking collapse)]
  (println)
  (println "held frontier source:" held-input)
  (println "held collapse:" (:status collapse))
  (println "blocking:" blocking)

  (assert (= :held (:status collapse)))
  (assert (map? blocking))
  (assert (or (contains? blocking :layer)
              (contains? blocking :pair))))

(println)
(println "결론: pnix-clj tower는 collapse 결과만이 아니라 layer/pair/witness와 held blocking point까지 전시한다.")
