(ns pnix-clj.mirror-error
  "Error-path mirror corpus: sources that must HOLD/ERROR, checking all lanes agree on the same failure (reason/error), not just on successes."
  (:require [clojure.java.io :as io]
            [clojure.string :as str]
            [pnix-clj.core :as pnix]
            [pnix-clj.hash :as hash]))

(def lane-classification
  {:lane :proof-only
   :scope :negative-mirror-error-fixture-corpus
   :product-runtime :forbidden
   :semantic-authority :forbidden
   :mutation :forbidden
   :admission :forbidden
   :fixture-authority :committed-error-corpus-only
   :allowed-output :mirror-error-alignment-report})

(def cases-resource
  "pnix_clj/mirror_error/cases.edn")

(defn fixture-set
  []
  (if-let [resource (io/resource cases-resource)]
    (read-string (slurp resource))
    (throw (ex-info "mirror-error fixtures missing"
                    {:resource cases-resource}))))

(defn cases
  []
  (let [{:keys [lineage cases]} (fixture-set)]
    (mapv (fn [{:keys [name] :as case}]
            (assoc case
                   :source-id (keyword "mirror-error" name)
                   :batch :mirror-error/internal-runtime-basics
                   :source-origin (:source lineage)
                   :source-lineage lineage
                   :fixture-hash (hash/sha256 (pr-str case))))
          cases)))

(defn- runtime-receipt
  [receipt]
  (or (get-in receipt [:px-runtime :source-execution :pnix-mirror-receipt])
      (get-in receipt [:pnix-mirror :runtime-receipt])))

(defn- match-row
  [label actual expected]
  {:label label
   :actual actual
   :expected expected
   :ok? (= expected actual)})

(defn- contains-row
  [label actual expected-substring]
  {:label label
   :actual actual
   :expected-contains expected-substring
   :ok? (and (string? actual)
             (str/includes? actual expected-substring))})

(defn- expected-error-alignments
  [case receipt runtime-receipt]
  (let [expected-class (get {:unbound-var :unknown-variable
                             :missing-attr :attribute-missing
                             :call-target-not-callable :not-callable
                             :infinite-recursion :cycle-detected}
                            (:expected-eval-reason case)
                            (:expected-eval-reason case))]
  [(match-row :top-status (:status receipt) :failed)
   (match-row :top-reason (:reason receipt) (:expected-reason case))
   (match-row :eval-status (get-in receipt [:eval-result :status]) :failed)
   (match-row :eval-reason
              (get-in receipt [:eval-result :reason])
              (:expected-eval-reason case))
   (match-row :eval-error-schema
              (get-in receipt [:eval-result :error :schema])
              :pnix.machine.eval-error-model.v1)
   (match-row :eval-error-phase
              (get-in receipt [:eval-result :error :phase])
              :eval)
   (match-row :eval-error-class
              (get-in receipt [:eval-result :error :class])
              expected-class)
   (match-row :px-runtime-status
              (get-in receipt [:px-runtime :status])
              :failed)
   (match-row :px-runtime-error-class
              (get-in receipt [:px-runtime :error :class])
              expected-class)
   (match-row :runtime-mirror-schema
              (get runtime-receipt "mirror_schema")
              "pnixc-pnix.eval.run-mirror.v0")
   (match-row :runtime-mirror-status
              (get runtime-receipt "status")
              "error")
   (match-row :runtime-mirror-phase
              (get runtime-receipt "phase")
              (:expected-runtime-phase case))
   (match-row :runtime-mirror-ast-tag
              (get runtime-receipt "ast_tag")
              (:expected-runtime-ast-tag case))
   (contains-row :runtime-mirror-error
                 (get runtime-receipt "error")
                 (:expected-error-contains case))]))

(defn- lane-statuses
  [receipt]
  (mapv #(select-keys % [:lane :status :reason :frontier])
        (:lane-summary receipt)))

