(ns pnix-clr.main
  (:require [clojure.string :as str]
            [pnix-clr.evaluator :as evaluator]
            [pnix-clr.host :as host]
            [pnix-clr.json :as json]
            [pnix-clr.outcome :as outcome]
            [pnix-clr.production-outcome :as production-outcome]))

(defn usage! []
  (binding [*out* *err*]
    (println
     (str "usage: pnix-clr -e SOURCE | FILE.px"
          " | --production-outcome-self-check"
          " | --production-outcome CASES.tsv"))))

(defn projection
  [result]
  (cond-> {"schema" "pnix-clr.cli-result.v1"
           "host" "pnix-clr"
           "outcome_kind" (name (outcome/kind result))}
    (outcome/done? result) (assoc "value" (outcome/value-of result))
    (outcome/failed? result) (assoc "error" (outcome/error-of result))))

(defn- path-inside?
  [root path]
  (let [relative (-> (System.IO.Path/GetRelativePath
                      (host/canonical-path root)
                      (host/canonical-path path))
                     (str/replace "\\" "/"))]
    (not (or (= relative "..")
             (str/starts-with? relative "../")
             (System.IO.Path/IsPathRooted relative)))))

(defn- file-root
  [base-root file]
  (if (path-inside? base-root file)
    base-root
    (System.IO.Path/GetDirectoryName (host/canonical-path file))))

(defn- print-result!
  [result]
  (println (json/write-json (projection result)))
  (when (outcome/failed? result)
    (host/exit! 1)))

(defn -main
  [& args]
  (let [args (vec args)
        host-root (host/default-root)]
    (cond
      (and (= 2 (count args)) (contains? #{"-e" "--eval"} (first args)))
      (print-result!
       (evaluator/eval-source
        (second args)
        {:root host-root
         :file (host/combine host-root "pnix-clr-inline.px")}))

      (= ["--production-outcome-self-check"] args)
      (production-outcome/-main "--self-check")

      (and (= 2 (count args))
           (= "--production-outcome" (first args)))
      (production-outcome/-main (second args))

      (= 1 (count args))
      (let [file (host/canonical-path (first args))
            root (file-root host-root file)]
        (print-result! (evaluator/eval-file root file)))

      :else
      (do
        (usage!)
        (host/exit! 2)))))
