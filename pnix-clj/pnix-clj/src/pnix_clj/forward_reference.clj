(ns pnix-clj.forward-reference
  (:require [clojure.java.io :as io]
            [pnix-clj.core :as pnix]
            [pnix-clj.hash :as hash]))

(def lane-classification
  {:lane :proof-only
   :scope :forward-reference-fixture-corpus
   :product-runtime :forbidden
   :semantic-authority :forbidden
   :mutation :forbidden
   :admission :forbidden
   :fixture-authority :committed-corpus-only
   :allowed-output :forward-reference-evidence-report})

(def cases-resource
  "pnix_clj/forward_reference/cases.edn")

(defn fixture-set
  []
  (if-let [resource (io/resource cases-resource)]
    (read-string (slurp resource))
    (throw (ex-info "forward-reference fixtures missing"
                    {:resource cases-resource}))))

(defn cases
  []
  (let [{:keys [lineage cases]} (fixture-set)]
    (mapv (fn [{:keys [name] :as case}]
            (assoc case
                   :source-id (keyword "forward-reference" name)
                   :batch :forward-reference/lift
                   :source-origin (:source lineage)
                   :source-lineage lineage
                   :fixture-hash (hash/sha256 (pr-str case))))
          cases)))

(defn- lane-statuses
  [receipt]
  (mapv #(select-keys % [:lane :status :reason :frontier])
        (:lane-summary receipt)))

(defn- check-row
  [label ok? expected actual]
  {:label label
   :ok? (boolean ok?)
   :expected expected
   :actual actual})

(defn- eval-checks
  [{:keys [expected-eval]} receipt]
  (let [eval-result (:eval-result receipt)]
    (cond-> [(check-row :eval-status
                        (= (:status expected-eval) (:status eval-result))
                        (:status expected-eval)
                        (:status eval-result))]
      (contains? expected-eval :value)
      (conj (check-row :eval-value
                       (= (:value expected-eval) (:value eval-result))
                       (:value expected-eval)
                       (:value eval-result)))

      (:reason expected-eval)
      (conj (check-row :eval-reason
                       (= (:reason expected-eval) (:reason eval-result))
                       (:reason expected-eval)
                       (:reason eval-result))))))

(defn- forward-ok-checks
  [{:keys [expected-eval]} receipt lane-by lifted-lanes]
  (into [(check-row :top-status
                    (= :accepted (:status receipt))
                    :accepted
                    (:status receipt))
         (check-row :top-reason
                    (= :all-lanes-agree (:reason receipt))
                    :all-lanes-agree
                    (:reason receipt))
         (check-row :clj-meta-value
                    (= (:value expected-eval)
                       (get-in receipt [:clj-meta-result :value]))
                    (:value expected-eval)
                    (get-in receipt [:clj-meta-result :value]))
         (check-row :px-runtime-value
                    (= (:value expected-eval)
                       (get-in receipt [:px-runtime :value]))
                    (:value expected-eval)
                    (get-in receipt [:px-runtime :value]))]
        (map (fn [lane]
               (check-row (keyword (str "lane-" (name lane)))
                          (= :ok (:status (lane-by lane)))
                          :ok
                          (:status (lane-by lane))))
             lifted-lanes)))

(defn- semantic-error-checks
  "Cycle/unbound rows are deterministic semantic failures on every lifted
  lane (lineage note on the fixture set) — not policy holds or frontiers.
  Expect :failed, not the pre-R1 :held frontier shape."
  [_case _receipt lane-by lifted-lanes]
  (mapv (fn [lane]
          (check-row (keyword (str "lane-" (name lane)))
                     (= :failed (:status (lane-by lane)))
                     :failed
                     (:status (lane-by lane))))
        lifted-lanes))

(defn- row
  [lifted-lanes case]
  (let [receipt (pnix/verify-source case)
        lane-by (into {} (map (juxt :lane identity) (:lane-summary receipt)))
        checks (into (eval-checks case receipt)
                     (if (= :forward-ok (:class case))
                       (forward-ok-checks case receipt lane-by lifted-lanes)
                       (semantic-error-checks case receipt lane-by lifted-lanes)))
        ok? (every? :ok? checks)]
    {:kind :forward-reference-row
     :source-id (:source-id case)
     :class (:class case)
     :source-hash (:source-hash receipt)
     :ast-hash (:ast-hash receipt)
     :fixture-hash (:fixture-hash case)
     :status (if ok? :accepted :rejected)
     :reason (if ok? :forward-reference-contract-satisfied
                 :forward-reference-contract-diverged)
     :top-status (:status receipt)
     :top-reason (:reason receipt)
     :eval-result (:eval-result receipt)
     :clj-meta-status (get-in receipt [:clj-meta-result :status])
     :clj-meta-value (get-in receipt [:clj-meta-result :value])
     :px-runtime-status (get-in receipt [:px-runtime :status])
     :px-runtime-value (get-in receipt [:px-runtime :value])
     :checks checks
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
    (let [failed (first (remove :ok? (:checks row)))]
      (cond-> {:source-id (:source-id row)
               :status (:status row)
               :reason (:reason row)}
        failed
        (assoc :failed-check (:label failed)
               :actual (:actual failed)
               :expected (:expected failed))))))

(defn report
  []
  (let [{:keys [kind schema-version lineage lifted-lanes]} (fixture-set)
        rows (mapv (partial row lifted-lanes) (cases))
        counts (frequencies (map :status rows))]
    {:kind :forward-reference-lift-report
     :fixture-kind kind
     :fixture-schema-version schema-version
     :lineage lineage
     :lifted-lanes lifted-lanes
     :fixture-count (count rows)
     :total (count rows)
     :accepted (long (get counts :accepted 0))
     :held (long (get counts :held 0))
     :rejected (long (get counts :rejected 0))
     :forward-ok-count (count (filter #(= :forward-ok (:class %)) rows))
     :semantic-error-count (count (remove #(= :forward-ok (:class %)) rows))
     :reason-counts (->> rows (keep :reason) frequencies (into {}))
     :held-reason-counts (reason-counts rows :held)
     :rejected-reason-counts (reason-counts rows :rejected)
     :first-held (first (filter #(= :held (:status %)) rows))
     :first-rejected (first (filter #(= :rejected (:status %)) rows))
     :first-frontier (first (keep row-frontier rows))
     :fixture-hashes (mapv #(select-keys % [:source-id :fixture-hash])
                           (cases))
     :rows rows
     :receipt-hash (hash/data-hash {:kind :forward-reference-lift-report
                                    :fixture-kind kind
                                    :rows (mapv #(select-keys %
                                                              [:source-id
                                                               :class
                                                               :status
                                                               :reason
                                                               :top-status
                                                               :top-reason
                                                               :eval-result
                                                               :clj-meta-status
                                                               :clj-meta-value
                                                               :px-runtime-status
                                                               :px-runtime-value])
                                                 rows)} )}))

(defn -main
  [& _]
  (let [{:keys [fixture-count accepted rejected held forward-ok-count
                semantic-error-count reason-counts first-frontier receipt-hash]}
        (report)]
    (println (format "pnix-clj forward-reference: fixtures=%d accepted=%d rejected=%d held=%d forward-ok=%d semantic-error=%d"
                     fixture-count accepted rejected held forward-ok-count
                     semantic-error-count))
    (println "reason-counts:" (pr-str reason-counts))
    (println "receipt-hash:" receipt-hash)
    (when first-frontier
      (println "first frontier:" (pr-str first-frontier)))
    (shutdown-agents)
    (when (pos? (+ rejected held))
      (System/exit 1))))
