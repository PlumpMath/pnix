(ns demo.main
  "Entry namespace: requires demo.lib and prints (add 20 22) => 42."
  (:require [demo.lib :as L]))

(defn -main
  [& _]
  (println (L/add 20 22)))
