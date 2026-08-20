(ns pnix-cljs.main
  (:require [clojure.string :as string]
            [cljs.nodejs :as nodejs]
            [pnix-cljs.capabilities :as capabilities]
            [pnix-cljs.core :as core]
            [pnix-cljs.evaluator :as evaluator]
            [pnix-cljs.node-loader :as node-loader]))

(nodejs/enable-util-print!)

(defn usage! []
  (binding [*print-fn* *print-err-fn*]
    (println "usage: pnix-cljs -e SOURCE | pnix-cljs FILE | pnix-cljs --repl | pnix-cljs capabilities | pnix-cljs capabilities-check")))

(defn render
  "Render a PNIX value in Nix surface notation, the same shape the other hosts'
  pnix REPLs print, so a value reads identically whichever host evaluated it.
  Non-transparent values render as an opaque marker rather than exposing a host
  representation."
  [v]
  (cond
    (evaluator/integer-value? v) (.toString v)
    (nil? v) "null"
    (true? v) "true"
    (false? v) "false"
    (string? v) (js/JSON.stringify v)
    (number? v) (str v)
    (map? v) (str "{ "
                  (->> (sort-by key v)
                       (map (fn [[k x]] (str k " = " (render x) ";")))
                       (string/join " "))
                  " }")
    (sequential? v) (str "[ " (string/join " " (map render v)) " ]")
    :else "«opaque»"))

(defn- repl-eval-print!
  "Evaluate one REPL line through the same `core/projection` the batch modes
  use, then print the value in surface notation or the structured error."
  [source]
  (let [projection (core/projection source (node-loader/eval-context))]
    (if (= "failed" (get projection "outcome_kind"))
      (println "error:" (core/canonical-json (get projection "error")))
      (println (render (get projection "value"))))))

(defn- repl!
  "Interactive pnix REPL over Node's readline. One expression per line; `:q` or
  EOF ends it. This is a developer entry point and never becomes an evaluation
  authority — every line goes through the ordinary evaluator."
  []
  (let [readline (nodejs/require "readline")
        rl (.createInterface readline
                             #js {:input (.-stdin js/process)
                                  :output (.-stdout js/process)
                                  :prompt "pnix> "})]
    (println "pnix-cljs — pnix REPL. :q to quit.")
    (.prompt rl)
    (.on rl "line"
         (fn [line]
           (let [trimmed (string/trim line)]
             (if (contains? #{":q" ":quit" ":exit"} trimmed)
               (.close rl)
               (do
                 (when (seq trimmed)
                   (try
                     (repl-eval-print! trimmed)
                     (catch :default cause
                       (println "!" (.-message cause)))))
                 (.prompt rl))))))
    (.on rl "close" (fn [] (set! (.-exitCode js/process) 0)))))

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
  (let [args (vec argv)]
    (cond
      (= ["--repl"] args) (repl!)

      (= ["capabilities"] args)
      (.write (.-stdout js/process) (capabilities/capabilities-doc))

      (= ["capabilities-check"] args)
      (let [{:keys [pass message]} (capabilities/capabilities-check)]
        (println "[capabilities-check (docs drift gate)]")
        (println message)
        (when-not pass
          (set! (.-exitCode js/process) 1)))

      :else
      (if-let [{:keys [source module-context]} (source-from args)]
        (let [projection (core/projection source module-context)]
          (println (core/canonical-json projection))
          (when (= "failed" (get projection "outcome_kind"))
            (set! (.-exitCode js/process) 1)))
        (do
          (usage!)
          (set! (.-exitCode js/process) 2))))))

(set! *main-cli-fn* -main)
