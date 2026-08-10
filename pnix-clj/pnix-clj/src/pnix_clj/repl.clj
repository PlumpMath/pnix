(ns pnix-clj.repl
  "The pnix-clj PNIX lane runner/REPL -- pnix-clj is an INDEPENDENT runner for
  the pnix language (a Nix-like lazy pure functional language), analogous to
  pnix-hy's pnix lane.

  Modes, like a language launcher (and Nix's default.nix convention):
    pnix-clj-pnix <file.px>     evaluate a .px file, print its value
    pnix-clj-pnix -e '1 + 2'    evaluate an expression, print its value
    pnix-clj-pnix --server [p]  socket server evaluating pnix (network REPL)
    pnix-clj-pnix               eval ./default.px if present, else interactive

  Evaluation goes through pnix-clj.core/eval-source (the direct pnix evaluator).
  For a cross-substrate verified run, use `-M:tower` instead.

  These runners are Clojure-hosted Clojure extensions: they can grow their own
  deps.edn and be started as a network/nREPL server (--server) for editors."
  (:require [clojure.core.server :as server]
            [clojure.java.io :as io]
            [clojure.string :as str]
            [pnix-clj.core :as core]))

(def lane-classification
  {:lane :experimental
   :scope :developer-repl-entrypoint
   :role :manual-pnix-language-runner
   :product-runtime :forbidden
   :semantic-authority :forbidden
   :network :loopback-dev-only
   :mutation :io-only
   :admission :forbidden
   :allowed-output :rendered-repl-value-or-error})

(def ^:private default-entry
  "Nix's default.nix convention, for pnix: a bare invocation evaluates the
  project entry file if one is present (default.px preferred, then default.nix)."
  ["default.px" "default.nix"])

(declare render)

(defn- render-attrset
  [m]
  (str "{ "
       (->> (sort-by key m)
            (map (fn [[k v]] (str k " = " (render v) ";")))
            (str/join " "))
       " }"))

(defn render
  "Render a pnix value compactly (closures/builtins as opaque markers, never the
  captured environment; attrsets/lists structurally)."
  [v]
  (cond
    (nil? v)                         "null"
    (boolean? v)                     (str v)
    (string? v)                      (pr-str v)
    (number? v)                      (str v)
    (and (map? v) (= :closure (:kind v))) "«lambda»"
    (and (map? v) (= :builtin (:kind v))) (str "«builtin " (name (:name v)) "»")
    (and (map? v) (:kind v))         (str "«" (name (:kind v)) "»")
    (map? v)                         (render-attrset v)
    (vector? v)                      (str "[ " (str/join " " (map render v)) " ]")
    (seq? v)                         (str "[ " (str/join " " (map render v)) " ]")
    :else                            (pr-str v)))

(defn eval-print
  "Evaluate one pnix source string and print the value or the error. Returns the
  result status."
  [src]
  (let [{:keys [status value error] :as r} (core/eval-source src)]
    (if (= :ok status)
      (println (render value))
      (println "error:" (or (:message error) (pr-str (or error (:status r))))))
    status))

(defn repl-loop
  "A read-eval-print loop over the current *in*/*out* (used both for the
  interactive terminal and for each accepted socket connection)."
  [prompt?]
  (loop []
    (when prompt? (print "pnix> ") (flush))
    (when-let [line (read-line)]
      (let [t (str/trim line)]
        (when-not (#{":q" ":quit" ":exit"} t)
          (when (seq t)
            (try (eval-print line)
                 (catch Throwable e (println "!" (.getMessage e)) (flush))))
          (recur))))))

(defn- interactive!
  []
  (println "pnix-clj — pnix REPL. :q to quit.")
  (repl-loop true))

(defn socket-accept
  "clojure.core.server accept fn: a pnix REPL over the socket streams."
  []
  (repl-loop false))

(defn start-server!
  "Start a socket server evaluating pnix (a network REPL / server seam that can
  later host an nREPL transport). Blocks the calling thread."
  [port]
  (server/start-server {:name "pnix" :address "127.0.0.1" :port port
                        :accept `socket-accept})
  (println (str "pnix socket server listening on 127.0.0.1:" port
                " — connect and send pnix expressions (e.g. `nc 127.0.0.1 " port "`)."))
  (flush)
  @(promise))                                           ; block forever

(defn- entry-file
  "The default project entry file present in the CWD, or nil."
  []
  (first (filter #(.exists (io/file %)) default-entry)))

(defn -main
  [& args]
  (cond
    (and (seq args) (or (str/ends-with? (first args) ".px")
                        (str/ends-with? (first args) ".nix")))
    (System/exit (if (= :ok (eval-print (slurp (first args)))) 0 1))

    (and (seq args) (= "-e" (first args)))
    (System/exit (if (= :ok (eval-print (str/join " " (rest args)))) 0 1))

    (and (seq args) (#{"--server" "--nrepl"} (first args)))
    (start-server! (if (second args) (Integer/parseInt (second args)) 7888))

    (seq args)
    (System/exit (if (= :ok (eval-print (str/join " " args))) 0 1))

    ;; bare invocation: Nix default.nix convention -> eval ./default.px if present
    (entry-file)
    (do (println (str ";; evaluating " (entry-file)))
        (System/exit (if (= :ok (eval-print (slurp (entry-file)))) 0 1)))

    :else
    (do (interactive!) (shutdown-agents))))
