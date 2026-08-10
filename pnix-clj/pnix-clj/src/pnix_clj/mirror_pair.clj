(ns pnix-clj.mirror-pair
  "The core 4-lane cross-check corpus: every source must collapse to one value across the direct evaluator, clj-meta bytecode, .px runtime, and pnix mirror."
  (:require [clojure.java.io :as io]
            [pnix-clj.core :as pnix]
            [pnix-clj.hash :as hash]))

(def lane-classification
  {:lane :proof-only
   :scope :four-lane-cross-check-fixture-corpus
   :product-runtime :forbidden
   :semantic-authority :forbidden
   :mutation :forbidden
   :admission :forbidden
   :fixture-authority :committed-corpus-only
   :allowed-output :mirror-pair-evidence-report})

(def cases-resource
  "pnix_clj/mirror_pair/cases.edn")

(defn fixture-set
  []
  (if-let [resource (io/resource cases-resource)]
    (read-string (slurp resource))
    (throw (ex-info "mirror-pair fixtures missing"
                    {:resource cases-resource}))))

(defn cases
  []
  (let [{:keys [lineage cases]} (fixture-set)]
    (mapv (fn [{:keys [name source oracle-result import-modules] :as case}]
            (cond-> {:source-id (keyword "mirror-pair" name)
                     :source source
                     :batch :mirror-pair/internal-runtime-basics
                     :source-origin (:source lineage)
                     :source-lineage lineage
                     :fixture-hash (hash/sha256 (pr-str case))
                     :oracle-result oracle-result}
              ;; in-memory pnix module map for import/scopedImport fixtures;
              ;; threaded to run-source (and the tower climb) so the module
              ;; resolves on every lane without filesystem access.
              import-modules (assoc :import-modules import-modules)))
          cases)))

(defn- value-hash
  [value]
  (some-> value hash/data-hash))

(defn- lane-statuses
  [receipt]
  (mapv #(select-keys % [:lane :status :reason :frontier])
        (:lane-summary receipt)))

(defn- mirror-pair-ready?
  [receipt]
  (and (= :accepted (:status receipt))
       (= :ok (get-in receipt [:eval-result :status]))
       (= :ok (get-in receipt [:clj-meta-result :status]))
       (= :ok (get-in receipt [:clojure-mirror :clj-meta-status]))
       (= :ok (get-in receipt [:px-runtime :status]))
       (= :ok (get-in receipt [:pnix-mirror :status]))
       (= "ok" (get-in receipt [:pnix-mirror :runtime-receipt "status"]))
       (= :ok (get-in receipt [:cross-mirror-verdict :status]))
       (= :agree (get-in receipt [:cross-mirror-verdict :equivalence]))))

