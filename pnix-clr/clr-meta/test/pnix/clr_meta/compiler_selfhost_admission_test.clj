(ns pnix.clr-meta.compiler-selfhost-admission-test
  (:require [clojure.edn :as edn]
            [clojure.string :as str]
            [clojure.test :refer [deftest is run-tests testing]]
            [pnix.clr-meta.compiler-selfhost-admission :as admission]
            [pnix.clr-meta.runtime-artifact :as artifact]))

(declare clr-meta-root)

(defn- temp-directory
  []
  (let [path (System.IO.Path/Combine
              (clr-meta-root) "work"
              (str "pnix-clr-meta-selfhost-admission-test-"
                   (.ToString (System.Guid/NewGuid) "N")))]
    (System.IO.Directory/CreateDirectory path)
    path))

(defn- delete-tree!
  [path]
  (when (System.IO.Directory/Exists path)
    (System.IO.Directory/Delete path true)))

(defn- write-text!
  [path text]
  (let [parent (System.IO.Path/GetDirectoryName path)]
    (when (and parent (not= "" parent))
      (System.IO.Directory/CreateDirectory parent))
    (System.IO.File/WriteAllText
     path text (System.Text.UTF8Encoding. false true))))

(defn- read-text
  [path]
  (System.IO.File/ReadAllText path (System.Text.UTF8Encoding. false true)))

(defn- failure-data
  [f]
  (try
    (f)
    nil
    (catch clojure.lang.ExceptionInfo cause
      (ex-data cause))))

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
                   "compiler_selfhost_admission.clj")))
               candidates))]
    (when-not root
      (throw (ex-info "clr-meta source root is absent from CLOJURE_LOAD_PATH"
                      {:class :test-source-root-missing
                       :load-path load-path})))
    (System.IO.Path/GetFullPath root)))

(defn- clr-meta-root
  []
  (.FullName (System.IO.Directory/GetParent (source-root))))

(defn- canonical-paths
  []
  (let [root (System.IO.Path/Combine (clr-meta-root) "compiler-selfhost")]
    {:contract (System.IO.Path/Combine root "contract.edn")
     :profile (System.IO.Path/Combine root "profile.edn")
     :plan (System.IO.Path/Combine root "plan.edn")
     :source (System.IO.Path/Combine root "compiler_kernel.clj")}))

(def ^:private input-file-names
  {:contract "contract.edn"
   :profile "profile.edn"
   :plan "plan.edn"
   :source "compiler_kernel.clj"})

(defn- copy-inputs!
  [paths root]
  (reduce-kv
   (fn [copied kind source]
     (let [target (System.IO.Path/Combine root (get input-file-names kind))]
       (write-text! target (read-text source))
       (assoc copied kind target)))
   {}
   paths))

(defn- analyze-paths
  [{:keys [contract profile plan source]}]
  (admission/analyze! contract profile plan source))

(defn- mutate-input!
  [paths kind transform]
  (let [path (get paths kind)]
    (write-text! path (transform (read-text path)))
    paths))

(defn- replace-exactly-once
  [text before after]
  (let [first-index (str/index-of text before)
        last-index (str/last-index-of text before)]
    (when (or (nil? first-index) (not= first-index last-index))
      (throw (ex-info "test mutation anchor must occur exactly once"
                      {:class :test-mutation-anchor-mismatch
                       :anchor before
                       :first-index first-index
                       :last-index last-index})))
    (str (subs text 0 first-index)
         after
         (subs text (+ first-index (count before))))))

(defn- mutation-paths
  [root label kind transform]
  (let [directory (System.IO.Path/Combine root label)
        paths (copy-inputs! (canonical-paths) directory)]
    (mutate-input! paths kind transform)))

