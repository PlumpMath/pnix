(ns pnix-cljs.main
  (:require [cljs.nodejs :as nodejs]
            [pnix-cljs.core :as core]
            [pnix-cljs.node-loader :as node-loader]))

(nodejs/enable-util-print!)

(defn usage! []
  (binding [*print-fn* *print-err-fn*]
    (println "usage: pnix-cljs -e SOURCE | pnix-cljs FILE")))

(defn source-from [args]
  (cond
    (and (= 2 (count args))
         (contains? #{"-e" "--eval"} (first args)))
    {:source (second args)
     :module-context (node-loader/eval-context)}

    (= 1 (count args))
    (let [entry (node-loader/read-entry (first args))]
      {:source (:source entry)
       :module-context (node-loader/context (:source-id entry))})

    :else nil))

(defn -main [& argv]
  (if-let [{:keys [source module-context]} (source-from (vec argv))]
    (let [projection (core/projection source module-context)]
      (println (core/canonical-json projection))
      (when (= "failed" (get projection "outcome_kind"))
        (set! (.-exitCode js/process) 1)))
    (do
      (usage!)
      (set! (.-exitCode js/process) 2))))

(set! *main-cli-fn* -main)
