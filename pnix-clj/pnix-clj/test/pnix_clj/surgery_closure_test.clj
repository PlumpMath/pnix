(ns pnix-clj.surgery-closure-test
  (:require [clojure.string :as str]
            [clojure.test :refer [deftest is testing]]
            [pnix-clj.capabilities :as capabilities]
            [pnix-clj.lane-registry :as lane-registry]))

(defn- doc [path]
  (slurp path))

(defn- has-all? [text xs]
  (every? #(str/includes? text %) xs))

(deftest surgery-closure-documents-exist
  (testing "closure-critical documents exist"
    (doseq [path ["README.md"
                 "SCOPE_LOCK.md"
                 "LANE_CLASSIFICATION.md"
                 "docs/CAPABILITIES.md"
                 "docs/LANE_REGISTRY.md"]]
      (is (.exists (clojure.java.io/file path)) path))))

(deftest surgery-closure-generated-docs-are-current
  (testing "generated docs are not drifting"
    (is (= :ok (:status (capabilities/check))))
    (is (= :ok (:status (lane-registry/check)))))
  (testing "lane registry count is the current locked surface"
    (let [{:keys [row-count counts]} (lane-registry/check)]
      ;; F8 added pnix-clj.weval (proof-only): 71 -> 72
      ;; M7 added pnix-clj.machine (proof-only, the derived abstract machine): 72 -> 73
      ;; Common machine outcome and IO host adapters: 74 -> 76.
      ;; The production checked-i64 kernel is a closed core lane: 76 -> 77.
      ;; The direct clj-meta host executor is core: 77 -> 78.
      ;; The explicit filesystem convenience boundary is core: 78 -> 79.
      ;; The redb adapter remains outside the clean R1 extraction.
      (is (= 79 row-count))
      (is (= {:core 44 :experimental 7 :proof-only 28} counts)))))

(deftest surgery-closure-tests-are-wired
  (testing "all scope/lane identity gates are wired into the test runner"
    (let [runner (doc "test/pnix_clj/test_runner.clj")]
      (is (has-all?
           runner
           ["pnix-clj.scope-lock-test"
            "pnix-clj.lane-policy-test"
            "pnix-clj.lane-identity-test"
            "pnix-clj.scope-doc-identity-test"])))))

(deftest surgery-closure-scope-boundary-is-visible
  (testing "core scope and quarantine boundary remain visible"
    (let [scope-lock (doc "SCOPE_LOCK.md")
          lane-doc (doc "LANE_CLASSIFICATION.md")
          readme (doc "README.md")]
      (is (has-all?
           scope-lock
           ["Clojure-hosted pnix runtime"
            "meta-circular witness substrate"
            "Hangul codec"
            "MSV"
            "gate-graph"
            "multi-language emit registry"
            "tick runner"]))
      (is (has-all?
           lane-doc
           ["CORE: 38"
            "EXPERIMENTAL: 6"
            "PROOF-ONLY: 26"
            "TOTAL: 70"
            "not disposable dev-only surfaces"
            "QUARANTINE"]))
      (is (has-all?
           readme
           ["Clojure-hosted pnix"
            "NL/MSV/Hangul"
            "out of scope"])))))
