(ns pnix.clr-meta.compiler-stage1-bundle
  "Bootstrap/target phase separator for the profile-qualified CLR Stage1.

  The pinned host compiler may seed exactly two ClojureCLR compiler
  namespaces.  Target compilation then runs those AOT assemblies in a fresh
  process with a replaced load path and empty cwd.  This namespace owns the
  combined receipt artifact; it owns no target-language lowering semantics."
  (:require [clojure.edn :as edn]
            [clojure.set :as set]
            [clojure.string :as str]
            [pnix.clr-meta.compiler-stage1-core :as core]
            [pnix.clr-meta.runtime-artifact :as artifact]))

(def manifest-schema "pnix.clr-meta.compiler-stage1-artifact.v1")
(def compiler-entry "pnix.clr-meta.compiler-stage1")
(def compiler-namespaces
  ['pnix.clr-meta.compiler-stage1-core
   'pnix.clr-meta.compiler-stage1])
(def compiler-source-paths
  ["pnix/clr_meta/compiler_stage1_core.clj"
   "pnix/clr_meta/compiler_stage1.clj"])

(def ^:private utf8 (System.Text.UTF8Encoding. false true))
(def ^:private target-manifest-keys
  #{"backend" "target_form_fallback_calls" "compiler_self_reproduction"
    "compiler_stage" "compiler_stage2" "entry" "il_fixed_point"
    "input_kind" "ir_sha256" "node_accounting"
    "output_closure_sha256" "outputs" "plan_sha256" "profile_id"
    "profile_sha256" "raw_artifact_reproducibility" "schema"
    "source_sha256" "source_text_compilation" "target" "target_assembly_references"
    "target_resources"})

(defn- fail!
  [class message evidence]
  (throw (ex-info message
                  (merge {:schema :pnix.clr-meta.compiler-stage1-bundle-error.v1
                          :phase :compiler-stage1-bundle
                          :class class}
                         evidence))))

(defn- reparse-point?
  [path]
  (not (zero? (bit-and (int (System.IO.File/GetAttributes path))
                       (int System.IO.FileAttributes/ReparsePoint)))))

(defn- reparse-ancestor
  [path]
  (loop [candidate (System.IO.Path/GetFullPath (str path))]
    (when candidate
      (if (and (or (System.IO.File/Exists candidate)
                   (System.IO.Directory/Exists candidate))
               (reparse-point? candidate))
        candidate
        (recur (System.IO.Path/GetDirectoryName candidate))))))

(defn- physical-path
  [path]
  (let [full (System.IO.Path/GetFullPath (str path))
        root (System.IO.Path/GetPathRoot full)
        relative (.Substring full (.Length root))
        parts (remove str/blank? (str/split relative #"[\\/]+"))]
    (loop [current root remaining parts]
      (if-let [part (first remaining)]
        (let [candidate (System.IO.Path/Combine current part)
              exists-file (System.IO.File/Exists candidate)
              exists-dir (System.IO.Directory/Exists candidate)
              resolved (when (and (or exists-file exists-dir)
                                  (reparse-point? candidate))
                         (.ResolveLinkTarget
                          (if exists-dir
                            (System.IO.DirectoryInfo. candidate)
                            (System.IO.FileInfo. candidate))
                          true))]
          (recur (if resolved (.FullName resolved) candidate) (next remaining)))
        (System.IO.Path/TrimEndingDirectorySeparator current)))))

(defn- normalized-path
  [path]
  (physical-path path))

(defn- path-comparison
  []
  (if (or (System.OperatingSystem/IsWindows)
          (System.OperatingSystem/IsMacOS))
    System.StringComparison/OrdinalIgnoreCase
    System.StringComparison/Ordinal))

(defn- same-path?
  [left right]
  (.Equals (normalized-path left) (normalized-path right) (path-comparison)))

(defn- below-path?
  [candidate ancestor]
  (let [ancestor (normalized-path ancestor)
        suffix (str System.IO.Path/DirectorySeparatorChar)
        prefix (if (.EndsWith ancestor suffix (path-comparison))
                 ancestor
                 (str ancestor suffix))]
    (.StartsWith (normalized-path candidate) prefix (path-comparison))))

(defn- overlap?
  [left right]
  (or (same-path? left right)
      (below-path? left right)
      (below-path? right left)))

(defn- validate-paths!
  [source-root profile-path plan-path source-path output]
  (let [inputs [[:compiler-source-root source-root]
                [:profile profile-path]
                [:plan plan-path]
                [:source source-path]]]
    (doseq [[kind input] inputs]
      (when (overlap? input output)
        (fail! :output-path-overlap
               "Stage1 bundle output must be disjoint from all inputs"
               {:input-kind kind :input (normalized-path input)
                :output (normalized-path output)})))
    (doseq [[left-kind left] (rest inputs)
            [right-kind right] (rest inputs)
            :when (neg? (compare (name left-kind) (name right-kind)))]
      (when (overlap? left right)
        (fail! :input-path-overlap
               "Stage1 profile, plan, and source paths must be distinct"
               {:left-kind left-kind :right-kind right-kind})))))

(defn- require-file!
  [path kind]
  (let [path (System.IO.Path/GetFullPath (str path))]
    (when-not (System.IO.File/Exists path)
      (fail! :missing-input "Stage1 bundle input file is unavailable"
             {:kind kind :path path}))
    (when (reparse-point? path)
      (fail! :input-symlink
             "Stage1 bundle input itself must not be a symlink/reparse point"
             {:kind kind :path path}))
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
    (System.IO.File/WriteAllText path text utf8)))

(defn- relative-files
  [root]
  (->> (System.IO.Directory/GetFiles root "*" System.IO.SearchOption/AllDirectories)
       (map #(System.IO.Path/GetRelativePath root %))
       (map #(str/replace % "\\" "/"))
       set))

(defn- copy-tree!
  [source target]
  (when (reparse-point? source)
    (fail! :runtime-snapshot-symlink
           "runtime snapshot source directory must not be a symlink"
           {:path source}))
  (System.IO.Directory/CreateDirectory target)
  (doseq [file (System.IO.Directory/GetFiles
                source "*" System.IO.SearchOption/TopDirectoryOnly)]
    (when (reparse-point? file)
      (fail! :runtime-snapshot-symlink
             "runtime snapshot source file must not be a symlink"
             {:path file}))
    (System.IO.File/Copy file
                         (System.IO.Path/Combine target
                                                 (System.IO.Path/GetFileName file))
                         true))
  (doseq [directory (System.IO.Directory/GetDirectories
                     source "*" System.IO.SearchOption/TopDirectoryOnly)]
    (copy-tree! directory
                (System.IO.Path/Combine target
                                        (System.IO.Path/GetFileName directory)))))

(defn- closure-rows
  [root]
  (->> (relative-files root)
       sort
       (mapv (fn [relative]
               {"path" relative
                "sha256" (artifact/sha256-file
                          (System.IO.Path/Combine
                           root
                           (str/replace relative "/"
                                        (str System.IO.Path/DirectorySeparatorChar))))}))))

(defn- read-top-level-forms
  [path]
  (let [eof (Object.)]
    (with-open [reader (clojure.lang.LineNumberingTextReader.
                        (System.IO.StringReader.
                         (System.IO.File/ReadAllText path utf8)))]
      (binding [*read-eval* false]
        (loop [forms []]
          (let [form (read {:eof eof :read-cond :preserve} reader)]
            (if (identical? eof form)
              forms
              (recur (conj forms form)))))))))

(defn- classify-compiler-source
  [path]
  (let [forms (read-top-level-forms path)
        strip-metadata
        (fn strip-metadata [form]
          (let [rebuilt
                (cond
                  (map? form) (into (empty form)
                                    (map (fn [[key value]]
                                           [(strip-metadata key)
                                            (strip-metadata value)]))
                                    form)
                  (vector? form) (mapv strip-metadata form)
                  (set? form) (into #{} (map strip-metadata) form)
                  (seq? form) (apply list (map strip-metadata form))
                  :else form)]
            (if (instance? clojure.lang.IObj rebuilt)
              (with-meta rebuilt nil)
              rebuilt)))
        classifications
        (mapv (fn [form]
                (try
                  (core/lower core/expected-profile (strip-metadata form))
                  {:status :supported}
                  (catch clojure.lang.ExceptionInfo cause
                    {:status :unsupported
                     :class (:class (ex-data cause))})))
              forms)]
    {:top-level-forms (count forms)
     :supported (count (filter #(= :supported (:status %)) classifications))
     :unsupported (count (filter #(= :unsupported (:status %)) classifications))
     :rejection-classes (->> classifications
                             (keep :class)
                             (map name)
                             frequencies
                             (into (sorted-map)))}))

(defn- copy-compiler-source!
  [source-root staging-root]
  (mapv (fn [relative]
          (let [source (require-file!
                        (System.IO.Path/Combine source-root
                                                (str/replace relative "/" (str System.IO.Path/DirectorySeparatorChar)))
                        :compiler-source)
                target (System.IO.Path/Combine staging-root
                                               (str/replace relative "/" (str System.IO.Path/DirectorySeparatorChar)))]
            (System.IO.Directory/CreateDirectory
             (System.IO.Path/GetDirectoryName target))
            (System.IO.File/Copy source target true)
            (assoc (classify-compiler-source target) :path relative)))
        compiler-source-paths))

(defn- strict-read-one
  [text kind]
  (let [eof (Object.)]
    (try
      (with-open [reader (clojure.lang.LineNumberingTextReader.
                          (System.IO.StringReader. text))]
        (let [value (edn/read {:eof eof} reader)
              trailing (edn/read {:eof eof} reader)]
          (when (or (identical? eof value)
                    (not (identical? eof trailing)))
            (fail! :child-output-shape
                   "Stage1 child must print exactly one EDN value"
                   {:kind kind}))
          value))
      (catch clojure.lang.ExceptionInfo cause (throw cause))
      (catch System.Exception cause
        (fail! :child-output-edn
               "Stage1 child output is not strict EDN"
               {:kind kind :cause-type (str (type cause))})))))

(defn- current-runtime
  []
  (let [dotnet System.Environment/ProcessPath
        entry (System.Reflection.Assembly/GetEntryAssembly)
        main-dll (when entry (.Location entry))
        clojure-dll (.Location (.Assembly clojure.lang.RT))]
    (when (or (nil? dotnet) (= "" dotnet) (not (System.IO.File/Exists dotnet)))
      (fail! :dotnet-runtime-missing
             "absolute current dotnet executable is unavailable" {}))
    (when (or (nil? main-dll) (= "" main-dll)
              (not (System.IO.File/Exists main-dll)))
      (fail! :clojure-main-missing
             "current pinned Clojure.Main assembly is unavailable" {}))
    (when (or (nil? clojure-dll) (= "" clojure-dll)
              (not (System.IO.File/Exists clojure-dll)))
      (fail! :clojure-runtime-missing
             "current pinned Clojure runtime assembly is unavailable" {}))
    {:dotnet (System.IO.Path/GetFullPath dotnet)
     :main-dll (System.IO.Path/GetFullPath main-dll)
     :clojure-dll (System.IO.Path/GetFullPath clojure-dll)}))

(def ^:private sanitized-environment-keys
  ["COR_ENABLE_PROFILING"
   "COR_PROFILER"
   "COR_PROFILER_PATH"
   "CORECLR_ENABLE_PROFILING"
   "CORECLR_PROFILER"
   "CORECLR_PROFILER_PATH"
   "CORECLR_PROFILER_PATH_32"
   "CORECLR_PROFILER_PATH_64"
   "DOTNET_ADDITIONAL_DEPS"
   "DOTNET_DiagnosticPorts"
   "DOTNET_SHARED_STORE"
   "DOTNET_STARTUP_HOOKS"])

(defn- compiler-shadow-candidates
  []
  (vec
   (concat
    (for [base ["pnix/clr_meta/compiler_stage1"
                "pnix/clr_meta/compiler_stage1_core"]
          extension [".clj" ".cljc" ".cljr" ".clj.dll"]]
      (str base extension))
    (for [base ["pnix.clr_meta.compiler_stage1"
                "pnix.clr_meta.compiler_stage1_core"]
          extension [".cljr.dll" ".cljc.dll" ".clj.dll"]]
      (str base extension)))))

(defn- verified-loader-roots!
  [runtime-root main-dll clojure-dll working-dir seed-dir]
  (let [base (normalized-path runtime-root)
        roots (->> [base
                    (System.IO.Path/Combine base "bin")
                    working-dir
                    (System.IO.Path/GetDirectoryName clojure-dll)
                    (System.IO.Path/GetDirectoryName main-dll)]
                   (map normalized-path)
                   distinct
                   vec)]
    (doseq [root roots
            relative (compiler-shadow-candidates)
            :let [candidate (System.IO.Path/Combine
                             root
                             (str/replace relative "/"
                                          (str System.IO.Path/DirectorySeparatorChar)))]
            :when (or (System.IO.File/Exists candidate)
                      (System.IO.Directory/Exists candidate))]
      (fail! :runtime-compiler-shadow
             "an earlier ClojureCLR lookup root shadows the compiler seed"
             {:root root :candidate candidate :seed-dir seed-dir}))
    roots))

(defn- run-seeded-compiler!
  [runtime-root seed-dir profile-path plan-path source-path target-dir working-dir]
  (let [{:keys [dotnet]} (current-runtime)
        main-dll (System.IO.Path/Combine runtime-root "Clojure.Main.dll")
        clojure-dll (System.IO.Path/Combine runtime-root "Clojure.dll")
        loader-roots (verified-loader-roots!
                      runtime-root main-dll clojure-dll working-dir seed-dir)
        start-info (doto (System.Diagnostics.ProcessStartInfo. dotnet)
                     (.set_UseShellExecute false)
                     (.set_CreateNoWindow true)
                     (.set_RedirectStandardOutput true)
                     (.set_RedirectStandardError true)
                     (.set_WorkingDirectory working-dir))
        arguments (.get_ArgumentList start-info)
        environment (.get_Environment start-info)]
    (doseq [argument [main-dll "-m" compiler-entry
                      profile-path plan-path source-path target-dir]]
      (.Add arguments argument))
    ;; Clear the inherited process environment before installing the exact
    ;; language/runtime allowlist. This excludes startup hooks, profilers,
    ;; additional deps, caller PATH, and caller HOME in one operation.
    (.Clear environment)
    (.set_Item environment "CLOJURE_LOAD_PATH" seed-dir)
    (.set_Item environment "CLOJURE_COMPILE_PATH" "")
    (.set_Item environment "CLR_META_SOURCE_ROOT" "")
    (.set_Item environment "HOME" working-dir)
    (.set_Item environment "TMPDIR" working-dir)
    (.set_Item environment "LANG" "C")
    (.set_Item environment "LC_ALL" "C")
    (try
      (with-open [process (System.Diagnostics.Process/Start start-info)]
        (let [stdout* (future (.ReadToEnd (.get_StandardOutput process)))
              stderr* (future (.ReadToEnd (.get_StandardError process)))]
          (.WaitForExit process)
          (let [stdout @stdout*
                stderr @stderr*
                exit (.get_ExitCode process)]
            (when-not (zero? exit)
              (fail! :seeded-compiler-failed
                     "compiler-source-hidden seeded Stage1 compiler failed"
                     {:exit exit :stdout stdout :stderr stderr}))
            {:manifest (strict-read-one stdout :seeded-compiler)
             :loader-roots loader-roots})))
      (catch clojure.lang.ExceptionInfo cause (throw cause))
      (catch System.Exception cause
        (fail! :seeded-compiler-start-failed
               "compiler-source-hidden seeded Stage1 compiler could not start"
               {:cause-type (str (type cause))
                :message (.Message cause)})))))

(defn- verify-target!
  [target-dir manifest profile-path plan-path source-path]
  (let [outputs (get manifest "outputs")
        accounting (get manifest "node_accounting")]
    (when-not (and (map? manifest)
                 (= target-manifest-keys (set (keys manifest)))
                 (= "pnix.clr-meta.compiler-stage1-target.v1"
                    (get manifest "schema"))
                 (= 1 (get manifest "compiler_stage"))
                 (= "direct-system-reflection-emit" (get manifest "backend"))
                 (= "ClrMeta.Stage1.Program/Main" (get manifest "entry"))
                 (= "strict-edn-one-form" (get manifest "input_kind"))
                 (= "net10.0" (get manifest "target"))
                 (= {"compile" 0 "eval" 0 "load" 0
                     "load_file" 0 "load_string" 0}
                    (get manifest "target_form_fallback_calls"))
                 (false? (get manifest "compiler_stage2"))
                 (false? (get manifest "compiler_self_reproduction"))
                 (false? (get manifest "il_fixed_point"))
                 (false? (get manifest "raw_artifact_reproducibility"))
                 (true? (get manifest "source_text_compilation"))
                 (= (artifact/sha256-file profile-path)
                    (get manifest "profile_sha256"))
                 (= (artifact/sha256-file plan-path)
                    (get manifest "plan_sha256"))
                 (= (artifact/sha256-file source-path)
                    (get manifest "source_sha256"))
                 (= "pnix.clr-meta.checked-i64-expression.v1"
                    (get manifest "profile_id"))
                 (re-matches #"[0-9a-f]{64}" (get manifest "ir_sha256"))
                 (re-matches #"[0-9a-f]{64}"
                             (get manifest "output_closure_sha256"))
                 (= ["System.Console" "System.Private.CoreLib"]
                    (get manifest "target_assembly_references"))
                 (= [] (get manifest "target_resources"))
                 (= #{"directly_lowered" "max_depth" "nodes"
                      "runtime_primitive_nodes" "unknown_nodes"}
                    (set (keys accounting)))
                 (= 0 (get accounting "unknown_nodes"))
                 (= 0 (get accounting "runtime_primitive_nodes"))
                 (= (get accounting "nodes")
                    (get accounting "directly_lowered"))
                 (vector? outputs)
                 (= 2 (count outputs))
                 (= ["program.dll" "program.runtimeconfig.json"]
                    (mapv #(get % "path") outputs))
                 (every? #(= #{"path" "sha256"} (set (keys %))) outputs)
                 (every? #(and (string? (get % "sha256"))
                               (re-matches #"[0-9a-f]{64}"
                                           (get % "sha256")))
                         outputs))
      (fail! :target-manifest-contract
             "seeded compiler returned an invalid target manifest" {})))
  (let [expected #{{"path" "program.dll"}
                   {"path" "program.runtimeconfig.json"}}
        path-set (set (map #(select-keys % ["path"])
                           (get manifest "outputs")))
        expected-files #{"manifest.json" "program.dll"
                         "program.runtimeconfig.json"}]
    (when-not (= expected path-set)
      (fail! :target-output-declaration
             "target output declaration is not exact"
             {:actual path-set}))
    (when-not (= expected-files (relative-files target-dir))
      (fail! :target-file-set
             "target artifact file set is not exact"
             {:actual (relative-files target-dir)}))
    (doseq [row (get manifest "outputs")]
      (let [path (System.IO.Path/Combine target-dir (get row "path"))]
        (when (reparse-point? path)
          (fail! :target-output-symlink
                 "target artifact output must not be a symlink"
                 {:path (get row "path")}))
        (when-not (= (artifact/sha256-file path) (get row "sha256"))
          (fail! :target-output-hash
                 "target artifact output hash is stale"
                 {:path (get row "path")}))))
    (when-not (= (artifact/closure-hash (get manifest "outputs"))
                 (get manifest "output_closure_sha256"))
      (fail! :target-output-closure
             "target artifact output closure hash is stale" {}))
    (when-not (= (str (artifact/manifest-json manifest) "\n")
                 (System.IO.File/ReadAllText
                  (System.IO.Path/Combine target-dir "manifest.json") utf8))
      (fail! :target-manifest-bytes
             "target manifest bytes do not match the child receipt" {}))
    manifest))

(defn- publish-directory!
  [temporary output]
  (when (or (System.IO.File/Exists output)
            (System.IO.Directory/Exists output))
    (fail! :output-exists
           "Stage1 bundle publication requires an absent output path"
           {:output output}))
  (System.IO.Directory/Move temporary output))

(defn build!
  "Seed, compiler-source-hide, run, verify, and publish one Stage1 bundle."
  [profile-path plan-path source-path output-dir]
  (let [profile-path (require-file! profile-path :profile)
        plan-path (require-file! plan-path :plan)
        source-path (require-file! source-path :source)
        source-root-raw (System.Environment/GetEnvironmentVariable
                         "CLR_META_SOURCE_ROOT")
        _ (when (str/blank? source-root-raw)
            (fail! :compiler-source-root-missing
                   "CLR_META_SOURCE_ROOT is unset or blank" {}))
        source-root (System.IO.Path/GetFullPath source-root-raw)
        output (System.IO.Path/GetFullPath (str output-dir))
        parent (System.IO.Path/GetDirectoryName output)
        nonce (.ToString (System.Guid/NewGuid) "N")
        temporary (str output ".building." nonce)
        workspace (System.IO.Path/Combine
                   (or parent (System.IO.Path/GetTempPath))
                   (str ".clr-meta-stage1-work." nonce))
        runtime (current-runtime)]
    (when-not (System.IO.Directory/Exists source-root)
      (fail! :compiler-source-root-missing
             "CLR_META_SOURCE_ROOT does not name the clr-meta source tree" {}))
    (when (or (nil? parent) (= "" parent))
      (fail! :output-parent-missing
             "Stage1 bundle output must have a parent directory" {}))
    (validate-paths! source-root profile-path plan-path source-path output)
    (when (or (System.IO.File/Exists output)
              (System.IO.Directory/Exists output))
      (fail! :output-exists
             "Stage1 bundle refuses to replace an existing output"
             {:output output}))
    (System.IO.Directory/CreateDirectory parent)
    (when (or (System.IO.File/Exists temporary)
              (System.IO.Directory/Exists temporary)
              (System.IO.File/Exists workspace)
              (System.IO.Directory/Exists workspace))
      (fail! :nonce-path-collision
             "fresh Stage1 workspace path unexpectedly exists" {}))
    (System.IO.Directory/CreateDirectory temporary)
    (System.IO.Directory/CreateDirectory workspace)
    (try
      (let [input-root (System.IO.Path/Combine workspace "inputs")
            profile-snapshot (System.IO.Path/Combine input-root "profile.edn")
            plan-snapshot (System.IO.Path/Combine input-root "plan.edn")
            source-snapshot (System.IO.Path/Combine input-root "source.clj")
            seed-source (System.IO.Path/Combine workspace "compiler-source")
            seed-plan (System.IO.Path/Combine workspace "compiler-seed.edn")
            seed-dir (System.IO.Path/Combine temporary "compiler-seed")
            target-dir (System.IO.Path/Combine temporary "target")
            child-cwd (System.IO.Path/Combine workspace "empty-cwd")
            runtime-source-root (System.IO.Path/GetDirectoryName (:main-dll runtime))
            runtime-snapshot (System.IO.Path/Combine workspace "runtime")]
        (System.IO.Directory/CreateDirectory input-root)
        ;; All target semantics and receipt hashes use these same private
        ;; snapshots, closing live-input replacement races.
        (System.IO.File/Copy profile-path profile-snapshot true)
        (System.IO.File/Copy plan-path plan-snapshot true)
        (System.IO.File/Copy source-path source-snapshot true)
        (copy-tree! runtime-source-root runtime-snapshot)
        (let [source-accounting (copy-compiler-source! source-root seed-source)]
        (write-text! seed-plan
                     (str (pr-str {:schema artifact/plan-schema
                                   :entry 'pnix.clr-meta.compiler-stage1
                                   :namespaces compiler-namespaces})
                          "\n"))
        (let [seed-manifest (artifact/build! seed-plan seed-dir seed-source)
              seed-manifest-path (System.IO.Path/Combine seed-dir "manifest.json")]
          ;; Remove the staged compiler source and its bootstrap plan before
          ;; target compilation. The fresh child can load only the AOT seed.
          (delete-tree! seed-source)
          (System.IO.File/Delete seed-plan)
          (System.IO.Directory/CreateDirectory child-cwd)
          (let [runtime-rows (closure-rows runtime-snapshot)
                runtime-closure (artifact/closure-hash runtime-rows)
                child-result (run-seeded-compiler!
                              runtime-snapshot seed-dir
                              profile-snapshot plan-snapshot
                              source-snapshot target-dir child-cwd)
                target-manifest (verify-target!
                                 target-dir (:manifest child-result)
                                 profile-snapshot plan-snapshot source-snapshot)
                self-total (reduce + (map :top-level-forms source-accounting))
                self-supported (reduce + (map :supported source-accounting))
                self-unsupported (reduce + (map :unsupported source-accounting))
                rejection-classes
                (->> source-accounting
                     (map :rejection-classes)
                     (apply merge-with +)
                     (into (sorted-map)))
                compiler-outputs
                (mapv (fn [row]
                        {"path" (str "compiler-seed/" (get row "path"))
                         "sha256" (get row "sha256")})
                      (get seed-manifest "outputs"))
                bundle-paths
                ["compiler-seed/manifest.json"
                 "compiler-seed/pnix.clr_meta.compiler_stage1_core.clj.dll"
                 "compiler-seed/pnix.clr_meta.compiler_stage1.clj.dll"
                 "target/manifest.json"
                 "target/program.dll"
                 "target/program.runtimeconfig.json"]
                actual-bundle-files (relative-files temporary)
                bundle-file-set-check
                (when-not (= (set bundle-paths) actual-bundle-files)
                  (fail! :bundle-file-set
                         "pre-publication Stage1 bundle file set is not exact"
                         {:actual actual-bundle-files}))
                bundle-files
                (mapv (fn [relative]
                        {"path" relative
                         "sha256" (artifact/sha256-file
                                   (System.IO.Path/Combine
                                    temporary
                                    (str/replace relative "/"
                                                 (str System.IO.Path/DirectorySeparatorChar))))})
                      bundle-paths)
                runtime-closure-after
                (artifact/closure-hash (closure-rows runtime-snapshot))
                runtime-snapshot-check
                (when-not (= runtime-closure runtime-closure-after)
                  (fail! :runtime-snapshot-mutated
                         "pinned runtime snapshot changed during target compilation"
                         {:before runtime-closure :after runtime-closure-after}))
                manifest
                {"bootstrap"
                 {"compiler" "pinned-host-clojureclr"
                  "environment_policy" "clear-then-allowlist"
                  "host_executable" "current-process-absolute"
                  "manifest_sha256" (artifact/sha256-file seed-manifest-path)
                  "use" "seed-only"}
                 "boundaries"
                 {"compiler_implementation_source_at_target" false
                  "host_compiler_use" "seed-and-pinned-runtime-startup"
                  "pinned_runtime_startup_source_compilation" true
                  "standalone_source_free_distribution" false
                  "target_form_host_compiler_fallback" false
                  "target_form_host_eval_fallback" false
                  "target_source_evaluator" false}
                 "bundle_closure_sha256" (artifact/closure-hash bundle-files)
                 "bundle_files" bundle-files
                 "claim_scope" "profile-qualified-clojure-form-to-managed-pe"
                 "compiler_entry" compiler-entry
                 "compiler_output_closure_sha256"
                 (artifact/closure-hash compiler-outputs)
                 "compiler_outputs" compiler-outputs
                 "compiler_self_reproduction" false
                 "compiler_source_closure_sha256"
                 (get seed-manifest "source_closure_sha256")
                 "compiler_sources" (get seed-manifest "sources")
                 "compiler_stage15_n" false
                 "compiler_stage2" false
                 "emitter"
                 {"api" "System.Reflection.Emit.PersistedAssemblyBuilder"
                  "semantic_owner" "clojureclr-stage1-compiler"}
                 "il_fixed_point" false
                 "input_kind" "strict-edn-one-form"
                 "profile_id" (get target-manifest "profile_id")
                 "profile_sha256" (artifact/sha256-file profile-snapshot)
                 "raw_artifact_reproducibility" false
                 "runtime_dependencies"
                 ["CoreCLR/.NET BCL"
                  "pinned ClojureCLR runtime/startup compiler"
                  "clojure.edn strict reader (frontend TCB)"]
                 "runtime_isolation"
                 {"allowed_environment_keys"
                  ["CLOJURE_COMPILE_PATH" "CLOJURE_LOAD_PATH"
                   "CLR_META_SOURCE_ROOT" "HOME" "LANG" "LC_ALL" "TMPDIR"]
                  "compiler_lookup_roots" (:loader-roots child-result)
                  "environment_policy" "clear-then-allowlist"
                  "sanitized_environment_keys" sanitized-environment-keys}
                 "runtime_snapshot"
                 {"closure_sha256" runtime-closure
                  "file_count" (count runtime-rows)
                  "files" runtime-rows
                  "unchanged_after_target" true}
                 "schema" manifest-schema
                 "self_source_accounting"
                 {"classified" self-total
                  "compiler_source_within_stage1_profile" (= self-total self-supported)
                  "coverage_complete" true
                  "rejection_classes" rejection-classes
                  "stage2_ready" false
                  "supported" self-supported
                  "total" self-total
                  "unit" "top-level-clojure-reader-form"
                  "unsupported" self-unsupported}
                 "stage" 1
                 "target"
                 {"artifact" target-manifest
                  "framework" "net10.0"
                  "plan_sha256" (artifact/sha256-file plan-snapshot)
                  "source_sha256" (artifact/sha256-file source-snapshot)}
                 "toolchain"
                 {"clojure_main_sha256"
                  (artifact/sha256-file
                   (System.IO.Path/Combine runtime-snapshot "Clojure.Main.dll"))
                  "clojure_runtime_sha256"
                  (artifact/sha256-file
                   (System.IO.Path/Combine runtime-snapshot "Clojure.dll"))
                  "clojure_version" (clojure-version)
                  "coreclr_version" (str System.Environment/Version)
                  "dotnet_host_sha256" (artifact/sha256-file (:dotnet runtime))}}]
            (write-text! (System.IO.Path/Combine temporary "manifest.json")
                         (str (artifact/manifest-json manifest) "\n"))
            (when-not (= (conj (set bundle-paths) "manifest.json")
                         (relative-files temporary))
              (fail! :published-bundle-file-set
                     "final Stage1 bundle file set is not exact" {}))
            (publish-directory! temporary output)
            manifest))))
      (finally
        (delete-tree! temporary)
        (delete-tree! workspace)))))
