(ns foundation.basic
  (:require [pnix-clj.core :as pnix]))

(defn public-result [result]
  (select-keys result [:status :value :error :reason]))

(let [done (pnix/run-source "let x = 20; in x + 22")
      failed (pnix/run-source "1 / 0")
      guest-held (pnix/run-source "{ status = \"held\"; value = 42; }")]
  (println "done:" (public-result done))
  (println "failed:" (public-result failed))
  (println "guest held-shaped value:" (public-result guest-held))

  (assert (= :ok (:status done)))
  (assert (= 42 (:value done)))
  (assert (not= :held (:status failed)))
  (assert (= :ok (:status guest-held)))
  (assert (not-any? #(contains? done %)
                    [:receipts :mirror-run :cross-mirror-verdict])))

(shutdown-agents)
