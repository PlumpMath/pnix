(ns pnix-clj.self-improve
  "The self-* loop BODY -- evaluate a batch of candidate self-modifications and
  produce ranked, GATED proposals for owner review. This is the honest,
  bounded shape of the north-star recursive self-* loop: the machine may PROPOSE
  and gather admitting evidence for each candidate, but every proposal stays
  HELD (no auto-promotion) until the owner authorizes -- the constitution's
  invariant, enforced by pnix-clj.self-mod-gate.

  ★Honest scope: this driver is GENERATION-AGNOSTIC -- it takes candidates as
  input (from a synthesizer, a mutation pass, or a human), witnesses each via
  run-witnessed, gates each, ranks them by admitting evidence, and records the
  whole round as §5 events. It does NOT claim to invent candidates; it is the
  loop body a generator plugs into. The output is a review queue, not an applied
  change."
  (:require [pnix-clj.hash :as hash]
            [pnix-clj.self-mod-gate :as gate]
            [pnix-clj.store :as store]))

(def lane-classification
  {:lane :experimental
   :scope :bounded-proof-experiment
   :admission :owner-gated
   :auto-promotion :forbidden
   :autonomous-mutation :forbidden
   :runtime-write :forbidden
   :allowed-output :held-review-queue})

(defn- rank-key
  "Order proposals best-first: admitted-witness candidates ahead of the rest,
  then deterministically by proposed-hash (stable, no clock)."
  [{:keys [decision witness-status proposed-hash]}]
  [(case decision :admitted 0 :held 1 :rejected 2 3)
   (if (= :admitted witness-status) 0 1)
   (str proposed-hash)])

(defn evaluate-round
  "Evaluate a batch of candidate self-modifications under `policy` (default
  :owner-hold). Each candidate is {:target :new-source :rationale}. Returns
  {:proposals [..] :ranked [..] :all-held? bool :round-hash ..}. With the
  default policy EVERY admitted proposal is :held -- the review queue for the
  owner, never auto-applied."
  ([store candidates] (evaluate-round store candidates :owner-hold))
  ([store candidates policy]
   (let [proposals
         (mapv (fn [cand]
                 (let [p (gate/propose! store cand)
                       d (gate/decide store p policy)]
                   {:target (:target cand)
                    :rationale (:rationale cand)
                    :proposed-hash (:proposed-hash p)
                    :witness-status (:witness-status p)
                    :decision (:decision d)
                    :reason (:reason d)}))
               candidates)
         ranked (vec (sort-by rank-key proposals))]
     (store/append! store :self-improve/round
                    {:candidate-count (count candidates)
                     :policy policy
                     :admitted-witnesses (count (filter #(= :admitted (:witness-status %)) proposals))
                     :held (count (filter #(= :held (:decision %)) proposals))})
     {:proposals proposals
      :ranked ranked
      :all-held? (every? #(not= :admitted (:decision %)) proposals)
      :best (first ranked)
      :round-hash (hash/data-hash proposals)})))

;; ---- report --------------------------------------------------------------

(defn report
  []
  (let [store (store/open-store)
        candidates [{:target :inc      :new-source "(x: x + 1) 41"      :rationale "increment"}
                    {:target :double   :new-source "(x: x * 2) 21"      :rationale "double"}
                    {:target :sum      :new-source "let a = 1; b = 2; in a + b" :rationale "sum"}
                    {:target :bad      :new-source "x: x"               :rationale "bare lambda (no value)"}]
        held-round (evaluate-round store candidates :owner-hold)
        auth-round (evaluate-round (store/open-store) candidates :owner-authorized)
        rows [{:id :every-candidate-evaluated
               :ok? (= 4 (count (:proposals held-round)))}
              {:id :value-candidates-earn-admitted-witness
               :ok? (= 3 (count (filter #(= :admitted (:witness-status %))
                                        (:proposals held-round))))}
              {:id :default-policy-holds-all
               :ok? (:all-held? held-round)}
              {:id :owner-authorized-can-promote
               :ok? (not (:all-held? auth-round))}
              {:id :ranked-best-first
               :ok? (let [ds (map :decision (:ranked held-round))]
                      (= ds (sort-by {:admitted 0 :held 1 :rejected 2} ds)))}
              {:id :round-recorded-as-event
               :ok? (= 1 (count (store/events-of store :self-improve/round)))}
              {:id :bare-lambda-not-admitted
               :ok? (= :rejected (:decision (first (filter #(= :bad (:target %))
                                                           (:proposals held-round)))))}]
        rejected (count (remove :ok? rows))
        body {:kind :pnix-self-improve-report
              :schema :pnix-clj.self-improve-report.v0
              :policy :evaluate-candidates-witness-and-gate-produce-ranked-held-proposals
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
    (println (format "pnix-clj self-improve: status=%s total=%d accepted=%d rejected=%d"
                     (name status) total accepted rejected))
    (shutdown-agents)))
