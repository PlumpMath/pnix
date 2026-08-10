(ns pnix.clr-meta.runtime-artifact
  "PNIX-agnostic ClojureCLR AOT artifact producer.

  The caller owns the namespace plan and source tree.  clr-meta validates that
  closed input, asks the pinned host ClojureCLR compiler to AOT-compile the
  namespaces in the declared order, and publishes a hash-bound artifact
  directory.  This is deliberately a host-compiler backend, not a claim that
  the current evaluator tower is a self-reproducing compiler or an IL fixed
  point."
  (:require [clojure.edn :as edn]
            [clojure.set :as set]
            [clojure.string :as str]))

(def plan-schema :pnix.clr-meta.runtime-artifact-plan.v1)
(def manifest-schema "pnix.clr-meta.runtime-artifact.v1")
(def ^:private aot-environment-allowlist
  ["DOTNET_ROOT" "DOTNET_ROOT_X64" "HOME" "LANG" "LC_ALL"
   "TEMP" "TMP" "TMPDIR" "USERPROFILE"])
(def producer "clr-meta")
(def backend "host-clojureclr-aot")
(def target "net10.0")

(def ^:private plan-keys #{:schema :entry :namespaces})
(def ^:private namespace-pattern
  #"[A-Za-z_][A-Za-z0-9_-]*(\.[A-Za-z_][A-Za-z0-9_-]*)*")
(def ^:private utf8 (System.Text.UTF8Encoding. false true))

(defn- fail!
  [class message evidence]
  (throw (ex-info message
                  (merge {:schema :pnix.clr-meta.runtime-artifact-error.v1
                          :phase :artifact-build
                          :class class}
                         evidence))))

(defn sha256-bytes
  "Return a lower-case SHA-256 digest for a CLR byte array."
  [bytes]
  (with-open [digest (System.Security.Cryptography.SHA256/Create)]
    (-> (System.Convert/ToHexString (.ComputeHash digest bytes))
        (.ToLowerInvariant))))

(defn sha256-string
  "Return the SHA-256 digest of a string's strict UTF-8 bytes."
  [value]
  (sha256-bytes (.GetBytes utf8 (str value))))

(defn sha256-file
  "Return the SHA-256 digest of the exact bytes in path."
  [path]
  (let [path (System.IO.Path/GetFullPath (str path))]
    (when-not (System.IO.File/Exists path)
      (fail! :missing-file "artifact input file is unavailable" {:path path}))
    (with-open [stream (System.IO.File/OpenRead path)
                digest (System.Security.Cryptography.SHA256/Create)]
      (-> (System.Convert/ToHexString (.ComputeHash digest stream))
          (.ToLowerInvariant)))))

(defn closure-hash
  "Hash ordered `{path, sha256}` rows without binding an absolute host path."
  [rows]
  (sha256-string
   (apply str
          (map (fn [row]
                 (str (or (:sha256 row) (get row "sha256"))
                      "  "
                      (or (:path row) (get row "path"))
                      "\n"))
               rows))))

(defn- strict-read-edn
  [source path]
  (let [eof (Object.)]
    (with-open [reader (clojure.lang.LineNumberingTextReader.
                        (System.IO.StringReader. source))]
      (let [value (edn/read {:eof eof} reader)
            trailing (edn/read {:eof eof} reader)]
        (when (identical? value eof)
          (fail! :empty-plan "artifact plan is empty" {:path path}))
        (when-not (identical? trailing eof)
          (fail! :trailing-plan-data
                 "artifact plan must contain exactly one EDN value"
                 {:path path}))
        value))))

(defn read-plan
  "Read exactly one strict EDN value from plan-path."
  [plan-path]
  (let [path (System.IO.Path/GetFullPath (str plan-path))]
    (when-not (System.IO.File/Exists path)
      (fail! :missing-plan "artifact plan is unavailable" {:path path}))
    (try
      (strict-read-edn (System.IO.File/ReadAllText path utf8) path)
      (catch clojure.lang.ExceptionInfo cause
        (throw cause))
      (catch Exception cause
        (fail! :invalid-plan-edn
               "artifact plan is not valid strict EDN"
               {:path path :cause-type (str (type cause))})))))

(defn namespace-source-path
  "Project a validated namespace symbol to its ClojureCLR source path."
  [namespace]
  (-> (str namespace)
      (str/replace "-" "_")
      (str/replace "." "/")
      (str ".clj")))

(defn namespace-output-path
  "Project a validated namespace symbol to ClojureCLR's AOT DLL name."
  [namespace]
  (str (str/replace (str namespace) "-" "_") ".clj.dll"))

(defn validate-plan
  "Validate the closed artifact plan.

  With source-root, also require that its complete `.clj` file set is exactly
  the namespace set declared by the plan and attach ordered source hashes."
  ([plan]
   (when-not (map? plan)
     (fail! :plan-not-map "artifact plan must be a map" {}))
   (when-not (= plan-keys (set (keys plan)))
     (fail! :plan-key-set
            "artifact plan keys must match the v1 schema exactly"
            {:expected (vec (sort plan-keys))
             :actual (vec (sort (keys plan)))}))
   (when-not (= plan-schema (:schema plan))
     (fail! :plan-schema "artifact plan schema is unsupported"
            {:expected plan-schema :actual (:schema plan)}))
   (let [entry (:entry plan)
         namespaces (:namespaces plan)]
     (when-not (symbol? entry)
       (fail! :entry-not-symbol "artifact entry must be a namespace symbol"
              {:entry entry}))
     (when-not (vector? namespaces)
       (fail! :namespaces-not-vector
              "artifact namespaces must be a non-empty ordered vector"
              {}))
     (when (empty? namespaces)
       (fail! :empty-namespaces
              "artifact namespaces must be a non-empty ordered vector"
              {}))
     (doseq [namespace namespaces]
       (when-not (and (symbol? namespace)
                      (re-matches namespace-pattern (str namespace)))
         (fail! :invalid-namespace
                "artifact namespace is not a safe dotted namespace symbol"
                {:namespace namespace})))
     (when-not (and (re-matches namespace-pattern (str entry))
                    (some #{entry} namespaces))
       (fail! :entry-not-declared
              "artifact entry must occur in the namespace vector"
              {:entry entry}))
     (when-not (= (count namespaces) (count (distinct namespaces)))
       (fail! :duplicate-namespace
              "artifact namespace vector contains duplicates"
              {}))
     (let [source-paths (mapv namespace-source-path namespaces)
           output-paths (mapv namespace-output-path namespaces)]
       (when-not (= (count source-paths) (count (distinct source-paths)))
         (fail! :source-path-collision
                "artifact namespaces collide after CLR source munging"
                {}))
       (when-not (= (count output-paths) (count (distinct output-paths)))
         (fail! :output-path-collision
                "artifact namespaces collide after CLR output munging"
                {}))
       {:schema plan-schema
        :entry entry
        :namespaces namespaces
        :source-paths source-paths
        :output-paths output-paths})))
  ([plan source-root]
   (let [validated (validate-plan plan)
         root (System.IO.Path/GetFullPath (str source-root))]
     (when-not (System.IO.Directory/Exists root)
       (fail! :missing-source-root
              "artifact source root is unavailable"
              {:source-root root}))
     (let [expected (set (:source-paths validated))
           actual (->> (System.IO.Directory/GetFiles
                        root "*.clj" System.IO.SearchOption/AllDirectories)
                       (map #(System.IO.Path/GetRelativePath root %))
                       (map #(str/replace % "\\" "/"))
                       set)]
       (when-not (= expected actual)
         (fail! :source-set-mismatch
                "artifact source root must contain exactly the declared namespaces"
                {:missing (vec (sort (set/difference expected actual)))
                 :extra (vec (sort (set/difference actual expected)))}))
       (assoc validated
              :source-root root
              :sources
              (mapv (fn [path]
                      {:path path
                       :sha256 (sha256-file
                                (System.IO.Path/Combine root
                                                        (str/replace path "/" (str System.IO.Path/DirectorySeparatorChar))))})
                    (:source-paths validated)))))))

(defn- json-string
  [value]
  (str "\""
       (apply str
              (map (fn [character]
                     (case character
                       \" "\\\""
                       \\ "\\\\"
                       \backspace "\\b"
                       \formfeed "\\f"
                       \newline "\\n"
                       \return "\\r"
                       \tab "\\t"
                       (if (< (int character) 32)
                         (format "\\u%04x" (int character))
                         (str character))))
                   (str value)))
       "\""))

(declare json-encode)

(defn- json-map
  [value]
  (str "{"
       (str/join ","
                 (map (fn [[key item]]
                        (when-not (string? key)
                          (fail! :manifest-key-not-string
                                 "manifest JSON keys must be strings"
                                 {:key key}))
                        (str (json-string key) ":" (json-encode item)))
                      (sort-by key value)))
       "}"))

(defn json-encode
  "Encode the closed manifest value domain as deterministic JSON."
  [value]
  (cond
    (nil? value) "null"
    (string? value) (json-string value)
    (true? value) "true"
    (false? value) "false"
    (integer? value) (str value)
    (map? value) (json-map value)
    (sequential? value) (str "[" (str/join "," (map json-encode value)) "]")
    :else (fail! :manifest-value-unsupported
                 "manifest contains a value outside the JSON contract"
                 {:value-type (str (type value))})))

(defn manifest-json
  "Return deterministic JSON bytes-as-text for a build! manifest map."
  [manifest]
  (json-encode manifest))

(defn- delete-tree!
  [path]
  (when (System.IO.Directory/Exists path)
    (System.IO.Directory/Delete path true)))

(defn- path-comparison
  []
  (if (System.OperatingSystem/IsWindows)
    System.StringComparison/OrdinalIgnoreCase
    System.StringComparison/Ordinal))

(defn- normalized-path
  [path]
  (System.IO.Path/TrimEndingDirectorySeparator
   (System.IO.Path/GetFullPath (str path))))

(defn- same-path?
  [left right]
  (.Equals (normalized-path left)
           (normalized-path right)
           (path-comparison)))

(defn- path-below?
  [candidate ancestor]
  (let [candidate (normalized-path candidate)
        ancestor (normalized-path ancestor)
        prefix (str ancestor System.IO.Path/DirectorySeparatorChar)]
    (.StartsWith candidate prefix (path-comparison))))

(defn- paths-overlap?
  [left right]
  (or (same-path? left right)
      (path-below? left right)
      (path-below? right left)))

(defn- validate-disjoint-paths!
  [plan-path source-root output]
  (doseq [[left-kind left right-kind right]
          [[:plan plan-path :source-root source-root]
           [:plan plan-path :output output]
           [:source-root source-root :output output]]]
    (when (paths-overlap? left right)
      (fail! :path-overlap
             "artifact plan, source root, and output must be pairwise disjoint"
             {:left-kind left-kind
              :left (normalized-path left)
              :right-kind right-kind
              :right (normalized-path right)}))))

(defn- compile-namespaces!
  [namespaces source-root output-root]
  (let [entry-assembly (System.Reflection.Assembly/GetEntryAssembly)
        main-dll (when entry-assembly (.Location entry-assembly))
        dotnet System.Environment/ProcessPath
        expression (str "(doseq [namespace (quote ["
                        (str/join " " (map str namespaces))
                        "])] (compile namespace))")]
    (when (or (nil? main-dll)
              (= "" main-dll)
              (not (System.IO.File/Exists main-dll)))
      (fail! :aot-host-runtime-missing
             "current Clojure.Main assembly is unavailable for isolated AOT"
             {}))
    (when (or (nil? dotnet)
              (= "" dotnet)
              (not (System.IO.File/Exists dotnet)))
      (fail! :aot-dotnet-host-missing
             "absolute current dotnet executable is unavailable for isolated AOT"
             {}))
    (try
      (let [start-info (doto (System.Diagnostics.ProcessStartInfo. dotnet)
                         (.set_UseShellExecute false)
                         (.set_CreateNoWindow true)
                         (.set_RedirectStandardOutput true)
                         (.set_RedirectStandardError true)
                         (.set_WorkingDirectory source-root))
            arguments (.get_ArgumentList start-info)
            environment (.get_Environment start-info)
            allowed-environment
            (mapv (fn [key]
                    [key (System.Environment/GetEnvironmentVariable key)])
                  aot-environment-allowlist)]
        ;; ArgumentList is intentional: no namespace, path, or expression is
        ;; ever reparsed by a shell.
        (.Add arguments main-dll)
        (.Add arguments "-e")
        (.Add arguments expression)
        ;; Replace rather than append. The child starts in source-root, sees no
        ;; clr-meta source/already-loaded parent namespace, and cannot inherit
        ;; CLR startup-hook/profiler/dependency injection variables.
        (.Clear environment)
        (doseq [[key value] allowed-environment
                :when (and value (not (str/blank? value)))]
          (.set_Item environment key value))
        (.set_Item environment "CLOJURE_LOAD_PATH" source-root)
        (.set_Item environment "CLOJURE_COMPILE_PATH" output-root)
        (with-open [process (System.Diagnostics.Process/Start start-info)]
          (let [stdout* (future (.ReadToEnd (.get_StandardOutput process)))
                stderr* (future (.ReadToEnd (.get_StandardError process)))]
            (.WaitForExit process)
            (let [stdout @stdout*
                  stderr @stderr*
                  exit (.get_ExitCode process)]
              (when-not (zero? exit)
                (fail! :aot-child-failed
                       "isolated host ClojureCLR AOT compiler failed"
                       {:exit exit :stdout stdout :stderr stderr}))))))
      (catch clojure.lang.ExceptionInfo cause
        (throw cause))
      (catch Exception cause
        (fail! :aot-child-start-failed
               "isolated host ClojureCLR AOT compiler could not start"
               {:cause-type (str (type cause))
                :message (.Message cause)})))))

(defn- output-rows
  [output-root expected-paths]
  (let [actual (->> (System.IO.Directory/GetFiles
                     output-root "*" System.IO.SearchOption/AllDirectories)
                    (map #(System.IO.Path/GetRelativePath output-root %))
                    (map #(str/replace % "\\" "/"))
                    set)
        expected (set expected-paths)]
    (when-not (= expected actual)
      (fail! :aot-output-set-mismatch
             "host ClojureCLR AOT outputs do not match the declared namespaces"
             {:missing (vec (sort (set/difference expected actual)))
              :extra (vec (sort (set/difference actual expected)))}))
    (mapv (fn [path]
            {:path path
             :sha256 (sha256-file (System.IO.Path/Combine output-root path))})
          expected-paths)))

(defn- publish-directory!
  [temporary output]
  (let [backup (str output ".previous." (.ToString (System.Guid/NewGuid) "N"))
        had-output (System.IO.Directory/Exists output)]
    (when (System.IO.File/Exists output)
      (fail! :output-is-file
             "artifact output path names an existing file"
             {:output output}))
    (when had-output
      (System.IO.Directory/Move output backup))
    (try
      (System.IO.Directory/Move temporary output)
      (when had-output
        (delete-tree! backup))
      (catch Exception cause
        (when (and had-output
                   (not (System.IO.Directory/Exists output))
                   (System.IO.Directory/Exists backup))
          (System.IO.Directory/Move backup output))
        (throw cause)))))

(defn build!
  "Build and atomically publish one closed ClojureCLR runtime artifact.

  `plan-path` supplies all product-specific namespace identities.  This
  generic clr-meta producer never hardcodes or interprets them."
  [plan-path output-dir source-root]
  (let [plan-path (System.IO.Path/GetFullPath (str plan-path))
        output (System.IO.Path/GetFullPath (str output-dir))
        source-root (System.IO.Path/GetFullPath (str source-root))
        validated (validate-plan (read-plan plan-path) source-root)
        parent (System.IO.Path/GetDirectoryName output)
        temporary (str output ".building." (.ToString (System.Guid/NewGuid) "N"))]
    ;; This precedes every mkdir/move/delete.  In particular, an output that
    ;; contains the source or plan must never be renamed away during publish,
    ;; and an output below the source must never mutate the hashed closure.
    (validate-disjoint-paths! plan-path source-root output)
    (when (or (nil? parent) (= "" parent))
      (fail! :output-has-no-parent
             "artifact output must have a parent directory"
             {:output output}))
    (System.IO.Directory/CreateDirectory parent)
    (delete-tree! temporary)
    (System.IO.Directory/CreateDirectory temporary)
    (try
      (compile-namespaces! (:namespaces validated) source-root temporary)
      (let [outputs (output-rows temporary (:output-paths validated))
            manifest
            {"backend" backend
             "compiler_self_reproduction" false
             "compiler_stage15_n" false
             "entry" (str (:entry validated))
             "evaluator_generations" 3
             "il_fixed_point" false
             "output_closure_sha256" (closure-hash outputs)
             "outputs" (mapv (fn [{:keys [path sha256]}]
                                {"path" path "sha256" sha256})
                              outputs)
             "plan_sha256" (sha256-file plan-path)
             "producer" producer
             "schema" manifest-schema
             "source_closure_sha256" (closure-hash (:sources validated))
             "sources" (mapv (fn [{:keys [path sha256]}]
                                {"path" path "sha256" sha256})
                              (:sources validated))
             "target" target}]
        (System.IO.File/WriteAllText
         (System.IO.Path/Combine temporary "manifest.json")
         (str (manifest-json manifest) "\n")
         utf8)
        ;; manifest.json is intentionally outside output_closure_sha256: the
        ;; manifest binds the AOT files and cannot recursively hash itself.
        (publish-directory! temporary output)
        manifest)
      (finally
        (delete-tree! temporary)))))
