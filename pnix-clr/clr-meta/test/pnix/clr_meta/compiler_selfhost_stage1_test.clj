(ns pnix.clr-meta.compiler-selfhost-stage1-test
  (:require [clojure.edn :as edn]
            [clojure.string :as str]
            [clojure.test :refer [deftest is run-tests testing]]
            [pnix.clr-meta.compiler-selfhost-stage1 :as stage1]))

(defn- source-root
  []
  (let [load-path (or (System.Environment/GetEnvironmentVariable
                       "CLOJURE_LOAD_PATH")
                      "")
        separator (System.Text.RegularExpressions.Regex/Escape
                   (str System.IO.Path/PathSeparator))
        candidates (remove str/blank? (str/split load-path
                                                  (re-pattern separator)))
        root (first
              (filter
               (fn [candidate]
                 (System.IO.File/Exists
                  (System.IO.Path/Combine
                   candidate "pnix" "clr_meta"
                   "compiler_selfhost_stage1.clj")))
               candidates))]
    (when-not root
      (throw (ex-info "clr-meta source root is absent from CLOJURE_LOAD_PATH"
                      {:class :test-source-root-missing})))
    (System.IO.Path/GetFullPath root)))

(defn- clr-meta-root
  []
  (.FullName (System.IO.Directory/GetParent (source-root))))

(defn- read-edn
  [path]
  (edn/read-string
   {:readers {} :default (fn [tag value] [tag value])}
   (System.IO.File/ReadAllText path (System.Text.UTF8Encoding. false true))))

(defn- executable-contract
  []
  (read-edn
   (System.IO.Path/Combine
    (clr-meta-root) "compiler-selfhost" "executable-contract.edn")))

(defn- source-contract
  []
  (read-edn
   (System.IO.Path/Combine
    (clr-meta-root) "compiler-selfhost" "contract.edn")))

(defn- bootstrap-mapping
  []
  (->> stage1/support-spec
       (mapcat
        (fn [[namespace entries]]
          (map (fn [[name [type method arity]]]
                 {:symbol (symbol (str namespace) (str name))
                  :type type
                  :method method
                  :arity arity})
               entries)))
       (sort-by (comp str :symbol))
       vec))

(deftest executable-contract-freezes-the-generated-object-abi
  (let [contract (executable-contract)]
    (is (= :pnix.clr-meta.compiler-selfhost-executable-contract.v1
           (:schema contract)))
    (is (= :c2-executable-compiler-stage1 (:checkpoint contract)))
    (is (= {:attributes [:public :static]
            :return :system-object
            :parameters :system-object-fixed-arity
            :max-arity 5}
           (get-in contract [:generated-abi :method-shape])))
    (is (= :system-object (get-in contract [:generated-abi :local-type])))
    (is (= [:system-null :boxed-system-false]
           (get-in contract [:expression-semantics :truthiness :false-values])))
    (is (false? (get-in contract [:reader :keywords])))
    (is (= 5 (get-in contract [:reader :max-parameters])))
    (is (= 64 (get-in contract [:reader :max-bindings-per-let])))
    (is (= :reject
           (get-in contract [:environment :shadowing :duplicate-parameter])))
    (is (= :reject
           (get-in contract [:environment :shadowing
                             :local-over-argument-or-local])))
    (is (= :same-directory-temporary-then-atomic-rename
           (get-in contract [:pesink :publication])))
    (is (= :every-emission-and-join
           (get-in contract [:pesink :stack-height-verification])))
    (is (false? (get-in contract [:claims :stage2])))
    (is (false? (get-in contract [:claims :self_reproduction])))))

(deftest all-source-support-calls-have-one-exact-clr-target
  (let [source-rows (->> (get-in (source-contract) [:support-abi :calls])
                         (map #(select-keys % [:symbol :arity]))
                         (sort-by (comp str :symbol))
                         vec)
        executable-rows (->> (:clr-map (executable-contract))
                             (sort-by (comp str :symbol))
                             vec)]
    (is (= 33 (count source-rows)))
    (is (= 33 (count executable-rows)))
    (is (= source-rows
           (mapv #(select-keys % [:symbol :arity]) executable-rows)))
    (is (= executable-rows (bootstrap-mapping)))
    (is (= 33 (count (set (map :symbol executable-rows)))))
    (is (= 33 (count (set (map (juxt :type :method :arity)
                                executable-rows)))))))

(deftest bootstrap-boundary-is-explicit-and-does-not-promote-stage2
  (let [contract (executable-contract)]
    (testing "host Compiler/load is seed-only"
      (is (= :seed-only (get-in contract [:bootstrap :host-compiler-use])))
      (is (= :exact-canonical-source-seed-only
             (get-in contract [:bootstrap :host-load-use])))
      (is (false? (get-in contract [:bootstrap :generated-artifact-host-compiler-use])))
      (is (false? (get-in contract [:bootstrap :generated-artifact-host-source-load-use])))
      (is (false? (get-in contract [:bootstrap :generated-artifact-clojureclr-reference]))))
    (testing "C2 closes only the generated Stage1 artifact"
      (is (= :gate-required (get-in contract [:claims :stage1_artifact])))
      (doseq [claim [:stage2 :self_reproduction :fixed_point
                     :raw_reproducibility :clojureclr_replacement]]
        (is (false? (get-in contract [:claims claim])))))))

(deftest public-builder-requires-admission-and-no-replace-publication
  (let [contract (executable-contract)]
    (is (= :mandatory-before-bootstrap-b0
           (get-in contract [:builder :c1-admission])))
    (is (= :clear-then-allowlist
           (get-in contract [:builder :environment])))
    (is (= :all-regular-files-hashed-before-and-after
           (get-in contract [:builder :bootstrap-runtime-closure])))
    (is (= :same-filesystem-directory-move-no-replace
           (get-in contract [:builder :publication])))
    (is (= [:file-preserve :directory-preserve :symlink-preserve]
           (get-in contract [:builder :existing-output])))
    (is (true? (get-in contract [:required-evidence
                                 :fresh-post-stage1-nonce-target])))))

(defn -main
  [& _]
  (let [{:keys [fail error]}
        (run-tests 'pnix.clr-meta.compiler-selfhost-stage1-test)]
    (when (pos? (+ fail error))
      (System.Environment/Exit 1))))
