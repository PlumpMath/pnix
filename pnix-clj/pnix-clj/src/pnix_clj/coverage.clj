(ns pnix-clj.coverage
  "Dynamic evaluator coverage over the repo-owned source corpus -- which ops, builtins, and branches the fixtures actually exercise."
  (:require [clojure.set :as set]
            [pnix-clj.core :as pnix]
            [pnix-clj.evaluator :as evaluator]
            [pnix-clj.strict-audit :as strict-audit]))

(def lane-classification
  {:lane :proof-only
   :scope :dynamic-evaluator-coverage-evidence
   :product-runtime :forbidden
   :semantic-authority :forbidden
   :mutation :coverage-counters-only
   :admission :forbidden
   :coverage-authority :measurement-only
   :allowed-output :coverage-report})

(def categories
  [:op :builtin :binary-operator :branch])

(def known-ops
  #{:int :float :bool :null :string :path :string-template
    :var :list :attrset :let :if :assert :with :lambda :select :has-attr
    :not :neg :import :call :binary})

(def known-binary-operators
  #{"+" "-" "*" "/" "//" "++" "==" "!=" "<" ">" "<=" ">="
    "&&" "||" "->"})

(def known-branches
  #{:if/then :if/else
    :assert/pass :assert/fail
    :binary/and-right :binary/and-short-circuit
    :binary/or-right :binary/or-short-circuit
    :binary/implies-right :binary/implies-short-circuit})

(defn- known-builtins
  []
  (->> (get evaluator/default-env "builtins")
       vals
       (map :name)
       set))

(defn- key-compare
  [a b]
  (compare (pr-str a) (pr-str b)))

(defn- sorted-counts
  [m]
  (into (sorted-map-by key-compare) m))

(defn- canonical-coverage
  [coverage]
  (into {}
        (map (fn [category]
               [category (sorted-counts (get coverage category {}))]))
        categories))

(defn- merge-counts
  [& maps]
  (apply merge-with + maps))

(defn- merge-coverage
  [rows]
  (into {}
        (map (fn [category]
               [category
                (sorted-counts
                 (apply merge-counts
                        (map #(get-in % [:coverage category] {}) rows)))]))
        categories))

(defn- metric
  [known covered-counts]
  (let [covered (set (keys covered-counts))
        missing (set/difference known covered)
        covered-count (count covered)
        total (count known)]
    {:covered covered-count
     :total total
     :coverage-pct (if (pos? total)
                     (long (Math/round (* 100.0 (/ covered-count total))))
                     100)
     :missing (vec (sort-by pr-str missing))}))

(defn- coverage-row
  [{:keys [source-id source import-modules] :as source-row}]
  (let [coverage* (atom {})
        result (binding [evaluator/*coverage* coverage*]
                 (if (seq import-modules)
                   (pnix/eval-source-with-imports source import-modules)
                   (pnix/eval-source source)))
        coverage (canonical-coverage @coverage*)]
    (-> source-row
        (dissoc :source)
        (assoc :eval-status (:status result)
               :eval-reason (:reason result)
               :coverage coverage
               :coverage-event-counts
               (into {}
                     (map (fn [category]
                            [category (reduce + (vals (get coverage category)))])
                          categories))
               :source-preview (subs source 0 (min 120 (count source)))))))

(defn report
  "Measure dynamic evaluator coverage over the repo-owned source corpus."
  ([] (report {}))
  ([{:keys [include-runtime?]
     :or {include-runtime? false}}]
   (let [source-rows (strict-audit/source-rows {:include-runtime? include-runtime?})
         rows (mapv coverage-row source-rows)
         totals (merge-coverage rows)
         summary {:op (metric known-ops (:op totals))
                  :builtin (metric (known-builtins) (:builtin totals))
                  :binary-operator (metric known-binary-operators
                                           (:binary-operator totals))
                  :branch (metric known-branches (:branch totals))}]
     {:kind :pnix-evaluation-coverage-report
      :schema :pnix-clj.evaluation-coverage.v0
      :policy :dynamic-evaluator-coverage
      :include-runtime? include-runtime?
      :source-count (count rows)
      :summary summary
      :totals totals
      :rows rows})))

(defn -main
  [& _]
  (let [{:keys [source-count summary]} (report)]
    (println (format "pnix-clj coverage: sources=%d ops=%d/%d builtins=%d/%d branches=%d/%d"
                     source-count
                     (get-in summary [:op :covered])
                     (get-in summary [:op :total])
                     (get-in summary [:builtin :covered])
                     (get-in summary [:builtin :total])
                     (get-in summary [:branch :covered])
                     (get-in summary [:branch :total])))
    (shutdown-agents)))
