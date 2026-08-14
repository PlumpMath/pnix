(ns pnix-clj.witnessed-run
  "Spine INTEGRATION -- one witnessed run that ties the evidence-store spine to
  the capability pillars (see docs/SPINE_ROADMAP.md §15 'integrates all').

  `run-witnessed` evaluates one pnix source and, sharing a single §5 event log
  and §8 snapshot, records the whole pipeline as durable evidence:
    §3 term-hash  ·  §8 snapshot pin  ·  cross-substrate tower collapse (§6) as
    a :tower/collapse event  ·  repeated-run mirror chain (§6.6-6.7) as a
    :mirror/run event  ·  determinism witnessed by re-run (§9) as a :purity/run
    event  ·  the Futamura residual bytecode CONTENT-ADDRESSED by
    (term-hash ⊕ snapshot-id ⊕ bytecode-hash).
  It then emits a §15 witness bound to every evidence-event hash and runs it
  through the admission lattice: :admitted iff the tower collapsed AND the
  chain converged AND determinism held under a matching snapshot; else
  :rejected. This is what 'keys what' in the research: everything keys off the
  §3 term hash, pinned by the §8 snapshot, recorded as §5 events, admitted by
  the §15 witness."
  (:require [pnix-clj.cas :as cas]
            [pnix-clj.hash :as hash]
            [pnix-clj.mirror-chain :as mirror-chain]
            [pnix-clj.persist :as persist]
            [pnix-clj.purity :as purity]
            [pnix-clj.snapshot :as snapshot]
            [pnix-clj.store :as store]
            [pnix-clj.witness :as witness]))

(def lane-classification
  {:lane :core
   :scope :integrated-witnessed-run-spine
   :role :tie-term-snapshot-tower-mirror-purity-and-witness
   :product-runtime :allowed
   :semantic-authority :admission-evidence-pipeline
   :mutation :append-only-evidence-and-explicit-persistence
   :admission :witness-lattice
   :determinism :required-before-admission
   :allowed-output :witnessed-run-result-or-report})

(defn residual-key
  "Content-address the Futamura residual: hash of (term-hash ⊕ snapshot-id ⊕
  bytecode-hash). Same source term + same runtime pin ⇒ same residual key."
  [term-hash snapshot-id bytecode-hash]
  (hash/data-hash [:pnix-residual term-hash snapshot-id bytecode-hash]))

