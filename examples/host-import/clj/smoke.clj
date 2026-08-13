(ns smoke
  "Minimal host-main import: classpath inject of pnix-clj + eval-file.
  Run from this directory: clojure -M -m smoke"
  (:require [clojure.java.io :as io]
            [pnix-clj.core :as c]))

(defn -main
  [& _]
  (let [px (.getCanonicalPath (io/file ".." "hello.px"))
        result (c/eval-file px)]
    (when-not (= :ok (:status result))
      (binding [*out* *err*]
        (println "eval-file failed:" (pr-str result)))
      (System/exit 1))
    (println (:value result))))
