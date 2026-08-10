(ns pnix-clj.oracle
  (:require [clojure.edn :as edn]
            [clojure.java.io :as io]))

(def lane-classification
  {:lane :proof-only
   :scope :static-oracle-fixtures
   :product-runtime :forbidden
   :external-authority :forbidden
   :mutation :forbidden
   :admission :forbidden
   :allowed-input :committed-oracle-resources
   :allowed-output :oracle-fixture-set})

(def literal-oracle-resource
  "pnix_clj/oracles/literals.edn")

(def ground-truth-oracle-resource
  "pnix_clj/oracles/ground_truth.edn")

(defn load-oracle-resource
  [resource-path]
  (if-let [resource (io/resource resource-path)]
    (edn/read-string (slurp resource))
    (throw (ex-info "oracle resource not found" {:resource resource-path}))))

(defn literal-cases
  []
  (load-oracle-resource literal-oracle-resource))

(defn ground-truth-fixture-set
  []
  (load-oracle-resource ground-truth-oracle-resource))

(defn ground-truth-cases
  []
  (:cases (ground-truth-fixture-set)))