(defn run-witnessed
  "Evaluate `source`, recording the full pipeline as spine evidence and emitting
  an admitted/rejected §15 witness. Returns
  {:witness .. :status .. :events [..] :residual-key .. :collapse .. :chain ..}."
  [source & {:keys [runs store snapshot] :or {runs 3}}]
  (let [verify-source (requiring-resolve 'pnix-clj.core/verify-source)
        parse (requiring-resolve 'pnix-clj.parser/parse-source)
        log (or store (store/open-store))
        snap (or snapshot (snapshot/make-snapshot))
        parsed (parse source)
        term-hash (when (= :ok (:status parsed)) (cas/term-hash (:ast parsed)))
        ;; §6 cross-substrate collapse — must use verify-source (multi-lane
        ;; receipts + cross-mirror-verdict). run-source is the basic semantic
        ;; path only and intentionally omits mirror verdicts.
        row (verify-source source)
        cross (:cross-mirror-verdict row)
        collapsed? (= :agree (:equivalence cross))
        rkey (residual-key term-hash (:snapshot/id snap) (:bytecode-hash row))
        _ (store/append! log :tower/collapse
                         {:source-hash (:source-hash row)
                          :term-hash term-hash
                          :equivalence (:equivalence cross)
                          :lane-summary (:lane-summary row)
                          :residual-key rkey})
        ;; §6.6-6.7 temporal chain convergence (shares the log)
        chain (mirror-chain/mirror-chain! source {:runs runs :store log})
        ;; §9 determinism witnessed by re-run (shares log + snapshot)
        pc (purity/purity-check! source {:runs runs :store log :snapshot snap})
        ;; §15 witness bound to every evidence event
        ok? (and collapsed? (:chain-converged? chain) (= :ok (:status pc)))
        w (witness/make-witness
           {:input-hash (hash/sha256 source)
            :term-hash term-hash
            :result-hash (:result-hash pc)
            :runtime-version (snapshot/symbol-version)
            :evaluator-version (snapshot/evaluator-version)
            :snapshot/id (:snapshot/id snap)
            :stage :witnessed-run
            :status :candidate
            :evidence-events (mapv :event-hash (store/events log))})
        admitted (-> w
                     (witness/status-transition :evidence)
                     (witness/status-transition (if ok? :admitted :rejected)))]
    {:witness admitted
     :status (:status admitted)
     :collapse (:equivalence cross)
     :chain-converged? (:chain-converged? chain)
     :determinism (:status pc)
     :residual-key rkey
     :term-ast (:ast parsed)
     :log log
     :events (mapv (juxt :seq :kind) (store/events log))
     :log-intact? (= :intact (:status (store/verify-chain log)))}))

(defn run-witnessed-durable
  "Run `source` witnessed AND persist its evidence to `dir` (content-addressed
  term + append-only events) -- the full loop leaves a durable, replayable audit
  trail. Returns the witnessed-run result plus {:persisted {:dir :term-hash
  :events-written}}."
  [source dir & opts]
  (let [result (apply run-witnessed source opts)
        pstore (persist/open-persistent-store dir)
        term-hash (when (:term-ast result)
                    (persist/persist-term! pstore (:term-ast result)))
        written (persist/persist-events! pstore (:log result))
        input-hash (persist/persist-source! pstore source)
        witness-id (persist/persist-witness! pstore (:witness result))]
    (-> result
        (dissoc :term-ast :log)
        (assoc :persisted {:dir (:dir pstore)
                           :term-hash term-hash
                           :input-hash input-hash
                           :witness-id witness-id
                           :events-written written}))))

;; ---- report --------------------------------------------------------------

(defn report
  []
  (let [good (run-witnessed "let x = 40; in x + 2")
        pat  (run-witnessed "({ a }@args: a + args.a) { a = 21; }")
        rk1  (:residual-key (run-witnessed "1 + 2 * 3"))
        rk2  (:residual-key (run-witnessed "1 + 2 * 3"))
        dir  (str (System/getProperty "java.io.tmpdir") "/pnix-wr-" (System/nanoTime))
        dur  (run-witnessed-durable "let x = 40; in x + 2" dir)
        reloaded (:verify (persist/load-events
                           (persist/open-persistent-store dir)))
        _ (doseq [f (reverse (file-seq (clojure.java.io/file dir)))] (.delete f))
        rows [{:id :arith-admitted        :ok? (= :admitted (:status good))}
              {:id :pattern-admitted      :ok? (= :admitted (:status pat))}
              {:id :collapse-recorded     :ok? (= :agree (:collapse good))}
              {:id :chain-converged       :ok? (:chain-converged? good)}
              {:id :determinism-ok        :ok? (= :ok (:determinism good))}
              {:id :witness-binds-term    :ok? (string? (get-in good [:witness :term-hash]))}
              {:id :witness-carries-events :ok? (<= 3 (count (get-in good [:witness :evidence-events])))}
              {:id :residual-content-addressed :ok? (= rk1 rk2)}
              {:id :evidence-log-intact   :ok? (:log-intact? good)}
              {:id :events-cover-spine
               ;; tower/collapse + mirror/run + purity/run all present
               :ok? (= #{:tower/collapse :mirror/run :purity/run}
                       (set (map second (:events good))))}
              {:id :durable-run-persists-evidence
               :ok? (and (= :admitted (:status dur))
                         (string? (get-in dur [:persisted :term-hash]))
                         (pos? (get-in dur [:persisted :events-written])))}
              {:id :durable-events-reload-intact
               :ok? (= :intact (:status reloaded))}]
        rejected (count (remove :ok? rows))
        body {:kind :pnix-witnessed-run-report
              :schema :pnix-clj.witnessed-run-report.v0
              :policy :one-run-ties-spine-to-pillars-term-keyed-snapshot-pinned-witness-admitted
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
    (println (format "pnix-clj witnessed-run: status=%s total=%d accepted=%d rejected=%d"
                     (name status) total accepted rejected))
    (shutdown-agents)))
