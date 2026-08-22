(ns pnix-cljs.module
  (:require [pnix-cljs.core :as core]
            [pnix-cljs.evaluator :as evaluator]
            [pnix-cljs.node-loader :as node-loader]
            [pnix-cljs.outcome :as outcome]))

(defn file-input [filename]
  (let [entry (node-loader/read-entry filename)]
    {:source (:source entry)
     :module-context (node-loader/context (:source-id entry))}))

(defn eval-file-js [filename]
  (let [{:keys [source module-context]} (file-input filename)]
    (clj->js (core/projection source module-context))))

(defn eval-file-json-js [filename]
  (let [{:keys [source module-context]} (file-input filename)]
    (core/projection-json source module-context)))

(defn eval-file-value-json-js [filename]
  (let [{:keys [source module-context]} (file-input filename)]
    (core/value-json source module-context)))

(defn eval-file-value-js [filename]
  (let [{:keys [source module-context]} (file-input filename)]
    (clj->js (core/value source module-context))))

(defn call-file-result [filename entry arguments-json]
  (try
    (let [{:keys [source module-context]} (file-input filename)
          module-value (core/value source module-context)
          result (evaluator/call-module-entry module-value entry arguments-json)]
      (outcome/done (evaluator/materialize result)))
    (catch :default cause
      (outcome/failed (core/error-data cause)))))

(defn call-file-js [filename entry arguments-json]
  (clj->js (outcome/project
            (call-file-result filename entry arguments-json))))

(defn call-file-json-js [filename entry arguments-json]
  (core/canonical-json
   (outcome/project (call-file-result filename entry arguments-json))))

(defn call-file-value-json-js [filename entry arguments-json]
  (let [result (call-file-result filename entry arguments-json)]
    (if (outcome/done? result)
      (core/canonical-json (:value result))
      (throw (ex-info "PNIX callFile failed" (:error result))))))

(set! (.-exports js/module)
      #js {:evalSource core/eval-source-js
           :evalSourceJson core/eval-source-json-js
           :evalValueJson core/eval-value-json-js
           :evalValue core/eval-value-js
           :evalFile eval-file-js
           :evalFileJson eval-file-json-js
           :evalFileValueJson eval-file-value-json-js
           :evalFileValue eval-file-value-js
           :callFile call-file-js
           :callFileJson call-file-json-js
           :callFileValueJson call-file-value-json-js})

(defn -main [& _])
(set! *main-cli-fn* -main)
