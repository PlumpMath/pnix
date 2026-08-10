(ns pnix-clj.tower
  "Meta-circular tower entrypoint (roadmap M2).

  ONE call climbs every layer and collapses: source → read (parse) → emit
  round-trip (unparse, M1) → direct eval → lowering → clj-meta host
  compile/eval (bytecode receipt) → .px self-runtime → pnix mirror — then a
  COLLAPSE verdict states whether all layers agreed on one meaning.

  This is a REPACKAGING, not a re-computation: the layers and their
  adjacent-pair agreements come from the existing verify-source machinery
  (mirror/run-mirror singleton, cross-mirror-verdict, receipt/verdict). The
  evidence the tower ADDS: the read↔emit round-trip layer (parse(unparse(ast))
  structural equality via M1's unparser) and the specialize-residual layer
  (fold-only specialization whose residual must re-evaluate to the direct
  value — M1 battle-tested across the whole corpus every gate). Cases reuse
  the mirror-pair corpus — no second fixture set."
  (:require [pnix-clj.core :as pnix]
            [pnix-clj.evaluator :as evaluator]
            [pnix-clj.hash :as hash]
            [pnix-clj.mirror-pair :as mirror-pair]
            [pnix-clj.parser :as parser]
            [pnix-clj.specialize :as specialize]
            [pnix-clj.unparse :as unparse]))

(def lane-classification
  {:lane :proof-only
   :scope :meta-circular-tower-collapse-evidence
   :product-runtime :forbidden
   :semantic-authority :tower-evidence-only
   :mutation :forbidden
   :admission :forbidden
   :collapse-authority :evidence-report-only
   :allowed-output :tower-collapse-report})

(defn- emit-roundtrip
  "The tower's own layer: re-emit the AST as pnix source and re-read it; the
  structures must match up to span/source metadata."
  [ast]
  (try
    (let [emitted (unparse/unparse ast)
          reparsed (parser/parse-source emitted)]
      (if (not= :ok (:status reparsed))
        {:status :rejected :reason :emitted-source-does-not-parse
         :emitted emitted}
        (if (= (unparse/strip-positions ast)
               (unparse/strip-positions (:ast reparsed)))
          {:status :ok :emitted emitted}
          {:status :rejected :reason :emit-roundtrip-structural-mismatch
           :emitted emitted})))
    (catch Throwable _
      {:status :failed
       :reason :emit-unsupported
       :error {:phase :projection
               :class :emit-unsupported}})))

(defn- specialize-roundtrip
  "M1×M2 composition layer: a fold-only specialization (empty statics) whose
  residual, re-evaluated, must reproduce the direct evaluation — the
  specializer's meaning preservation is re-proven on every tower climb (and
  therefore battle-tested across the whole tower corpus every gate)."
  [source eval-result]
  (try
    (let [sp (specialize/specialize-cached source {})]
      (if (not= :ok (:status sp))
        {:status (if (= :suspended (:status sp)) :suspended :failed)
         :reason (or (:reason sp) :specialize-failed)}
        (let [r (pnix/eval-source (:residual-source sp))
              same? (and (= (:status eval-result) (:status r))
                         (= (:value eval-result) (:value r)))]
          {:status (if same? :ok :rejected)
           :fully-static? (:fully-static? sp)
           :gap-count (count (:gaps sp))})))
    (catch Throwable _
      {:status :failed
       :reason :specialize-failed
       :error {:phase :specialization
               :class :specialize-failed}})))

(defn- layer-row
  [layer status detail]
  (merge {:layer layer :status (or status :missing)} detail))

