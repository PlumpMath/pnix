(ns pnix.clj-meta.nrepl
  "An nREPL server whose `eval` op is routed through clj-meta's OWN
  meta-circular bytecode compiler (pnix.clj-meta.compiler/eval-string), NOT
  stock clojure.lang.Compiler.

  This makes clj-meta an editor-drivable independent evaluator: connect any
  nREPL client (CIDER, Calva, Conjure, `nrepl.core`) and every form you send is
  compiled to JVM bytecode by clj-meta's backend. Non-eval ops (clone/describe/
  close/...) are delegated to nREPL's default handler so sessions work normally.

  Honest boundary: only the `eval` op is re-routed; clj-meta's compiler covers
  its self-host Clojure subset (the same surface as `-M:repl`). A form outside
  that subset surfaces clj-meta's own error, not a silent stock-compiler
  fallback -- the routing is genuine."
  (:require [nrepl.middleware :refer [set-descriptor!]]
            [nrepl.misc :refer [response-for]]
            [nrepl.server :as server]
            [nrepl.transport :as transport]
            [pnix.clj-meta.compiler :as compiler]))

(defn wrap-clj-meta-eval
  "Middleware: handle the `eval` op via clj-meta's compiler; delegate the rest."
  [h]
  (fn [{:keys [op code transport] :as msg}]
    (if (and (= op "eval") code)
      (do
        (try
          (let [v (compiler/eval-string code)]
            (transport/send transport
                            (response-for msg :value (pr-str v) :ns "user")))
          (catch Throwable e
            (transport/send transport
                            (response-for msg :err (str "clj-meta: " (.getMessage e))))))
        (transport/send transport (response-for msg :status #{:done})))
      (h msg))))

(set-descriptor! #'wrap-clj-meta-eval
  {:requires #{}
   :expects #{}
   :handles {"eval" {:doc "Evaluate code via clj-meta's meta-circular compiler."
                     :requires {"code" "The Clojure code to compile+run via clj-meta."}
                     :returns {"value" "The pr-str of the result."}}}})

(defn start!
  "Start the clj-meta-backed nREPL server on `port` (blocks)."
  [port]
  (let [srv (server/start-server
             :port port
             :handler (wrap-clj-meta-eval (server/default-handler)))]
    (println (str "clj-meta nREPL (backend = clj-meta bytecode compiler) on port "
                   port " — nrepl://127.0.0.1:" port))
    (flush)
    srv))

(defn -main
  [& args]
  (start! (if (seq args) (Integer/parseInt (first args)) 7889))
  @(promise))                                           ; block forever
