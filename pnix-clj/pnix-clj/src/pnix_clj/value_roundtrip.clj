(ns pnix-clj.value-roundtrip
  "Value roundtrip: a normalized pnix value -> canonical Clojure form -> re-created value, checking meaning-preserving value projection."
  (:require [pnix-clj.clj-meta :as clj-meta]
            [pnix-clj.core :as pnix]
            [pnix-clj.error :as err]
            [pnix-clj.hash :as hash]
            [pnix-clj.lowering :as lowering]))

(def lane-classification
  {:lane :proof-only
   :scope :value-roundtrip-evidence
   :product-runtime :forbidden
   :semantic-authority :forbidden
   :mutation :forbidden
   :admission :forbidden
   :roundtrip-authority :value-projection-evidence-only
   :allowed-output :value-roundtrip-report})

(def cases
  [{:id :int-literal
    :source "42"}
   {:id :bool-literal
    :source "true"}
   {:id :null-literal
    :source "null"}
   {:id :string-literal
    :source "\"hello\""}
   {:id :list-literal
    :source "[1 2 3]"}
   {:id :attrset-literal
    :source "{ b = [true null]; a = 1; }"}
   {:id :let-arithmetic
    :source "let x = 40; in x + 2"}
   {:id :builtin-attr-values
    :source "builtins.attrValues { b = 2; a = 1; }"}])

(defn- throwable-data
  [^Throwable t]
  {:class (.getName (class t))
   :message (.getMessage t)
   :data (ex-data t)})

(declare synthesize-form)

(defn- ok-form
  [form]
  {:status :ok
   :form form
   :form-hash (hash/data-hash form)})

(defn- function-like-map?
  [value]
  (and (map? value)
       (contains? #{:closure :builtin} (:kind value))))

(defn- synthesize-vector-form
  [value]
  (loop [remaining value
         forms []]
    (if (seq remaining)
      (let [result (synthesize-form (first remaining))]
        (if (= :ok (:status result))
          (recur (rest remaining) (conj forms (:form result)))
          result))
      (ok-form (vec forms)))))

(defn- synthesize-map-form
  [value]
  (if (function-like-map? value)
    (err/failed :projection
                :function-value-not-synthesizable
                {:value-kind (:kind value)})
    (loop [remaining (sort-by (comp pr-str key) value)
           forms []]
      (if-let [[k v] (first remaining)]
        (let [key-result (synthesize-form k)
              value-result (synthesize-form v)]
          (cond
            (not= :ok (:status key-result))
            key-result

            (not= :ok (:status value-result))
            value-result

            :else
            (recur (rest remaining)
                   (conj forms (:form key-result) (:form value-result)))))
        (ok-form (apply list 'array-map forms))))))

(defn synthesize-form
  "Turn a normalized pnix value into a canonical Clojure form that re-creates it.

  This is intentionally value-level only: functions, builtins, and effectful
  host objects are held instead of guessed. Maps are emitted through `array-map`
  with sorted pairs so the value->form->value closure has a stable hash."
  [value]
  (let [value (lowering/force-normal value)]
    (cond
      (nil? value) (ok-form nil)
      (or (true? value) (false? value)) (ok-form value)
      (number? value) (ok-form value)
      (string? value) (ok-form value)
      (vector? value) (synthesize-vector-form value)
      (map? value) (synthesize-map-form value)
      :else (err/failed :projection :value-not-synthesizable))))

(defn- normalized-ok-value
  [result]
  (when (= :ok (:status result))
    (lowering/force-normal (:value result))))

(defn- case-row
  [{:keys [id source]}]
  (try
    (let [eval-result (pnix/eval-source source)
          pnix-value (normalized-ok-value eval-result)
          lowering-result (pnix/lower-source source)
          forward-result (when (= :ok (:status lowering-result))
                           (clj-meta/eval-lowered (:form lowering-result)))
          forward-value (normalized-ok-value forward-result)
          synth-result (when (= :ok (:status eval-result))
                         (synthesize-form pnix-value))
          synthesized-result (when (= :ok (:status synth-result))
                               (clj-meta/eval-lowered (:form synth-result)))
          synthesized-value (normalized-ok-value synthesized-result)
          closure-result (when (= :ok (:status synthesized-result))
                           (synthesize-form synthesized-value))
          forward-same? (= pnix-value forward-value)
          synthesized-same? (= pnix-value synthesized-value)
          closure-same? (= (:form synth-result) (:form closure-result))
          ok? (and (= :ok (:status eval-result))
                   (= :ok (:status lowering-result))
                   (= :ok (:status forward-result))
                   (= :ok (:status synth-result))
                   (= :ok (:status synthesized-result))
                   (= :ok (:status closure-result))
                   forward-same?
                   synthesized-same?
                   closure-same?)]
      {:id id
       :source source
       :source-hash (hash/sha256 source)
       :status (if ok? :ok :failed)
       :reason (if ok?
                 :pnix-value-roundtrip-ok
                 :pnix-value-roundtrip-failed)
       :eval-status (:status eval-result)
       :eval-reason (:reason eval-result)
       :lowering-status (:status lowering-result)
       :lowering-reason (:reason lowering-result)
       :forward-status (:status forward-result)
       :forward-reason (:reason forward-result)
       :synthesis-status (:status synth-result)
       :synthesis-reason (:reason synth-result)
       :synthesized-status (:status synthesized-result)
       :synthesized-reason (:reason synthesized-result)
       :closure-status (:status closure-result)
       :closure-reason (:reason closure-result)
       :pnix-value pnix-value
       :forward-value forward-value
       :synthesized-value synthesized-value
       :lowered-form (:form lowering-result)
       :lowered-form-hash (:form-hash lowering-result)
       :synthesized-form (:form synth-result)
       :synthesized-form-hash (:form-hash synth-result)
       :closure-form (:form closure-result)
       :closure-form-hash (:form-hash closure-result)
       :forward-value-equal? forward-same?
       :synthesized-value-equal? synthesized-same?
       :closure-form-equal? closure-same?})
    (catch Throwable t
      (merge {:id id
              :source source
              :source-hash (hash/sha256 source)}
             (err/failed-throwable :projection
                                   :pnix-value-roundtrip-threw
                                   t)))))

(defn report
  []
  (let [rows (mapv case-row cases)
        held-or-rejected (remove #(= :ok (:status %)) rows)
        canonical (mapv #(select-keys % [:id :status :source-hash
                                         :lowered-form-hash
                                         :synthesized-form-hash
                                         :closure-form-hash])
                        rows)]
    {:kind :pnix-value-roundtrip-report
     :schema :pnix-clj.value-roundtrip-report.v0
     :policy :pnix-value-to-clojure-form-synthesis
     :status (if (seq held-or-rejected) :failed :ok)
     :reason (if (seq held-or-rejected)
               :pnix-value-roundtrip-failed
               :pnix-value-roundtrip-ok)
     :case-count (count rows)
     :ok (count (filter #(= :ok (:status %)) rows))
     :held-or-rejected (count held-or-rejected)
     :first-held-or-rejected (first held-or-rejected)
     :rows rows
     :receipt-hash (hash/data-hash canonical)}))

(defn -main
  [& _]
  (let [{:keys [status case-count ok held-or-rejected receipt-hash]} (report)]
    (println (format "pnix-clj value roundtrip: status=%s cases=%d ok=%d held=%d hash=%s"
                     (name status) case-count ok held-or-rejected receipt-hash))
    (shutdown-agents)
    (when (not= :ok status)
      (System/exit 1))))
