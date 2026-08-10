(ns pnix-cljs.outcome)

(def schema "pnix.machine.host-outcome.v1")

(defrecord Done [value])
(defrecord Failed [error])
(defrecord Requested [request continuation])
(defrecord Suspended [checkpoint reason])

(defn done [value]
  (->Done value))

(defn failed [error]
  (->Failed error))

(defn requested [request continuation]
  (->Requested request continuation))

(defn suspended [checkpoint reason]
  (->Suspended checkpoint reason))

(defn done? [value]
  (instance? Done value))

(defn failed? [value]
  (instance? Failed value))

(defn requested? [value]
  (instance? Requested value))

(defn suspended? [value]
  (instance? Suspended value))

(defn outcome? [value]
  (or (done? value) (failed? value) (requested? value) (suspended? value)))

(defn project [outcome]
  (cond
    (done? outcome)
    {"schema" schema
     "outcome_kind" "done"
     "value" (:value outcome)}

    (failed? outcome)
    {"schema" schema
     "outcome_kind" "failed"
     "error" (:error outcome)}

    (requested? outcome)
    {"schema" schema
     "outcome_kind" "requested"
     "request" (:request outcome)
     "continuation" (:continuation outcome)}

    (suspended? outcome)
    {"schema" schema
     "outcome_kind" "suspended"
     "checkpoint" (:checkpoint outcome)
     "resource_reason" (:reason outcome)
     "divergence_proven" false}

    :else
    (throw (ex-info "not a pnix-cljs outcome"
                    {"phase" "boundary"
                     "class" "invalid-machine-outcome"}))))
