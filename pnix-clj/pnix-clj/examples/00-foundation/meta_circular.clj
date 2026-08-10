(ns foundation.meta-circular
  (:require [pnix-clj.core :as pnix]))

;; This is a basic host mechanism: PNIX is lowered and executed by clj-meta.
;; It does not wait for mirror, deployment, or proof-policy approval.
(let [result (pnix/compile-source "let double = x: x * 2; in double 21")]
  (println "clj-meta execution:"
           (select-keys (:clj-meta-result result)
                        [:status :value :mode :execution-api]))
  (assert (= :ok (:status result)))
  (assert (= 42 (get-in result [:clj-meta-result :value])))
  (assert (nil? (:compile-receipt result))))

;; Verification remains available, but through a different explicit API.
(println "independent verification API available:" (fn? pnix/verify-source))

(shutdown-agents)
