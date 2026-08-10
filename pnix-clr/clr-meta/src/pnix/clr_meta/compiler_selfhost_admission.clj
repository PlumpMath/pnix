(ns pnix.clr-meta.compiler-selfhost-admission
  "Thin C0/C1 selfhost admission coordinator and deterministic receipt CLI.

  Contract validation and source semantics remain separate owners. This layer
  binds their results to exact input bytes and atomically publishes evidence."
  (:require [clojure.string :as str]
            [pnix.clr-meta.compiler-selfhost-contract :as contract]
            [pnix.clr-meta.compiler-selfhost-source :as source]
            [pnix.clr-meta.runtime-artifact :as artifact]))

(def receipt-schema "pnix.clr-meta.compiler-selfhost-admission.v1")
(def ^:private utf8 (System.Text.UTF8Encoding. false true))

(defn- fail!
  [class message evidence]
  (throw (ex-info message
                  (merge {:schema :pnix.clr-meta.compiler-selfhost-admission-error.v1
                          :phase :compiler-selfhost-admission
                          :class class}
                         evidence))))

(defn- reparse-point?
  [path]
  (not (zero? (bit-and (int (System.IO.File/GetAttributes path))
                       (int System.IO.FileAttributes/ReparsePoint)))))

(defn- physical-path
  [path]
  (let [full (System.IO.Path/GetFullPath (str path))
        root (System.IO.Path/GetPathRoot full)
        relative (.Substring full (.Length root))
        parts (remove empty? (str/split relative #"[\\/]+"))]
    (loop [current root remaining parts]
      (if-let [part (first remaining)]
        (let [candidate (System.IO.Path/Combine current part)
              file? (System.IO.File/Exists candidate)
              directory? (System.IO.Directory/Exists candidate)
              resolved (when (and (or file? directory?)
                                  (reparse-point? candidate))
                         (.ResolveLinkTarget
                          (if directory?
                            (System.IO.DirectoryInfo. candidate)
                            (System.IO.FileInfo. candidate))
                          true))]
          (recur (if resolved (.FullName resolved) candidate)
                 (next remaining)))
        (System.IO.Path/GetFullPath current)))))

(defn- path-identity
  [path]
  (let [full (physical-path path)]
    (if (or (System.OperatingSystem/IsWindows)
            (System.OperatingSystem/IsMacOS))
      (.ToUpperInvariant full)
      full)))

(defn- require-file!
  [path kind]
  (let [full (System.IO.Path/GetFullPath (str path))]
    (when-not (System.IO.File/Exists full)
      (fail! :missing-input "selfhost admission input file is unavailable"
             {:kind kind :path full}))
    (when (reparse-point? full)
      (fail! :input-reparse-point
             "selfhost admission input itself must not be a link"
             {:kind kind :path full}))
    ;; Resolve every existing ancestor once, then read/hash the physical path.
    ;; This accepts stable platform aliases such as macOS /var -> /private/var
    ;; without leaving later reads exposed to ancestor-link retargeting.
    (physical-path full)))

(defn- require-distinct-inputs!
  [paths]
  (let [identities (map path-identity paths)]
    (when-not (= (count paths) (count (distinct identities)))
      (fail! :input-path-collision
             "contract, profile, plan, and source must be distinct files"
             {}))))

(defn analyze!
  "Read and statically admit one exact compiler-selfhost C0/C1 family.

  The returned value is a deterministic JSON-domain receipt. Admitted source
  is never compiled, loaded, or evaluated."
  [contract-path profile-path plan-path source-path]
  (let [paths {:contract (require-file! contract-path :contract)
               :profile (require-file! profile-path :profile)
               :plan (require-file! plan-path :plan)
               :source (require-file! source-path :source)}
        paths-checked (require-distinct-inputs!
                       (mapv paths [:contract :profile :plan :source]))
        input-rows (mapv (fn [kind]
                           {"path" (name kind)
                            "sha256" (artifact/sha256-file (get paths kind))})
                         [:contract :profile :plan :source])
        contract-value (contract/validate-contract
                        (contract/read-one! (:contract paths) :contract)
                        (:contract paths))
        profile-value (contract/validate-profile
                       (contract/read-one! (:profile paths) :profile)
                       contract-value (:profile paths))
        plan-value (contract/validate-plan
                    (contract/read-one! (:plan paths) :plan)
                    contract-value profile-value (:plan paths))
        forms (source/read-forms! (:source paths)
                                  (get-in profile-value
                                          [:limits :max-source-bytes])
                                  (get-in profile-value
                                          [:limits :max-top-level-forms]))
        source-analysis (source/analyze! forms contract-value profile-value)
        final-input-rows (mapv (fn [kind]
                                 {"path" (name kind)
                                  "sha256" (artifact/sha256-file
                                            (get paths kind))})
                               [:contract :profile :plan :source])
        inputs-stable (when-not (= input-rows final-input-rows)
                        (fail! :input-mutated
                               "selfhost admission input changed during analysis"
                               {:before input-rows :after final-input-rows}))
        claims (:claims contract-value)]
    {"accounting" (:accounting source-analysis)
     "admitted" true
     "checkpoint" "c1-source-admission"
     "compiled_language_id" (name (get-in contract-value
                                            [:language-invariant
                                             :compiled-language-id]))
     "compiler_executable" false
     "compiler_self_reproduction" false
     "compiler_stage1_artifact" false
     "compiler_stage2" false
     "compiler_stage3" false
     "contract_id" (name (:id contract-value))
     "entry" (str (get-in contract-value [:source :entry]))
     "executable_stage" false
     "family" (name (:family contract-value))
     "fixed_point" (get claims :fixed_point)
     "il_fixed_point" false
     "input_closure_sha256" (artifact/closure-hash input-rows)
     "inputs" input-rows
     "lowering_coverage" (:lowering-coverage source-analysis)
     "mutation_sites" (:mutation-sites source-analysis)
     "mutation_propagation" false
     "nodes" (:rows source-analysis)
     "plan_id" (name (:id plan-value))
     "profile_id" (name (:id profile-value))
     "raw_artifact_reproducibility" false
     "raw_reproducibility" (get claims :raw_reproducibility)
     "schema" receipt-schema
     "seed_bindings" (:seed-bindings source-analysis)
     "self_reproduction" (get claims :self_reproduction)
     "same_source_recompile_executed" false
     "source_language_equals_compiled_language"
     (get-in contract-value [:language-invariant
                             :source-language-equals-compiled-language])
     "source_language_id" (name (get-in contract-value
                                          [:language-invariant
                                           :source-language-id]))
     "source_namespace" (str (get-in contract-value [:source :namespace]))
     "stage1_artifact" (get claims :stage1_artifact)
     "stage2" (get claims :stage2)}))

(defn- write-receipt!
  [path receipt input-paths]
  (let [output (physical-path path)
        output-id (path-identity output)
        temporary (str output ".building."
                       (.ToString (System.Guid/NewGuid) "N"))]
    (when (some #(= output-id (path-identity %)) input-paths)
      (fail! :output-input-collision
             "admission receipt output must not replace an input"
             {:output output}))
    (when (or (System.IO.File/Exists output)
              (System.IO.Directory/Exists output))
      (fail! :output-exists
             "admission receipt output must be a new path"
             {:output output}))
    (let [parent (System.IO.Path/GetDirectoryName output)]
      (when (or (nil? parent) (= "" parent))
        (fail! :output-parent-missing
               "admission receipt output needs a parent directory"
               {:output output}))
      (System.IO.Directory/CreateDirectory parent))
    (try
      (System.IO.File/WriteAllText temporary
                                  (str (artifact/manifest-json receipt) "\n")
                                  utf8)
      (System.IO.File/Move temporary output)
      (finally
        (when (System.IO.File/Exists temporary)
          (System.IO.File/Delete temporary))))))

(defn -main
  [& arguments]
  (when-not (contains? #{4 5} (count arguments))
    (fail! :cli-arity
           "usage: compiler-selfhost-admission CONTRACT PROFILE PLAN SOURCE [OUTPUT]"
           {:actual (count arguments)}))
  (let [[contract-path profile-path plan-path source-path output] arguments
        receipt (analyze! contract-path profile-path plan-path source-path)]
    (if output
      (write-receipt! output receipt
                      (mapv #(System.IO.Path/GetFullPath (str %))
                            [contract-path profile-path plan-path source-path]))
      (println (artifact/manifest-json receipt)))))