(defn- assert-rejected!
  [root label paths expected-class]
  (let [data (failure-data #(analyze-paths paths))
        output (System.IO.Path/Combine root (str label ".receipt.json"))
        stdout (System.IO.StringWriter.)
        cli-data (binding [*out* stdout]
                   (failure-data
                    #(admission/-main (:contract paths)
                                      (:profile paths)
                                      (:plan paths)
                                      (:source paths)
                                      output)))]
    (is (map? data) (str label " must throw ExceptionInfo"))
    (is (= :pnix.clr-meta.compiler-selfhost-admission-error.v1
           (:schema data))
        (str label " must use the admission error schema: " (pr-str data)))
    (is (= :compiler-selfhost-admission (:phase data))
        (str label " must identify the admission phase: " (pr-str data)))
    (is (keyword? (:class data))
        (str label " must classify its rejection: " (pr-str data)))
    (is (= expected-class (:class data))
        (str label " used the wrong rejection class: " (pr-str data)))
    (is (= expected-class (:class cli-data))
        (str label " CLI used the wrong rejection class: "
             (pr-str cli-data)))
    (is (str/blank? (str stdout))
        (str label " emitted a partial receipt: " (str stdout)))
    (is (false? (System.IO.File/Exists output))
        (str label " published a failed receipt"))
    data))

(def ^:private source-limits-definition
  "(def source-limits-id \"pnix.clr-meta.compiler-kernel-source-limits.v1\")")

(def ^:private receipt-keys
  #{"accounting" "admitted" "checkpoint" "compiled_language_id"
    "compiler_executable" "compiler_self_reproduction"
    "compiler_stage1_artifact" "compiler_stage2" "compiler_stage3"
    "contract_id" "entry" "executable_stage" "family" "fixed_point"
    "il_fixed_point" "input_closure_sha256"
    "inputs" "lowering_coverage" "mutation_propagation" "mutation_sites"
    "nodes" "plan_id" "profile_id" "raw_artifact_reproducibility"
    "raw_reproducibility" "same_source_recompile_executed" "schema"
    "seed_bindings" "self_reproduction" "source_language_equals_compiled_language"
    "source_language_id" "source_namespace" "stage1_artifact" "stage2"})

(def ^:private false-receipt-keys
  ["compiler_executable" "compiler_self_reproduction"
   "compiler_stage1_artifact" "compiler_stage2" "compiler_stage3"
   "executable_stage" "fixed_point" "il_fixed_point" "mutation_propagation"
   "raw_artifact_reproducibility"
   "raw_reproducibility" "same_source_recompile_executed"
   "self_reproduction" "stage1_artifact" "stage2"])

(defn- read-edn-file
  [path]
  (edn/read-string (read-text path)))

