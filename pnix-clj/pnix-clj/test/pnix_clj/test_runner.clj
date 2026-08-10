(ns pnix-clj.test-runner
  "Gate runner with SELECTIVE gating (verifying-trace discipline applied to the
  gate itself: re-verify what changed, not everything, while the FULL gate
  stays the pre-push authority).

    clojure -M:test --foundation    required basic product gate
    clojure -M:test                 opt-in full research/history suite
    clojure -M:test cegis cas       only deftest vars whose name matches a
                                    pattern (substring, any test namespace)
    clojure -M:test --changed       derive patterns from `git status` -- run the
                                    changed modules' tests + the drift guards;
                                    FALLS BACK to the full gate when a CORE
                                    module changed (parser/evaluator/... touch
                                    everything, so selection would be unsound)
    add --timing                    print per-test wall times (find the hogs)

  Honest boundary: a selective run is a DEV-LOOP gate; the foundation gate is
  the commit/push authority. The full research suite is informative and cannot
  enlarge or block the basic product closure."
  (:require [clojure.java.shell :as shell]
            [clojure.string :as str]
            [clojure.test :as test]
            [pnix-clj.bootstrap-test]
            [pnix-clj.scope-lock-test]
            [pnix-clj.lane-policy-test]
            [pnix-clj.lane-identity-test]
            [pnix-clj.scope-doc-identity-test]
            [pnix-clj.surgery-closure-test]))

(def test-namespaces
  ['pnix-clj.bootstrap-test
   'pnix-clj.scope-lock-test
   'pnix-clj.lane-policy-test
   'pnix-clj.lane-identity-test
   'pnix-clj.scope-doc-identity-test
   'pnix-clj.surgery-closure-test])

(def non-foundation-test-names
  "Explicit opt-in research/content tests. Every test not listed here belongs
  to the required basic foundation by default, so a newly added test cannot
  silently escape CI. These tests remain runnable through the full local gate."
  '#{weval-ir-pe-dispatch-elimination
     run-source-closes-mirror-spine-for-small-source
     deterministic-classfile-report-pins-asm-and-generated-classes
     common-mode-risk-report-records-independent-oracle-mitigation
     translation-validation-report-frames-receipts-as-validators
     emit-form-roundtrip-report-checks-analyzer-emitted-values
     value-roundtrip-report-synthesizes-stable-clojure-forms
     benchmark-reports-stable-receipt-baseline
     report-summarizes-accepted-first-slice
     mirror-pair-report-tracks-basic-runtime-fixtures
     mirror-error-report-aligns-expected-error-boundaries
     clojure-projection-report-validates-reader-term-shape-in-px
     clojure-form-report-compares-host-and-clj-meta-semantics
     evaluator-domain-extension-stubs-are-not-nix-coverage
     specialize-refuses-position-observing-sources
     cached-eval-content-addressed
     synthesize-reverse-projection
     safe-eval-sandbox-tier
     tower-single-entrypoint-collapse
     property-fuzzer-cross-lane-collapse
     bool-proof-truth-table-equivalence
     replay-reverifies-persisted-witness
     persist-durable-content-addressed-store
     cegis-counterexample-guided-refinement
     generate-observational-enumerative-synthesis
     self-improve-loop-body-holds-for-owner
     self-mod-gate-no-auto-promotion
     witnessed-run-spine-integration
     witness-schema-and-admission-lattice
     mirror-chain-repeated-run-convergence
     search-content-event-similarity
     purity-determinism-as-events
     snapshot-runtime-pin-fail-closed
     reflect-host-snapshots
     store-append-only-event-log
     cas-content-addressed-term-store
     arith-proof-proven-equivalence
     form-analysis-ast-pass-lane
     synthesize-form-analysis-convergence
     futamura-second-projection
     specialize-futamura-first-slice
     strict-audit-report-classifies-current-corpus
     strict-gate-runs-current-strict-ok-fixtures
     pnix-evaluation-determinism-report-hashes-current-corpus
     pnix-evaluation-coverage-report-measures-current-corpus
     grammar-fuzzer-differential-gate-runs-generated-programs
     optional-live-oracle-is-gated-and-compares-when-available
     forward-reference-report-records-r1-lift
     report-separates-semantic-mismatch-from-held-frontier
     runtime-run-plan-is-human-trackable
     runtime-import-graph-analysis-detects-cycles
     runtime-import-scanner-includes-scopedImport
     stage15-control-plan-is-human-trackable
     stage15-execution-report-runs-selected-commands
     rust-grounded-batch-is-repo-owned-and-held
     report-artifact-is-persisted-as-edn
     stage7-core-lockins-cross-internal-px-runtime-boundary})

(def core-modules
  "Modules whose change invalidates selection -- they underlie (nearly) every
  test, so the only sound gate for them is the full one."
  #{"parser" "evaluator" "lowering" "core" "tower" "mirror" "px_runtime"
    "clj_meta" "interop" "receipt" "hash" "error" "unparse" "stage15"
    "stage7_core" "specialize"})

(def always-patterns
  "Drift guards that must run in every selective gate (docs regenerate per
  slice; catching drift early is the point of the gate)."
  ["capabilities-index" "wiki-doc"])

(defn- all-test-vars
  []
  (for [ns-sym test-namespaces
        v (vals (ns-interns ns-sym))
        :when (:test (meta v))]
    v))

(defn- foundation-test-vars
  []
  (remove (fn [v]
            (contains? non-foundation-test-names (:name (meta v))))
          (all-test-vars)))

