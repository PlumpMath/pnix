(ns pnix-cljs.node-loader)

(def fs (js/require "fs"))
(def node-path (js/require "path"))

(defn canonical-path [value]
  (.realpathSync fs (.resolve node-path value)))

(defn read-entry [filename]
  (let [source-id (canonical-path filename)]
    {:source-id source-id
     :source (.readFileSync fs source-id "utf8")}))

(defn load-source [parent-source-id requested-path]
  (let [candidate (.resolve node-path
                            (.dirname node-path parent-source-id)
                            requested-path)
        source-id (canonical-path candidate)]
    {:source-id source-id
     :source (.readFileSync fs source-id "utf8")}))

(defn context [source-id]
  {:source-id source-id
   :load-source load-source})

(defn eval-context []
  (context (.resolve node-path (.cwd js/process) "__pnix_eval__.px")))