(deftest canonical-source-is-completely-and-deterministically-admitted
  (let [paths (canonical-paths)
        receipt (analyze-paths paths)
        replay (analyze-paths paths)
        accounting (get receipt "accounting")
        nodes (get receipt "nodes")
        inputs (get receipt "inputs")
        contract (read-edn-file (:contract paths))
        profile (read-edn-file (:profile paths))
        plan (read-edn-file (:plan paths))]
    (testing "the canonical bytes produce one deterministic JSON-domain receipt"
      (is (= receipt replay))
      (is (= receipt-keys (set (keys receipt))))
      (is (= "pnix.clr-meta.compiler-selfhost-admission.v1"
             (get receipt "schema")))
      (is (true? (get receipt "admitted")))
      (is (= "c1-source-admission" (get receipt "checkpoint")))
      (is (= "pnix.clr-meta.compiler-kernel.v1" (get receipt "family")))
      (is (= "pnix.clr-meta.compiler-kernel-c0-c1.v1"
             (get receipt "contract_id")))
      (is (= "pnix.clr-meta.compiler-kernel-source.v1"
             (get receipt "profile_id")))
      (is (= "pnix.clr-meta.compiler-kernel-c0-c1-plan.v1"
             (get receipt "plan_id")))
      (is (= "pnix.clr-meta.compiler-kernel.v1"
             (get receipt "source_namespace")))
      (is (= "compile-source" (get receipt "entry"))))
    (testing "every input and the ordered closure are independently hash-bound"
      (is (= ["contract" "profile" "plan" "source"]
             (mapv #(get % "path") inputs)))
      (doseq [[kind row] (map vector [:contract :profile :plan :source] inputs)]
        (is (= (artifact/sha256-file (get paths kind))
               (get row "sha256"))))
      (is (= (artifact/closure-hash inputs)
             (get receipt "input_closure_sha256")))
      (is (re-matches #"[0-9a-f]{64}"
                      (get receipt "input_closure_sha256"))))
    (testing "all 2,237 recursively visited nodes are classified exactly once"
      (is (= 2237 (get accounting "nodes") (count nodes)))
      (is (= 37 (get accounting "top_level_forms")))
      (is (= 36 (get accounting "definitions")))
      (is (= 20 (get accounting "max_depth")))
      (is (= 2237 (reduce + (vals (get accounting "node_kinds")))))
      (is (= (count nodes) (count (distinct (map #(get % "path") nodes)))))
      (is (every? #(and (vector? (get % "path"))
                        (seq (get % "path"))
                        (not (str/blank? (get % "kind"))))
                  nodes))
      (doseq [key ["source_unknown_nodes" "source_rejected_nodes"
                   "source_interpreted_nodes" "source_opaque_nodes"
                   "source_opaque_payload_nodes"]]
        (is (zero? (get accounting key)) key)))
    (testing "source and compiled language identities are equal but unexecuted"
      (is (true? (get receipt "source_language_equals_compiled_language")))
      (is (= "pnix.clr-meta.compiler-kernel-source.v1"
             (get receipt "source_language_id")
             (get receipt "compiled_language_id")))
      (doseq [key false-receipt-keys]
        (is (false? (get receipt key)) key)))
    (testing "intrinsic and lowering ownership close over the same source family"
      (is (= [{:symbol '+ :arity 2 :opcode "add.ovf"
               :semantics :checked-system-int64-add}
              {:symbol '- :arity 2 :opcode "sub.ovf"
               :semantics :checked-system-int64-subtract}
              {:symbol '= :arity 2 :opcode "ceq"
               :semantics :closed-value-equality}
              {:symbol '< :arity 2 :opcode "clt"
               :semantics :system-int64-less-than}]
             (:intrinsics contract)))
      (is (= 33 (count (get-in contract [:support-abi :calls]))))
      (is (= {:source [:support-abi :calls]
              :contract "contract.edn"
              :exact true}
             (get-in profile [:calls :support-global])))
      (is (= ["ns" "def-constant" "def-fn" "fn*" "if" "let*" "do"
              "literal" "lexical-symbol" "global-symbol" "call" "intrinsic"]
             (mapv #(get % "construct")
                   (get receipt "lowering_coverage"))))
      (doseq [row (get receipt "lowering_coverage")]
        (is (true? (get row "owner_is_function")))
        (is (pos? (get row "observed_nodes")))
        (is (re-matches #"[0-9a-f]{64}" (get row "semantic_sha256"))))
      (is (= ["support-abi" "intrinsics"]
             (mapv #(get % "kind") (get receipt "seed_bindings"))))
      (is (= [33 4]
             (mapv #(count (get % "rows"))
                   (get receipt "seed_bindings"))))
      (is (= ["d81d4b447c625829ce65a04b863f27fae1c40cecbaf9c15d6c127ab3463f250b"
              "9c683349d160006d4a19b1d681db6f0847cf8574a056d5920a883f54762e1c89"]
             (mapv #(get % "semantic_sha256")
                   (get receipt "seed_bindings")))))
    (testing "C1 remains admission only; compiler generations start after bootstrap B0"
      (is (= [{:from :bootstrap-b0
               :input "compiler_kernel.clj"
               :language-id :pnix.clr-meta.compiler-kernel-source.v1
               :output :compiler-stage1-pe}
              {:from :compiler-stage1-pe
               :input "compiler_kernel.clj"
               :language-id :pnix.clr-meta.compiler-kernel-source.v1
               :output :compiler-stage2-pe}
              {:from :compiler-stage2-pe
               :input "compiler_kernel.clj"
               :language-id :pnix.clr-meta.compiler-kernel-source.v1
               :output :compiler-stage3-pe}]
             (:future-chain plan)))
      (is (= :same-language-canonical-source-admission
             (->> (:stages plan)
                  (filter #(= :c1 (:id %)))
                  first
                  :kind)))
      (is (every? false? (map :executable (:stages plan))))
      (is (every? #(= :reject (:c1-required-result %))
                  (:mutation-checks plan)))
      (is (every? #(= :propagate (:executable-required-result %))
                  (:mutation-checks plan)))
      (is (every? #(= :unexecuted (:executable-status %))
                  (:mutation-checks plan))))
    (testing "canonical semantic mutation sites are separate, explicit hashes"
      (is (= [{"definition" "kernel-identity"
               "id" "identity-literal"
               "observed" "pnix.clr-meta.compiler-kernel.v1"
               "selector" "string-literal"
               "semantic_sha256"
               "c1bc2cdcd782bfc151c9a8f54223462b63534fc4f256824a8a982477e8427b09"}
              {"definition" "select-intrinsic-opcode"
               "id" "add-lowering-rule"
               "input_symbol" "+"
               "observed" "add.ovf"
               "selector" "operator-branch"
               "semantic_sha256"
               "d092a6b3595cb8f72395da8097a6fb78d9fb06d39f7a1dfaa4b66741f3b1d005"}
              {"definition" "select-intrinsic-opcode"
               "id" "subtract-lowering-rule"
               "input_symbol" "-"
               "observed" "sub.ovf"
               "selector" "operator-branch"
               "semantic_sha256"
               "3f983eedd94e93d19c30c35a51083cd8bbff5841c84d7d95ae43178c6fa222a5"}]
             (get receipt "mutation_sites")))
      (is (= 3 (count (distinct
                       (map #(get % "semantic_sha256")
                            (get receipt "mutation_sites")))))))
    (testing "all declarative claims remain exact and false"
      (doseq [claims [(:claims contract) (:claims profile) (:claims plan)]]
        (is (= #{:stage1_artifact :stage2 :self_reproduction :fixed_point
                 :raw_reproducibility}
               (set (keys claims))))
        (is (every? false? (vals claims)))))))

(deftest forbidden-and-unknown-source-surfaces-fail-without-a-receipt
  (let [root (temp-directory)]
    (try
      (doseq [[label before after expected-class]
              [["defn-macro"
                source-limits-definition
                "(defn source-limits-id [] nil)"
                :top-level-form-not-def]
               ["quote"
                source-limits-definition
                "(def source-limits-id (quote \"escape\"))"
                :forbidden-form]
               ["host-eval"
                source-limits-definition
                "(def source-limits-id (clojure.core/eval 1))"
                :forbidden-symbol]
               ["host-compiler"
                source-limits-definition
                "(def source-limits-id (clojure.lang.Compiler/eval 1))"
                :forbidden-symbol]
               ["host-process"
                source-limits-definition
                "(def source-limits-id (System.Diagnostics.Process/Start \"dotnet\"))"
                :forbidden-symbol]
               ["host-reflection"
                source-limits-definition
                "(def source-limits-id (System.Reflection.Assembly/Load \"opaque\"))"
                :forbidden-symbol]
               ["unknown-support-call"
                source-limits-definition
                "(def source-limits-id (pnix.clr-meta.compiler-support.data.v1/unknown 1))"
                :support-call-not-admitted]
               ["wrong-support-arity"
                source-limits-definition
                "(def source-limits-id (pnix.clr-meta.compiler-support.data.v1/count))"
                :call-arity-not-admitted]
               ["unknown-lexical"
                source-limits-definition
                "(def source-limits-id absent-lexical-value)"
                :unknown-symbol]
               ["metadata"
                source-limits-definition
                (str "^:escape " source-limits-definition)
                :reader-sugar-not-admitted]]]
        (let [paths (mutation-paths
                     root label :source
                     #(replace-exactly-once % before after))]
          (assert-rejected! root label paths expected-class)))
      (finally
        (delete-tree! root)))))

(deftest malformed-crossed-and-tampered-inputs-fail-without-a-receipt
  (let [root (temp-directory)]
    (try
      (let [malformed-source
            (mutation-paths root "malformed-source" :source #(str % "("))
            trailing-contract
            (mutation-paths root "trailing-contract" :contract #(str % "\n{}\n"))
            malformed-profile
            (mutation-paths root "malformed-profile" :profile #(str % "{"))]
        (assert-rejected! root "malformed-source" malformed-source
                          :invalid-source)
        (assert-rejected! root "trailing-contract" trailing-contract
                          :trailing-edn)
        (assert-rejected! root "malformed-profile" malformed-profile
                          :invalid-edn))
      (doseq [[label left right]
              [["cross-contract-profile" :contract :profile]
               ["cross-contract-plan" :contract :plan]
               ["cross-profile-plan" :profile :plan]]]
        (let [paths (copy-inputs! (canonical-paths)
                                  (System.IO.Path/Combine root label))]
          (assert-rejected! root label
                            (assoc paths
                                   left (get paths right)
                                   right (get paths left))
                            :key-set-mismatch)))
      (doseq [[label kind before after expected-class]
              [["tampered-contract" :contract
                ":pnix.clr-meta.compiler-selfhost-contract.v1"
                ":pnix.clr-meta.compiler-selfhost-contract.poison"
                :contract-schema-mismatch]
               ["tampered-profile" :profile
                ":pnix.clr-meta.compiler-selfhost-profile.v1"
                ":pnix.clr-meta.compiler-selfhost-profile.poison"
                :profile-schema-mismatch]
               ["tampered-plan" :plan
                ":pnix.clr-meta.compiler-selfhost-plan.v1"
                ":pnix.clr-meta.compiler-selfhost-plan.poison"
                :plan-schema-mismatch]]]
        (let [paths (mutation-paths
                     root label kind
                     #(replace-exactly-once % before after))]
          (assert-rejected! root label paths expected-class)))
      (let [paths
            (mutation-paths
             root "compiled-language-divergence" :contract
             #(replace-exactly-once
               %
               "  :compiled-language-id :pnix.clr-meta.compiler-kernel-source.v1"
               "  :compiled-language-id :pnix.clr-meta.different-language.v1"))]
        (assert-rejected! root "compiled-language-divergence" paths
                          :language-invariant-mismatch))
      (finally
        (delete-tree! root)))))

(deftest identity-and-lowering-mutations-are-separately-classified
  (let [root (temp-directory)]
    (try
      (doseq [[label before after expected-class]
              [["identity-mutation"
                "(def kernel-identity \"pnix.clr-meta.compiler-kernel.v1\")"
                "(def kernel-identity \"pnix.clr-meta.compiler-kernel.mutated\")"
               :mutation-literal-mismatch]
               ["add-lowering-mutation"
                "(def add-opcode \"add.ovf\")"
                "(def add-opcode \"mul.ovf\")"
                :mutation-lowering-mismatch]
               ["subtract-lowering-mutation"
                "(def subtract-opcode \"sub.ovf\")"
                "(def subtract-opcode \"mul.ovf\")"
                :mutation-lowering-mismatch]]]
        (let [paths (mutation-paths
                     root label :source
                     #(replace-exactly-once % before after))]
          (assert-rejected! root label paths expected-class)))
      (finally
        (delete-tree! root)))))

(defn -main
  [& _]
  (let [{:keys [fail error]}
        (run-tests 'pnix.clr-meta.compiler-selfhost-admission-test)]
    (when (pos? (+ fail error))
      (System.Environment/Exit 1))))