(defn- test-var-id
  [v]
  (let [{:keys [ns name]} (meta v)]
    (str (ns-name ns) "/" name)))

(defn- parse-shard-spec
  [spec]
  (when-let [[_ index total]
             (re-matches #"([0-9]+)/([1-9][0-9]*)" (or spec ""))]
    (let [index (Long/parseLong index)
          total (Long/parseLong total)]
      (when (< index total)
        [index total]))))

(defn- shard-vars
  [index total foundation?]
  (->> ((if foundation? foundation-test-vars all-test-vars))
       (sort-by test-var-id)
       (map-indexed vector)
       (keep (fn [[position v]]
               (when (= index (mod position total))
                 v)))
       vec))

(defn select-vars
  "Test vars whose (unqualified) name contains any of `patterns`."
  [patterns]
  (let [pats (map str/lower-case patterns)]
    (filterv (fn [v]
               (let [n (str/lower-case (name (symbol v)))]
                 (boolean (some #(str/includes? n %) pats))))
             (all-test-vars))))

(defn changed-modules
  "Basenames (sans .clj, underscores kept) of changed source files vs HEAD --
  staged + unstaged + untracked, from git status --porcelain."
  []
  (let [{:keys [out]} (shell/sh "git" "status" "--porcelain")]
    (->> (str/split-lines (or out ""))
         (map #(str/trim (subs % (min 3 (count %)))))
         (filter #(re-find #"src/pnix_clj/.+\.clj$" %))
         (map #(-> % (str/replace #".*/" "") (str/replace #"\.clj$" "")))
         set)))

(defn changed->patterns
  "Map changed module basenames to test-name patterns, or :full when a core
  module changed. Test names use dashes; files use underscores."
  [modules]
  (if (some core-modules modules)
    :full
    (->> modules
         (map #(str/replace % "_" "-"))
         (concat always-patterns)
         vec)))

(defn- run-vars
  "Run the given test vars (with each-fixtures) and return the merged report
  counters. Optionally prints per-test timing."
  [vars timing?]
  (binding [test/*report-counters* (ref test/*initial-report-counters*)]
    (doseq [v vars]
      (let [t0 (System/nanoTime)]
        (test/test-vars [v])
        (when timing?
          (println (format "  %6.1fs  %s" (/ (- (System/nanoTime) t0) 1e9)
                           (name (symbol v)))))))
    @test/*report-counters*))

(defn -main
  [& args]
  (let [timing? (boolean (some #{"--timing"} args))
        args (vec (remove #{"--timing"} args))]
    (if (contains? #{"--shard" "--foundation-shard"} (first args))
      (if-let [[index total]
               (and (= 2 (count args))
                    (parse-shard-spec (second args)))]
        (let [foundation? (= "--foundation-shard" (first args))
              vars (shard-vars index total foundation?)
              _ (println (format "%s deterministic shard %d/%d: %d test(s)"
                                 (if foundation? "foundation" "full")
                                 index total (count vars)))
              t0 (System/nanoTime)
              {:keys [test pass fail error]} (run-vars vars timing?)]
          (println (format "Ran %d tests containing %d assertions."
                           test (+ pass fail error)))
          (println (format "%d failures, %d errors." fail error))
          (println (format "shard %d/%d gate in %.1fs"
                           index total (/ (- (System/nanoTime) t0) 1e9)))
          (shutdown-agents)
          (when (or (zero? test) (pos? (+ fail error))) (System/exit 1)))
        (do
          (binding [*out* *err*]
            (println "invalid shard; expected --{foundation-,}shard INDEX/TOTAL with 0 <= INDEX < TOTAL"))
          (System/exit 2)))
      (let [mode (cond
                   (empty? args) :full
                   (= ["--foundation"] args) :foundation
                   (= ["--changed"] args) (changed->patterns (changed-modules))
                   :else args)]
        (if (= :full mode)
          (let [t0 (System/nanoTime)
                {:keys [fail error]} (apply test/run-tests test-namespaces)]
            (println (format "full gate in %.1fs" (/ (- (System/nanoTime) t0) 1e9)))
            (shutdown-agents)
            (when (pos? (+ fail error)) (System/exit 1)))
          (if (= :foundation mode)
            (let [vars (vec (foundation-test-vars))
                  _ (println (format "foundation gate: %d required test(s), %d opt-in test(s)"
                                     (count vars) (count non-foundation-test-names)))
                  t0 (System/nanoTime)
                  {:keys [test pass fail error]} (run-vars vars timing?)]
              (println (format "Ran %d tests containing %d assertions."
                               test (+ pass fail error)))
              (println (format "%d failures, %d errors." fail error))
              (println (format "foundation gate in %.1fs"
                               (/ (- (System/nanoTime) t0) 1e9)))
              (shutdown-agents)
              (when (or (zero? test) (pos? (+ fail error))) (System/exit 1)))
          (let [vars (select-vars mode)
                _ (println (format "selective gate: %d test(s) matching %s"
                                   (count vars) (pr-str mode)))
                t0 (System/nanoTime)
                {:keys [test pass fail error]} (run-vars vars timing?)]
            (println (format "Ran %d tests containing %d assertions."
                             test (+ pass fail error)))
            (println (format "%d failures, %d errors." fail error))
            (println (format "selective gate in %.1fs (foundation gate remains the push authority)"
                             (/ (- (System/nanoTime) t0) 1e9)))
            (shutdown-agents)
            (when (or (zero? test) (pos? (+ fail error))) (System/exit 1)))))))))
