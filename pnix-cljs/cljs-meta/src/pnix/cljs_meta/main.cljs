(ns pnix.cljs-meta.main
  (:require [cljs.nodejs :as nodejs]
            [pnix.cljs-meta.core :as core]))

(nodejs/enable-util-print!)

(def fs (js/require "fs"))

(defn usage! []
  (binding [*print-fn* *print-err-fn*]
    (println "usage: cljs-meta -e EXPR | cljs-meta FILE")))

(defn source-from [args]
  (cond
    (and (= 2 (count args))
         (contains? #{"-e" "--eval"} (first args)))
    (second args)

    (= 1 (count args))
    (.readFileSync fs (first args) "utf8")

    :else nil))

(defn -main [& argv]
  (if-let [source (source-from (vec argv))]
    (core/evaluate
      source
      (fn [result]
        (let [projection (core/result-projection result)]
          (println (js/JSON.stringify (clj->js projection)))
          (when (= "failed" (get projection "outcome_kind"))
            (set! (.-exitCode js/process) 1)))))
    (do
      (usage!)
      (set! (.-exitCode js/process) 2))))

(set! *main-cli-fn* -main)
