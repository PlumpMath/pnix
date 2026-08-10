(ns pnix-clj.interop
  "Host-side adapter for the Clojure/JVM <-> pnix interop boundary.
  Clojure/JVM <-> pnix interop 경계용 호스트 측 어댑터입니다.

  This namespace isolates Clojure/JVM HOST machinery (host eval, fresh-namespace creation, and later host reflection) so the pnix runtime files do not own it directly. 
  이 네임스페이스는 Clojure/JVM HOST 기계(호스트 평가, 새로운 네임스페이스 생성 및 이후 호스트 반영)를 격리하므로 pnix 런타임 파일이 이를 직접 소유하지 않습니다.

  Per `clj-meta-separation.md`, host meta-circular machinery belongs to the host proof lane; pnix-clj reaches it only through this boundary, and every crossing is tagged with an explicit interop classification (direction / effect-class / loss-status). 
  `clj-meta-separation.md`에 따라 호스트 메타 순환 기계는 ​​호스트 증명 레인에 속합니다. pnix-clj는 이 경계를 통해서만 도달하며 모든 교차에는 명시적인 상호 운용성 분류(방향/효과 클래스/손실 상태)가 태그로 지정됩니다.

  Interop is NOT a mirror: it converts/executes at the boundary and works whether or not any mirror observes it.
  Interop은 미러가 아닙니다. Interop은 경계에서 변환/실행하며 미러가 이를 관찰하는지 여부에 관계없이 작동합니다.
"
  (:require [pnix.clj-meta.io :as meta-io]
            [pnix-clj.hash :as hash]))

(def lane-classification
  {:lane :core
   :scope :meta-circular-runtime-interop-boundary
   :role :capability-checked-clojure-pnix-crossing-metadata-and-witness
   :product-runtime :allowed
   :semantic-authority :capability-gate-only
   :mutation :forbidden-by-default
   :admission :capability-checked
   :determinism :witness-hash-required
   :clojure-runtime :stage15-to-N-compiler-evaluator-interpreter
   :pnix-runtime :runtime-compiler-evaluator-interpreter
   :allowed-output :interop-result-with-crossing-witness})

(defn interop-meta
  "
A small explicit classification attached to a host crossing. See the shared interop protocol in `clj-meta-separation.md` (Section 5).
호스트 교차에 연결된 작은 명시적 분류입니다. `clj-meta-separation.md`(섹션 5)에서 공유 상호 운용성 프로토콜을 참조하세요.
"
  [{:keys [direction effect-class loss-status]}]
  {:schema :pnix-clj.interop.v0
   :direction direction
   :effect-class effect-class
   :loss-status loss-status})

(def effect-classes
  "
The closed set of effect classes the interop boundary recognizes.
Interop 경계가 인식하는 닫힌 효과 클래스 세트입니다.
"
  #{:pure :host-call :host-eval :host-compile :macroexpand :dynamic-binding
    :reflection :require :resolve-var :classloader :file-read :file-write
    :thread :time :random :process :network :global-mutation
    :namespace-mutation :var-mutation :unknown})

