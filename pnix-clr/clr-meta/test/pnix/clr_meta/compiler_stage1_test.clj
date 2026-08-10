(ns pnix.clr-meta.compiler-stage1-test
  (:require [clojure.edn :as edn]
            [clojure.string :as str]
            [clojure.test :refer [deftest is testing]]
            [pnix.clr-meta.compiler-stage1 :as stage1]
            [pnix.clr-meta.compiler-stage1-bundle :as bundle]
            [pnix.clr-meta.compiler-stage1-core :as core]
            [pnix.clr-meta.runtime-artifact :as artifact]))

(def ^:private zero-fallback-counts
  {"compile" 0
   "eval" 0
   "load" 0
   "load_file" 0
   "load_string" 0})

(def ^:private example-form
  (edn/read-string "(+ (* arg 3) (- 10 4))"))

(def ^:private target-manifest-keys
  #{"backend" "target_form_fallback_calls" "compiler_self_reproduction"
    "compiler_stage" "compiler_stage2" "entry" "il_fixed_point"
    "input_kind" "ir_sha256" "node_accounting"
    "output_closure_sha256" "outputs" "plan_sha256" "profile_id"
    "profile_sha256" "raw_artifact_reproducibility" "schema"
    "source_sha256" "source_text_compilation" "target"
    "target_assembly_references" "target_resources"})

(def ^:private bundle-manifest-keys
  #{"bootstrap" "boundaries" "bundle_closure_sha256" "bundle_files"
    "claim_scope" "compiler_entry"
    "compiler_output_closure_sha256" "compiler_outputs"
    "compiler_self_reproduction" "compiler_source_closure_sha256"
    "compiler_sources" "compiler_stage15_n" "compiler_stage2" "emitter"
    "il_fixed_point" "input_kind" "profile_id" "profile_sha256"
    "raw_artifact_reproducibility" "runtime_dependencies"
    "runtime_isolation" "runtime_snapshot" "schema"
    "self_source_accounting" "stage" "target" "toolchain"})

