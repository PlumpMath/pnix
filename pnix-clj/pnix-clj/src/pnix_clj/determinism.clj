(ns pnix-clj.determinism
  "Evaluation determinism: parse/eval each source K times and assert stable AST + result hashes (no hidden nondeterminism)."
  (:require [pnix-clj.core :as pnix]
            [pnix-clj.hash :as hash]
            [pnix-clj.strict-audit :as strict-audit]))

(def lane-classification
  {:lane :core
   :scope :evaluation-determinism-audit
   :role :repeat-parse-eval-hash-stability
   :product-runtime :allowed
   :semantic-authority :evidence-only
   :mutation :forbidden
   :admission :fail-closed
   :determinism :required
   :allowed-output :determinism-report})

(def default-runs
  3)

(defn- result-projection
  [result]
  (cond-> {:status (:status result)
           :reason (:reason result)}
    (= :ok (:status result))
    (assoc :value (:value result))

    (not= :ok (:status result))
    (assoc :error (select-keys result [:error :target]))))

(defn- sample-once
  [source import-modules]
  (let [parsed (pnix/parse-source source)
        result (if (seq import-modules)
                 (pnix/eval-source-with-imports source import-modules)
                 (pnix/eval-source source))
        projected-result (result-projection result)]
    {:parse-status (:status parsed)
     :parse-reason (:reason parsed)
     :ast-hash (when (= :ok (:status parsed))
                 (hash/data-hash (:ast parsed)))
     :result projected-result
     :result-hash (hash/data-hash projected-result)}))

(defn- comparable-sample
  [sample]
  (select-keys sample [:parse-status :parse-reason :ast-hash :result-hash]))

(defn- determinism-row
  [runs {:keys [source-id source import-modules] :as source-row}]
  (let [samples (mapv (fn [_] (sample-once source import-modules)) (range runs))
        comparable (mapv comparable-sample samples)
        stable? (= 1 (count (set comparable)))]
    (cond-> (-> source-row
                (dissoc :source)
                (assoc :run-count runs
                       :stable? stable?
                       :first-sample (first samples)
                       :sample-hashes (mapv :result-hash samples)
                       :comparable-samples comparable
                       :source-preview (subs source 0 (min 120 (count source)))))
      (not stable?)
      (assoc :reason :pnix-evaluation-nondeterministic
             :source-id source-id))))

(defn- count-by
  [f xs]
  (frequencies (keep f xs)))

(defn report
  "Run each source through parse/eval K times and assert stable AST/result hashes.

  The default corpus is the repo-owned pnix fixture corpus. Runtime .px files can
  be included with `{:include-runtime? true}` when a slower inventory check is
  wanted."
  ([] (report {}))
  ([{:keys [runs include-runtime?]
     :or {runs default-runs
          include-runtime? false}}]
   (let [source-rows (strict-audit/source-rows {:include-runtime? include-runtime?})
         rows (mapv #(determinism-row runs %) source-rows)
         unstable (remove :stable? rows)]
     {:kind :pnix-evaluation-determinism-report
      :schema :pnix-clj.evaluation-determinism.v0
      :policy :repeat-parse-eval-hash-stability
      :runs-per-source runs
      :include-runtime? include-runtime?
      :source-count (count rows)
      :stable (count (filter :stable? rows))
      :unstable (count unstable)
      :source-family-counts (count-by :source-family rows)
      :first-unstable (first unstable)
      :rows rows})))

(defn -main
  [& [runs]]
  (let [{:keys [source-count stable unstable first-unstable] :as report}
        (report {:runs (if runs (parse-long runs) default-runs)})]
    (println (format "pnix-clj determinism: sources=%d stable=%d unstable=%d runs=%d"
                     source-count stable unstable (:runs-per-source report)))
    (when first-unstable
      (println "first unstable:" (pr-str first-unstable)))
    (shutdown-agents)
    (when (pos? unstable)
      (System/exit 1))))
