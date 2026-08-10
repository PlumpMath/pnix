(ns pnix-clj.smoke
  "Fast smoke report over the ground-truth oracle cases -- the quickest all-lanes-agree sanity signal."
  (:require [pnix-clj.core :as pnix]
            [pnix-clj.oracle :as oracle]))

(def lane-classification
  {:lane :proof-only
   :scope :fast-smoke-evidence
   :product-runtime :forbidden
   :semantic-authority :forbidden
   :mutation :forbidden
   :admission :forbidden
   :fixture-authority :ground-truth-oracle-corpus-only
   :allowed-output :smoke-report})

(def smoke-sources
  (oracle/ground-truth-cases))

(defn -main
  [& _]
  (let [{:keys [total accepted rejected held first-rejected first-held]} (pnix/report smoke-sources)]
    (println (format "pnix-clj smoke: total=%d accepted=%d rejected=%d held=%d"
                     total accepted rejected held))
    (when first-held
      (println "first held:" (pr-str (select-keys first-held [:source :reason]))))
    (when first-rejected
      (println "first rejected:" (pr-str (select-keys first-rejected [:source :reason]))))
    (when (pos? rejected)
      (System/exit 1))))
