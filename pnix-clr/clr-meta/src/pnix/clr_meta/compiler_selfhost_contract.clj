(ns pnix.clr-meta.compiler-selfhost-contract
  "Strict EDN and exact C0/C1 contract/profile/plan validation.

  This namespace owns only the declarative compiler-family boundary. It does
  not read or classify compiler source and does not publish receipts."
  (:require [clojure.edn :as edn]
            [clojure.set :as set]
            [clojure.string :as str]))

(def contract-schema :pnix.clr-meta.compiler-selfhost-contract.v1)
(def profile-schema :pnix.clr-meta.compiler-selfhost-profile.v1)
(def plan-schema :pnix.clr-meta.compiler-selfhost-plan.v1)

(def ^:private utf8 (System.Text.UTF8Encoding. false true))
(def ^:private simple-symbol-pattern
  #"[A-Za-z_*!+?<>=$%-][A-Za-z0-9_*!+?<>=$%-]*")
(def ^:private namespace-symbol-pattern
  #"[A-Za-z_][A-Za-z0-9_-]*(\.[A-Za-z_][A-Za-z0-9_-]*)+")
(def ^:private claim-keys
  #{:stage1_artifact :stage2 :self_reproduction :fixed_point
    :raw_reproducibility})
(def ^:private contract-keys
  #{:schema :id :family :phase :source :support-abi :forbidden
    :language-invariant :compiler-abi :intrinsics :lowering-coverage
    :mutation-sites :claim-boundary :claims})
(def ^:private profile-keys
  #{:schema :id :family :phase :envelope :top-level :expressions :literals
    :language-invariant :calls :lowering-coverage :limits :forbidden :claims})
(def ^:private plan-keys
  #{:schema :id :family :contract :profile :source :stages
    :compiler-abi :future-chain :mutation-checks :claims})

(defn- fail!
  [class message evidence]
  (throw (ex-info message
                  (merge {:schema :pnix.clr-meta.compiler-selfhost-admission-error.v1
                          :phase :compiler-selfhost-admission
                          :class class}
                         evidence))))