(defn- mirror-pair-row
  [receipt]
  {:kind :mirror-pair-row
   :source-id (:source-id receipt)
   :status (:status receipt)
   :reason (:reason receipt)
   :ready? (mirror-pair-ready? receipt)
   :source-hash (:source-hash receipt)
   :ast-hash (:ast-hash receipt)
   :fixture-hash (get-in receipt [:source-meta :fixture-hash])
   :value-hashes
   {:evaluator (value-hash (get-in receipt [:eval-result :value]))
    :clj-meta (value-hash (get-in receipt [:clj-meta-result :value]))
    :px-runtime (value-hash (get-in receipt [:px-runtime :value]))
    :pnix-mirror (value-hash (get-in receipt [:pnix-mirror :value]))}
   :clojure-mirror
   {:kind (get-in receipt [:clojure-mirror :kind])
    :clj-meta-status (get-in receipt [:clojure-mirror :clj-meta-status])
    :clj-meta-mode (get-in receipt [:clojure-mirror :clj-meta-mode])
    :determinism-status
    (get-in receipt [:clojure-mirror :clj-meta-determinism :status])
    :same-class-name?
    (get-in receipt [:clojure-mirror :clj-meta-determinism
                     :same-class-name?])
    :fallback?
    (get-in receipt [:clojure-mirror :clj-meta-fallback :fallback?])
    :stage15-control-status
    (get-in receipt [:clojure-mirror :stage15-control :status])
    :stage15-control-reason
    (get-in receipt [:clojure-mirror :stage15-control :reason])}
   :px-runtime
   {:kind (get-in receipt [:px-runtime :kind])
    :status (get-in receipt [:px-runtime :status])
    :reason (get-in receipt [:px-runtime :reason])
    :artifact (select-keys (get-in receipt [:px-runtime :artifact])
                           [:root :relative-path :hash])
    :source-execution-kind
    (get-in receipt [:px-runtime :source-execution :kind])
    :source-execution-reason
    (get-in receipt [:px-runtime :source-execution :reason])
    :runtime-mirror-status
    (get-in receipt [:px-runtime :source-execution :pnix-mirror-receipt "status"])
    :runtime-mirror-phase
    (get-in receipt [:px-runtime :source-execution :pnix-mirror-receipt "phase"])
    :runtime-mirror-ast-tag
    (get-in receipt [:px-runtime :source-execution :pnix-mirror-receipt "ast_tag"])}
   :pnix-mirror
   {:kind (get-in receipt [:pnix-mirror :kind])
    :status (get-in receipt [:pnix-mirror :status])
    :reason (get-in receipt [:pnix-mirror :reason])
    :px-runtime-status (get-in receipt [:pnix-mirror :px-runtime-status])
    :runtime-mirror-schema
    (get-in receipt [:pnix-mirror :runtime-receipt "mirror_schema"])
    :runtime-mirror-owner
    (get-in receipt [:pnix-mirror :runtime-receipt "owner"])}
   :cross-mirror
   {:kind (get-in receipt [:cross-mirror-verdict :kind])
    :status (get-in receipt [:cross-mirror-verdict :status])
    :reason (get-in receipt [:cross-mirror-verdict :reason])
    :equivalence (get-in receipt [:cross-mirror-verdict :equivalence])
    :host-mirror-agrees?
    (get-in receipt [:cross-mirror-verdict :host-mirror-agrees?])}
   :lane-summary (lane-statuses receipt)})

(defn report
  []
  (let [fixtures (fixture-set)
        cases (cases)
        pnix-report (pnix/report cases)
        rows (mapv mirror-pair-row (:receipts pnix-report))
        ready-count (count (filter :ready? rows))]
    (assoc pnix-report
           :kind :mirror-pair-report
           :fixture-kind (:kind fixtures)
           :fixture-schema-version (:schema-version fixtures)
           :lineage (:lineage fixtures)
           :fixture-count (count cases)
           :fixture-hashes (mapv #(select-keys % [:source-id :fixture-hash])
                                 cases)
           :mirror-pair-rows rows
           :mirror-pair-ready-count ready-count
           :mirror-pair-not-ready-count (count (remove :ready? rows)))))

(defn -main
  [& _]
  (let [{:keys [fixture-count accepted rejected held mirror-pair-ready-count
                mirror-pair-not-ready-count mirror-pair-rows first-frontier
                reason-counts]}
        (report)]
    (println (format "pnix-clj mirror-pair: fixtures=%d accepted=%d rejected=%d held=%d ready=%d not-ready=%d"
                     fixture-count accepted rejected held
                     mirror-pair-ready-count mirror-pair-not-ready-count))
    (println "reason-counts:" (pr-str reason-counts))
    (doseq [{:keys [source-id status reason ready? cross-mirror px-runtime]}
            mirror-pair-rows]
      (println (format "  %s status=%s reason=%s ready=%s cross=%s px=%s"
                       (name source-id)
                       (name status)
                       (name reason)
                       ready?
                       (name (:equivalence cross-mirror))
                       (name (:status px-runtime)))))
    (when first-frontier
      (println "first frontier:" (pr-str first-frontier)))
    (shutdown-agents)
    (when (pos? rejected)
      (System/exit 1))))
