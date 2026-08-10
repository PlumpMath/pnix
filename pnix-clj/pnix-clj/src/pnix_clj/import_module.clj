(ns pnix-clj.import-module
  (:require [clojure.edn :as edn]
            [clojure.java.io :as io]))

(def lane-classification
  {:lane :proof-only
   :scope :import-module-fixture-corpus
   :product-runtime :forbidden
   :semantic-authority :forbidden
   :mutation :forbidden
   :admission :forbidden
   :fixture-authority :committed-corpus-only
   :allowed-output :import-module-fixture-set})

(def cases-resource
  "pnix_clj/import_module/cases.edn")

(defn fixture-set
  []
  (if-let [resource (io/resource cases-resource)]
    (edn/read-string (slurp resource))
    (throw (ex-info "import module fixtures missing"
                    {:resource cases-resource}))))

(defn cases
  []
  (:cases (fixture-set)))
