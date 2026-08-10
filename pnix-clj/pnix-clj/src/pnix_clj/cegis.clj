(ns pnix-clj.cegis
  "Candidate GENERATOR #2 -- CEGIS refinement (counterexample-guided inductive
  synthesis; the Smyth/Burst loop shape from docs/GENERATOR_DECISION.md).

  The verifier DRIVES the generator (the CEGIS division of labor): the
  generator (pnix-clj.generate, observational enumeration) only has to satisfy
  a FINITE example set; the verifier probes the winning candidate against a
  REFERENCE implementation over a deterministic input set, and the first
  divergence becomes a COUNTEREXAMPLE added to the examples -- strengthening
  the spec and re-running synthesis (angelic -> analyze -> strengthen -> retry).

  ★Proven-vs-heuristic boundary (constitution): surviving all probes is still
  only bounded evidence -- :heuristic-no-counterexample. But when the converged
  candidate and the reference are both arithmetic, arith-proof/equivalent?
  (canonical polynomial) UPGRADES the verdict to :proven -- a genuine proof of
  semantic equality, not observation. The label always states which side of the
  boundary the result landed on; proposals stay HELD (no auto-promotion)."
  (:require [pnix-clj.arith-proof :as arith-proof]
            [pnix-clj.generate :as generate]
            [pnix-clj.hash :as hash]
            [pnix-clj.self-improve :as self-improve]
            [pnix-clj.store :as store]))

(def lane-classification
  {:lane :experimental
   :scope :bounded-candidate-generation
   :equivalence :counterexample-guided-refinement
   :proof-status :heuristic-unless-arith-proof-upgrades
   :admission :forbidden
   :auto-promotion :forbidden
   :runtime-write :forbidden
   :mutation :forbidden
   :allowed-output :candidate-set
   :handoff :self-improve-held-review-queue})

(def default-probes
  "Deterministic verification inputs (no clock, no randomness -- replayable)."
  (vec (range -7 21)))

(def wide-probes
  "Deterministic WIDE-RANGE hardening inputs: a fixed LCG walk over large
  magnitudes (seeded constant -- replayable, still no clock/randomness).
  Used after convergence to catch divergences outside the small probe range."
  (vec (take 32 (iterate (fn [x] (mod (+ (* 1103515245 x) 12345) 2147483647))
                         987654321))))

(defn- val-at
  "Value of single-var `expr` at probe `p`, or nil if it errors."
  [expr v p]
  (first (generate/value-vector expr [{:in {v p}}])))

(defn counterexample
  "The first probe input where candidate and reference DISAGREE (including one
  erroring where the other does not), or nil if all probes agree."
  [candidate reference v probes]
  (first (filter (fn [p] (not= (val-at candidate v p) (val-at reference v p)))
                 probes)))

