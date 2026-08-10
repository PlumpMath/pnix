(ns pnix-clj.clojure-form
  "Clojure-form fixture lane: pnix sources whose lowered Clojure form is pinned, exercising the pnix->Clojure-form projection."
  (:require [clojure.java.io :as io]
            [pnix-clj.clj-meta :as clj-meta]
            [pnix-clj.clojure-projection :as projection]
            [pnix-clj.hash :as hash]
            [pnix-clj.interop :as interop]))

(def lane-classification
  {:lane :proof-only
   :scope :clojure-form-fixture-corpus
   :product-runtime :forbidden
   :semantic-authority :forbidden
   :mutation :forbidden
   :admission :forbidden
   :fixture-authority :committed-corpus-only
   :allowed-output :clojure-form-evidence-report})

(def cases-resource
  "pnix_clj/clojure_form/cases.edn")

(defn fixture-set
  []
  (if-let [resource (io/resource cases-resource)]
    (read-string (slurp resource))
    (throw (ex-info "clojure-form fixtures missing"
                    {:resource cases-resource}))))

(defn cases
  []
  (let [{:keys [lineage cases]} (fixture-set)]
    (mapv (fn [{:keys [name form-source] :as case}]
            (assoc case
                   :source-id (keyword "clojure-form" name)
                   :batch :clojure-form/semantics-first-slice
                   :source form-source
                   :source-origin (:source lineage)
                   :source-lineage lineage
                   :fixture-hash (hash/sha256 (pr-str case))))
          cases)))

(defn- read-form
  [source]
  (try
    {:status :ok
     :value (binding [*read-eval* false]
              (read-string source))}
    (catch Throwable _
      {:status :failed
       :reason :clojure-form-read-failed
       :error {:phase :host-read
               :class :clojure-form-read-failed}})))

(defn- clj-meta-eval-form
  [form]
  (clj-meta/eval-lowered form))

(defn- case-row
  [case]
  (let [read-result (read-form (:form-source case))
        form (:value read-result)
        host-result (when (= :ok (:status read-result))
                      (interop/host-eval-form (:source-id case)
                                              form
                                              interop/host-eval-capabilities))
        clj-meta-result (when (= :ok (:status read-result))
                          (clj-meta-eval-form form))
        term (when (= :ok (:status read-result))
               (projection/project-reader-value form))
        projection-validation (when term
                                (projection/validate-term term))
        projection-value (get projection-validation :validation-value)
        lanes-ok? (and (= :ok (:status read-result))
                       (= :ok (:status host-result))
                       (= :ok (:status clj-meta-result))
                       (= :ok (:status projection-validation))
                       (= "ok" (get projection-value "status")))
        values-agree? (and (= (:expected-value case) (:value host-result))
                           (= (:value host-result) (:value clj-meta-result)))
        status (cond
                 (and lanes-ok? values-agree?) :accepted
                 ;; Every lane produced a value but host eval, clj-meta, and the
                 ;; expected oracle disagree: that is a real semantic mismatch,
                 ;; not an unsupported frontier. Match receipt/verdict policy
                 ;; (both-succeed-but-differ -> :rejected; lane failure -> :failed).
                 lanes-ok? :rejected
                 :else :failed)
        ;; NB: the fn parameter is named `case`, which shadows clojure.core/case,
        ;; so use a literal-map lookup instead of the `case` macro here.
        reason ({:accepted :host-clj-meta-form-semantics-agree
                 :rejected :host-clj-meta-form-semantics-mismatch
                 :failed :host-clj-meta-form-semantics-failed}
                status)]
    {:kind :clojure-form-row
     :source-id (:source-id case)
     :source-hash (hash/sha256 (:form-source case))
     :fixture-hash (:fixture-hash case)
     :status status
     :reason reason
     :form-source (:form-source case)
     :form form
     :form-hash (some-> form hash/data-hash)
     :expected-value (:expected-value case)
     :host-result (select-keys host-result
                               [:status :reason :value :ns :interop
                                :capability :witness])
     :clj-meta-result (select-keys clj-meta-result
                                   [:status :reason :value :mode :diagnostics
                                    :execution-api :evidence-apis
                                    :api-values-agree?
                                    :compile-receipt])
     :projection-term term
     :projection-term-hash (some-> term hash/data-hash)
     :projection-validation projection-validation}))

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
    {:source-id (:source-id row)
     :status (:status row)
     :reason (:reason row)
     :host-status (get-in row [:host-result :status])
     :clj-meta-status (get-in row [:clj-meta-result :status])
     :projection-status (get-in row [:projection-validation :status])
     :projection-value (get-in row [:projection-validation :validation-value])}))

(defn report
  []
  (let [fixtures (fixture-set)
        cases (cases)
        rows (mapv case-row cases)
        counts (frequencies (map :status rows))]
    {:kind :clojure-form-report
     :fixture-kind (:kind fixtures)
     :fixture-schema-version (:schema-version fixtures)
     :lineage (:lineage fixtures)
     :fixture-count (count cases)
     :total (count rows)
     :accepted (long (get counts :accepted 0))
     :held (long (get counts :held 0))
     :failed (long (get counts :failed 0))
     :rejected (long (get counts :rejected 0))
     :reason-counts (->> rows (keep :reason) frequencies (into {}))
     :held-reason-counts (reason-counts rows :held)
     :rejected-reason-counts (reason-counts rows :rejected)
     :first-held (first (filter #(= :held (:status %)) rows))
     :first-failed (first (filter #(= :failed (:status %)) rows))
     :first-rejected (first (filter #(= :rejected (:status %)) rows))
     :first-frontier (first (keep row-frontier rows))
     :fixture-hashes (mapv #(select-keys % [:source-id :fixture-hash])
                           cases)
     :clojure-form-rows rows}))

(defn -main
  [& _]
  (let [{:keys [fixture-count accepted rejected held failed reason-counts
                clojure-form-rows first-frontier]}
        (report)]
    (println (format "pnix-clj clojure-form: fixtures=%d accepted=%d rejected=%d failed=%d held=%d"
                     fixture-count accepted rejected failed held))
    (println "reason-counts:" (pr-str reason-counts))
    (doseq [{:keys [source-id status reason host-result clj-meta-result
                    projection-validation]}
            clojure-form-rows]
      (println (format "  %s status=%s reason=%s host=%s clj-meta=%s projection=%s"
                       (name source-id)
                       (name status)
                       (name reason)
                       (pr-str (:value host-result))
                       (pr-str (:value clj-meta-result))
                       (get-in projection-validation
                               [:validation-value "status"]))))
    (when first-frontier
      (println "first frontier:" (pr-str first-frontier)))
    (shutdown-agents)
    (when (or (pos? rejected) (pos? failed) (pos? held))
      (System/exit 1))))
