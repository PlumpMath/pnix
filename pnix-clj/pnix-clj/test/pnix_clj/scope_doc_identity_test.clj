(ns pnix-clj.scope-doc-identity-test
  (:require [clojure.string :as str]
            [clojure.test :refer [deftest is testing]]))

(defn- doc [path]
  (slurp path))

(defn- has-all? [text xs]
  (every? #(str/includes? text %) xs))

(deftest scope-documents-preserve-identity
  (testing "README and SCOPE_LOCK keep the core boundary"
    (is (has-all? (doc "README.md")
                  ["Clojure-hosted pnix" "NL/MSV/Hangul" "out of scope"]))
    (is (has-all? (doc "SCOPE_LOCK.md")
                  ["Clojure-hosted pnix runtime"
                   "meta-circular witness substrate"
                   "Hangul codec"
                   "MSV"
                   "gate-graph"
                   "multi-language emit registry"
                   "tick runner"])))

  (testing "LANE_CLASSIFICATION keeps current identity lock"
    (is (has-all? (doc "LANE_CLASSIFICATION.md")
                  ["Current identity lock addendum"
                   "CORE: 38"
                   "EXPERIMENTAL: 6"
                   "PROOF-ONLY: 26"
                   "TOTAL: 70"
                   "interop"
                   "Clojure runtime"
                   "pnix runtime"
                   "nREPL"
                   "meta-circular interactive control surface"
                   "wiki"
                   "self-documenting capability"
                   "not disposable dev-only surfaces"
                   "QUARANTINE"
                   "Hangul codec"
                   "MSV / meaning sentence variants"
                   "graph-gate / gate-graph"
                   "multi-language emit registry"
                   "puck-cli bridge"
                   "tick runner"
                   "redb ingest brain"])))

  (testing "generated lane registry exposes current core surfaces"
    (is (has-all? (doc "docs/LANE_REGISTRY.md")
                  ;; M7 added pnix-clj.machine (proof-only): 27 -> 28
              ["`:core`: 44"
                   "`:experimental`: 7"
                   "`:proof-only`: 28"
                   "pnix-clj.interop"
                   "meta-circular-runtime-interop-boundary"
                   "pnix-clj.nrepl"
                   "meta-circular-interactive-control-surface"
                   "pnix-clj.wiki"
                   "self-documenting-capability-and-roadmap-substrate"
                   "pnix-clj.lane-registry"]))))
