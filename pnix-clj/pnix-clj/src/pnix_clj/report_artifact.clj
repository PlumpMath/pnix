(ns pnix-clj.report-artifact
  (:require [clojure.java.io :as io]
            [clojure.pprint :refer [pprint]]
            [pnix-clj.classfile-receipt :as classfile-receipt]
            [pnix-clj.clojure-form :as clojure-form]
            [pnix-clj.clojure-projection :as clojure-projection]
            [pnix-clj.core :as pnix]
            [pnix-clj.coverage :as coverage]
            [pnix-clj.determinism :as determinism]
            [pnix-clj.emit-form-roundtrip :as emit-form-roundtrip]
            [pnix-clj.forward-reference :as forward-reference]
            [pnix-clj.grammar-fuzzer :as grammar-fuzzer]
            [pnix-clj.hash :as hash]
            [pnix-clj.live-oracle :as live-oracle]
            [pnix-clj.mirror-chain :as mirror-chain]
            [pnix-clj.mirror-error :as mirror-error]
            [pnix-clj.cached-eval :as cached-eval]
            [pnix-clj.mirror-pair :as mirror-pair]
            [pnix-clj.safe-eval :as safe-eval]
            [pnix-clj.arith-proof :as arith-proof]
            [pnix-clj.cas :as cas]
            [pnix-clj.persist :as persist]
            [pnix-clj.purity :as purity]
            [pnix-clj.replay :as replay]
            [pnix-clj.search :as search]
            [pnix-clj.reflect :as reflect]
            [pnix-clj.snapshot :as snapshot]
            [pnix-clj.store :as store]
            [pnix-clj.bool-proof :as bool-proof]
            [pnix-clj.form-analysis :as form-analysis]
            [pnix-clj.property-fuzzer :as property-fuzzer]
            [pnix-clj.futamura :as futamura]
            [pnix-clj.machine :as machine]
            [pnix-clj.weval :as weval]
            [pnix-clj.specialize :as specialize]
            [pnix-clj.synthesize :as synthesize]
            [pnix-clj.tower :as tower]
            [pnix-clj.oracle :as oracle]
            [pnix-clj.rust-batch :as rust-batch]
            [pnix-clj.stage7-core :as stage7-core]
            [pnix-clj.stage15 :as stage15]
            [pnix-clj.strict-audit :as strict-audit]
            [pnix-clj.translation-validation :as translation-validation]
            [pnix-clj.trust :as trust]
            [pnix-clj.witness :as witness]
            [pnix-clj.cegis :as cegis]
            [pnix-clj.generate :as generate]
            [pnix-clj.self-improve :as self-improve]
            [pnix-clj.self-mod-gate :as self-mod-gate]
            [pnix-clj.witnessed-run :as witnessed-run]
            [pnix-clj.value-roundtrip :as value-roundtrip]))

(def lane-classification
  {:lane :core
   :scope :report-artifact-registry
   :role :dispatch-and-materialize-repo-owned-evidence-reports
   :product-runtime :allowed
   :semantic-authority :report-packaging-only
   :mutation :explicit-report-file-write-only
   :admission :forbidden
   :determinism :report-hash-required
   :allowed-output :report-artifact-or-unknown-kind-verdict})

(def default-report-dir
  "target/pnix-clj/reports")

(def supported-kinds
  "Registry of report-artifact kinds — the machine source for the capability
  index (M4) and the unknown-kind error."
  [:smoke :mirror-pair :mirror-error
                                       :forward-reference
                                       :clojure-projection
                                       :clojure-form
                                       :rust-batch
                                       :stage7-core
                                       :strict-audit
                                       :specialize
                                       :futamura
                                       :weval
                                       :form-analysis
                                       :arith-proof
                                       :bool-proof
                                       :cas
                                       :store
                                       :reflect
                                       :snapshot
                                       :purity
                                       :search
                                       :mirror-chain
                                       :witness
                                       :witnessed-run
                                       :self-mod-gate
                                       :self-improve
                                       :generate
                                       :cegis
                                       :persist
                                       :replay
                                       :property-fuzzer
                                       :tower
                                       :safe-eval
                                       :synthesize
                                       :cached-eval
                                       :determinism
                                       :coverage
                                       :grammar-fuzzer
                                       :live-oracle
                                       :classfile-receipt
                                       :trust
                                       :stage15-exec
                                       :translation-validation
                                       :emit-form-roundtrip
                                       :value-roundtrip
                                       :machine])

