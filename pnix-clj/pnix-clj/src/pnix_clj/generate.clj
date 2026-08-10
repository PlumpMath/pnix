(ns pnix-clj.generate
  "The FIRST candidate GENERATOR for the self-* loop -- observational-
  equivalence-reduced bottom-up enumerative synthesis (Escher, CAV'13). See
  docs/GENERATOR_DECISION.md for why this was chosen.

  Given a spec = input vars + input/output EXAMPLES, enumerate pnix expressions
  bottom-up and keep at most ONE representative per behavioral class: every
  candidate is evaluated (via the existing pnix evaluator, core/eval-source) on
  the example inputs to get a VALUE-VECTOR, and a candidate whose value-vector
  was already seen is discarded (observational-equivalence reduction -- the
  mechanism that tames the combinatorial blow-up). A candidate whose value-
  vector equals the example OUTPUTS is a match.

  ★Proven-vs-heuristic boundary (constitution): a value-vector match on FINITE
  examples is OBSERVATIONAL equivalence -- a HEURISTIC PROPOSE, never a proof of
  semantic equivalence. So `synthesize` only PROPOSES; `synthesize-and-propose`
  hands each match to self-improve, where run-witnessed proves it is a
  well-behaved pnix program and self-mod-gate holds it for the owner (no
  auto-promotion). arith-proof/bool-proof can later upgrade a match to PROVEN."
  (:require [clojure.string :as str]
            [pnix-clj.arith-proof :as arith-proof]
            [pnix-clj.core :as core]
            [pnix-clj.hash :as hash]
            [pnix-clj.self-improve :as self-improve]
            [pnix-clj.store :as store]))

(def lane-classification
  {:lane :experimental
   :scope :bounded-candidate-generation
   :equivalence :observational-finite-examples
   :proof-status :heuristic-proposal-only
   :admission :forbidden
   :auto-promotion :forbidden
   :runtime-write :forbidden
   :mutation :forbidden
   :allowed-output :candidate-set
   :handoff :self-improve-held-review-queue})

(def default-ops ["+" "-" "*"])
(def default-literals ["0" "1" "2"])

(defn- eval-in
  "Evaluate an expression string with `env` bound (integers), returning the
  value or nil if it errors."
  [expr env]
  (let [binds (str/join " " (map (fn [[k v]] (str k " = " v ";")) env))
        src (str "let " binds " in (" expr ")")
        r (core/eval-source src)]
    (when (= :ok (:status r)) (:value r))))

(defn value-vector
  "The behavior of `expr` over the example inputs, or nil if any input errors."
  [expr examples]
  (let [vs (mapv #(eval-in expr (:in %)) examples)]
    (when (every? some? vs) vs)))

(defn synthesize
  "Bottom-up enumerative synthesis with observational-equivalence dedup.
  spec = {:vars [\"x\"] :examples [{:in {\"x\" 1} :out 2} ..] :max-size n
          :ops [..] :literals [..]}.
  Returns {:matches [expr-strings] :classes <#behavioral-classes>
           :enumerated <#candidates-tried>}. A match's value-vector equals the
  example outputs (HEURISTIC -- observational, not proven)."
  [{:keys [vars examples max-size ops literals canonical-prune?]
    :or {max-size 3 ops default-ops literals default-literals
         canonical-prune? true}}]
  (let [targets (mapv :out examples)
        terminals (concat vars literals)
        seen (atom {})                      ; value-vector -> representative expr
        seen-polys (atom #{})               ; canonical polynomials (Knuth-Bendix-style)
        enumerated (atom 0)
        pruned (atom 0)
        add! (fn [expr]
               (swap! enumerated inc)
               ;; generator #3 -- canonical pre-pruning: a candidate whose
               ;; canonical POLYNOMIAL (arith-proof) was already seen is a
               ;; PROVEN duplicate -- skip the expensive evaluation entirely.
               ;; SOUND: same polynomial => equal for ALL inputs => same
               ;; value-vector => the observational dedup would drop it anyway;
               ;; the match set is provably unchanged. Non-arithmetic
               ;; candidates (poly nil) fall back to observational dedup.
               (let [poly (when canonical-prune?
                            (arith-proof/poly-of-source expr))]
                 (if (and poly (contains? @seen-polys poly))
                   (swap! pruned inc)
                   (do (when poly (swap! seen-polys conj poly))
                       (when-let [vv (value-vector expr examples)]
                         (when-not (contains? @seen vv)
                           (swap! seen assoc vv expr)))))))]
    (doseq [t terminals] (add! t))
    (dotimes [_ (max 0 (dec max-size))]
      (let [reps (vec (vals @seen))]
        (doseq [a reps, b reps, op ops]
          (add! (str a " " op " " b)))))
    {:matches (vec (sort (for [[vv expr] @seen :when (= vv targets)] expr)))
     :classes (count @seen)
     :enumerated @enumerated
     :pruned-proven @pruned
     :evaluated (- @enumerated @pruned)}))

(defn synthesize-and-propose
  "Synthesize candidates for a single-variable spec and feed each match to
  self-improve as a witnessable, HELD proposal (the lambda applied to the first
  example input). Returns the self-improve round result + {:matches ..}."
  ([store spec] (synthesize-and-propose store spec :owner-hold))
  ([store {:keys [vars examples] :as spec} policy]
   (let [{:keys [matches] :as syn} (synthesize spec)
         v (first vars)
         arg (get (:in (first examples)) v)
         candidates (mapv (fn [expr]
                            {:target :synthesized
                             :new-source (str "(" v ": " expr ") " arg)
                             :rationale (str "observational match (heuristic): "
                                             v ": " expr)})
                          matches)
         round (self-improve/evaluate-round store candidates policy)]
     (assoc round :matches matches :synthesis (dissoc syn :matches)))))

;; ---- report --------------------------------------------------------------

(defn report
  []
  (let [;; f(x) = x + 1  from examples (1,2) (2,3) (3,4)
        inc-spec {:vars ["x"]
                  :examples [{:in {"x" 1} :out 2} {:in {"x" 2} :out 3}
                             {:in {"x" 3} :out 4}]
                  :max-size 3}
        inc-syn (synthesize inc-spec)
        ;; f(x) = x * x  (needs a product; reachable at size 2)
        sq-syn (synthesize {:vars ["x"]
                            :examples [{:in {"x" 2} :out 4} {:in {"x" 3} :out 9}
                                       {:in {"x" 4} :out 16}]
                            :max-size 3})
        ;; unreachable within the grammar/size -> honestly no match
        none-syn (synthesize {:vars ["x"]
                              :examples [{:in {"x" 1} :out 100}]
                              :max-size 2 :literals ["0" "1"]})
        store (store/open-store)
        proposed (synthesize-and-propose store inc-spec)
        rows [{:id :finds-increment
               :ok? (some #(= [2 3 4] (value-vector % (:examples inc-spec)))
                          (:matches inc-syn))}
              {:id :finds-square
               :ok? (boolean (seq (:matches sq-syn)))}
              {:id :observational-dedup-shrinks-space
               ;; far fewer behavioral classes than candidates enumerated
               :ok? (< (:classes inc-syn) (:enumerated inc-syn))}
              {:id :canonical-prune-sound
               ;; generator #3: pruning provably changes nothing but the cost
               :ok? (= (:matches inc-syn)
                       (:matches (synthesize (assoc inc-spec
                                                    :canonical-prune? false))))}
              {:id :canonical-prune-effective
               :ok? (and (pos? (:pruned-proven inc-syn))
                         (< (:evaluated inc-syn) (:enumerated inc-syn)))}
              {:id :unreachable-honestly-empty
               :ok? (empty? (:matches none-syn))}
              {:id :matches-are-observational-only
               ;; every match reproduces the example outputs (by construction)
               :ok? (every? #(= [2 3 4] (value-vector % (:examples inc-spec)))
                            (:matches inc-syn))}
              {:id :proposals-fed-to-self-improve-held
               :ok? (and (seq (:proposals proposed)) (:all-held? proposed))}
              {:id :proposals-have-admitted-witnesses
               :ok? (some #(= :admitted (:witness-status %)) (:proposals proposed))}]
        rejected (count (remove :ok? rows))
        body {:kind :pnix-generate-report
              :schema :pnix-clj.generate-report.v0
              :policy :observational-equivalence-reduced-enumeration-proposes-heuristic-matches
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
    (println (format "pnix-clj generate: status=%s total=%d accepted=%d rejected=%d"
                     (name status) total accepted rejected))
    (shutdown-agents)))
