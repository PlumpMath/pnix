(ns pnix-clj.nrepl
  "An nREPL server whose `eval` op evaluates the PNIX language (via
  pnix-clj.core/eval-source), mirroring pnix.clj-meta.nrepl for the Clojure
  lane. This makes the pnix lane editor-drivable: connect any nREPL client and
  every message you send is parsed + evaluated as pnix, the value rendered
  compactly (closures as «lambda», attrsets/lists structurally).

  Non-eval ops (clone/describe/close/...) are delegated to nREPL's default
  handler so sessions work normally. Honest boundary: only `eval` is re-routed
  to pnix; a parse/eval error surfaces as an nREPL :err, never a silent Clojure
  fallback."
  (:require [nrepl.middleware :refer [set-descriptor!]]
            [nrepl.misc :refer [response-for]]
            [nrepl.server :as server]
            [nrepl.transport :as transport]
            [pnix-clj.core :as core]
            [pnix-clj.repl :as repl]))

(def lane-classification
  {:lane :core
   :scope :meta-circular-interactive-control-surface
   :role :editor-driven-pnix-and-clj-meta-interaction-gateway
   :product-runtime :allowed
   :semantic-authority :eval-routes-through-core-only
   :network :loopback-or-explicit-dev-server
   :mutation :server-process-only
   :admission :forbidden
   :interop :capability-gated
   :allowed-output :interactive-eval-session})

(defn wrap-pnix-eval
  "Middleware: handle the `eval` op by evaluating pnix; delegate the rest."
  [h]
  (fn [{:keys [op code transport] :as msg}]
    (if (and (= op "eval") code)
      (do
        (let [{:keys [status value error]} (core/eval-source code)]
          (if (= :ok status)
            (transport/send transport
                            (response-for msg :value (repl/render value) :ns "pnix"))
            (transport/send transport
                            (response-for msg :err (str "pnix: "
                                                        (or (:message error)
                                                            (pr-str (or error status))))))))
        (transport/send transport (response-for msg :status #{:done})))
      (h msg))))

(set-descriptor! #'wrap-pnix-eval
  {:requires #{}
   :expects #{}
   :handles {"eval" {:doc "Evaluate a pnix expression via pnix-clj.core/eval-source."
                     :requires {"code" "The pnix source to evaluate."}
                     :returns {"value" "The rendered pnix value."}}}})

(defn start!
  "Start the pnix-lane nREPL server on `port` (returns the server)."
  [port]
  (let [srv (server/start-server
             :port port
             :handler (wrap-pnix-eval (server/default-handler)))]
    (println (str "pnix-clj nREPL (pnix language lane) on port " port
                   " — nrepl://127.0.0.1:" port))
    (flush)
    srv))

(defn -main
  [& args]
  (start! (if (seq args) (Integer/parseInt (first args)) 7890))
  @(promise))                                           ; block forever