(defn cegis-synthesize
  "CEGIS loop: synthesize from examples -> probe the candidate against
  `reference` -> a divergence becomes a counterexample appended to the examples
  -> retry. On survival, attempt the PROVEN upgrade via arith-proof.
  spec = {:vars [v] :reference expr :max-iters n :probes [..] :max-size n
          :store <§5 log>}.
  Returns {:status :converged|:exhausted :candidate :iterations :examples-used
           :proof-status :proven|:heuristic-no-counterexample ...}."
  [{:keys [vars reference max-iters probes max-size store seed-probe]
    :or {max-iters 8 probes default-probes max-size 3}}]
  (let [v (first vars)
        ref-out #(val-at reference v %)
        seed-probe (or seed-probe
                       (first (filter #(some? (ref-out %)) probes)))]
    (if (nil? seed-probe)
      {:status :exhausted :reason :reference-not-evaluable :reference reference}
      (loop [examples [{:in {v seed-probe} :out (ref-out seed-probe)}]
             iter 1]
        (let [{:keys [matches]} (generate/synthesize
                                 {:vars vars :examples examples :max-size max-size})
              cand (first matches)]
          (when store
            (store/append! store :cegis/iteration
                           {:iter iter :examples (count examples)
                            :matches (count matches)}))
          (cond
            (nil? cand)
            {:status :exhausted :reason :no-candidate-for-examples
             :iterations iter :examples-used (count examples)}

            :else
            (if-let [cex (or (counterexample cand reference v probes)
                             ;; hardening: wide-range deterministic probes --
                             ;; a divergence out here is a counterexample too
                             (counterexample cand reference v wide-probes))]
              (if (>= iter max-iters)
                {:status :exhausted :reason :max-iters-reached
                 :iterations iter :last-candidate cand :last-counterexample cex}
                (recur (conj examples {:in {v cex} :out (ref-out cex)})
                       (inc iter)))
              (let [proven? (arith-proof/equivalent? cand reference)]
                {:status :converged
                 :candidate cand
                 :reference reference
                 :iterations iter
                 :examples-used (count examples)
                 :probes-survived (count probes)
                 :proof-status (if proven? :proven :heuristic-no-counterexample)}))))))))

(defn cegis-and-propose
  "Run CEGIS and, on convergence, feed the candidate to self-improve as a HELD
  proposal (rationale carries the proof-status honestly)."
  ([log spec] (cegis-and-propose log spec :owner-hold))
  ([log {:keys [vars] :as spec} policy]
   (let [result (cegis-synthesize (assoc spec :store log))]
     (if (not= :converged (:status result))
       (assoc result :proposals [])
       (let [v (first vars)
             arg (first (filter #(some? (val-at (:reference spec) v %))
                                (or (:probes spec) default-probes)))
             round (self-improve/evaluate-round
                    log
                    [{:target :cegis-synthesized
                      :new-source (str "(" v ": " (:candidate result) ") " arg)
                      :rationale (str "CEGIS converged in " (:iterations result)
                                      " iteration(s); " (name (:proof-status result))
                                      ": " v ": " (:candidate result)
                                      " vs reference " (:reference spec))}]
                    policy)]
         (merge result round))))))

;; ---- report --------------------------------------------------------------

(defn report
  []
  (let [log (store/open-store)
        ;; seed at x=0 makes the constant "2" collide with x+2 -> the probe
        ;; counterexample DRIVES refinement (deterministic CEGIS demo)
        r1 (cegis-synthesize {:vars ["x"] :reference "x + 2" :seed-probe 0
                              :store log})
        ;; harder reference, still converges + proven (2x+3 reachable at size 3)
        r2 (cegis-synthesize {:vars ["x"] :reference "2 * x + 3"})
        ;; UNREACHABLE at max-size 2 (x*x + 1 needs a size-3 combine) -> honest exhaustion
        r3 (cegis-synthesize {:vars ["x"] :reference "x * x + 1" :max-size 2
                              :max-iters 4})
        prop (cegis-and-propose (store/open-store)
                                {:vars ["x"] :reference "2 * x + 3"})
        rows [{:id :converges-to-reference :ok? (= :converged (:status r1))}
              {:id :proof-upgraded-to-proven
               ;; the constitution's boundary crossed HONESTLY: polynomial proof
               :ok? (= :proven (:proof-status r1))}
              {:id :counterexamples-drove-refinement
               ;; 1 seed example underdetermines; probes must have refined
               :ok? (> (:iterations r1) 1)}
              {:id :survivor-agrees-on-all-probes
               :ok? (nil? (counterexample (:candidate r1) "x + 2" "x"
                                          default-probes))}
              {:id :survivor-agrees-on-wide-probes
               :ok? (nil? (counterexample (:candidate r1) "x + 2" "x"
                                          wide-probes))}
              {:id :harder-reference-converges-proven
               :ok? (and (= :converged (:status r2)) (= :proven (:proof-status r2)))}
              {:id :unreachable-exhausts-honestly
               :ok? (= :exhausted (:status r3))}
              {:id :iterations-recorded-as-events
               :ok? (pos? (count (store/events-of log :cegis/iteration)))}
              {:id :proposal-fed-and-held
               :ok? (and (= :converged (:status prop)) (:all-held? prop))}]
        rejected (count (remove :ok? rows))
        body {:kind :pnix-cegis-report
              :schema :pnix-clj.cegis-report.v0
              :policy :counterexample-guided-refinement-verifier-drives-generator-proven-upgrade
              :total (count rows)
              :accepted (- (count rows) rejected)
              :rejected rejected
              :rows (mapv (fn [r] (assoc r :status (if (:ok? r) :accepted :rejected))) rows)}]
    (assoc body
           :status (if (zero? rejected) :ok :failed)
           :report-hash (hash/data-hash (:rows body)))))

(defn -main
  [& _]
  (let [{:keys [status total accepted rejected]} (report)]
    (println (format "pnix-clj cegis: status=%s total=%d accepted=%d rejected=%d"
                     (name status) total accepted rejected))
    (shutdown-agents)))
