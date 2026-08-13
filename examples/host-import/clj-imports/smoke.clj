(ns smoke
  "Multi-module host-main demo: eval-source-with-imports loads sibling .px files
  as an in-memory module map (no FS import resolver beyond what we feed it).

  Run from this directory:
    clojure -M -m smoke
  Expect: 3"
  (:require [clojure.java.io :as io]
            [pnix-clj.core :as c]))

(defn- slurp-px
  [name]
  (slurp (io/file name)))

(defn -main
  [& _]
  (let [modules {"./lib.px" (slurp-px "lib.px")}
        result (c/eval-source-with-imports (slurp-px "main.px") modules)]
    (when-not (= :ok (:status result))
      (binding [*out* *err*]
        (println "eval-source-with-imports failed:" (pr-str result)))
      (System/exit 1))
    (println (:value result))))
