(ns pnix-clj.emit-form-roundtrip
  "Clojure-form emit <-> tools.analyzer roundtrip: a lowered form re-analyzed must round-trip, checking the emitter against the JVM analyzer."
  (:require [clojure.tools.analyzer.jvm :as analyzer]
            [clojure.tools.analyzer.passes.jvm.emit-form :as emit-form]
            [pnix-clj.error :as err]
            [pnix-clj.hash :as hash]))

(def lane-classification
  {:lane :proof-only
   :scope :emit-form-roundtrip-evidence
   :product-runtime :forbidden
   :semantic-authority :forbidden
   :mutation :fresh-namespace-only
   :admission :forbidden
   :roundtrip-authority :value-equality-evidence-only
   :allowed-output :emit-form-roundtrip-report})

(def cases
  [{:id :literal-int
    :form 42}
   {:id :arithmetic
    :form '(+ 20 22)}
   {:id :let-arithmetic
    :form '(let [x 20] (+ x 22))}
   {:id :if-branch
    :form '(if true 1 2)}
   {:id :vector-literal
    :form '[1 2 3]}
   {:id :host-call
    :form '(.length "hello")}])

(defn- fresh-ns
  [prefix id]
  (let [ns-sym (symbol (str prefix "." (name id) "."
                            (subs (hash/sha256 (str id)) 0 12)))
        target (create-ns ns-sym)]
    (binding [*ns* target]
      (clojure.core/refer 'clojure.core))
    target))

(defn- eval-in-fresh-ns
  [id form suffix]
  (let [target (fresh-ns "pnix-clj.emit-form" (keyword (str (name id) "-" suffix)))]
    (binding [*ns* target]
      (eval form))))

(defn- throwable-data
  [^Throwable t]
  {:class (.getName (class t))
   :message (.getMessage t)
   :data (ex-data t)})

(defn- case-row
  [{:keys [id form]}]
  (try
    (let [ast (analyzer/analyze form)
          emitted (emit-form/emit-form ast)
          original-value (eval-in-fresh-ns id form "original")
          emitted-value (eval-in-fresh-ns id emitted "emitted")
          same? (= original-value emitted-value)]
      {:id id
       :status (if same? :ok :rejected)
       :reason (if same?
                 :emit-form-roundtrip-value-equal
                 :emit-form-roundtrip-value-mismatch)
       :form form
       :form-hash (hash/data-hash form)
       :ast-op (:op ast)
       :ast-hash (hash/data-hash (select-keys ast [:op :form :env :children]))
       :emitted-form emitted
       :emitted-form-hash (hash/data-hash emitted)
       :original-value original-value
       :emitted-value emitted-value})
    (catch Throwable t
      (merge {:id id :form form}
             (err/failed-throwable :projection
                                   :emit-form-roundtrip-threw
                                   t)))))

(defn report
  []
  (let [rows (mapv case-row cases)
        held-or-rejected (remove #(= :ok (:status %)) rows)
        canonical (mapv #(select-keys % [:id :status :form-hash
                                         :emitted-form-hash])
                        rows)]
    {:kind :pnix-emit-form-roundtrip-report
     :schema :pnix-clj.emit-form-roundtrip-report.v0
     :policy :analyzer-emit-form-value-roundtrip
     :status (if (seq held-or-rejected) :failed :ok)
     :reason (if (seq held-or-rejected)
               :emit-form-roundtrip-held
               :emit-form-roundtrip-ok)
     :case-count (count rows)
     :ok (count (filter #(= :ok (:status %)) rows))
     :held-or-rejected (count held-or-rejected)
     :first-held-or-rejected (first held-or-rejected)
     :rows rows
     :receipt-hash (hash/data-hash canonical)}))

(defn -main
  [& _]
  (let [{:keys [status case-count ok held-or-rejected receipt-hash]} (report)]
    (println (format "pnix-clj emit-form roundtrip: status=%s cases=%d ok=%d held=%d hash=%s"
                     (name status) case-count ok held-or-rejected receipt-hash))
    (shutdown-agents)
    (when (not= :ok status)
      (System/exit 1))))
