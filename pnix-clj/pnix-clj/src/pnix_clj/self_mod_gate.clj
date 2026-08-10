(ns pnix-clj.self-mod-gate
  "§14.3 — self-modification gate. Operationalizes the pnix-clj constitution's
  load-bearing invariant -- NO AUTO-PROMOTION -- as a runtime gate over
  self-generated changes.

  A self-modification proposal is evaluated by `run-witnessed` (the spine
  integration): it earns a §15 witness that is :admitted only if the new code
  collapses across substrates, converges over repeated runs, and is
  deterministic. But even a fully-admitted witness is NOT auto-promoted: the
  gate FAILS CLOSED to :held by default, because promotion is the owner's call
  (boundary discipline). Only an EXPLICIT :owner-authorized policy -- a
  deliberate owner act -- turns an admitted witness into :admitted.

  Every step is recorded as §5 events (:self-mod/proposed → :self-mod/held |
  :self-mod/admitted | :self-mod/rejected), so the decision trail is durable
  and auditable. This is how self-* stays robust: the machine may PROPOSE and
  gather admitting evidence, but a human authorizes the actual change."
  (:require [pnix-clj.hash :as hash]
            [pnix-clj.store :as store]
            [pnix-clj.witnessed-run :as witnessed-run]))

(def lane-classification
  {:lane :experimental
   :scope :gate-only
   :direct-mutation :forbidden
   :auto-admission :forbidden
   :auto-promotion :forbidden
   :owner-authorization-required true
   :default-decision :held
   :allowed-output :proposal-decision-event})

(def policies
  "The gate policy set. :owner-hold (default) never auto-promotes; the explicit
  :owner-authorized policy represents a deliberate owner act."
  #{:owner-hold :owner-authorized})

(defn propose!
  "Evaluate a proposed self-modification through the witnessed-run spine and
  record it as a :self-mod/proposed event. Returns the proposal with its
  witness (still unpromoted)."
  [store {:keys [target new-source rationale]}]
  (let [wr (witnessed-run/run-witnessed new-source)
        proposal {:target target
                  :rationale rationale
                  :proposed-hash (hash/sha256 new-source)
                  :term-hash (get-in wr [:witness :term-hash])
                  :witness (:witness wr)
                  :witness-status (:status wr)}]
    (store/append! store :self-mod/proposed
                   {:target target
                    :proposed-hash (:proposed-hash proposal)
                    :witness-status (:witness-status proposal)})
    proposal))

(defn decide
  "Decide a proposal under `policy`. CONSTITUTION: no auto-promotion.
    - witness not :admitted            → :rejected (evidence insufficient)
    - :admitted witness + :owner-hold  → :held  (default -- owner must sign off)
    - :admitted witness + :owner-authorized → :admitted (explicit owner act)
  Records the decision as a §5 event. FAILS CLOSED to :held for anything it does
  not recognize as an explicit authorization."
  [store proposal policy]
  (let [base {:target (:target proposal) :proposed-hash (:proposed-hash proposal)}
        [decision reason kind]
        (cond
          (not= :admitted (:witness-status proposal))
          [:rejected :witness-not-admitted :self-mod/rejected]

          (= :owner-authorized policy)
          [:admitted :owner-authorized :self-mod/admitted]

          :else
          [:held :no-auto-promotion-owner-required :self-mod/held])]
    (store/append! store kind (assoc base :reason reason))
    {:decision decision :reason reason
     :target (:target proposal)
     :witness-status (:witness-status proposal)}))

(defn propose-and-gate
  "Convenience: propose then decide in one call, sharing a store."
  ([store proposal] (propose-and-gate store proposal :owner-hold))
  ([store proposal policy]
   (decide store (propose! store proposal) policy)))

;; ---- report --------------------------------------------------------------

(defn report
  []
  (let [store (store/open-store)
        ;; a valid deterministic self-modification earns an admitted witness...
        good {:target :new-fn :new-source "(x: x + 1) 41" :rationale "add increment"}
        held    (propose-and-gate store good :owner-hold)        ; ...but is HELD
        admitted (decide store (propose! store good) :owner-authorized)
        ;; a proposal whose witness is not admitted is rejected regardless
        prop-good (propose! store good)
        rows [{:id :admitted-witness-still-held-by-default
               :ok? (and (= :held (:decision held))
                         (= :no-auto-promotion-owner-required (:reason held)))}
              {:id :owner-authorized-promotes
               :ok? (= :admitted (:decision admitted))}
              {:id :non-admitted-witness-rejected
               :ok? (= :rejected (:decision (decide store
                                              (assoc prop-good :witness-status :rejected)
                                              :owner-authorized)))}
              {:id :unknown-policy-fails-closed-to-held
               :ok? (= :held (:decision (decide store prop-good :some-random-policy)))}
              {:id :decisions-recorded-as-events
               :ok? (and (pos? (count (store/events-of store :self-mod/proposed)))
                         (pos? (count (store/events-of store :self-mod/held)))
                         (pos? (count (store/events-of store :self-mod/admitted))))}
              {:id :event-log-intact
               :ok? (= :intact (:status (store/verify-chain store)))}]
        rejected (count (remove :ok? rows))
        body {:kind :pnix-self-mod-gate-report
              :schema :pnix-clj.self-mod-gate-report.v0
              :policy :no-auto-promotion-admitted-witness-held-until-owner-authorizes
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
    (println (format "pnix-clj self-mod-gate: status=%s total=%d accepted=%d rejected=%d"
                     (name status) total accepted rejected))
    (shutdown-agents)))