(defn- report-for*
  [kind]
  (case (keyword kind)
    :smoke
    (assoc (pnix/report (oracle/ground-truth-cases))
           :kind :pnix-clj-smoke-report)

    :mirror-pair
    (mirror-pair/report)

    :mirror-error
    (mirror-error/report)

    :forward-reference
    (forward-reference/report)

    :clojure-projection
    (clojure-projection/report)

    :clojure-form
    (clojure-form/report)

    :rust-batch
    (rust-batch/report)

    :stage7-core
    (stage7-core/report)

    :strict-audit
    (strict-audit/report)

    :specialize
    (specialize/report)

    :futamura
    (futamura/report)

    :weval
    (weval/report)

    :machine
    (machine/report)

    :form-analysis
    (form-analysis/report)

    :arith-proof
    (arith-proof/report)

    :bool-proof
    (bool-proof/report)

    :cas
    (cas/report)

    :store
    (store/report)

    :reflect
    (reflect/report)

    :snapshot
    (snapshot/report)

    :purity
    (purity/report)

    :search
    (search/report)

    :mirror-chain
    (mirror-chain/report)

    :witness
    (witness/report)

    :witnessed-run
    (witnessed-run/report)

    :self-mod-gate
    (self-mod-gate/report)

    :self-improve
    (self-improve/report)

    :generate
    (generate/report)

    :cegis
    (cegis/report)

    :persist
    (persist/report)

    :replay
    (replay/report)

    :property-fuzzer
    (property-fuzzer/report {:num-tests 60 :seed 42})

    :tower
    (tower/report)

    :safe-eval
    (safe-eval/report)

    :synthesize
    (synthesize/report)

    :cached-eval
    (cached-eval/report)

    :determinism
    (determinism/report)

    :coverage
    (coverage/report)

    :grammar-fuzzer
    (grammar-fuzzer/report)

    :live-oracle
    (live-oracle/report)

    :classfile-receipt
    (classfile-receipt/report)

    :trust
    (trust/report)

    :stage15-exec
    (stage15/execute-plan)

    :translation-validation
    (translation-validation/report)

    :emit-form-roundtrip
    (emit-form-roundtrip/report)

    :value-roundtrip
    (value-roundtrip/report)

    (throw (ex-info "unknown pnix-clj report kind"
                    {:kind kind
                     :supported-kinds supported-kinds}))))

(defn- report-file
  [out-dir kind]
  (io/file out-dir (str (name (keyword kind)) ".edn")))

(def ^:private report-cache
  "Constructive-trace cache (Build-Systems-a-la-Carte §4.2.3) for report
  artifacts WITHIN one JVM run: {kind {:snapshot-id .. :report ..}}. Sound
  because reports are deterministic (§9 witnesses it; every report carries its
  :report-hash) and the trace key's other components -- renderer code and
  corpus resources -- are constant within a run; the §8 runtime-snapshot pin
  guards the runtime component and invalidates on mismatch. Fresh per JVM
  (CLI aliases always render fresh); drift gates never route through here.
  docs/REMAINING_DECISION.md item C."
  (atom {}))

(defn report-for
  "Render (or reuse) the report for `kind`. A cache hit requires the current
  §8 snapshot id to match the one the report was rendered under -- otherwise
  re-render (fail-safe toward freshness)."
  [kind]
  (let [k (keyword kind)
        snap-id (:snapshot/id (snapshot/make-snapshot))
        hit (get @report-cache k)]
    (if (and hit (= snap-id (:snapshot-id hit)))
      (:report hit)
      (let [r (report-for* k)]
        (swap! report-cache assoc k {:snapshot-id snap-id :report r})
        r))))

(defn write-report!
  ([kind]
   (write-report! kind default-report-dir))
  ([kind out-dir]
   (let [kind (keyword kind)
         report (assoc (report-for kind)
                       :report-artifact/version 1
                       :report-artifact/kind kind)
         f (report-file out-dir kind)]
     (.mkdirs (.getParentFile f))
     (binding [*print-length* nil
               *print-level* nil]
       (spit f (with-out-str (pprint report))))
     {:kind kind
      :path (.getCanonicalPath f)
      :hash (hash/sha256 (slurp f))
      :bytes (.length f)
      :report report})))