(defn run-tower
  "Climb the whole tower for one source and collapse. Returns
  {:kind :pnix-tower :layers [..] :pairs [..] :collapse {..}}.

  `input` is a source string, or a map {:source .. :import-modules ..}. When
  import-modules are present they are bound dynamically for the WHOLE climb so
  every layer that re-evaluates (verify-source, specialize's fold + residual
  re-eval) resolves imports against the same in-memory module map."
  [input]
  (let [source (if (map? input) (:source input) input)
        modules (when (map? input) (:import-modules input))]
    (binding [evaluator/*import-modules* (or modules
                                             evaluator/*import-modules*)]
  (let [row (pnix/verify-source (if modules
                               {:source source :import-modules modules}
                               source))
        ast (:ast row)
        emit (when ast (emit-roundtrip ast))
        sp-layer (if ast
                   (specialize-roundtrip source (:eval-result row))
                   {:status :missing})
        cross (:cross-mirror-verdict row)
        layers
        [(layer-row :read (if ast :ok (:status row))
                    {:ast-hash (:ast-hash row)})
         (layer-row :emit-roundtrip (:status emit)
                    (select-keys emit [:reason]))
         (layer-row :direct-eval (:status (:eval-result row))
                    {:value (:value (:eval-result row))
                     :reason (:reason (:eval-result row))})
         (layer-row :specialize-residual (:status sp-layer)
                    (select-keys sp-layer [:reason :fully-static? :gap-count]))
         (layer-row :lowering (:status (:lowering-result row))
                    {:form-hash (:lowered-form-hash row)})
         (layer-row :clj-meta-host (:status (:clj-meta-result row))
                    {:bytecode-determinism
                     (get-in row [:clj-meta-result :compile-receipt
                                  :determinism :status])})
         (layer-row :px-runtime (:status (:px-runtime row))
                    {:reason (:reason (:px-runtime row))})
         (layer-row :pnix-mirror (:status (:pnix-mirror row))
                    {:reason (:reason (:pnix-mirror row))})]
        pairs
        [{:pair [:read :emit-roundtrip]
          :ok? (= :ok (:status emit))
          :evidence :parse-unparse-structural-equality}
         {:pair [:direct-eval :specialize-residual]
          :ok? (= :ok (:status sp-layer))
          :evidence :fold-residual-reproduces-evaluation}
         {:pair [:direct-eval :clj-meta-host]
          :ok? (and (= :ok (:status (:eval-result row)))
                    (= :ok (:status (:clj-meta-result row)))
                    (= (:value (:eval-result row))
                       (:value (:clj-meta-result row))))
          :evidence :cross-mirror-host-agreement}
         {:pair [:direct-eval :px-runtime]
          :ok? (and (= :ok (:status (:px-runtime row)))
                    (= (:value (:eval-result row))
                       (:value (:px-runtime row))))
          :evidence :cross-mirror-px-agreement}
         {:pair [:px-runtime :pnix-mirror]
          :ok? (and (= :ok (:status (:pnix-mirror row)))
                    (= (:value (:px-runtime row))
                       (:value (:pnix-mirror row))))
          :evidence :pnix-mirror-receipt}]
        all-ok? (and (= :accepted (:status row))
                     (= :ok (:status emit))
                     (every? :ok? pairs))
        collapse (if all-ok?
                   {:status :collapsed
                    :value (:value (:eval-result row))
                    :agreeing-layers (mapv :layer layers)
                    :witness {:source-hash (:source-hash row)
                              :ast-hash (:ast-hash row)
                              :cross-mirror (:status cross)}}
                   {:status (cond
                              (= :rejected (:status row)) :rejected
                              (some #(= :suspended (:status %)) layers) :suspended
                              :else :failed)
                    :blocking (or (some (fn [{:keys [layer status reason]}]
                                          (when (not= :ok status)
                                            {:layer layer :reason reason}))
                                        layers)
                                  (some (fn [{:keys [pair ok?]}]
                                          (when-not ok? {:pair pair}))
                                        pairs))
                    :verify-source-status (:status row)
                    :verify-source-reason (:reason row)})]
    {:kind :pnix-tower
     :source source
     :source-hash (:source-hash row)
     :layers layers
     :pairs pairs
     :collapse collapse}))))

(defn report
  "Tower report over the mirror-pair corpus (sources every lane already
  supports) plus one deliberately-held probe showing the collapse verdict
  degrades honestly instead of pretending."
  []
  (let [cases (mirror-pair/cases)
        rows (mapv (fn [c]
                     ;; carry per-case import-modules into the climb so import
                     ;; sources collapse instead of holding module-less.
                     (let [t (run-tower (if (:import-modules c)
                                          (select-keys c [:source :import-modules])
                                          (:source c)))]
                       {:source (:source c)
                        :status (if (= :collapsed (get-in t [:collapse :status]))
                                  :accepted
                                  :rejected)
                        :collapse (:collapse t)
                        :pairs (:pairs t)}))
                   cases)
        ;; the probe walks the frontier as lifts land: appendContext ->
        ;; derivation -> functionArgs -> @as -> builtin values -> application
        ;; laziness -> select-on-tryEval -> import-with-modules -> scopedImport
        ;; scope injection (now collapses 4-lane) -> now scopedImport with an
        ;; unused ERRORING scope key: the direct + clj lanes keep the scope
        ;; lazy, but px deep-forces it across the marshalling boundary, so the
        ;; erroring key holds the px lane -- the honest cost of forcing there.
        ;; the erroring-unused-scope probe COLLAPSES since the px lazy-scope
        ;; bridge landed (px-scope-laziness); the module-free import is the
        ;; remaining honestly-held frontier.
        failure-probe (run-tower "import ./mod.px")
        failure-classified? (= :failed
                               (get-in failure-probe [:collapse :status]))
        rejected (count (remove #(= :accepted (:status %)) rows))
        body {:kind :pnix-tower-report
              :schema :pnix-clj.tower-report.v0
              :policy :single-entrypoint-collapse-over-existing-lanes
              :total (count rows)
              :accepted (- (count rows) rejected)
              :rejected rejected
              :failure-probe {:source (:source failure-probe)
                              :collapse-status
                              (get-in failure-probe [:collapse :status])
                              :classified? failure-classified?}
              :rows rows}]
    (assoc body
           :status (if (and (zero? rejected) failure-classified?) :ok :failed)
           :report-hash (hash/data-hash rows))))

(defn -main
  [& _]
  (let [{:keys [status total accepted rejected failure-probe rows]} (report)]
    (println (format "pnix-clj tower: status=%s total=%d accepted=%d rejected=%d failure-probe=%s"
                     (name status) total accepted rejected
                     (name (:collapse-status failure-probe))))
    (doseq [{:keys [source status collapse]} rows]
      (println (format "  [%s] %s -> %s %s"
                       (if (= :accepted status) "OK" "REJECT")
                       (pr-str source)
                       (name (:status collapse))
                       (pr-str (:value collapse)))))
    (shutdown-agents)))