(defn- mirror-error-row
  [case]
  (let [receipt (pnix/verify-source case)
        runtime-receipt (runtime-receipt receipt)
        alignments (expected-error-alignments case receipt runtime-receipt)
        ok? (every? :ok? alignments)]
    {:kind :mirror-error-row
     :source-id (:source-id case)
     :source-hash (:source-hash receipt)
     :ast-hash (:ast-hash receipt)
     :fixture-hash (:fixture-hash case)
     :status (if ok? :accepted :rejected)
     :reason (if ok? :error-lanes-align :error-lanes-diverge)
     :alignment (if ok? :agree :disagree)
     :expected
     {:top-reason (:expected-reason case)
      :eval-reason (:expected-eval-reason case)
      :runtime-reason (:expected-runtime-reason case)
      :runtime-phase (:expected-runtime-phase case)
      :runtime-ast-tag (:expected-runtime-ast-tag case)
      :runtime-error-contains (:expected-error-contains case)}
     :observed
     {:top-status (:status receipt)
      :top-reason (:reason receipt)
      :eval-status (get-in receipt [:eval-result :status])
      :eval-reason (get-in receipt [:eval-result :reason])
      :eval-error-schema (get-in receipt [:eval-result :error :schema])
      :eval-error-phase (get-in receipt [:eval-result :error :phase])
      :eval-error-reason (get-in receipt [:eval-result :error :reason])
      :px-runtime-status (get-in receipt [:px-runtime :status])
      :px-runtime-reason (get-in receipt [:px-runtime :reason])
      :runtime-mirror-schema (get runtime-receipt "mirror_schema")
      :runtime-mirror-status (get runtime-receipt "status")
      :runtime-mirror-phase (get runtime-receipt "phase")
      :runtime-mirror-ast-tag (get runtime-receipt "ast_tag")
      :runtime-mirror-error (get runtime-receipt "error")}
     :alignments alignments
     :lane-summary (lane-statuses receipt)}))

(defn- reason-counts
  [rows status]
  (->> rows
       (filter #(= status (:status %)))
       (keep :reason)
       frequencies
       (into {})))

(defn- row-frontier
  [row]
  (when (not= :accepted (:status row))
    (let [failed (first (remove :ok? (:alignments row)))]
      (cond-> {:source-id (:source-id row)
               :status (:status row)
               :reason (:reason row)}
        failed
        (assoc :failed-alignment (:label failed)
               :actual (:actual failed)
               :expected (or (:expected failed)
                             (:expected-contains failed)))))))

(defn report
  []
  (let [fixtures (fixture-set)
        cases (cases)
        rows (mapv mirror-error-row cases)
        counts (frequencies (map :status rows))]
    {:kind :mirror-error-report
     :fixture-kind (:kind fixtures)
     :fixture-schema-version (:schema-version fixtures)
     :lineage (:lineage fixtures)
     :fixture-count (count cases)
     :total (count rows)
     :accepted (long (get counts :accepted 0))
     :held (long (get counts :held 0))
     :rejected (long (get counts :rejected 0))
     :reason-counts (->> rows (keep :reason) frequencies (into {}))
     :held-reason-counts (reason-counts rows :held)
     :rejected-reason-counts (reason-counts rows :rejected)
     :first-held (first (filter #(= :held (:status %)) rows))
     :first-rejected (first (filter #(= :rejected (:status %)) rows))
     :first-frontier (first (keep row-frontier rows))
     :fixture-hashes (mapv #(select-keys % [:source-id :fixture-hash])
                           cases)
     :mirror-error-rows rows}))

(defn -main
  [& _]
  (let [{:keys [fixture-count accepted rejected held reason-counts
                mirror-error-rows first-frontier]}
        (report)]
    (println (format "pnix-clj mirror-error: fixtures=%d accepted=%d rejected=%d held=%d"
                     fixture-count accepted rejected held))
    (println "reason-counts:" (pr-str reason-counts))
    (doseq [{:keys [source-id status reason alignment observed]} mirror-error-rows]
      (println (format "  %s status=%s reason=%s alignment=%s eval=%s px=%s runtime=%s/%s"
                       (name source-id)
                       (name status)
                       (name reason)
                       (name alignment)
                       (name (:eval-reason observed))
                       (name (:px-runtime-reason observed))
                       (:runtime-mirror-status observed)
                       (:runtime-mirror-ast-tag observed))))
    (when first-frontier
      (println "first frontier:" (pr-str first-frontier)))
    (shutdown-agents)
    (when (or (pos? rejected) (pos? held))
      (System/exit 1))))