(defn- temp-directory
  []
  (let [path (System.IO.Path/Combine
              (System.IO.Path/GetTempPath)
              (str "pnix-clr-meta-compiler-stage1-test-"
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

(defn- failure-data
  [f]
  (try
    (f)
    nil
    (catch clojure.lang.ExceptionInfo cause
      (ex-data cause))))

(defn- failure-class
  [f]
  (:class (failure-data f)))

(defn- write-inputs!
  [root profile plan form]
  (let [profile-path (System.IO.Path/Combine root "profile.edn")
        plan-path (System.IO.Path/Combine root "plan.edn")
        source-path (System.IO.Path/Combine root "source.clj")]
    (write-text! profile-path (str (pr-str profile) "\n"))
    (write-text! plan-path (str (pr-str plan) "\n"))
    (write-text! source-path (str (pr-str form) "\n"))
    {:profile profile-path :plan plan-path :source source-path}))

(defn- balanced-expression
  [levels]
  (loop [remaining levels
         form 'arg]
    (if (zero? remaining)
      form
      (recur (dec remaining) (list '+ form form)))))

(defn- deep-expression
  [levels]
  (loop [remaining levels
         form 'arg]
    (if (zero? remaining)
      form
      (recur (dec remaining) (list '+ form 0)))))

(defn- relative-files
  [root]
  (->> (System.IO.Directory/GetFiles
        root "*" System.IO.SearchOption/AllDirectories)
       (map #(System.IO.Path/GetRelativePath root %))
       (map #(str/replace % "\\" "/"))
       set))

(defn- compiler-source-root
  []
  (let [load-path (or (System.Environment/GetEnvironmentVariable
                       "CLOJURE_LOAD_PATH")
                      "")
        separator (System.Text.RegularExpressions.Regex/Escape
                   (str System.IO.Path/PathSeparator))
        candidates (remove str/blank? (str/split load-path
                                                  (re-pattern separator)))
        source-root
        (first
         (filter
          (fn [candidate]
            (System.IO.File/Exists
             (System.IO.Path/Combine
              candidate "pnix" "clr_meta" "compiler_stage1.clj")))
          candidates))]
    (when-not source-root
      (throw (ex-info "clr-meta source root is absent from CLOJURE_LOAD_PATH"
                      {:class :test-source-root-missing
                       :load-path load-path})))
    (System.IO.Path/GetFullPath source-root)))

(defn- run-process
  [program arguments working-directory]
  (let [start-info (doto (System.Diagnostics.ProcessStartInfo. program)
                     (.set_UseShellExecute false)
                     (.set_CreateNoWindow true)
                     (.set_RedirectStandardOutput true)
                     (.set_RedirectStandardError true)
                     (.set_WorkingDirectory working-directory))]
    (doseq [argument arguments]
      (.Add (.get_ArgumentList start-info) (str argument)))
    (with-open [process (System.Diagnostics.Process/Start start-info)]
      (let [stdout* (future (.ReadToEnd (.get_StandardOutput process)))
            stderr* (future (.ReadToEnd (.get_StandardError process)))]
        (.WaitForExit process)
        {:exit (.get_ExitCode process)
         :stdout @stdout*
         :stderr @stderr*}))))

(deftest pure-lowering-produces-the-exact-canonical-ir
  (is (= {:schema :pnix.clr-meta.compiler-stage1-ir.v1
          :profile :pnix.clr-meta.checked-i64-expression.v1
          :instructions [[:ldarg-0]
                         [:ldc-i8 3]
                         [:mul-ovf]
                         [:ldc-i8 10]
                         [:ldc-i8 4]
                         [:sub-ovf]
                         [:add-ovf]]
          :accounting {:nodes 7
                       :max-depth 3
                       :node-kinds {:call-add 1
                                    :call-multiply 1
                                    :parameter 1
                                    :i64-literal 3
                                    :call-subtract 1}
                       :directly-lowered 7
                       :runtime-primitive-nodes 0
                       :rejected-nodes 0
                       :unknown-nodes 0}}
         (core/lower core/expected-profile example-form))))

(deftest pure-lowering-rejects-every-value-outside-the-admitted-profile
  (testing "the complete profile is an exact structural contract"
    (let [data (failure-data
                #(core/lower (assoc core/expected-profile
                                    :id :tampered-profile)
                             example-form))]
      (is (= {:schema :pnix.clr-meta.compiler-stage1-error.v1
              :phase :compiler-stage1-lowering
              :class :profile-contract-mismatch}
             (select-keys data [:schema :phase :class])))
      (is (= core/profile-id (:expected-id data)))
      (is (= :tampered-profile (:actual-id data)))))
  (testing "form, operator, arity, symbol, and integer domains fail closed"
    (doseq [[expected form]
            [[:unsupported-form [:unknown "form"]]
             [:unsupported-operator (edn/read-string "(quot arg 2)")]
             [:unsupported-arity (edn/read-string "(+ arg)")]
             [:unsupported-symbol (edn/read-string "unknown")]
             [:non-i64-integer (bigint "9223372036854775808")]
             [:non-i64-integer (bigint "1")]
             [:metadata-not-admitted (edn/read-string "^:foo (+ arg 1)")]
             [:metadata-not-admitted (edn/read-string "(^:foo + arg 1)")]
             [:metadata-not-admitted (edn/read-string "(+ ^:foo arg 1)")]]]
      (is (= expected (failure-class #(core/lower core/expected-profile form)))
          (str "expected " expected " for " (pr-str form)))))
  (testing "depth and node budgets are independent hard limits"
    (is (= :depth-budget-exceeded
           (failure-class
            #(core/lower core/expected-profile
                         (deep-expression (inc core/max-depth))))))
    (is (= :node-budget-exceeded
           (failure-class
            #(core/lower core/expected-profile
                         (balanced-expression 8)))))))

(deftest direct-backend-failure-preserves-state-and-never-publishes-partials
  (testing "unknown source form leaves no output or staging directory"
    (let [root (temp-directory)
          {:keys [profile plan source]}
          (write-inputs! root core/expected-profile stage1/expected-plan
                         [:unknown "form"])
          output (System.IO.Path/Combine root "target")]
      (try
        (is (= :unsupported-form
               (failure-class
                #(stage1/build! profile plan source output
                                zero-fallback-counts))))
        (is (false? (System.IO.Directory/Exists output)))
        (is (empty?
             (System.IO.Directory/GetDirectories
              root "target.building.*"
              System.IO.SearchOption/TopDirectoryOnly)))
        (finally
          (delete-tree! root)))))
  (testing "tampered profile cannot replace an already-published directory"
    (let [root (temp-directory)
          {:keys [profile plan source]}
          (write-inputs! root (assoc core/expected-profile
                                     :unsupported-policy :host-eval-fallback)
                         stage1/expected-plan example-form)
          output (System.IO.Path/Combine root "target")
          sentinel (System.IO.Path/Combine output "keep.txt")]
      (try
        (write-text! sentinel "published\n")
        (is (= :output-exists
               (failure-class
                #(stage1/build! profile plan source output
                                zero-fallback-counts))))
        (is (= #{"keep.txt"} (relative-files output)))
        (is (= "published\n" (System.IO.File/ReadAllText sentinel)))
        (finally
          (delete-tree! root)))))
  (testing "a nonzero fallback witness invalidates and cleans the build"
    (let [root (temp-directory)
          {:keys [profile plan source]}
          (write-inputs! root core/expected-profile stage1/expected-plan
                         example-form)
          output (System.IO.Path/Combine root "target")]
      (try
        (is (= :fallback-counter-nonzero
               (failure-class
                #(stage1/build! profile plan source output
                                (assoc zero-fallback-counts "eval" 1)))))
        (is (false? (System.IO.Directory/Exists output)))
        (is (empty?
             (System.IO.Directory/GetDirectories
              root "target.building.*"
              System.IO.SearchOption/TopDirectoryOnly)))
        (finally
          (delete-tree! root))))))

(deftest bundle-refuses-destructive-overlap-and-unowned-output-before-mutation
  (let [root (temp-directory)
        {:keys [profile plan source]}
        (write-inputs! root core/expected-profile stage1/expected-plan
                       example-form)
        original-source-root (compiler-source-root)
        staged-source-root (System.IO.Path/Combine root "compiler-source")
        old-source-root (System.Environment/GetEnvironmentVariable
                         "CLR_META_SOURCE_ROOT")]
    (try
      (doseq [relative ["compiler_stage1_core.clj" "compiler_stage1.clj"]]
        (let [target (System.IO.Path/Combine staged-source-root
                                             "pnix" "clr_meta" relative)]
          (System.IO.Directory/CreateDirectory
           (System.IO.Path/GetDirectoryName target))
          (System.IO.File/Copy
           (System.IO.Path/Combine original-source-root
                                   "pnix" "clr_meta" relative)
           target true)))
      (System.Environment/SetEnvironmentVariable "CLR_META_SOURCE_ROOT"
                                                  staged-source-root)
      (is (= :output-path-overlap
             (failure-class
              #(bundle/build! profile plan source staged-source-root))))
      (is (= 2 (count (System.IO.Directory/GetFiles
                       staged-source-root "*.clj"
                       System.IO.SearchOption/AllDirectories))))
      (let [unowned (System.IO.Path/Combine root "unowned")
            sentinel (System.IO.Path/Combine unowned "keep.txt")]
        (write-text! sentinel "keep\n")
        (is (= :output-exists
               (failure-class #(bundle/build! profile plan source unowned))))
        (is (= "keep\n" (System.IO.File/ReadAllText sentinel))))
      (finally
        (System.Environment/SetEnvironmentVariable "CLR_META_SOURCE_ROOT"
                                                    old-source-root)
        (delete-tree! root)))))

(deftest bundle-seeds-a-source-hidden-compiler-and-emits-a-fresh-executable
  (let [root (temp-directory)
        {:keys [profile plan source]}
        (write-inputs! root core/expected-profile stage1/expected-plan
                       example-form)
        output (System.IO.Path/Combine root "artifact")
        run-cwd (System.IO.Path/Combine root "empty-run-cwd")
        source-root (compiler-source-root)
        old-source-root (System.Environment/GetEnvironmentVariable
                         "CLR_META_SOURCE_ROOT")]
    (try
      (System.IO.Directory/CreateDirectory run-cwd)
      (System.Environment/SetEnvironmentVariable "CLR_META_SOURCE_ROOT"
                                                  source-root)
      (let [manifest (bundle/build! profile plan source output)
            target (get-in manifest ["target" "artifact"])
            target-root (System.IO.Path/Combine output "target")
            program (System.IO.Path/Combine target-root "program.dll")
            outer-manifest (System.IO.Path/Combine output "manifest.json")
            target-manifest (System.IO.Path/Combine target-root "manifest.json")]
        (testing "the Stage1 bundle and target manifests are exact"
          (is (= bundle-manifest-keys (set (keys manifest))))
          (is (= target-manifest-keys (set (keys target))))
          (is (= bundle/manifest-schema (get manifest "schema")))
          (is (= stage1/target-manifest-schema (get target "schema")))
          (is (= 1 (get manifest "stage")))
          (is (= 1 (get target "compiler_stage")))
          (is (= "profile-qualified-clojure-form-to-managed-pe"
                 (get manifest "claim_scope")))
          (is (= "direct-system-reflection-emit" (get target "backend")))
          (is (true? (get target "source_text_compilation")))
          (is (= zero-fallback-counts (get target "target_form_fallback_calls")))
          (is (= {"directly_lowered" 7
                  "max_depth" 3
                  "nodes" 7
                  "runtime_primitive_nodes" 0
                  "unknown_nodes" 0}
                 (get target "node_accounting")))
          (is (= ["System.Console" "System.Private.CoreLib"]
                 (get target "target_assembly_references")))
          (is (= [] (get target "target_resources")))
          (is (= (str (artifact/manifest-json manifest) "\n")
                 (System.IO.File/ReadAllText outer-manifest)))
          (is (= (str (stage1/json-encode target) "\n")
                 (System.IO.File/ReadAllText target-manifest))))
        (testing "all higher compiler and reproducibility claims remain false"
          (doseq [[document key]
                  [[manifest "compiler_stage15_n"]
                   [manifest "compiler_stage2"]
                   [manifest "compiler_self_reproduction"]
                   [manifest "il_fixed_point"]
                   [manifest "raw_artifact_reproducibility"]
                   [target "compiler_stage2"]
                   [target "compiler_self_reproduction"]
                   [target "il_fixed_point"]
                   [target "raw_artifact_reproducibility"]]]
            (is (false? (get document key)) key))
          (is (= {"compiler_implementation_source_at_target" false
                  "host_compiler_use" "seed-and-pinned-runtime-startup"
                  "pinned_runtime_startup_source_compilation" true
                  "standalone_source_free_distribution" false
                  "target_form_host_compiler_fallback" false
                  "target_form_host_eval_fallback" false
                  "target_source_evaluator" false}
                 (get manifest "boundaries")))
          (is (= "seed-only" (get-in manifest ["bootstrap" "use"])))
          (is (= {"api" "System.Reflection.Emit.PersistedAssemblyBuilder"
                  "semantic_owner" "clojureclr-stage1-compiler"}
                 (get manifest "emitter")))
          (let [accounting (get manifest "self_source_accounting")]
            (is (true? (get accounting "coverage_complete")))
            (is (false? (get accounting "stage2_ready")))
            (is (false? (get accounting
                             "compiler_source_within_stage1_profile")))
            (is (= "top-level-clojure-reader-form" (get accounting "unit")))
            (is (zero? (get accounting "supported")))
            (is (pos? (get accounting "total")))
            (is (= (get accounting "total")
                   (get accounting "classified")
                   (get accounting "unsupported"))))
          (is (= "clear-then-allowlist"
                 (get-in manifest ["runtime_isolation" "environment_policy"])))
          (is (true? (get-in manifest
                             ["runtime_snapshot" "unchanged_after_target"])))
          (is (pos? (get-in manifest ["runtime_snapshot" "file_count"]))))
        (testing "the published closure has exactly the declared files"
          (is (= #{"compiler-seed/manifest.json"
                   "compiler-seed/pnix.clr_meta.compiler_stage1.clj.dll"
                   "compiler-seed/pnix.clr_meta.compiler_stage1_core.clj.dll"
                   "manifest.json"
                   "target/manifest.json"
                   "target/program.dll"
                   "target/program.runtimeconfig.json"}
                 (relative-files output)))
          (is (= #{"program.dll" "program.runtimeconfig.json"}
                 (set (map #(get % "path") (get target "outputs")))))
          (doseq [row (get manifest "compiler_outputs")]
            (is (= (get row "sha256")
                   (artifact/sha256-file
                    (System.IO.Path/Combine output (get row "path"))))))
          (doseq [row (get target "outputs")]
            (is (= (get row "sha256")
                   (artifact/sha256-file
                    (System.IO.Path/Combine target-root (get row "path"))))))
          (doseq [row (get manifest "bundle_files")]
            (is (= (get row "sha256")
                   (artifact/sha256-file
                    (System.IO.Path/Combine output (get row "path"))))))
          (is (= (artifact/closure-hash (get manifest "bundle_files"))
                 (get manifest "bundle_closure_sha256"))))
        (testing "the managed PE accepts dynamic arguments in fresh processes"
          (doseq [[argument expected] [[-9 -21] [0 6] [7 27] [100 306]]]
            (let [result (run-process System.Environment/ProcessPath
                                      [program argument]
                                      run-cwd)]
              (is (= 0 (:exit result)) (pr-str result))
              (is (= (str expected System.Environment/NewLine)
                     (:stdout result)))
              (is (= "" (:stderr result)))))))
      (finally
        (System.Environment/SetEnvironmentVariable "CLR_META_SOURCE_ROOT"
                                                    old-source-root)
        (delete-tree! root)))))