(defn- print-report-summary!
  "Print one written report's summary lines (path/hash/bytes + the per-kind
  counts). Returns true when the report carries a failure signal."
  [{:keys [path hash bytes report]}]
  (println "pnix-clj report artifact")
  (println "path:" path)
  (println "hash:" hash)
  (println "bytes:" bytes)
  (let [_ (cond
      (= :pnix-tower-report (:kind report))
      (println (format "counts: sources=%d collapsed=%d rejected=%d failure-probe=%s"
                       (:total report)
                       (:accepted report)
                       (:rejected report)
                       (name (get-in report [:failure-probe :collapse-status]))))

      (= :pnix-specialize-report (:kind report))
      (println (format "counts: differential=%d futamura=%d accepted=%d rejected=%d"
                       (:differential-total report)
                       (:futamura-total report)
                       (:accepted report)
                       (:rejected report)))

      (= :machine-report (:kind report))
      (println (format "counts: rows=%d divergent=%d constant-stack-witness=%s"
                       (:row-count report)
                       (count (:divergent report))
                       (if (get-in report [:constant-stack-witness :ok?])
                         "ok" "FAILED")))

      (= :strict-audit-report (:kind report))
      (println (format "counts: sources=%d strict-ok=%d violations=%d held=%d events=%d"
                       (:source-count report)
                       (:strict-ok report)
                       (:strict-violation report)
                       (:held report)
                       (:violation-count report)))

      (= :pnix-evaluation-determinism-report (:kind report))
      (println (format "counts: sources=%d stable=%d unstable=%d runs=%d"
                       (:source-count report)
                       (:stable report)
                       (:unstable report)
                       (:runs-per-source report)))

      (= :pnix-evaluation-coverage-report (:kind report))
      (println (format "counts: sources=%d ops=%d/%d builtins=%d/%d branches=%d/%d"
                       (:source-count report)
                       (get-in report [:summary :op :covered])
                       (get-in report [:summary :op :total])
                       (get-in report [:summary :builtin :covered])
                       (get-in report [:summary :builtin :total])
                       (get-in report [:summary :branch :covered])
                       (get-in report [:summary :branch :total])))

      (= :pnix-grammar-fuzzer-report (:kind report))
      (println (format "counts: sources=%d ok=%d failed=%d seed=%d"
                       (:source-count report)
                       (:ok report)
                       (:failed report)
                       (:seed report)))

      (= :pnix-live-oracle-report (:kind report))
      (println (format "counts: status=%s sources=%d matched=%d mismatched=%d pnix-held=%d oracle-held=%d"
                       (name (:status report))
                       (:source-count report)
                       (:matched report)
                       (:mismatched report)
                       (:pnix-held report)
                       (:oracle-held report)))

      (= :pnix-deterministic-classfile-report (:kind report))
      (println (format "counts: status=%s rows=%d hash=%s"
                       (name (:status report))
                       (:row-count report)
                       (:receipt-hash report)))

      (= :pnix-common-mode-risk-report (:kind report))
      (println (format "counts: status=%s mitigations=%d hash=%s"
                       (name (:status report))
                       (count (:mitigations report))
                       (:report-hash report)))

      (= :stage15-execution-report (:kind report))
      (println (format "counts: status=%s commands=%d held=%d hash=%s"
                       (name (:status report))
                       (:selected-command-count report)
                       (:held-count report)
                       (:receipt-hash report)))

      (= :pnix-translation-validation-report (:kind report))
      (println (format "counts: status=%s validators=%d hash=%s"
                       (name (:status report))
                       (:validator-count report)
                       (:receipt-hash report)))

      (= :pnix-emit-form-roundtrip-report (:kind report))
      (println (format "counts: status=%s cases=%d ok=%d held=%d hash=%s"
                       (name (:status report))
                       (:case-count report)
                       (:ok report)
                       (:held-or-rejected report)
                       (:receipt-hash report)))

      (= :pnix-value-roundtrip-report (:kind report))
      (println (format "counts: status=%s cases=%d ok=%d held=%d hash=%s"
                       (name (:status report))
                       (:case-count report)
                       (:ok report)
                       (:held-or-rejected report)
                       (:receipt-hash report)))

      (= :forward-reference-lift-report (:kind report))
      (println (format "counts: fixtures=%d accepted=%d held=%d forward-ok=%d semantic-error=%d hash=%s"
                       (:fixture-count report)
                       (:accepted report)
                       (:held report)
                       (:forward-ok-count report)
                       (:semantic-error-count report)
                       (:receipt-hash report)))

      :else
      (println (format "counts: total=%d accepted=%d rejected=%d held=%d"
                       (:total report) (:accepted report) (:rejected report) (:held report))))]
    (when-let [frontier (:first-frontier report)]
      (println "first frontier:" (pr-str frontier)))
    (when-let [unstable (:first-unstable report)]
      (println "first unstable:" (pr-str unstable)))
    (pos? (+ (or (:rejected report) 0)
             (or (:unstable report) 0)
             (or (:failed report) 0)
             (or (:mismatched report) 0)
             (if (= :held (:status report)) 1 0)))))

(defn -main
  "Single kind: `clojure -M:report-<kind>` (optional out-dir second arg).
  BATCH mode (gate-speed): `... report-artifact batch k1 k2 ...` renders every
  kind in ONE JVM — thirteen fewer cold starts per gate, and the
  constructive-trace report cache (item C) finally shares renders ACROSS
  kinds run on a warm JIT (the report cache stays per-kind). Per-kind failure semantics are
  preserved: any failing kind exits 1 after all summaries print."
  [& [kind & more]]
  (if (= "batch" (some-> kind name))
    (let [kinds (or (seq more)
                    (throw (ex-info "batch mode needs explicit kinds" {})))
          failures (into []
                         (comp (map #(write-report! % default-report-dir))
                               (filter print-report-summary!)
                               (map (comp :kind :report)))
                         kinds)]
      (println (format "pnix-clj report batch: %d kinds, %d failed %s"
                       (count kinds) (count failures) (pr-str failures)))
      (shutdown-agents)
      (when (seq failures)
        (System/exit 1)))
    (let [failed? (print-report-summary!
                   (write-report! (or kind :smoke)
                                  (or (first more) default-report-dir)))]
      (shutdown-agents)
      (when failed?
        (System/exit 1)))))
