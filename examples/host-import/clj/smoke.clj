(ns smoke
  "최소 host-main import: pnix-clj classpath 주입 + eval-file.
  이 디렉터리에서: clojure -M -m smoke"
  (:require [clojure.java.io :as io]
            [pnix-clj.core :as c]))

(defn -main
  [& _]
  (let [px (.getCanonicalPath (io/file ".." "hello.px"))
        result (c/eval-file px)]
    (when-not (= :ok (:status result))
      (binding [*out* *err*]
        (println "eval-file 실패:" (pr-str result)))
      (System/exit 1))
    (println (:value result))))
