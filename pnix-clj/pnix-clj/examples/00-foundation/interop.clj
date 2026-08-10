(ns foundation.interop
  (:require [pnix-clj.core :as pnix]
            [pnix-clj.interop :as interop]))

;; A PNIX attrset becomes an ordinary host value at the observation boundary.
(let [result (pnix/eval-source
               "{ answer = 20 + 22; nested = { enabled = true; }; }")]
  (println "PNIX -> Clojure:" (:value result))
  (assert (= :ok (:status result))))

;; Host objects that are not PNIX values cross nominally as opaque references.
(let [object (StringBuilder. "pnix")
      reference (interop/from-host object)
      restored (interop/to-host reference)]
  (println "opaque reference:" (select-keys reference [:kind :id :class]))
  (assert (interop/opaque-host-ref? reference))
  (assert (identical? object restored))
  (interop/release-opaque-ref! reference))

(shutdown-agents)
