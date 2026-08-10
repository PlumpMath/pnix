(ns pnix-clj.witness
  "§15 — explicit witness schema + admission lattice (spine BUILD 8th, CAPSTONE;
  see docs/SPINE_ROADMAP.md).

  The witness is the integration point of the whole evidence spine: it binds a
  result to the exact runtime + inputs that produced it, in-toto / SLSA-shaped.
  A `witness-eval` captures, for one source:
    input-hash / term-hash (§3)  ·  snapshot-id + evaluator/symbol versions (§8)
    · determinism run recorded as evidence events (§5/§9)  ·  result-hash + status.

  ADMISSION LATTICE (the checklist's status lattice): held → candidate →
  admitted | rejected; candidate → held; terminal evidence/failed/ok. Transitions
  are checked (`status-transition`); an invalid transition is refused, so a
  result can never be 'admitted' without passing through candidacy + evidence."
  (:require [pnix-clj.cas :as cas]
            [pnix-clj.hash :as hash]
            [pnix-clj.purity :as purity]
            [pnix-clj.snapshot :as snapshot]
            [pnix-clj.store :as store]))

(def lane-classification
  {:lane :core
   :scope :witness-admission-lattice
   :role :bind-result-to-runtime-inputs-snapshot-evidence
   :product-runtime :allowed
   :semantic-authority :admission-lattice-only
   :mutation :append-only-evidence-events
   :admission :explicit-lattice-transition
   :determinism :required-before-admission
   :allowed-output :content-addressed-witness-or-admission-result})

;; ---- admission lattice ---------------------------------------------------

(def statuses #{:held :candidate :admitted :rejected :evidence :failed :ok})

(def ^:private transitions
  {:held      #{:candidate :failed}
   :candidate #{:admitted :rejected :held :evidence}
   :evidence  #{:admitted :rejected}
   :admitted  #{}
   :rejected  #{}
   :ok        #{}
   :failed    #{}})

(defn valid-transition?
  [from to]
  (boolean (and (statuses from) (statuses to)
                (contains? (get transitions from #{}) to))))

(defn status-transition
  "Move a witness to `to` iff the transition is valid; else refuse (the witness
  keeps its status and gains a :refused-transition marker)."
  [witness to]
  (if (valid-transition? (:status witness) to)
    (assoc witness :status to)
    (assoc witness :refused-transition {:from (:status witness) :to to})))

;; ---- witness schema ------------------------------------------------------

(def witness-fields
  [:witness/id :input-hash :term-hash :result-hash
   :runtime-version :evaluator-version :snapshot/id
   :stage :status :evidence-events])

(defn make-witness
  "Build a witness with the full schema; :witness/id is the content hash of all
  fields except itself (pure EDN)."
  [m]
  (let [content (select-keys m (remove #{:witness/id} witness-fields))]
    (assoc content :witness/id (hash/data-hash content))))

(defn witness-eval
  "Evaluate `source` and produce a fully-bound witness: content-address the
  source term (§3), pin the runtime (§8), witness determinism by re-run
  recorded as §5 events (§9), and bind the result. Status is :ok when the
  determinism check passes under a matching snapshot, else :failed/:held."
  [source & {:keys [store snapshot stage] :or {stage :direct-eval}}]
  (let [parse (requiring-resolve 'pnix-clj.parser/parse-source)
        snap (or snapshot (snapshot/make-snapshot))
        log (or store (store/open-store))
        parsed (parse source)
        term-hash (when (= :ok (:status parsed)) (cas/term-hash (:ast parsed)))
        pc (purity/purity-check! source {:runs 3 :store log :snapshot snap})
        base {:input-hash (hash/sha256 source)
              :term-hash term-hash
              :result-hash (:result-hash pc)
              :runtime-version (snapshot/symbol-version)
              :evaluator-version (snapshot/evaluator-version)
              :snapshot/id (:snapshot/id snap)
              :stage stage
              :evidence-events (mapv :event-hash (store/events log))}
        status (if (= :ok (:status pc)) :ok :failed)]
    (make-witness (assoc base :status status))))

(defn admit
  "Run a source through the full admission pipeline: candidate → evidence →
  admitted (only if the witness's determinism status is :ok)."
  [source & {:as opts}]
  (let [w (apply witness-eval source (mapcat identity opts))
        ok? (= :ok (:status w))
        w (make-witness (assoc w :status :candidate))]
    (-> w
        (status-transition :evidence)
        (status-transition (if ok? :admitted :rejected)))))

;; ---- report --------------------------------------------------------------

(defn report
  []
  (let [log (store/open-store)
        snap (snapshot/make-snapshot)
        w (witness-eval "let x = 40; in x + 2" :store log :snapshot snap)
        adm (admit "1 + 2 * 3")
        rows [{:id :witness-has-id :ok? (string? (:witness/id w))}
              {:id :witness-binds-term-hash :ok? (string? (:term-hash w))}
              {:id :witness-binds-snapshot :ok? (= (:snapshot/id snap) (:snapshot/id w))}
              {:id :witness-binds-versions
               :ok? (and (:evaluator-version w) (:runtime-version w))}
              {:id :witness-carries-evidence :ok? (seq (:evidence-events w))}
              {:id :witness-id-deterministic
               :ok? (= (:witness/id w) (:witness/id (make-witness w)))}
              {:id :deterministic-source-admitted :ok? (= :admitted (:status adm))}
              {:id :lattice-rejects-invalid-transition
               :ok? (:refused-transition (status-transition {:status :admitted} :held))}
              {:id :lattice-valid-path
               :ok? (and (valid-transition? :held :candidate)
                         (valid-transition? :candidate :admitted)
                         (not (valid-transition? :held :admitted)))}]
        rejected (count (remove :ok? rows))
        body {:kind :pnix-witness-report
              :schema :pnix-clj.witness-report.v0
              :policy :integrated-witness-binds-result-to-runtime-plus-admission-lattice
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
    (println (format "pnix-clj witness: status=%s total=%d accepted=%d rejected=%d"
                     (name status) total accepted rejected))
    (shutdown-agents)))