(defn- reject-reader-sugar!
  [text kind path]
  (loop [index 0 in-string? false escaped? false in-comment? false]
    (when (< index (count text))
      (let [character (.get_Chars text index)]
        (cond
          in-comment?
          (recur (inc index) false false (not (or (= character \newline)
                                                  (= character \return))))

          in-string?
          (cond
            escaped? (recur (inc index) true false false)
            (= character \\) (recur (inc index) true true false)
            (= character \") (recur (inc index) false false false)
            :else (recur (inc index) true false false))

          (= character \;) (recur (inc index) false false true)
          (= character \") (recur (inc index) true false false)
          (contains? #{\^ \` \~ \@ \#} character)
          (fail! :edn-reader-sugar-not-admitted
                 "contract EDN may not contain metadata or reader-macro sugar"
                 {:kind kind :path path :offset index
                  :character (str character)})
          :else (recur (inc index) false false false))))))

(defn read-one!
  "Read exactly one strict, tag-free EDN value from path."
  [path kind]
  (let [eof (Object.)
        text (System.IO.File/ReadAllText path utf8)]
    (reject-reader-sugar! text kind path)
    (try
      (with-open [reader (clojure.lang.LineNumberingTextReader.
                          (System.IO.StringReader. text))]
        (let [reject-tag (fn [tag _]
                           (fail! :tagged-edn-not-admitted
                                  "tagged values are outside the exact EDN contracts"
                                  {:kind kind :path path :tag tag}))
              value (edn/read {:eof eof :readers {} :default reject-tag} reader)
              trailing (edn/read {:eof eof :readers {} :default reject-tag} reader)]
          (when (identical? eof value)
            (fail! :empty-edn "selfhost contract EDN is empty"
                   {:kind kind :path path}))
          (when-not (identical? eof trailing)
            (fail! :trailing-edn
                   "selfhost contract EDN must contain exactly one value"
                   {:kind kind :path path}))
          value))
      (catch clojure.lang.ExceptionInfo cause
        (throw cause))
      (catch Exception cause
        (fail! :invalid-edn "selfhost contract input is not strict EDN"
               {:kind kind :path path :cause-type (str (type cause))})))))

(defn- keys!
  [value expected kind path]
  (when-not (map? value)
    (fail! :map-required "versioned selfhost contract value must be a map"
           {:kind kind :path path :value-type (str (type value))}))
  (let [actual (set (keys value))]
    (when-not (= expected actual)
      (fail! :key-set-mismatch
             "selfhost contract map keys must equal the versioned schema"
             {:kind kind :path path
              :expected (vec (sort expected))
              :actual (vec (sort actual))})))
  value)

(defn local-symbol?
  [value]
  (and (symbol? value)
       (nil? (namespace value))
       (boolean (re-matches simple-symbol-pattern (name value)))
       (not (contains? '#{& . /} value))))

(defn- namespace-symbol?
  [value]
  (and (symbol? value)
       (nil? (namespace value))
       (boolean (re-matches namespace-symbol-pattern (name value)))))

(defn- false-claims!
  [claims kind]
  (keys! claims claim-keys kind nil)
  (doseq [[claim value] claims]
    (when-not (false? value)
      (fail! :claim-not-false
             "unproven compiler claims must remain explicitly false"
             {:kind kind :claim claim :actual value})))
  claims)

(defn- call-rows!
  [rows kind qualified?]
  (when-not (and (vector? rows) (seq rows))
    (fail! :call-table-not-vector
           "static call allowlist must be a nonempty vector"
           {:kind kind}))
  (doseq [[index row] (map-indexed vector rows)]
    (keys! row #{:symbol :arity} kind index)
    (when-not (and (symbol? (:symbol row))
                   (if qualified?
                     (some? (namespace (:symbol row)))
                     (local-symbol? (:symbol row)))
                   (integer? (:arity row))
                   (not (neg? (:arity row))))
      (fail! :static-call-shape
             "static calls require the admitted symbol class and one arity"
             {:kind kind :index index :row row})))
  (when-not (= (count rows) (count (distinct (map :symbol rows))))
    (fail! :duplicate-call-symbol
           "static call symbols must be unique per allowlist"
           {:kind kind}))
  rows)

(defn- kernel-call-rows!
  [rows]
  (when-not (and (vector? rows) (seq rows))
    (fail! :kernel-call-table
           "kernel call allowlist must be a nonempty vector" {}))
  (doseq [[index row] (map-indexed vector rows)]
    (let [entry? (contains? row :role)
          expected (if entry? #{:symbol :arity :role} #{:symbol :arity})]
      (keys! row expected :profile-kernel-call index)
      (when-not (and (local-symbol? (:symbol row))
                     (integer? (:arity row)) (not (neg? (:arity row)))
                     (if entry? (= :entry (:role row)) true))
        (fail! :kernel-call-shape "kernel call row is malformed"
               {:index index :row row}))))
  (when-not (= 1 (count (filter #(= :entry (:role %)) rows)))
    (fail! :kernel-entry-count
           "kernel call allowlist requires exactly one entry role" {}))
  (when-not (= (count rows) (count (distinct (map :symbol rows))))
    (fail! :duplicate-kernel-call "kernel call symbols must be unique" {}))
  rows)

(defn validate-contract
  [contract path]
  (keys! contract contract-keys :contract path)
  (when-not (= contract-schema (:schema contract))
    (fail! :contract-schema-mismatch "unsupported selfhost contract schema"
           {:expected contract-schema :actual (:schema contract)}))
  (doseq [key [:id :family :phase]]
    (when-not (keyword? (get contract key))
      (fail! :contract-identity "contract identity values must be keywords"
             {:key key :actual (get contract key)})))
  (let [source (keys! (:source contract)
                      #{:path :namespace :entry :identity-def :identity-literal
                        :profile-id}
                      :contract-source path)]
    (when-not (and (string? (:path source))
                   (= (:path source) (System.IO.Path/GetFileName (:path source)))
                   (str/ends-with? (:path source) ".clj")
                   (namespace-symbol? (:namespace source))
                   (local-symbol? (:entry source))
                   (local-symbol? (:identity-def source))
                   (string? (:identity-literal source))
                   (not (str/blank? (:identity-literal source)))
                   (keyword? (:profile-id source)))
      (fail! :contract-source-shape "contract source identity is malformed"
             {:source source})))
  (let [invariant (keys! (:language-invariant contract)
                         #{:source-language-id :compiled-language-id
                           :source-language-equals-compiled-language
                           :same-source :same-source-recompilable}
                         :contract-language-invariant path)
        source-id (:source-language-id invariant)
        compiled-id (:compiled-language-id invariant)]
    (when-not (and (keyword? source-id)
                   (= source-id compiled-id
                      (get-in contract [:source :profile-id]))
                   (true? (:source-language-equals-compiled-language invariant))
                   (= (get-in contract [:source :path]) (:same-source invariant))
                   (= :future-executable-gate
                      (:same-source-recompilable invariant)))
      (fail! :language-invariant-mismatch
             "source and compiled language identities must be exactly equal"
             {:invariant invariant
              :source-profile (get-in contract [:source :profile-id])})))
  (let [compiler-abi
        (keys! (:compiler-abi contract)
               #{:kind :entry :arguments :result :output :structured-error
                 :atomicity :partial-output}
               :contract-compiler-abi path)]
    (when-not (= compiler-abi
                 {:kind :in-process-source-pesink
                  :entry (get-in contract [:source :entry])
                  :arguments [:source-text :pe-sink]
                  :result :artifact-descriptor
                  :output :managed-compiler-pe
                  :structured-error :compiler-rejection
                  :atomicity :commit-only-on-pesink-finish
                  :partial-output :forbidden})
      (fail! :compiler-abi-mismatch
             "compiler ABI must equal the closed transactional source-to-PE seam"
             {:actual compiler-abi})))
  (let [support (keys! (:support-abi contract)
                       #{:id :boundary :data-contract :calls}
                       :contract-support-abi path)
        calls (:calls support)]
    (when-not (and (keyword? (:id support)) (keyword? (:boundary support))
                   (vector? calls) (seq calls)
                   (= {:environment :persistent-opaque-map
                       :binding [:kind :target :arity]
                       :missing-binding nil
                       :pe-sink :transactional-opaque-handle}
                      (:data-contract support)))
      (fail! :support-abi-shape "support ABI identity/calls are malformed" {}))
    (doseq [[index row] (map-indexed vector calls)]
      (keys! row #{:symbol :arity :partition :effect :returns}
             :contract-support-call index)
      (when-not (and (symbol? (:symbol row))
                     (some? (namespace (:symbol row)))
                     (str/starts-with? (namespace (:symbol row))
                                       "pnix.clr-meta.compiler-support.")
                     (integer? (:arity row))
                     (not (neg? (:arity row)))
                     (contains? #{:reader :data :pesink} (:partition row))
                     (contains? #{:read :pure :reject :sink-begin
                                  :sink-write :sink-finish}
                                (:effect row))
                     (keyword? (:returns row)))
        (fail! :support-call-shape "support ABI row is malformed"
               {:index index :row row})))
    (when-not (= (count calls) (count (distinct (map :symbol calls))))
      (fail! :duplicate-support-call "support ABI symbols must be unique" {}))
    (doseq [{:keys [partition effect] :as row} calls]
      (when-not (case partition
                  :reader (= :read effect)
                  :data (contains? #{:pure :reject} effect)
                  :pesink (contains? #{:sink-begin :sink-write :sink-finish}
                                     effect)
                  false)
        (fail! :support-effect-partition
               "support effect does not match its partition"
               {:row row}))))
  (let [intrinsics (:intrinsics contract)]
    (when-not (and (vector? intrinsics) (seq intrinsics))
      (fail! :intrinsic-table-shape
             "intrinsics must be a nonempty exact vector" {}))
    (doseq [[index row] (map-indexed vector intrinsics)]
      (keys! row #{:symbol :arity :opcode :semantics}
             :contract-intrinsic index)
      (when-not (and (local-symbol? (:symbol row))
                     (integer? (:arity row)) (not (neg? (:arity row)))
                     (string? (:opcode row)) (not (str/blank? (:opcode row)))
                     (keyword? (:semantics row)))
        (fail! :intrinsic-row-shape "intrinsic row is malformed"
               {:index index :row row})))
    (when-not (= (count intrinsics)
                 (count (distinct (map :symbol intrinsics))))
      (fail! :duplicate-intrinsic "intrinsic symbols must be unique" {})))
  (let [coverage (:lowering-coverage contract)
        required #{:ns :def-constant :def-fn :fn* :if :let* :do :literal
                   :lexical-symbol :global-symbol :call :intrinsic}]
    (when-not (and (vector? coverage) (seq coverage))
      (fail! :lowering-coverage-shape
             "lowering coverage must be a nonempty vector" {}))
    (doseq [[index row] (map-indexed vector coverage)]
      (keys! row #{:construct :owner} :contract-lowering-coverage index)
      (when-not (and (keyword? (:construct row))
                     (local-symbol? (:owner row)))
        (fail! :lowering-coverage-row
               "lowering coverage needs a construct and owner definition"
               {:index index :row row})))
    (let [actual (set (map :construct coverage))]
      (when-not (= required actual)
        (fail! :lowering-coverage-constructs
               "lowering coverage must close every admitted source construct"
               {:missing (vec (sort (set/difference required actual)))
                :extra (vec (sort (set/difference actual required)))})))
    (when-not (= (count coverage)
                 (count (distinct (map :construct coverage))))
      (fail! :duplicate-lowering-construct
             "lowering coverage constructs must be unique" {})))
  (let [forbidden (keys! (:forbidden contract)
                         #{:forms :symbols :symbol-prefixes :payloads}
                         :contract-forbidden path)]
    (doseq [[key predicate] [[:forms symbol?]
                             [:symbols symbol?]
                             [:symbol-prefixes #(and (string? %)
                                                     (not (str/blank? %)))]
                             [:payloads keyword?]]]
      (let [values (get forbidden key)]
        (when-not (and (vector? values) (seq values)
                       (every? predicate values)
                       (= (count values) (count (distinct values))))
          (fail! :forbidden-table-shape
                 "forbidden table must be a nonempty unique vector"
                 {:key key :actual values})))))
  (let [sites (:mutation-sites contract)]
    (when-not (and (vector? sites) (seq sites))
      (fail! :mutation-sites-shape
             "contract mutation sites must be a nonempty vector" {}))
    (doseq [[index site] (map-indexed vector sites)]
      (let [selector (:selector site)
            expected (case selector
                       :string-literal #{:id :def :selector :expected}
                       :operator-branch #{:id :def :selector
                                          :input-symbol :expected-opcode}
                       nil)]
        (when-not expected
          (fail! :mutation-selector "unsupported mutation selector"
                 {:index index :selector selector}))
        (keys! site expected :contract-mutation-site index)
        (when-not (and (keyword? (:id site)) (local-symbol? (:def site)))
          (fail! :mutation-site-identity "mutation site identity is malformed"
                 {:index index :site site}))
        (when-not (case selector
                    :string-literal (string? (:expected site))
                    :operator-branch (and (string? (:input-symbol site))
                                          (string? (:expected-opcode site))))
          (fail! :mutation-site-expected "mutation semantic value is malformed"
                 {:index index :site site}))))
    (when-not (= (count sites) (count (distinct (map :id sites))))
      (fail! :duplicate-mutation-site "mutation site ids must be unique" {})))
  (let [boundary (keys! (:claim-boundary contract)
                        #{:c1_source_admission :executable_stage
                          :same_source_recompile_executed :promotion}
                        :contract-claim-boundary path)]
    (when-not (= boundary
                 {:c1_source_admission :static-source-and-lowering-closure-only
                  :executable_stage false
                  :same_source_recompile_executed false
                  :promotion :held})
      (fail! :claim-boundary-mismatch
             "C1 must remain static, non-executable, and held"
             {:actual boundary})))
  (false-claims! (:claims contract) :contract-claims)
  contract)

(defn validate-profile
  [profile contract path]
  (keys! profile profile-keys :profile path)
  (when-not (= profile-schema (:schema profile))
    (fail! :profile-schema-mismatch "unsupported selfhost profile schema"
           {:expected profile-schema :actual (:schema profile)}))
  (when-not (and (keyword? (:id profile))
                 (= (:family contract) (:family profile))
                 (= :c1-source-admission (:phase profile)))
    (fail! :profile-identity "profile identity/family/phase is inconsistent"
           {:id (:id profile) :family (:family profile) :phase (:phase profile)}))
  (let [invariant (keys! (:language-invariant profile)
                         #{:source-language-id :compiled-language-id
                           :source-language-equals-compiled-language}
                         :profile-language-invariant path)]
    (when-not (= invariant
                 (select-keys (:language-invariant contract)
                              [:source-language-id :compiled-language-id
                               :source-language-equals-compiled-language]))
      (fail! :profile-language-invariant-mismatch
             "profile must project the exact contract language invariant"
             {:actual invariant}))
    (when-not (= (:id profile) (:source-language-id invariant)
                 (:compiled-language-id invariant))
      (fail! :profile-language-id-mismatch
             "profile id must be both source and compiled language id"
             {:profile (:id profile) :invariant invariant})))
  (let [expected-ns (get-in contract [:source :namespace])
        envelope (keys! (:envelope profile) #{:first-form :exactly-one}
                        :profile-envelope path)]
    (when-not (= envelope {:first-form (list 'ns expected-ns)
                           :exactly-one true})
      (fail! :profile-envelope-mismatch
             "profile namespace envelope differs from contract"
             {:actual envelope :expected-namespace expected-ns})))
  (let [top (keys! (:top-level profile)
                   #{:after-envelope :initializer-kinds :definitions :entry}
                   :profile-top-level path)
        definitions (:definitions top)]
    (when-not (and (= :def-only (:after-envelope top))
                   (= [:literal :named-fn*] (:initializer-kinds top))
                   (vector? definitions) (seq definitions)
                   (every? local-symbol? definitions)
                   (= (count definitions) (count (distinct definitions)))
                   (= (get-in contract [:source :entry]) (:entry top))
                   (some #{(:entry top)} definitions)
                   (some #{(get-in contract [:source :identity-def])} definitions))
      (fail! :profile-top-level-shape
             "profile top-level closure is malformed"
             {:top-level top})))
  (let [expressions (keys! (:expressions profile)
                           #{:special-forms :named-fn-recursion
                             :multi-arity :variadic :nested-fn
                             :symbol-values :call-targets}
                           :profile-expressions path)]
    (when-not (= expressions {:special-forms '[fn* if let* do]
                              :named-fn-recursion true
                              :multi-arity false
                              :variadic false
                              :nested-fn false
                              :symbol-values [:argument :local :global-constant]
                              :call-targets [:named-fn-recursion :kernel-global
                                             :support-global :intrinsic]})
      (fail! :profile-expression-family
             "profile must equal macro-free fn*/if/let*/do"
             {:actual expressions})))
  (let [literals (keys! (:literals profile)
                        #{:value-kinds :structural-vectors}
                        :profile-literals path)]
    (when-not (= literals {:value-kinds [:nil :boolean :string :system-int64]
                           :structural-vectors [:fn-parameters :let-bindings]})
      (fail! :profile-literal-family "unsupported literal/vector family"
             {:actual literals})))
  (let [calls (keys! (:calls profile)
                     #{:support-global :intrinsic :kernel-global :lexical}
                     :profile-calls path)
        support (:support-global calls)
        intrinsic (:intrinsic calls)
        kernel (:kernel-global calls)
        lexical (keys! (:lexical calls) #{:policy :calls}
                       :profile-lexical-calls path)
        recursive (:calls lexical)
        intrinsic-projection (mapv #(select-keys % [:symbol :arity])
                                   (:intrinsics contract))]
    (when-not (= support
                 {:source [:support-abi :calls]
                  :contract "contract.edn"
                  :exact true})
      (fail! :support-call-projection-mismatch
             "profile must reference the exact contract support ABI"
             {:actual support}))
    (when-not (= intrinsic-projection intrinsic)
      (fail! :intrinsic-projection-mismatch
             "profile intrinsics must exactly project contract intrinsics"
             {:expected intrinsic-projection :actual intrinsic}))
    (call-rows! intrinsic :profile-intrinsics false)
    (kernel-call-rows! kernel)
    (call-rows! recursive :profile-recursive-calls false)
    (when-not (= :named-fn-recursion-only (:policy lexical))
      (fail! :lexical-call-policy "only named fn* recursion is admitted"
             {:actual (:policy lexical)}))
    (let [definitions (set (get-in profile [:top-level :definitions]))
          kernel-symbols (set (map :symbol kernel))
          recursive-symbols (set (map :symbol recursive))]
      (when-not (set/subset? kernel-symbols definitions)
        (fail! :kernel-call-not-defined
               "kernel call allowlist contains an undefined global"
               {:unknown (vec (sort (set/difference kernel-symbols definitions)))}))
      (when-not (set/subset? recursive-symbols kernel-symbols)
        (fail! :recursive-call-not-kernel
               "recursive functions must also be kernel globals"
               {:unknown (vec (sort (set/difference recursive-symbols
                                                     kernel-symbols)))}))))
  (when-not (= {:source [:lowering-coverage]
                :contract "contract.edn"
                :exact true}
               (keys! (:lowering-coverage profile)
                      #{:source :contract :exact}
                      :profile-lowering-coverage path))
    (fail! :profile-lowering-coverage-reference
           "profile must reference the exact contract lowering ledger" {}))
  (let [limits (keys! (:limits profile)
                      #{:max-source-bytes :max-top-level-forms :max-nodes
                        :max-depth :max-definitions :max-parameters
                        :max-bindings}
                      :profile-limits path)]
    (doseq [[key value] limits]
      (when-not (and (integer? value) (pos? value))
        (fail! :invalid-limit "profile limits must be positive integers"
               {:key key :actual value})))
    (when-not (= (:max-top-level-forms limits)
                 (inc (count (get-in profile [:top-level :definitions]))))
      (fail! :top-level-limit-mismatch
             "top-level limit must close exactly over envelope and definitions"
             {}))
    (when-not (= (:max-definitions limits)
                 (count (get-in profile [:top-level :definitions])))
      (fail! :definition-limit-mismatch
             "definition limit must equal declared definition closure" {})))
  (when-not (= {:contract "contract.edn"
                :source [:forbidden]
                :inherit-exact true}
               (keys! (:forbidden profile)
                      #{:contract :source :inherit-exact}
                      :profile-forbidden path))
    (fail! :profile-forbidden-policy
           "profile must inherit exact contract forbidden surface" {}))
  (false-claims! (:claims profile) :profile-claims)
  (when-not (= (:claims contract) (:claims profile))
    (fail! :claim-cross-reference
           "contract and profile false claims differ" {}))
  profile)

(defn validate-plan
  [plan contract profile path]
  (keys! plan plan-keys :plan path)
  (when-not (= plan-schema (:schema plan))
    (fail! :plan-schema-mismatch "unsupported selfhost plan schema"
           {:expected plan-schema :actual (:schema plan)}))
  (when-not (and (keyword? (:id plan))
                 (= (:family contract) (:family plan))
                 (= "contract.edn" (:contract plan))
                 (= "profile.edn" (:profile plan)))
    (fail! :plan-identity "plan identity/family/input names are inconsistent"
           {:id (:id plan) :family (:family plan)
            :contract (:contract plan) :profile (:profile plan)}))
  (let [source (keys! (:source plan)
                      #{:path :namespace :entry :source-language-id
                        :compiled-language-id}
                      :plan-source path)
        invariant (:language-invariant contract)
        expected (assoc (select-keys (:source contract)
                                     [:path :namespace :entry])
                        :source-language-id (:source-language-id invariant)
                        :compiled-language-id (:compiled-language-id invariant))]
    (when-not (= expected source)
      (fail! :plan-source-mismatch
             "plan source must exactly project contract source"
             {:expected expected :actual source})))
  (let [abi (keys! (:compiler-abi plan)
                   #{:contract :source :arguments :atomicity}
                   :plan-compiler-abi path)]
    (when-not (= abi
                 {:contract "contract.edn"
                  :source [:compiler-abi]
                  :arguments (get-in contract [:compiler-abi :arguments])
                  :atomicity (get-in contract [:compiler-abi :atomicity])})
      (fail! :plan-compiler-abi-mismatch
             "plan must project the exact transactional compiler ABI"
             {:actual abi})))
  (let [stages (:stages plan)]
    (when-not (and (vector? stages) (seq stages))
      (fail! :plan-stages-shape "plan stages must be a nonempty vector" {}))
    (doseq [[index stage] (map-indexed vector stages)]
      (keys! stage #{:id :kind :status :executable} :plan-stage index)
      (when-not (and (keyword? (:id stage)) (keyword? (:kind stage))
                     (keyword? (:status stage)) (false? (:executable stage)))
        (fail! :plan-stage-not-static
               "C0/C1 plan stages must remain typed and non-executable"
               {:index index :stage stage})))
    (when-not (= (count stages) (count (distinct (map :id stages))))
      (fail! :duplicate-plan-stage "plan stage ids must be unique" {})))
  (let [chain (:future-chain plan)
        source-path (get-in contract [:source :path])
        language-id (get-in contract [:language-invariant :source-language-id])]
    (when-not (and (vector? chain) (seq chain))
      (fail! :future-chain-shape "future compiler chain must be a vector" {}))
    (doseq [[index link] (map-indexed vector chain)]
      (keys! link #{:from :input :language-id :output}
             :plan-future-chain index)
      (when-not (and (keyword? (:from link))
                     (= source-path (:input link))
                     (= language-id (:language-id link))
                     (keyword? (:output link)))
        (fail! :future-chain-language-mismatch
               "every future transition must compile the same source language"
               {:index index :link link})))
    (when-not (= (count chain) (count (distinct (map :output chain))))
      (fail! :future-chain-output-duplicate
             "future chain outputs must be unique" {})))
  (let [checks (:mutation-checks plan)
        sites (:mutation-sites contract)
        site-ids (set (map :id sites))]
    (when-not (vector? checks)
      (fail! :mutation-checks-shape "mutation checks must be a vector" {}))
    (doseq [[index check] (map-indexed vector checks)]
      (keys! check #{:id :contract-site :mutation :c1-required-result
                     :executable-required-result :executable-status}
             :plan-mutation-check index)
      (when-not (and (keyword? (:id check))
                     (= (:id check) (:contract-site check))
                     (contains? site-ids (:contract-site check))
                     (keyword? (:mutation check))
                     (= :reject (:c1-required-result check))
                     (= :propagate (:executable-required-result check))
                     (= :unexecuted (:executable-status check)))
        (fail! :mutation-check-cross-reference
               "plan mutation must map one-to-one to a contract reject site"
               {:index index :check check})))
    (when-not (= site-ids (set (map :contract-site checks)))
      (fail! :mutation-check-closure
             "plan mutation checks do not exactly cover contract sites" {})))
  (false-claims! (:claims plan) :plan-claims)
  (when-not (= (:claims contract) (:claims profile) (:claims plan))
    (fail! :claim-cross-reference
           "contract, profile, and plan false claims differ" {}))
  plan)
