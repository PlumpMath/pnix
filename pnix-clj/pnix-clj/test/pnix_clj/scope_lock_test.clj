(ns pnix-clj.scope-lock-test
  (:require [clojure.java.io :as io]
            [clojure.string :as str]
            [clojure.test :refer [deftest is testing]]))

(def forbidden-core-lane-tokens
  ["hangul"
   "ko-dictionary"
   "korean-mirror"
   "domain-token"
   "domain-generic"
   "gate-graph"
   "gate_graph"
   "langs/"
   "tick-runner"
   "tick_runner"
   "puck-cli"
   "puck_cli"
   "redb-ingest"
   "meaning-graph"
   "msv"])

(defn regular-source-file? [f]
  (and (.isFile f)
       (let [p (.getPath f)]
         (or (str/ends-with? p ".clj")
             (str/ends-with? p ".cljc")
             (str/ends-with? p ".px")
             (str/ends-with? p ".edn")))))

(defn scan-files [root]
  (let [dir (io/file root)]
    (if (.exists dir)
      (filter regular-source-file? (file-seq dir))
      [])))

(deftest scope-lock-forbids-nl-emit-side-lanes-in-core
  (testing "src stays pure meta-circular pnix-clj core"
    (let [hits
          (vec
           (for [f (mapcat scan-files ["src"])
                 :let [path (.getPath f)
                       text (slurp f)]
                 token forbidden-core-lane-tokens
                 :when (str/includes? text token)]
             {:file path :token token}))]
      (is (= [] hits)))))