(def default-capabilities
  "
Deny-by-default: only :pure crosses without an explicit grant.
기본 거부: 명시적 승인 없이 :pure만 교차합니다.
"
  #{:pure})

(def host-eval-capabilities
  "
Small grant set for the explicit host-eval proof lane.
명시적인 호스트 평가 증명 레인에 대한 소규모 부여 세트입니다.
"
  #{:pure :host-eval})

(def host-compile-capabilities
  "
Small grant set for the explicit clj-meta host-compile proof lane.
  명시적인 clj-meta 호스트 컴파일 증명 레인에 대한 소규모 부여 세트입니다.
"
  #{:pure :host-compile})

(def ^:dynamic *capabilities*
  "
Dynamic capability set used by host crossing helpers when a caller does not pass an explicit grant. Deny-by-default.
  호출자가 명시적 허가를 전달하지 않을 때 호스트 교차 도우미가 사용하는 동적 기능 집합입니다. 기본적으로 거부됩니다.
"
  default-capabilities)

(defn effect-class?
  [k]
  (contains? effect-classes k))

(defn check-capability
  "
Deny-by-default capability check for an effect class. `granted` is the set of allowed effect classes (defaults to {:pure}). Unknown and denied capabilities are deterministic failures, never owner-policy Held values.
  효과 클래스에 대한 기본 거부 기능 확인. `granted`는 허용되는 효과 클래스 집합입니다(기본값은 {:pure}). 
알 수 없거나 거부된 기능은 결정론적 실패이며 소유자 정책 보유 값이 아닙니다.
"
  ([effect] (check-capability effect default-capabilities))
  ([effect granted]
   (let [granted (or granted default-capabilities)]
     (cond
       (not (effect-class? effect))
       {:status :failed
        :reason :unknown-effect-class
        :effect effect
        :error {:phase :interop-contract
                :class :unknown-effect-class
                :evidence {:effect effect}}}

       (contains? granted effect)
       {:status :ok :effect effect :granted true}

       :else
       {:status :failed
        :reason :capability-denied
        :effect effect
        :error {:phase :capability
                :class :capability-denied
                :evidence {:effect effect}}}))))

;; --- Interop crossing witness ------------------------------------------------
;;
;; Every boundary crossing can emit a witness: a pure, content-hashed evidence record (direction / effect-class / loss-status / in+out hashes).
;; Unlike an opaque ref's runtime id, a witness is pure data and IS content-addressed, so it can live in an evidence log (clj-meta-separation.md §10 / checklist §15).
;; 
;; 모든 경계 교차는 순수한 콘텐츠 해시 증거 기록(방향 / 효과 클래스 /
;; 손실 상태 / 인+아웃 해시)이라는 증인을 내보낼 수 있습니다.  불투명
;; 참조의 런타임 ID와 달리 감시는 순수 데이터이고 IS 콘텐츠 주소가
;; 지정되므로 증거 로그(clj-meta-separation.md §10 / checklist §15)에
;; 존재할 수 있습니다.

(defn- stable-witness-value
  [v]
  (cond
    (nil? v) nil
    (or (boolean? v) (number? v) (string? v) (char? v)
        (keyword? v) (symbol? v))
    v

    (map? v)
    {:kind :map
     :entries (mapv (fn [[k val]]
                      [(stable-witness-value k) (stable-witness-value val)])
                    (sort-by (comp pr-str key) v))}

    (sequential? v)
    {:kind :sequential
     :items (mapv stable-witness-value v)}

    (set? v)
    {:kind :set
     :items (mapv stable-witness-value (sort-by pr-str v))}

    :else
    {:kind :host-object
     :class (some-> v class .getName)}))

(defn make-witness
  [{:keys [kind direction effect-class loss-status input-hash output-hash]}]
  (let [body {:schema :pnix-clj.interop.witness.v0
              :kind kind
              :direction direction
              :effect-class effect-class
              :loss-status loss-status
              :input-hash input-hash
              :output-hash output-hash}]
    (assoc body :witness-hash (hash/data-hash body))))

(defn witness?
  [v]
  (and (map? v) (= :pnix-clj.interop.witness.v0 (:schema v))))

(defn crossing-witness
  "
Build a stable witness for one interop crossing.
하나의 Interop 교차에 대한 안정적인 감시를 구축합니다.
"
  [kind meta input output]
  (make-witness {:kind kind
                 :direction (:direction meta)
                 :effect-class (:effect-class meta)
                 :loss-status (:loss-status meta)
                 :input-hash (hash/data-hash (stable-witness-value input))
                 :output-hash (hash/data-hash
                               (stable-witness-value output))}))

(defn attach-witness
  "
Attach a crossing witness to a result map without changing its status/value.
상태/값을 변경하지 않고 교차 증인을 결과 맵에 연결합니다.
"
  [kind meta input result]
  (assoc result :witness (crossing-witness kind meta input result)))

(defn run-crossing
  "
Gate a concrete interop crossing, run `f` only when explicitly granted, and attach interop/capability/witness evidence to the returned result map.
구체적인 Interop 교차를 게이트하고, 명시적으로 허용된 경우에만 'f'를
실행하고, 반환된 결과 맵에 상호 운용성/기능/감시 증거를 연결합니다.
"
  [kind meta input granted f]
  (let [capability (check-capability (:effect-class meta) granted)]
    (if (not= :ok (:status capability))
      (attach-witness kind
                      meta
                      input
                      {:status :failed
                       :reason (:reason capability)
                       :error (:error capability)
                       :interop meta
                       :capability capability})
      (try
        (attach-witness kind
                        meta
                        input
                        (assoc (f)
                               :interop meta
                               :capability capability))
        (catch Throwable _
          (attach-witness kind
                          meta
                          input
                          {:status :failed
                           :reason :interop-crossing-failed
                           :interop meta
                           :capability capability
                           :error {:phase :interop
                                   :class :host-call-failed
                                   :evidence {:kind kind
                                              :effect-class (:effect-class meta)}}}))))))

;; --- pnix effect-request read-only adapter ---------------------------------
;; Portable validation and request construction stay in pnix-meta. 
;; This is host mechanics only, backed by clj-meta's pnix-agnostic I/O substrate.
;; --- pnix effect-request 읽기 전용 어댑터
;; 휴대용 검증 및 요청 구성은 pnix-meta에 유지됩니다.  이것은 clj-meta의 pnix 독립적 I/O 기판이
;; 지원하는 호스트 메커니즘일 뿐입니다.

(def read-only-effect-names
  #{"fs.path-exists" "fs.open" "fs.file-type" "fs.read-dir"})

(defn- effect-field [m key]
  (when (map? m)
    (if (contains? m key) (get m key) (get m (keyword key)))))

(defn- effect-receipt [effect capability-class executed]
  {"kind" "effect-request-receipt"
   "effect" effect
   "risk_tier" (or (effect-field capability-class "risk_tier") "unknown")
   "capability_id" (or (effect-field capability-class "capability_id") "unknown")
   "executed" executed
   "adapter" "host-meta-io-v1"})

(deftype EffectExecuted [operation value receipt])
(deftype EffectFailed [phase errorClass operation receipt])

(defn- project-effect-adapter-result [result]
  (cond
    (instance? EffectExecuted result)
    {"outcome" "effect-executed"
     "effect" (.-operation ^EffectExecuted result)
     "value" (.-value ^EffectExecuted result)
     "receipt" (.-receipt ^EffectExecuted result)}

    (instance? EffectFailed result)
    {"outcome" "failed"
     "error" {"phase" (name (.-phase ^EffectFailed result))
              "class" (name (.-errorClass ^EffectFailed result))}
     "effect" (.-operation ^EffectFailed result)
     "receipt" (.-receipt ^EffectFailed result)}

    :else
    (throw (ex-info "invalid effect adapter result"
                    {:error-class :invalid-effect-adapter-result}))))

(defn- failed-effect [effect capability-class reason]
  (let [[phase error-class]
        (case reason
          "effect-adapter-unsupported" [:effect-contract :unknown-effect-operation]
          "effect-args-invalid" [:effect-contract :invalid-effect-args]
          "capability-denied" [:effect :effect-denied]
          [:effect :effect-adapter-error])]
    (EffectFailed. phase error-class effect
                   (effect-receipt effect capability-class false))))

(defn- apply-effect-request-outcome
  "
Execute one validated pnix-meta read-only effect request when :file-read is explicitly granted. 
Returns a nominal host result; Held is not an adapter failure variant.

  :file-read가 명시적으로 허용되면 검증된 pnix-meta 읽기 전용 효과 요청 하나를 실행합니다.  
  명목상 호스트 결과를 반환합니다. 보류는 어댑터 오류 변형이 아닙니다.
"
  ([request] (apply-effect-request-outcome request *capabilities*))
  ([request granted]
   (let [effect (effect-field request "operation_id")
         args (effect-field request "args")
         capability-class (effect-field request "capability_class")
         path (effect-field args "path")
         normalized-grants (set (map #(if (string? %) (keyword %) %) granted))
         capability (check-capability :file-read normalized-grants)]
     (cond
       (not (contains? read-only-effect-names effect))
       (failed-effect effect capability-class "effect-adapter-unsupported")

       (not (string? path))
       (failed-effect effect capability-class "effect-args-invalid")

       (not= :ok (:status capability))
       (failed-effect effect capability-class "capability-denied")

       :else
       (try
         (let [value (case effect
                       "fs.path-exists" (meta-io/path-exists path normalized-grants)
                       "fs.open" (meta-io/read-utf8 path normalized-grants)
                       "fs.file-type" (meta-io/file-type path normalized-grants)
                       "fs.read-dir" (meta-io/read-dir path normalized-grants))]
           (EffectExecuted. effect value
                            (effect-receipt effect capability-class true)))
         (catch clojure.lang.ExceptionInfo e
           (failed-effect effect capability-class
                          (name (or (:error-class (ex-data e)) :io-error))))
         (catch Throwable _
           (failed-effect effect capability-class "io-error")))))))

(defn apply-effect-request
  "Compatibility projection of the nominal effect adapter result."
  ([request] (apply-effect-request request *capabilities*))
  ([request granted]
   (project-effect-adapter-result
     (apply-effect-request-outcome request granted))))

(defn fresh-host-ns
  "
Create a fresh, content-named host Clojure namespace with clojure.core referred. 
This is a host namespace mutation.
  clojure.core를 참조하여 콘텐츠 이름이 지정된 새로운 호스트 Clojure 네임스페이스를 만듭니다.  
이는 호스트 네임스페이스 변형입니다.

"
  [prefix source-id]
  (let [ns-sym (symbol (str prefix "." (name source-id) "."
                            (subs (hash/sha256 (str source-id)) 0 12)))
        target (create-ns ns-sym)]
    (binding [*ns* target]
      (clojure.core/refer 'clojure.core))
    target))

(defn host-eval-form
  "
Evaluate a host Clojure `form` in a fresh host namespace, returning a pnix-side result map. 
Host `eval` is a host-call effect and the produced value is a host value (loss-status :opaque until projected/converted).
  새로운 호스트 네임스페이스에서 호스트 Clojure 'form'을 평가하고 pnix 측 결과 맵을 반환합니다.  
  호스트 'eval'은 호스트 호출 효과이며 생성된 값은 호스트 값입니다(투영/변환될 때까지 손실 상태:불투명).
"
  ([source-id form]
   (host-eval-form source-id form *capabilities*))
  ([source-id form granted]
   (let [meta (interop-meta {:direction :clojure->host-value
                             :effect-class :host-eval
                             :loss-status :opaque})
         capability (check-capability (:effect-class meta) granted)]
     (if (not= :ok (:status capability))
       (attach-witness :host-eval-form-denied
                       meta
                       {:source-id source-id :form form}
                       {:status :failed
                        :reason (:reason capability)
                        :error (:error capability)
                        :interop meta
                        :capability capability})
       (try
         (let [target (fresh-host-ns "pnix-clj.form-host" source-id)]
           (attach-witness :host-eval-form
                           meta
                           {:source-id source-id :form form}
                           {:status :ok
                            :value (binding [*ns* target]
                                     (eval form))
                            :ns (str (ns-name target))
                            :interop meta
                            :capability capability}))
         (catch Throwable _
           (attach-witness :host-eval-form
                           meta
                           {:source-id source-id :form form}
                           {:status :failed
                            :reason :host-clojure-form-eval-failed
                            :interop meta
                            :capability capability
                            :error {:phase :interop
                                    :class :host-eval-failed
                                    :evidence {:source-id source-id}}})))))))

;; --- Opaque host references (object-capability handles) ----------------------
;; --- 불투명 호스트 참조(객체 기능 핸들) ---------

;; A host (Clojure/JVM) object that is not a pure pnix value crosses the boundary as an opaque ref: the real object is kept in a process-local registry and only an unforgeable handle {:kind :opaque-host-ref ...} travels (the Kernel-FFI / object-capability pattern).
;; 순수한 pnix 값이 아닌 호스트(Clojure/JVM) 개체는 불투명 참조로 경계를 넘습니다. 
;; 실제 개체는 프로세스 로컬 레지스트리에 보관되고 위조할 수 없는 핸들 {:kind :opaque-host-ref ...}만
;; 이동합니다(커널-FFI/객체 기능 패턴).

;; Per clj-meta-separation.md §10, such host objects MUST NOT be value-serialized into a pnix canonical / content-addressed term; `host-object?` flags anything that would violate that, and `from-host` wraps it.
;; clj-meta-separation.md §10에 따라 이러한 호스트 개체는 pnix 표준/콘텐츠 주소 지정 용어로 값 직렬화되어서는 안
;; 됩니다. `host-object?`는 이를 위반하는 모든 항목에 플래그를 지정하고 `from-host`는 이를 래핑합니다.

(def ^:private opaque-registry (atom {}))
(def ^:private opaque-counter (atom 0))

(defn opaque-host-ref?
  [v]
  (and (map? v) (= :opaque-host-ref (:kind v))))

(defn make-opaque-host-ref
  "
  Wrap a host object as an opaque ref; the real object is kept in the registry  and only the handle (designation + class name) crosses.
  호스트 객체를 불투명 참조로 래핑합니다. 실제 객체는 레지스트리에 보관되며 핸들(지정 + 클래스 이름)만 교차됩니다.

  The :id is a per-process  counter (runtime identity) — opaque refs never enter content-addressed terms, so a non-content id is fine.
  :id는 프로세스별 카운터(런타임 ID)입니다. 불투명한 참조는 콘텐츠 주소가 지정된 용어를 입력하지 않으므로 콘텐츠가 아닌 ID도 괜찮습니다.
"
  [obj]
  (let [id (swap! opaque-counter inc)
        ref {:kind :opaque-host-ref
             :id id
             :class (some-> obj class .getName)}]
    (swap! opaque-registry assoc id obj)
    ref))

(defn opaque-ref-deref
  "
Recover the real host object behind an opaque ref. 
  Returns a result map; a released or non-ref input is a structured interop-contract failure.
불투명 참조 뒤에 있는 실제 호스트 개체를 복구합니다.  
  결과 맵을 반환합니다; 해제되거나 참조가 아닌 입력은 구조화된 상호 운용성 계약 실패입니다.
  "
  [ref]
  (cond
    (not (opaque-host-ref? ref))
    {:status :failed
     :reason :not-an-opaque-ref
     :error {:phase :interop-contract :class :not-an-opaque-ref}
     :value ref}

    (contains? @opaque-registry (:id ref))
    {:status :ok :value (get @opaque-registry (:id ref))}

    :else
    {:status :failed
     :reason :opaque-ref-released
     :error {:phase :interop-contract :class :opaque-ref-released}
     :ref ref}))

(defn release-opaque-ref!
  [ref]
  (when (opaque-host-ref? ref)
    (swap! opaque-registry dissoc (:id ref)))
  nil)

(defn- pnix-scalar?
  [v]
  (or (nil? v) (boolean? v) (number? v) (string? v) (char? v)
      (keyword? v) (symbol? v)))

(def ^:private pnix-tagged-kinds
  #{:closure :builtin :thunk :opaque-host-ref})

(defn host-object?
  "
True if v is a foreign host/JVM object that is NOT a pure pnix value and so must not enter a pnix canonical term — it should cross as an opaque ref instead.
  v가 순수 pnix 값이 아니므로 pnix 표준 용어를 입력하면 안 되는 외부 호스트/JVM 객체인 경우 참입니다. 대신 불투명 참조로 교차해야 합니다.

 pnix scalars, vectors/attrsets of pnix values, and pnix runtime tagged maps (closure/builtin/thunk/opaque-ref) are not host objects.
 pnix 스칼라, pnix 값의 벡터/attrsets 및 pnix 런타임 태그 맵(closure/builtin/thunk/opaque-ref)은 호스트 개체가 아닙니다.

"
  [v]
  (cond
    (pnix-scalar? v) false
    (opaque-host-ref? v) false
    (vector? v) (boolean (some host-object? v))
    (map? v) (if (contains? pnix-tagged-kinds (:kind v))
               false
               (boolean (or (some host-object? (keys v))
                            (some host-object? (vals v)))))
    :else true))

(defn from-host
  "
Marshal a host value into a pnix value: pure pnix values (and pnix tagged maps) pass through; 
  a foreign host object becomes an opaque ref.

  호스트 값을 pnix 값으로 마샬링합니다. 순수 pnix 값(및 pnix 태그가 지정된 맵)이 통과합니다; 
  외부 호스트 객체는 불투명한 참조가 됩니다.

"
  [v]
  (cond
    (pnix-scalar? v) v
    (opaque-host-ref? v) v
    (vector? v) (mapv from-host v)
    (and (map? v) (contains? pnix-tagged-kinds (:kind v))) v
    (map? v) (into {} (map (fn [[k val]] [(from-host k) (from-host val)])) v)
    :else (make-opaque-host-ref v)))

(defn to-host
  "
Marshal a pnix value back to a host value: pure values pass through; an opaque ref is dereferenced to the real host object (nil if released).
  pnix 값을 다시 호스트 값으로 마샬링합니다. 순수한 값은 통과합니다; 불투명 참조는 실제 호스트 객체로 역참조됩니다(해제된 경우 nil).

"
  [v]
  (cond
    (opaque-host-ref? v) (:value (opaque-ref-deref v))
    (pnix-scalar? v) v
    (vector? v) (mapv to-host v)
    (and (map? v) (contains? pnix-tagged-kinds (:kind v))) v
    (map? v) (into {} (map (fn [[k val]] [(to-host k) (to-host val)])) v)
    :else v))
