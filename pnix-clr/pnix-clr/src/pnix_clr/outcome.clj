(ns pnix-clr.outcome)

(def schema "pnix.machine.host-outcome.v1")
(def model-schema "pnix.machine.eval-outcome-model.v1")
(def production-projection-schema
  "pnix.production-basic-outcome-projection.v1")

;; These nominal CLR types keep machine control outside the forgeable guest
;; map domain.  The portable model remains owned by pnix-meta; this namespace
;; is only its ClojureCLR representation and observer adapter.
(deftype EvalError [phase className evidence])
(deftype EffectRequest [effect args])
(deftype Continuation [id])
(deftype Checkpoint [id])
(deftype ResourceReason [reasonClass divergenceProven])
(deftype Done [value])
(deftype Failed [error])
(deftype Requested [request continuation])
(deftype Suspended [checkpoint reason])

(defn done
  [value]
  (Done. value))

(defn failed
  ([phase class-name]
   (failed phase class-name {}))
  ([phase class-name evidence]
   (Failed.
    (EvalError. (name phase) (name class-name) (or evidence {})))))

(defn requested
  [effect args continuation-id]
  (Requested. (EffectRequest. effect (or args {}))
              (Continuation. continuation-id)))

(defn suspended
  [checkpoint-id reason-class]
  (Suspended. (Checkpoint. checkpoint-id)
              (ResourceReason. reason-class false)))

(defn machine-outcome?
  [value]
  (or (instance? Done value)
      (instance? Failed value)
      (instance? Requested value)
      (instance? Suspended value)))

(defn done?
  [value]
  (instance? Done value))

(defn failed?
  [value]
  (instance? Failed value))

(defn kind
  [value]
  (cond
    (instance? Done value) :done
    (instance? Failed value) :failed
    (instance? Requested value) :requested
    (instance? Suspended value) :suspended
    :else nil))

(defn value-of
  [value]
  (when (instance? Done value)
    (.-value ^Done value)))

(defn error-of
  [value]
  (when (instance? Failed value)
    (let [error (.-error ^Failed value)]
      {:phase (.-phase ^EvalError error)
       :class (.-className ^EvalError error)
       :evidence (.-evidence ^EvalError error)})))

(defn observe-boundary
  [value]
  (cond
    (instance? Done value)
    (sorted-map
     "status" "done"
     "value" (.-value ^Done value))

    (instance? Failed value)
    (let [error (.-error ^Failed value)]
      (sorted-map
       "error" (sorted-map
                "class" (.-className ^EvalError error)
                "evidence" (.-evidence ^EvalError error)
                "phase" (.-phase ^EvalError error))
       "status" "failed"))

    (instance? Requested value)
    (let [request (.-request ^Requested value)
          continuation (.-continuation ^Requested value)]
      (sorted-map
       "continuation" (sorted-map
                       "id" (.-id ^Continuation continuation))
       "request" (sorted-map
                  "args" (.-args ^EffectRequest request)
                  "effect" (.-effect ^EffectRequest request))
       "status" "requested"))

    (instance? Suspended value)
    (let [checkpoint (.-checkpoint ^Suspended value)
          reason (.-reason ^Suspended value)]
      (sorted-map
       "checkpoint" (sorted-map "id" (.-id ^Checkpoint checkpoint))
       "reason" (sorted-map
                 "class" (.-reasonClass ^ResourceReason reason)
                 "divergence_proven"
                 (.-divergenceProven ^ResourceReason reason))
       "status" "suspended"))

    :else
    (throw (ex-info "value is not a MachineOutcome" {}))))

(defn fail!
  ([phase class-name]
   (fail! phase class-name {}))
  ([phase class-name evidence]
   (throw (ex-info "structured pnix-clr failure"
                   {::failure true
                    ::phase phase
                    ::class class-name
                    ::evidence (or evidence {})}))))

(defn capture
  "Seal a PNIX computation in the nominal host MachineOutcome domain.
  Structured language failures become Failed. Unexpected CLR exceptions are
  infrastructure faults and remain loud rather than being reclassified."
  [f]
  (try
    (done (f))
    (catch System.Exception error
      (let [data (ex-data error)]
        (if (::failure data)
          (failed (::phase data) (::class data) (::evidence data))
          (throw error))))))

(defn self-check
  []
  (let [done-value (done "value")
        failed-value (failed :eval :not-callable)
        requested-value (requested "open" {} 1)
        suspended-value (suspended 2 "resource-budget-exhausted")
        guest-shape {"outcome_kind" "done"}
        done-observed (observe-boundary done-value)
        failed-observed (observe-boundary failed-value)
        requested-observed (observe-boundary requested-value)
        suspended-observed (observe-boundary suspended-value)]
    (assert (= "done" (get done-observed "status")))
    (assert (= "eval" (get-in failed-observed ["error" "phase"])))
    (assert (= "not-callable" (get-in failed-observed ["error" "class"])))
    (assert (= "open" (get-in requested-observed ["request" "effect"])))
    (assert (false?
             (get-in suspended-observed ["reason" "divergence_proven"])))
    (assert (not (machine-outcome? guest-shape)))
    (sorted-map
     "all_ok" true
     "done" (get done-observed "status")
     "failed_class" (get-in failed-observed ["error" "class"])
     "failed_phase" (get-in failed-observed ["error" "phase"])
     "guest_shape_is_outcome" (machine-outcome? guest-shape)
     "requested" (get requested-observed "status")
     "requested_effect" (get-in requested-observed ["request" "effect"])
     "schema" schema
     "suspended" (get suspended-observed "status")
     "suspended_divergence_proven"
     (get-in suspended-observed ["reason" "divergence_proven"]))))
