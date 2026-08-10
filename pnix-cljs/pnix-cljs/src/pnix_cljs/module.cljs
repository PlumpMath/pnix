(ns pnix-cljs.module
  (:require [pnix-cljs.core :as core]
            [pnix-cljs.node-loader :as node-loader]))

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

(set! (.-exports js/module)
      #js {:evalSource core/eval-source-js
           :evalSourceJson core/eval-source-json-js
           :evalValueJson core/eval-value-json-js
           :evalValue core/eval-value-js
           :evalFile eval-file-js
           :evalFileJson eval-file-json-js
           :evalFileValueJson eval-file-value-json-js
           :evalFileValue eval-file-value-js})

(defn -main [& _])
(set! *main-cli-fn* -main)
