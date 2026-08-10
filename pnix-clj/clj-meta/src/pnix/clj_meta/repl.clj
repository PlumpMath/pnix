(ns pnix.clj-meta.repl
  "The clj-meta CLOJURE lane runner/REPL -- clj-meta is an INDEPENDENT
  meta-circular Clojure compiler/evaluator (Clojure-on-Clojure, compiling forms
  to JVM bytecode via its own backend), analogous to hy-meta's hy lane.

  Every form typed here is evaluated THROUGH clj-meta's own compiler
  (pnix.clj-meta.compiler/eval-string), NOT stock clojure.lang.Compiler.

  Dual mode:
    clj-meta <file.clj>       evaluate a Clojure file through clj-meta
    clj-meta -e '(+ 1 2)'     evaluate an expression through clj-meta
    clj-meta                  interactive REPL (clj-meta> )"
  (:require [clojure.string :as str]
            [pnix.clj-meta.compiler :as compiler]))

(defn eval-print
  "Evaluate a Clojure source string through clj-meta and print the value."
  [src]
  (try
    (println (pr-str (compiler/eval-string src)))
    :ok
    (catch Throwable e
      (println "clj-meta error:" (.getMessage e))
      :error)))

(defn- interactive!
  []
  (println "clj-meta — meta-circular Clojure REPL (bytecode backend). :q to quit.")
  (loop []
    (print "clj-meta> ") (flush)
    (when-let [line (read-line)]
      (let [t (str/trim line)]
        (when-not (#{":q" ":quit" ":exit"} t)
          (when (seq t)
            (try (eval-print line)
                 (catch Throwable e (println "!" (.getMessage e)))))
          (recur))))))

(defn -main
  [& args]
  (cond
    (and (seq args) (or (str/ends-with? (first args) ".clj")
                        (str/ends-with? (first args) ".cljc")))
    (System/exit (if (= :ok (eval-print (slurp (first args)))) 0 1))

    (and (seq args) (= "-e" (first args)))
    (System/exit (if (= :ok (eval-print (str/join " " (rest args)))) 0 1))

    (seq args)
    (System/exit (if (= :ok (eval-print (str/join " " args))) 0 1))

    :else
    (do (interactive!) (shutdown-agents))))
