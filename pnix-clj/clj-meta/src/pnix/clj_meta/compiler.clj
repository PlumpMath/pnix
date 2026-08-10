(ns pnix.clj-meta.compiler
  "축 B-2 (주 경로): Clojure로 쓴 컴파일러.

  파이프라인:  form
    → 매크로확장 + 분석  (tools.analyzer.jvm — Clojure로 작성된 살아있는 부품)
    → AST
    → emit            (이 파일 — 우리가 clojure.asm 으로 직접 JVM bytecode 방출)
    → DynamicClassLoader 로 로드
    → host JVM 실행   (성능 host급)

  핵심: target 프로그램의 bytecode 를 *우리 코드가* 짠다. host clojure.lang.Compiler
  는 target fn 의 emit 에 개입하지 않는다(백엔드 A = host Compiler 0회).

  Phase 2b 첫 조각 범위(emit 가능 AST op):
  - :fn        단일 arity 함수 → clojure.lang.AFn 서브클래스 + invoke(Object…)
  - :fn-method 본문(:do)
  - :do        statements(pop) + ret
  - :const     정수(long, boxing) — 그 외 상수는 다음 슬라이스
  - :local     파라미터 → loadArg
  - :invoke 등 나머지 op 는 명시적 'unsupported' 에러(다음 슬라이스에서 확장)"
  (:require [clojure.tools.analyzer.jvm :as ana]
            [pnix.clj-meta.abstract-interval :as ai]
            [pnix.clj-meta.translation-validation :as tv])
  (:import [clojure.asm ClassWriter Label Opcodes Type]
           [clojure.asm.commons GeneratorAdapter Method]
           [clojure.lang DynamicClassLoader AFn AFunction IFn RT Var Keyword Symbol RestFn Numbers Util
            Reflector LineNumberingPushbackReader Namespace IPersistentMap ILookup IObj IMeta
            BigInt Ratio]
           [java.io File FileOutputStream StringReader]
           [java.math BigDecimal BigInteger]
           [java.nio.file Files]
           [java.security MessageDigest]
           [java.util.regex Pattern]))

(def ^:private ^Type obj-type (Type/getType Object))
;; fn 베이스: clojure 처럼 AFunction(AFn 하위 + __methodImplCache/IObj/Comparator)을 상속한다.
;; protocol method fn 이 AFunction 으로 캐스팅되는 경로(defprotocol)와 호환된다.
(def ^:private ^Type afn-type (Type/getType AFunction))
(def ^:private ^Type restfn-type (Type/getType RestFn))
(def ^:private ^Method getreqarity-method (Method/getMethod "int getRequiredArity()"))
(def ^:private ^Type ifn-type (Type/getType IFn))
(def ^:private ^Type rt-type  (Type/getType RT))
(def ^:private ^Type reflector-type (Type/getType Reflector))
(def ^:private init-method (Method. "<init>" Type/VOID_TYPE (into-array Type [])))
;; clojure.lang.Var RT.var(String ns, String name) — 런타임에 var 를 해소
(def ^:private rt-var-method (Method/getMethod "clojure.lang.Var var(String,String)"))
;; boolean RT.booleanCast(Object) — :if 의 진리값 판정(false/nil → false)
(def ^:private rt-booleancast-method (Method/getMethod "boolean booleanCast(Object)"))

(def ^:private ^Type kw-type  (Type/getType Keyword))
(def ^:private ^Type sym-type (Type/getType Symbol))
(def ^:private ^Type throwable-type (Type/getType Throwable))
(def ^:private ^Type var-type (Type/getType Var))
(def ^:private bindroot-method (Method/getMethod "void bindRoot(Object)"))
(def ^:private var-set-dynamic-method (Method/getMethod "clojure.lang.Var setDynamic()"))
(def ^:private var-deref-method (Method/getMethod "Object deref()"))
(def ^:private var-set-method (Method/getMethod "Object set(Object)"))
(def ^:private ^Class obj-array-class (class (make-array Object 0)))
(def ^:private ^Type obj-array-type (Type/getType obj-array-class))
(def ^:private ^Type string-type (Type/getType String))
(def ^:private ^Type biginteger-type (Type/getType BigInteger))
(def ^:private ^Type bigint-type (Type/getType BigInt))
(def ^:private ^Type bigdec-type (Type/getType BigDecimal))
(def ^:private ^Type ratio-type (Type/getType Ratio))
(def ^:private ^Type pattern-type (Type/getType Pattern))
(def ^:private biginteger-string-ctor
  (Method. "<init>" Type/VOID_TYPE (into-array Type [string-type])))
(def ^:private bigdec-string-ctor
  (Method. "<init>" Type/VOID_TYPE (into-array Type [string-type])))
(def ^:private ratio-ctor
  (Method. "<init>" Type/VOID_TYPE (into-array Type [biginteger-type biginteger-type])))

(defn- reflect-asm-method
  "리플렉션으로 정확한 메서드 디스크립터를 잡는다(반환타입 추측 회피)."
  ^Method [^Class cls ^String mname param-classes]
  (Method/getMethod (.getMethod cls mname (into-array Class param-classes))))

;; RT.vector/map/set(Object[]) — 컬렉션 리터럴, Keyword/Symbol.intern(ns,name)
(def ^:private rt-vector-method  (reflect-asm-method RT "vector" [obj-array-class]))
(def ^:private rt-map-method     (reflect-asm-method RT "map"    [obj-array-class]))
(def ^:private rt-set-method     (reflect-asm-method RT "set"    [obj-array-class]))
(def ^:private rt-array-to-list-method (reflect-asm-method RT "arrayToList" [obj-array-class]))
(def ^:private rt-unchecked-intcast-object-method
  (reflect-asm-method RT "uncheckedIntCast" [Object]))
(def ^:private rt-seq-method (reflect-asm-method RT "seq" [Object]))
(def ^:private kw-intern-method  (reflect-asm-method Keyword "intern" [String String]))
(def ^:private sym-intern-method (reflect-asm-method Symbol  "intern" [String String]))
(def ^:private bigint-from-biginteger-method
  (reflect-asm-method BigInt "fromBigInteger" [BigInteger]))
(def ^:private pattern-compile-method
  (reflect-asm-method Pattern "compile" [String]))
(def ^:private util-type (Type/getType Util))
(def ^:private util-hash-method (reflect-asm-method Util "hash" [Object]))
(def ^:private util-equiv-method (reflect-asm-method Util "equiv" [Object Object]))
;; :import op — host clojure.lang.Compiler.ImportExpr 와 동일 경로:
;; RT/CURRENT_NS deref → Namespace → importClass(classForNameNonLoading(name)).
(def ^:private ^Type ns-type (Type/getType Namespace))
(def ^:private rt-class-for-name-nonloading-method
  (reflect-asm-method RT "classForNameNonLoading" [String]))
(def ^:private ns-import-class-method
  (reflect-asm-method Namespace "importClass" [Class]))
;; reify auto-IObj 메타데이터: 모든 reify 는 host 처럼 IObj 를 구현한다(__meta 필드 +
;; meta()/withMeta() 자동 생성). withMeta 는 새 인스턴스를 만들어 메타데이터를 옮긴다.
(def ^:private ^Type ipm-type (Type/getType IPersistentMap))
(def ^:private ^Type ilookup-type (Type/getType ILookup))
(def ^:private ^Type iobj-type (Type/getType IObj))
(def ^:private ^Class ipm-class IPersistentMap)
(def ^:private ilookup-valat-method
  (reflect-asm-method ILookup "valAt" [Object]))
(def ^:private ipm-without-method
  (reflect-asm-method IPersistentMap "without" [Object]))
(def ^:private reify-meta-method (Method/getMethod "clojure.lang.IPersistentMap meta()"))
(def ^:private reify-withmeta-method
  (Method/getMethod "clojure.lang.IObj withMeta(clojure.lang.IPersistentMap)"))
(def ^:private number-type (Type/getType Number))
(def ^:private character-type (Type/getType Character))
(def ^:private reflector-noarg-instance-member-method
  (reflect-asm-method Reflector "invokeNoArgInstanceMember" [Object String Boolean/TYPE]))
(def ^:private reflector-instance-method-method
  (reflect-asm-method Reflector "invokeInstanceMethod" [Object String obj-array-class]))
(def ^:private reflector-set-instance-field-method
  (reflect-asm-method Reflector "setInstanceField" [Object String Object]))
(def ^:private numbers-primitive-methods
  #{"add" "multiply" "minus" "lt"
    "unchecked_add" "unchecked_multiply" "unchecked_minus"})

(def ^:dynamic ^:private *dcl* nil)         ; 컴파일 단위 공유 DynamicClassLoader
(def ^:dynamic ^:private *gen-counter* nil) ; 단위별 결정적 클래스명 카운터
(def ^:dynamic ^:private *class-unit-id* "unit") ; source-hash 기반 결정적 클래스명 prefix
(def ^:dynamic ^:private *emitted* nil)     ; {classname byte[]} 수집(self-host 고정점용)
(def ^:dynamic ^:private *typed-locals-enabled* false)
(def ^:dynamic ^:private *emit-line-numbers* true)
(def ^:dynamic ^:private *last-emitted-line* nil)
(def ^:dynamic ^:private *disable-locals-clearing* false)
(def ^:dynamic ^:private *disable-homogeneous-index-stride-gate* false)
(def ^:dynamic ^:private *disable-independent-range-admission* false)
(def ^:dynamic *fallback-diagnostics-cap* 1024)
(def ^:dynamic ^:private *compile-form-local-fallback-diagnostics* nil)
(def compile-form-fallback-diagnostics
  "compile-form 이 host fallback 을 탄 이유를 최근 순서대로 기록한다.
  정상 direct emit 경로는 기록하지 않는다."
  (atom []))
;; DynamicClassLoader 는 정의한 클래스를 SoftReference 로 캐시한다. 인스턴스화 전의
;; nested 클래스가 GC 되면 NEW 시 ClassNotFound 가 난다(깊은 중첩에서 발생). 정의한
;; 클래스를 컴파일 단위별로 강참조해 회피하되, 오래된 단위는 bounded cache 에서 밀어낸다.
(def ^:dynamic *kept-class-units-cap* 128)
(def ^:private kept-class-units (atom []))
(def ^:dynamic ^:private *unit-classes* nil)
(def ^:private compile-lock (Object.))
(declare emit-fn compile-fn-class)

(defn- compiler-session-loader
  []
  (or *dcl*
      (DynamicClassLoader. (.getContextClassLoader (Thread/currentThread)))))

(defn with-compiler-session
  "f 를 공유 DynamicClassLoader session 안에서 실행한다. compile-form/compile-form-strict/
  compile-ns/load-compiled-ns 가 같은 session 안에서는 loader 를 재사용하므로, cross-form
  generated class resolve 와 일괄 release scope 를 만들 수 있다."
  [f]
  (binding [*dcl* (compiler-session-loader)]
    (f)))

(defn- remember-class!
  [klass]
  (when *unit-classes*
    (swap! *unit-classes* conj klass))
  klass)

(defn- keep-class-unit!
  [classes]
  (when (seq classes)
    (swap! kept-class-units
           (fn [units]
             (let [units' (conj (vec units) {:loader *dcl*
                                             :classes (vec classes)})
                   n      (count units')
                   cap    (max 0 (long *kept-class-units-cap*))]
               (if (> n cap)
                 (subvec units' (- n cap))
                 units'))))))

(defn clear-kept-classes!
  "강참조 중인 generated class unit cache 를 비운다. 장수 host 가 compile session 을 해제한 뒤
  명시적으로 호출할 수 있는 release hook 이다."
  []
  (reset! kept-class-units []))

(defn clear-compile-form-fallback-diagnostics!
  []
  (reset! compile-form-fallback-diagnostics []))

;; pnix-agnostic lazy recursive value slots. This is intentionally a small
;; host capability: callers provide ordinary Clojure binding forms, and the
;; macro rewrites references to the bound names into cell forces. It gives
;; value-level letrec semantics without teaching this namespace anything about
;; pnix attrsets.
(defn make-rec-cell
  [name]
  (atom {:phase :pending
         :name name}))

(defn bind-rec-cell!
  [cell thunk]
  (let [{:keys [phase name]} @cell]
    (when-not (= :pending phase)
      (throw (ex-info "recursive binding cell already bound"
                      {:name name
                       :phase phase})))
    (reset! cell {:phase :bound
                  :name name
                  :thunk thunk})
    nil))

(defn force-rec-cell
  [cell name]
  (let [{:keys [phase thunk value error] :as state} @cell]
    (case phase
      :done value
      :failed (throw error)
      :forcing (let [t (ex-info "recursive binding cycle"
                                {:type :recursive-binding-cycle
                                 :name name})]
                 (reset! cell (assoc state
                                      :phase :failed
                                      :error t))
                 (throw t))
      :bound (do
               (reset! cell (assoc state :phase :forcing))
               (try
                 (let [v (thunk)]
                   (reset! cell {:phase :done
                                 :name name
                                 :value v})
                   v)
                 (catch Throwable t
                   (reset! cell {:phase :failed
                                 :name name
                                 :error t})
                   (throw t))))
      :pending (throw (ex-info "recursive binding cell is unbound"
                               {:name name
                                :phase phase})))))

(defn- rec-binding-symbols
  [binding-form]
  (cond
    (symbol? binding-form) #{binding-form}
    (vector? binding-form) (set (filter symbol? binding-form))
    (seq? binding-form) (set (filter symbol? binding-form))
    (map? binding-form) (set (filter symbol? (tree-seq coll? seq binding-form)))
    :else #{}))

(declare rewrite-recursive-refs)

(defn- rewrite-fn-recursive-refs
  [form env]
  (let [[_ & xs] form
        [fn-name xs] (if (symbol? (first xs))
                       [(first xs) (rest xs)]
                       [nil xs])
        env' (if fn-name (dissoc env fn-name) env)]
    (if (vector? (first xs))
      (let [params (first xs)
            body (rest xs)
            body-env (apply dissoc env' (rec-binding-symbols params))]
        (list* 'fn
               (concat (when fn-name [fn-name])
                       [params]
                       (map #(rewrite-recursive-refs % body-env) body))))
      (list* 'fn
             (concat (when fn-name [fn-name])
                     (map (fn [clause]
                            (let [params (first clause)
                                  body-env (apply dissoc env'
                                                  (rec-binding-symbols params))]
                              (list* params
                                     (map #(rewrite-recursive-refs % body-env)
                                          (rest clause)))))
                          xs))))))

(defn- rewrite-let-recursive-refs
  [op form env]
  (let [bindings (second form)
        body (nnext form)
        pairs (partition 2 bindings)]
    (loop [remaining pairs
           out []
           body-env env]
      (if-let [[binding init] (first remaining)]
        (let [init' (rewrite-recursive-refs init body-env)
              shadowed (rec-binding-symbols binding)]
          (recur (rest remaining)
                 (conj out binding init')
                 (apply dissoc body-env shadowed)))
        (list* op
               (vec out)
               (map #(rewrite-recursive-refs % body-env) body))))))

(defn- rewrite-letfn-recursive-refs
  [form env]
  (let [specs (second form)
        names (set (map first specs))
        outer-env (apply dissoc env names)
        specs' (mapv (fn [spec]
                       (let [name (first spec)
                             params (second spec)
                             body-env (apply dissoc outer-env
                                             (rec-binding-symbols params))]
                         (list* name
                                params
                                (map #(rewrite-recursive-refs % body-env)
                                     (nnext spec)))))
                     specs)]
    (list* 'letfn
           specs'
           (map #(rewrite-recursive-refs % outer-env) (nnext form)))))

(defn- rewrite-lazy-letrec-recursive-refs
  "A NESTED lazy-letrec is itself a (recursive) binding scope: its bound names
  shadow the enclosing env across BOTH its inits and its body. Without this, an
  outer lazy-letrec would rewrite a same-named inner binding's references to the
  outer cell and corrupt the inner scope (a real nested-shadowing bug)."
  [form env]
  (let [[op bindings & body] form
        pairs (partition 2 bindings)
        names (map first pairs)
        env' (apply dissoc env names)]
    (list* op
           (vec (mapcat (fn [[n init]]
                          [n (rewrite-recursive-refs init env')])
                        pairs))
           (map #(rewrite-recursive-refs % env') body))))

(defn- rewrite-recursive-refs
  [form env]
  (cond
    (symbol? form)
    (if-let [cell (get env form)]
      (list `force-rec-cell cell (list 'quote form))
      form)

    (seq? form)
    (let [op (first form)]
      (case op
        quote form
        fn (rewrite-fn-recursive-refs form env)
        let (rewrite-let-recursive-refs 'let form env)
        let* (rewrite-let-recursive-refs 'let* form env)
        loop (rewrite-let-recursive-refs 'loop form env)
        letfn (rewrite-letfn-recursive-refs form env)
        (if (and (symbol? op) (= "lazy-letrec" (name op)))
          (rewrite-lazy-letrec-recursive-refs form env)
          (apply list (map #(rewrite-recursive-refs % env) form)))))

    (vector? form)
    (mapv #(rewrite-recursive-refs % env) form)

    (map? form)
    (into (empty form)
          (map (fn [[k v]]
                 [(rewrite-recursive-refs k env)
                  (rewrite-recursive-refs v env)]))
          form)

    (set? form)
    (set (map #(rewrite-recursive-refs % env) form))

    :else
    form))

(defmacro lazy-letrec
  [bindings & body]
  (let [pairs (vec (partition 2 bindings))
        names (mapv first pairs)]
    (when (or (odd? (count bindings))
              (not (every? symbol? names)))
      (throw (ex-info "lazy-letrec requires simple symbol binding pairs"
                      {:bindings bindings})))
    (let [cells (zipmap names
                        (map (fn [name]
                               (gensym (str (munge (clojure.core/name name))
                                             "__cell__")))
                             names))
          env cells
          cell-bindings (mapcat (fn [name]
                                  [(get cells name)
                                   (list `make-rec-cell (list 'quote name))])
                                names)
          bind-forms (map (fn [[name init]]
                            (list `bind-rec-cell!
                                  (get cells name)
                                  (list 'fn []
                                        (rewrite-recursive-refs init env))))
                          pairs)]
      (list* 'let
             (vec cell-bindings)
             (concat bind-forms
                     (map #(rewrite-recursive-refs % env) body))))))

(defn- ensure-compile-namespace!
  [ns-ref]
  (cond
    (nil? ns-ref)
    *ns*

    (instance? Namespace ns-ref)
    ns-ref

    (symbol? ns-ref)
    (or (find-ns ns-ref)
        (let [target (create-ns ns-ref)]
          (binding [*ns* target]
            (clojure.core/refer 'clojure.core))
          target))

    (string? ns-ref)
    (ensure-compile-namespace! (symbol ns-ref))

    :else
    (throw (ex-info "compile opts :ns must be a symbol, string, Namespace, or nil"
                    {:type :invalid-compile-ns
                     :ns ns-ref}))))

(defn- compile-context-namespace
  [opts]
  (if (contains? opts :ns)
    (ensure-compile-namespace! (:ns opts))
    *ns*))

(defn- with-compile-context
  [opts f]
  (locking compile-lock
    (let [target (compile-context-namespace (or opts {}))]
      (binding [*ns* target]
        (f)))))

(defn- throwable-diagnostic
  [^Throwable t]
  {:class (.getName (class t))
   :message (.getMessage t)
   :data (ex-data t)})

(defn- compile-form-error
  [phase form ^Throwable t]
  (if (and (instance? clojure.lang.ExceptionInfo t)
           (= :compile-error (:type (ex-data t))))
    t
    (ex-info "compile-form failed"
             {:type :compile-error
              :phase phase
              :form (pr-str form)
              :cause t}
             t)))

(defn- throw-compile-form-error!
  [phase form ^Throwable t]
  (throw (compile-form-error phase form t)))

(defn- eval-compile-form-fallback
  ([phase form]
   (eval-compile-form-fallback phase form form))
  ([phase report-form eval-form]
   (try
     (eval eval-form)
     (catch Throwable t
       (throw-compile-form-error! phase report-form t)))))

(defn- cap-diagnostics
  [rows]
  (let [cap *fallback-diagnostics-cap*
        n   (count rows)]
    (if (and (integer? cap) (pos? cap) (> n cap))
      (subvec rows (- n cap))
      rows)))

(defn- record-diagnostic!
  [a row]
  (swap! a
         (fn [rows]
           (let [rows' (vec rows)
                 idx   (if-let [last-index (:index (peek rows'))]
                         (inc (long last-index))
                         0)]
             (conj rows' (assoc row :index idx))))))

(defn- record-bounded-diagnostic!
  [a row]
  (swap! a
         (fn [rows]
           (let [rows' (vec rows)
                 idx   (if-let [last-index (:index (peek rows'))]
                         (inc (long last-index))
                         0)]
             (cap-diagnostics (conj rows' (assoc row :index idx)))))))

(defn- record-compile-form-fallback!
  [row]
  (when *compile-form-local-fallback-diagnostics*
    (record-diagnostic! *compile-form-local-fallback-diagnostics* row))
  (record-bounded-diagnostic! compile-form-fallback-diagnostics row))

(defn- sha256-hex
  [s]
  (let [md    (MessageDigest/getInstance "SHA-256")
        bytes (.digest md (.getBytes (str s) "UTF-8"))]
    (apply str
           (map (fn [b]
                  (format "%02x" (bit-and (int b) 0xff)))
                bytes))))

(defn- compile-unit-id
  [form]
  (subs (sha256-hex
         (binding [*print-dup* false
                   *print-length* nil
                   *print-level* nil
                   *print-meta* false
                   *print-namespace-maps* false
                   *print-readably* true]
           (pr-str [(ns-name *ns*) form])))
        0 12))

(defn- define-generated-class
  [cname bytes]
  (try
    (.defineClass ^DynamicClassLoader *dcl* cname bytes nil)
    (catch LinkageError e
      (try
        (.loadClass ^DynamicClassLoader *dcl* cname)
        (catch Throwable _
          (throw e))))))

(defn- next-generated-class-name
  [kind]
  (str "pnix.clj_meta.gen."
       kind
       "__"
       *class-unit-id*
       "__"
       (swap! *gen-counter* inc)))

(defn- invoke-method
  "Object invoke(Object, Object, …)  (arity 개의 Object 인자)"
  ^Method [arity]
  (Method. "invoke" obj-type (into-array Type (repeat arity obj-type))))

;; ---------------------------------------------------------------------------
;; emit: AST 노드 하나를 평가해 스택에 Object 하나를 남긴다
;; ---------------------------------------------------------------------------

(declare emit-node emit-node-array emit-node-as emit-letfn emit-reify emit-deftype)

(declare emit-const-value)

(defn- node-line
  [node]
  (or (:line node) (get-in node [:env :line])))

(defn- node-source
  [node]
  (let [f (or (:file node) (get-in node [:env :file]))]
    (when (string? f)
      (if (= "NO_SOURCE_PATH" f)
        f
        (.getName (File. ^String f))))))

(defn- emit-line-number
  [^GeneratorAdapter ga node]
  (when-let [line (and *emit-line-numbers* (node-line node))]
    (when (and (integer? line) (pos? line))
      (let [line' (int line)]
        (when (or (nil? *last-emitted-line*)
                  (not= line' @*last-emitted-line*))
          (when *last-emitted-line*
            (reset! *last-emitted-line* line'))
          (let [label (.newLabel ga)]
            (.mark ga label)
            (.visitLineNumber ga line' label)))))))

(def ^:private eof-sentinel (Object.))

(defn- read-source-forms
  [^String s]
  (let [r (LineNumberingPushbackReader. (StringReader. s))]
    (loop [forms []]
      (let [form (read r false eof-sentinel)]
        (if (identical? form eof-sentinel)
          forms
          (recur (conj forms form)))))))

(defn- read-source-form
  [^String s]
  (first (read-source-forms s)))

(defn- push-str-or-null
  [^GeneratorAdapter ga s]
  (if (nil? s) (.visitInsn ga Opcodes/ACONST_NULL) (.push ga ^String s)))

(defn- box-value
  [^GeneratorAdapter ga ^Type t]
  (.valueOf ga t))

(defn- long-sized-integer?
  [v]
  (or (instance? Long v)
      (instance? Integer v)
      (instance? Short v)
      (instance? Byte v)))

(defn- emit-biginteger-value
  [^GeneratorAdapter ga v]
  (let [^BigInteger bi (biginteger v)]
    (.newInstance ga biginteger-type)
    (.dup ga)
    (.push ga (.toString bi))
    (.invokeConstructor ga biginteger-type biginteger-string-ctor)))

(defn- emit-const-array
  "상수 값들의 Object[] 를 스택에 만든다(컬렉션 리터럴 재료)."
  [^GeneratorAdapter ga vals]
  (.push ga (int (count vals)))
  (.newArray ga obj-type)
  (doseq [[i x] (map-indexed vector vals)]
    (.dup ga)
    (.push ga (int i))
    (emit-const-value ga x)
    (.arrayStore ga obj-type)))

(defn- emit-const-value
  "원시 Clojure 상수 하나를 bytecode 로(스택에 Object 하나)."
  [^GeneratorAdapter ga v]
  (cond
    (nil? v)              (.visitInsn ga Opcodes/ACONST_NULL)
    (long-sized-integer? v) (do (.push ga (long v))    (box-value ga Type/LONG_TYPE))
    (instance? BigInt v)  (do (emit-biginteger-value ga v)
                              (.invokeStatic ga bigint-type bigint-from-biginteger-method))
    (instance? BigInteger v) (emit-biginteger-value ga v)
    (instance? Boolean v) (do (.push ga (boolean v)) (box-value ga Type/BOOLEAN_TYPE))
    (char? v)             (do (.push ga (int v))     (box-value ga Type/CHAR_TYPE))
    (instance? Float v)   (do (.push ga (float v))   (box-value ga Type/FLOAT_TYPE))
    (double? v)           (do (.push ga (double v))  (box-value ga Type/DOUBLE_TYPE))
    (instance? BigDecimal v) (do (.newInstance ga bigdec-type)
                                 (.dup ga)
                                 (.push ga (.toString ^BigDecimal v))
                                 (.invokeConstructor ga bigdec-type bigdec-string-ctor))
    (instance? Ratio v)   (do (.newInstance ga ratio-type)
                              (.dup ga)
                              (emit-biginteger-value ga (.numerator ^Ratio v))
                              (emit-biginteger-value ga (.denominator ^Ratio v))
                              (.invokeConstructor ga ratio-type ratio-ctor))
    (instance? Pattern v) (do (.push ga (.pattern ^Pattern v))
                              (.invokeStatic ga pattern-type pattern-compile-method))
    (string? v)           (.push ga ^String v)
    (class? v)            (.push ga (Type/getType ^Class v))
    (instance? Var v)     (do (.push ga (str (ns-name (.ns ^Var v))))
                              (.push ga (str (.sym ^Var v)))
                              (.invokeStatic ga rt-type rt-var-method))
    (keyword? v)          (do (push-str-or-null ga (namespace v))
                              (.push ga ^String (name v))
                              (.invokeStatic ga kw-type kw-intern-method))
    (symbol? v)           (do (push-str-or-null ga (namespace v))
                              (.push ga ^String (name v))
                              (.invokeStatic ga sym-type sym-intern-method))
    (vector? v)           (do (emit-const-array ga v)
                              (.invokeStatic ga rt-type rt-vector-method))
    (set? v)              (do (emit-const-array ga (seq v))
                              (.invokeStatic ga rt-type rt-set-method))
    (seq? v)              (do (emit-const-array ga v)
                              (.invokeStatic ga rt-type rt-array-to-list-method))
    (map? v)              (do (emit-const-array ga (mapcat identity v))
                              (.invokeStatic ga rt-type rt-map-method))
    :else (throw (ex-info (str "unsupported const value: " (pr-str v))
                          {:type :unsupported-op :val v :class (class v)}))))

(defn- cacheable-const?
  "static field 로 캐싱할 가치가 있는 const(생성 비용 있는 복합값). primitive 는 ldc 로 충분."
  [v]
  (or (keyword? v) (symbol? v) (vector? v) (map? v) (set? v) (seq? v)
      (instance? BigInt v) (instance? BigInteger v) (instance? BigDecimal v)
      (instance? Ratio v) (instance? Pattern v)))

(defn- const-field!
  "클래스의 const-fields atom 에 const v 의 static field 이름을 등록/반환(값 기준 dedup)."
  [const-atom v]
  (or (get @const-atom v)
      (let [fname (str "CONST_" (count @const-atom))]
        (swap! const-atom assoc v fname)
        fname)))

(defn- strip-one-quote
  "code-literal 컬렉션 값에서 (quote x) → x 로 한 겹 벗긴다(중첩 컬렉션 재귀).
  tools.analyzer.jvm 가 `{:k 'sym}` 같은 *code-position* 컬렉션을 const-fold 할 때 값에
  quote 폼을 한 겹 더 남기는 quirk 를 host 평가와 맞춘다."
  [v]
  (cond
    (and (seq? v) (= 'quote (first v)) (= 2 (count v))) (second v)
    (map? v)    (into (empty v) (map (fn [[k val]] [(strip-one-quote k) (strip-one-quote val)])) v)
    (vector? v) (mapv strip-one-quote v)
    (set? v)    (into (empty v) (map strip-one-quote) v)
    :else v))

(defn- const-true-value
  "분석기 :const 의 진짜 값. :form 이 *bare* 컬렉션 리터럴(map/vector/set, quote 로 안 감싼
  code-position)이면 :val 에서 quote 한 겹을 벗긴다(host 평가와 일치). quoted-data
  (:form 이 (quote …))나 primitive 는 :val 그대로."
  [node]
  (let [form (:form node)
        v    (:val node)]
    (if (and (or (map? form) (vector? form) (set? form))
             (coll? v))
      (try (strip-one-quote v) (catch Throwable _ v))
      v)))

(defn- emit-const
  "복합 const(컬렉션/keyword/symbol)는 static field 캐싱(매번 생성 제거), primitive 는 직접."
  [^GeneratorAdapter ga env node]
  (let [v (const-true-value node)]
    (if (cacheable-const? v)
      (.getStatic ga ^Type (:self-ctype env) (const-field! (:const-fields env) v) obj-type)
      (emit-const-value ga v))))

(defn- emit-local
  "지역 참조. env 값은 [:arg i ?pclass ?range ?proof] | [:jvm-arg i pclass] |
  [:local slot ?pclass ?range ?proof] | [:this] | [:field fname](캡처)."
  [^GeneratorAdapter ga env node]
  (if-let [[kind idx pclass] (get env (:name node))]
    (condp = kind
      :arg   (.loadArg ga (int idx))
      :jvm-arg (do
                 (.loadArg ga (int idx))
                 (when (and (class? pclass) (.isPrimitive ^Class pclass))
                   (box-value ga (Type/getType ^Class pclass))))
      :local (do
               (.loadLocal ga (int idx))
               (when (and (class? pclass) (.isPrimitive ^Class pclass))
                 (box-value ga (Type/getType ^Class pclass))))
      :this  (.loadThis ga)
      :field (do (.loadThis ga)
                 (if (and (class? pclass) (.isPrimitive ^Class pclass))
                   ;; typed primitive 필드(예: defrecord __hash:int) → getfield 후 box
                   (do (.getField ga ^Type (:self-ctype env) ^String idx (Type/getType ^Class pclass))
                       (box-value ga (Type/getType ^Class pclass)))
                   (.getField ga ^Type (:self-ctype env) ^String idx obj-type)))
      (throw (ex-info (str "unknown local target kind: " kind)
                      {:kind kind :name (:name node)})))
    (throw (ex-info (str "unknown local: " (:name node)) {:name (:name node) :env (keys env)}))))

(defn- object-local-entry?
  [entry]
  (let [[kind slot pclass] entry]
    (and (= :local kind)
         (integer? slot)
         (not (and (class? pclass)
                   (.isPrimitive ^Class pclass))))))

(defn- clear-object-locals!
  [^GeneratorAdapter ga entries]
  (when-not *disable-locals-clearing*
    (doseq [slot (distinct (keep (fn [entry]
                                   (when (object-local-entry? entry)
                                     (second entry)))
                                 entries))]
      (.visitInsn ga Opcodes/ACONST_NULL)
      (.storeLocal ga (int slot)))))

(defn- push-var
  "Var 를 런타임에 해소해 스택에 올린다(RT.var(ns, name)). Var 는 IFn."
  [^GeneratorAdapter ga ^Var v]
  (let [m (meta v)]
    (.push ga (str (ns-name (:ns m))))
    (.push ga (str (:name m)))
    (.invokeStatic ga rt-type rt-var-method)))

(defn- var-field!
  "클래스의 var-fields atom 에 var v 의 static field 이름을 등록/반환(var 캐싱)."
  [var-atom ^Var v]
  (or (get @var-atom v)
      (let [fname (str "VAR_" (count @var-atom))]
        (swap! var-atom assoc v fname)
        fname)))

(defn- load-var
  "캐싱된 Var 를 getstatic 으로 올린다(매 호출 RT.var 조회 제거; host 와 동일)."
  [^GeneratorAdapter ga env ^Var v]
  (.getStatic ga ^Type (:self-ctype env) (var-field! (:var-fields env) v) var-type))

(defn- emit-var
  "값 위치의 var → 캐싱된 Var getstatic + deref(현재 값)."
  [^GeneratorAdapter ga env node]
  (load-var ga env (:var node))
  (.invokeVirtual ga var-type var-deref-method))

(defn- emit-the-var
  "#'x → 캐싱된 Var getstatic (deref 안 함)."
  [^GeneratorAdapter ga env node]
  (load-var ga env (:var node)))

(defn- emit-def
  "(def name init?) — Var 에 루트 바인딩 설정. def 는 Var 를 반환."
  [^GeneratorAdapter ga env node]
  (push-var ga (:var node))                 ; Var
  (when (:dynamic (meta (:var node)))
    (.dup ga)                                ; Var Var
    (.invokeVirtual ga var-type var-set-dynamic-method)
    (.pop ga))                               ; Var
  (when (:init node)
    (.dup ga)                                ; Var Var
    (emit-node ga env (:init node))          ; Var Var val
    (.invokeVirtual ga var-type bindroot-method)))  ; Var (남음)

(defn- emit-invoke
  "(f a b …) — 수신자 IFn 를 올리고 인자들을 올린 뒤 IFn.invoke(Object…) 호출."
  [^GeneratorAdapter ga env node]
  (emit-node ga env (:fn node))
  (.checkCast ga ifn-type)
  (doseq [a (:args node)]
    (emit-node ga env a))
  (.invokeInterface ga ifn-type (invoke-method (count (:args node)))))

(defn- emit-protocol-invoke
  "protocol fn 호출. analyzer AST 는 target 을 args 에서 분리하므로 첫 인자로 붙인다."
  [^GeneratorAdapter ga env node]
  (emit-node ga env (:protocol-fn node))
  (.checkCast ga ifn-type)
  (emit-node ga env (:target node))
  (doseq [a (:args node)]
    (emit-node ga env a))
  (.invokeInterface ga ifn-type (invoke-method (inc (count (:args node))))))

;; ── interop 공통: 인자 타입 해소 + coercion ──────────────────────────────
(defn- arg-class
  "인자 노드의 정적 타입(Class).
  interop 검증으로 좁혀진 primitive :tag 는 우선 사용하고, 그 외에는 :o-tag 를 쓴다."
  ^Class [node]
  (let [tag (:tag node)
        otag (:o-tag node)]
    (cond
      (and (class? tag) (.isPrimitive ^Class tag)) tag
      (and (class? tag) (not= Object tag)) tag
      (and (= :const (:op node)) (class? (:val node))) Class
      (and (= :const (:op node)) (char? (:val node))) Character/TYPE
      (and (= :const (:op node)) (string? (:val node))) String
      (and (= :const (:op node)) (instance? BigInt (:val node))) BigInt
      (and (= :const (:op node)) (instance? BigInteger (:val node))) BigInteger
      (and (= :const (:op node)) (instance? BigDecimal (:val node))) BigDecimal
      (and (= :const (:op node)) (instance? Ratio (:val node))) Ratio
      (and (= :const (:op node)) (instance? Pattern (:val node))) Pattern
      (and (= :const (:op node)) (instance? Float (:val node))) Float/TYPE
      (and (= :const (:op node)) (double? (:val node))) Double/TYPE
      (and (= :const (:op node)) (long-sized-integer? (:val node))) Long/TYPE
      (and (= :const (:op node)) (instance? Boolean (:val node))) Boolean/TYPE
      (class? otag) otag
      (class? tag) tag
      :else Object)))

(def ^:private primitive->wrapper
  {Boolean/TYPE Boolean
   Byte/TYPE Byte
   Character/TYPE Character
   Short/TYPE Short
   Integer/TYPE Integer
   Long/TYPE Long
   Float/TYPE Float
   Double/TYPE Double})

(def ^:private wrapper->primitive
  (into {} (map (fn [[p w]] [w p]) primitive->wrapper)))

(def ^:private primitive-widening
  {Byte/TYPE      [Short/TYPE Integer/TYPE Long/TYPE Float/TYPE Double/TYPE]
   Short/TYPE     [Integer/TYPE Long/TYPE Float/TYPE Double/TYPE]
   Character/TYPE [Integer/TYPE Long/TYPE Float/TYPE Double/TYPE]
   Integer/TYPE   [Long/TYPE Float/TYPE Double/TYPE]
   Long/TYPE      [Float/TYPE Double/TYPE]
   Float/TYPE     [Double/TYPE]})

(def ^:private primitive-param-widening
  {Byte/TYPE      [Short/TYPE Integer/TYPE Long/TYPE]
   Short/TYPE     [Integer/TYPE Long/TYPE]
   Character/TYPE [Integer/TYPE Long/TYPE]
   Integer/TYPE   [Long/TYPE]
   Float/TYPE     [Double/TYPE]})

(def ^:private typed-local-primitives
  #{Boolean/TYPE Long/TYPE Double/TYPE})

(defn- typed-local-primitive-class
  [node]
  (let [c (arg-class node)]
    (when (and (class? c)
               (.isPrimitive ^Class c)
               (contains? typed-local-primitives c))
      c)))

(defn- widening-distance
  [^Class from ^Class to]
  (when-let [xs (seq (get primitive-widening from))]
    (let [i (.indexOf ^java.util.List xs to)]
      (when-not (neg? i)
        (inc i)))))

(defn- primitive-param-distance
  [^Class from ^Class to]
  (when-let [xs (seq (get primitive-param-widening from))]
    (let [i (.indexOf ^java.util.List xs to)]
      (when-not (neg? i)
        (inc i)))))

(defn- class-distance
  "from 이 to 로 assignable 할 때의 대략적 거리. overload tie-break 용도."
  [^Class from ^Class to]
  (cond
    (= from to) 0
    (= Object to) 100
    :else
    (loop [seen #{from}
           q    (conj clojure.lang.PersistentQueue/EMPTY [from 0])]
      (if (empty? q)
        100
        (let [[[c d] q'] [(peek q) (pop q)]]
          (if (= c to)
            d
            (let [nexts (remove nil? (cons (.getSuperclass ^Class c)
                                           (seq (.getInterfaces ^Class c))))
                  fresh (remove seen nexts)]
              (recur (into seen fresh)
                     (into q' (map #(vector % (inc d)) fresh))))))))))

(defn- param-score
  "낮을수록 더 구체적인 overload 후보. nil 은 정적으로 안전하지 않은 후보."
  [^Class arg ^Class param]
  (cond
    (= arg param) 0

    (.isPrimitive arg)
    (cond
      (.isPrimitive param) (when-let [d (primitive-param-distance arg param)] (+ 10 d))
      (= param Object) 90
      :else (let [boxed (get primitive->wrapper arg)]
              (when (and boxed (.isAssignableFrom param boxed))
                (+ (if (= boxed param) 20 40)
                   (class-distance boxed param)))))

    (.isPrimitive param)
    (when-let [unboxed (get wrapper->primitive arg)]
      (cond
        (= unboxed param) 20
        :else (when-let [d (primitive-param-distance unboxed param)] (+ 30 d))))

    (.isAssignableFrom param arg)
    (+ (if (= param Object) 90 40)
       (class-distance arg param))

    :else nil))

(defn- executable-score
  [pclasses arg-classes]
  (when (= (count pclasses) (count arg-classes))
    (let [scores (map param-score arg-classes pclasses)]
      (when (every? some? scores)
        (reduce + scores)))))

(defn- permissive-param-score
  "Fallback for analyzer-lost reference types. Only Object→reference is relaxed;
  primitive targets still require a statically known primitive/wrapper."
  [^Class arg ^Class param]
  (or (param-score arg param)
      (when (and (= arg Object) (not (.isPrimitive param)))
        (+ 200 (if (.isArray param) 0 25)))))

(defn- permissive-executable-score
  [pclasses arg-classes]
  (when (= (count pclasses) (count arg-classes))
    (let [scores (map permissive-param-score arg-classes pclasses)]
      (when (every? some? scores)
        (reduce + scores)))))

(defn- reflect-key
  [member]
  (str member))

(defn- best-executable
  [members pclasses-fn arg-classes]
  (->> members
       (keep (fn [m]
               (when-let [score (executable-score (vec (pclasses-fn m)) arg-classes)]
                 [score (reflect-key m) m])))
       (sort-by (juxt first second))
       first
       peek))

(defn- best-executable-permissive
  "Pick a permissive overload only when the best score is unambiguous."
  [members pclasses-fn arg-classes]
  (let [ranked (->> members
                    (keep (fn [m]
                            (when-let [score (permissive-executable-score
                                              (vec (pclasses-fn m)) arg-classes)]
                              [score (reflect-key m) m])))
                    (sort-by (juxt first second))
                    vec)
        best-score (ffirst ranked)
        winners (when best-score
                  (filter #(= best-score (first %)) ranked))]
    (when (= 1 (count winners))
      (peek (first winners)))))

(defn- coerce-arg
  "스택의 Object 를 파라미터 타입 pc 로 맞춘다(primitive → unbox, ref → checkCast)."
  [^GeneratorAdapter ga ^Class pc]
  (cond
    (= pc Object)     nil
    (.isPrimitive pc) (.unbox ga (Type/getType pc))
    :else             (.checkCast ga (Type/getType pc))))

(defn- emit-args-coerced
  [^GeneratorAdapter ga env arg-nodes ^"[Ljava.lang.Class;" pclasses]
  (dotimes [i (count arg-nodes)]
    (let [pc (aget pclasses i)]
      (emit-node-as ga env (nth arg-nodes i) pc))))

(defn- const-long-literal
  [node]
  (when (and (= :const (:op node))
             (instance? Long (:val node)))
    (:val node)))

(declare long-range-of independent-long-range-of)

(defn- singleton-long-range
  [v]
  {:min (long v) :max (long v)})

(defn- ensure-entry-index
  [entry idx]
  (loop [v (vec entry)]
    (if (< idx (count v))
      v
      (recur (conj v nil)))))

(defn- attach-independent-long-range
  [entry r]
  (if r
    (let [entry' (ensure-entry-index entry 4)
          proof  (or (nth entry' 4) {})]
      (assoc entry' 4 (assoc proof :independent-range r)))
    entry))

(defn- ranged-env-entry
  [kind slot pclass r independent-r]
  (let [entry (cond
                (class? pclass) [kind slot pclass]
                (some? pclass)  [kind slot pclass]
                :else           [kind slot])
        entry' (if r
                 (assoc (ensure-entry-index entry 3) 3 r)
                 entry)]
    (attach-independent-long-range entry' independent-r)))

(defn- entry-long-range
  [entry]
  (when (and entry (< 3 (count entry)))
    (nth entry 3)))

(defn- entry-independent-long-range
  [entry]
  (when (and entry (< 4 (count entry)))
    (:independent-range (nth entry 4))))

(defn- local-long-range
  [env node]
  (entry-long-range (get env (:name node))))

(defn- local-independent-long-range
  [env node]
  (entry-independent-long-range (get env (:name node))))

(defn- long-range-intersection
  [a b]
  (let [lo (long (max (:min a) (:min b)))
        hi (long (min (:max a) (:max b)))]
    (when (<= lo hi)
      {:min lo
       :max hi})))

(defn- narrow-local-long-range
  [env local-name r]
  (if-let [entry (get env local-name)]
    (let [[kind] entry]
      (if (or (= :arg kind) (= :local kind))
        (let [entry' (ensure-entry-index entry 3)
              old    (entry-long-range entry')
              r'     (if old
                       (long-range-intersection old r)
                       r)]
          (if r'
            (assoc env local-name (assoc entry' 3 r'))
            env))
        env))
    env))

(defn- exact-long-op
  [^String mname a b]
  (try
    (cond
      (= "add" mname)
      {:value (Math/addExact (long a) (long b))}

      (= "minus" mname)
      {:value (Math/subtractExact (long a) (long b))}

      (= "multiply" mname)
      {:value (Math/multiplyExact (long a) (long b))}

      :else nil)
    (catch ArithmeticException _
      nil)))

(defn- exact-ceil-div-positive
  [n d]
  (when (and (not (neg? n))
             (pos? d))
    (try
      (let [q (quot n d)
            r (rem n d)]
        {:value (long (if (zero? r)
                        q
                        (Math/addExact (long q) 1)))})
      (catch ArithmeticException _
        nil))))

(defn- long-range-from-values
  [values]
  (when (seq values)
    {:min (long (reduce min values))
     :max (long (reduce max values))}))

(defn- checked-long-range-result
  [^String mname ar br]
  (let [amin (:min ar)
        amax (:max ar)
        bmin (:min br)
        bmax (:max br)]
    (cond
      (= "add" mname)
      (when-let [lo (exact-long-op mname amin bmin)]
        (when-let [hi (exact-long-op mname amax bmax)]
          (long-range-from-values [(:value lo) (:value hi)])))

      (= "minus" mname)
      (when-let [lo (exact-long-op mname amin bmax)]
        (when-let [hi (exact-long-op mname amax bmin)]
          (long-range-from-values [(:value lo) (:value hi)])))

      (= "multiply" mname)
      (let [pairs [[amin bmin] [amin bmax] [amax bmin] [amax bmax]]
            vals  (loop [xs pairs
                         acc []]
                    (if (seq xs)
                      (let [[a b] (first xs)]
                        (if-let [r (exact-long-op mname a b)]
                          (recur (next xs) (conj acc (:value r)))
                          nil))
                      acc))]
        (when (= 4 (count vals))
          (long-range-from-values vals)))

      :else nil)))

(defn- checked-long-result-range
  [env ^String mname args]
  (when (= 2 (count args))
    (let [ar (long-range-of env (first args))
          br (long-range-of env (second args))]
      (when (and ar br)
        (checked-long-range-result mname ar br)))))

(defn- independent-long-range-info
  [env node]
  (cond
    (some? (const-long-literal node))
    {:range (singleton-long-range (const-long-literal node))
     :independent? false}

    (= :local (:op node))
    (when-let [r (local-independent-long-range env node)]
      {:range r
       :independent? true})

    (and (= :static-call (:op node))
         (= Numbers (:class node))
         (= 2 (count (:args node))))
    (let [a (independent-long-range-info env (first (:args node)))
          b (independent-long-range-info env (second (:args node)))]
      (when (and a b (or (:independent? a) (:independent? b)))
        (when-let [r (checked-long-range-result
                      (str (:method node))
                      (:range a)
                      (:range b))]
          {:range r
           :independent? true})))

    :else nil))

(defn- independent-long-range-of
  [env node]
  (let [info (independent-long-range-info env node)]
    (when (:independent? info)
      (:range info))))

(defn- long-range-of
  [env node]
  (cond
    (some? (const-long-literal node))
    (singleton-long-range (const-long-literal node))

    (= :local (:op node))
    (local-long-range env node)

    (and (= :static-call (:op node))
         (= Numbers (:class node)))
    (checked-long-result-range env (str (:method node)) (:args node))

    :else nil))

(defn- range->interval
  [r]
  (ai/interval (:min r) (:max r)))

(defn- independent-range-spec
  [r]
  (when r
    {:interval (range->interval r)
     :source :compiler-bounded-unroll}))

(defn- checked-long-static-no-overflow?
  [env ^String mname args]
  (let [op (cond
             (= "add" mname) :+
             (= "minus" mname) :-
             (= "multiply" mname) :*
             :else nil)
        opcode (cond
                 (= "add" mname) :ladd
                 (= "minus" mname) :lsub
                 (= "multiply" mname) :lmul
                 :else nil)]
    (if (and op opcode (= 2 (count args)))
      (let [ar (long-range-of env (first args))
            br (long-range-of env (second args))
            iar (when-not *disable-independent-range-admission*
                  (independent-long-range-of env (first args)))
            ibr (when-not *disable-independent-range-admission*
                  (independent-long-range-of env (second args)))]
        (boolean
         (when (and ar br)
           (tv/lowering-sound?
            (tv/make-vc
             (cond-> {:id (keyword (str "compiler-checked-long-" mname))
                      :op op
                      :opcode opcode
                      :lhs (range->interval ar)
                      :rhs (range->interval br)}
               (or iar ibr)
               (assoc :independent
                      (cond-> {}
                        iar (assoc :lhs (independent-range-spec iar))
                        ibr (assoc :rhs (independent-range-spec ibr))))))))))
      false)))

(def ^:private checked-long-candidate-methods
  #{"add" "minus" "multiply"})

(defn- checked-long-candidate-node?
  [node]
  (and (= :static-call (:op node))
       (= Numbers (:class node))
       (contains? checked-long-candidate-methods (str (:method node)))
       (= 2 (count (:args node)))))

(defn- has-checked-long-candidate?
  [node]
  (letfn [(walk [n]
            (and (map? n)
                 (cond
                   (checked-long-candidate-node? n)
                   true

                   (or (= :fn (:op n))
                       (= :loop (:op n)))
                   false

                   :else
                   (some (fn [k]
                           (let [v (get n k)]
                             (if (sequential? v)
                               (some walk v)
                               (walk v))))
                         (:children n)))))]
    (boolean (walk node))))

(defn- same-local-name?
  [node local-name]
  (and (= :local (:op node))
       (= local-name (:name node))))

(defn- current-loop-recur-exprs*
  [node acc]
  (when (map? node)
    (cond
      (= :recur (:op node))
      (swap! acc conj (:exprs node))

      (or (= :fn (:op node))
          (= :loop (:op node)))
      nil

      :else
      (doseq [k (:children node)]
        (let [v (get node k)]
          (if (vector? v)
            (doseq [x v] (current-loop-recur-exprs* x acc))
            (current-loop-recur-exprs* v acc)))))))

(defn- current-loop-recur-exprs
  [body]
  (let [acc (atom [])]
    (current-loop-recur-exprs* body acc)
    @acc))

(defn- loop-binding-invariant?
  [recur-exprs idx local-name]
  (every? (fn [exprs]
            (same-local-name? (nth exprs idx nil) local-name))
          recur-exprs))

(defn- numbers-call?
  [node ^String mname]
  (and (= :static-call (:op node))
       (= Numbers (:class node))
       (= mname (str (:method node)))))

(defn- add-local-positive-stride
  [node local-name]
  (when (numbers-call? node "add")
    (let [[a b] (:args node)]
      (let [stride (cond
                     (same-local-name? a local-name)
                     (const-long-literal b)

                     (same-local-name? b local-name)
                     (const-long-literal a)

                     :else nil)]
        (when (and stride (pos? stride))
          stride)))))

(defn- subtract-local-positive-stride
  [node local-name]
  (when (numbers-call? node "minus")
    (let [[a b] (:args node)]
      (let [stride (when (same-local-name? a local-name)
                     (const-long-literal b))]
        (when (and stride (pos? stride))
          stride)))))

(defn- add-local-variable-term
  [node local-name]
  (when (numbers-call? node "add")
    (let [[a b] (:args node)]
      (cond
        (and (same-local-name? a local-name)
             (= :local (:op b)))
        (:name b)

        (and (same-local-name? b local-name)
             (= :local (:op a)))
        (:name a)

        :else nil))))

(defn- subtract-local-variable-term
  [node local-name]
  (when (numbers-call? node "minus")
    (let [[a b] (:args node)]
      (when (and (same-local-name? a local-name)
                 (= :local (:op b)))
        (:name b)))))

(defn- multiply-local-positive-factor
  [node local-name]
  (when (numbers-call? node "multiply")
    (let [[a b] (:args node)
          factor (cond
                   (same-local-name? a local-name)
                   (const-long-literal b)

                   (same-local-name? b local-name)
                   (const-long-literal a)

                   :else nil)]
      (when (and factor (pos? factor))
        factor))))

(defn- loop-lt-local-bound
  [body]
  (let [test (:test body)]
    (when (and (= :if (:op body))
               (numbers-call? test "lt"))
      (let [[a b] (:args test)]
        (when (and (= :local (:op a))
                   (some? (const-long-literal b)))
          {:name (:name a)
           :bound (const-long-literal b)})))))

(defn- loop-gt-local-bound
  [body]
  (let [test (:test body)]
    (when (and (= :if (:op body))
               (numbers-call? test "gt"))
      (let [[a b] (:args test)]
        (when (and (= :local (:op a))
                   (some? (const-long-literal b)))
          {:name (:name a)
           :bound (const-long-literal b)})))))

(defn- loop-positive-index-iteration-count
  [env body then-recurs else-recurs bindings]
  (let [lt-bound (loop-lt-local-bound body)]
    (when (and (seq then-recurs)
               (empty? else-recurs))
      (some (fn [[idx binding]]
              (let [local-name (:name binding)
                    init-range (long-range-of env (:init binding))
                    strides    (keep (fn [exprs]
                                       (add-local-positive-stride
                                        (nth exprs idx nil)
                                        local-name))
                                     then-recurs)]
                (when (and init-range
                           (= (:min init-range) (:max init-range))
                           (= local-name (:name lt-bound))
                           (<= (:max init-range) (:bound lt-bound))
                           (= (count strides) (count then-recurs)))
                  (let [stride (long (reduce min strides))]
                    (when-let [distance (exact-long-op
                                          "minus"
                                          (:bound lt-bound)
                                          (:max init-range))]
                      (when-let [iterations (exact-ceil-div-positive
                                             (:value distance)
                                             stride)]
                        {:idx idx
                         :name local-name
                         :init (:max init-range)
                         :stride stride
                         ;; 지수합(Σi) 닫힌식이 *정확*하려면 모든 then-recur 의 index
                         ;; stride 가 동일(homogeneous)해야 한다. min-stride 는 iteration
                         ;; *횟수* 상한엔 sound 하나, 이질적 stride 면 큰 초기 stride 가
                         ;; 이후 모든 항을 들어올려 실제 Σi > min-stride 닫힌식 → 도출
                         ;; range 가 너무 좁아짐(unsound). add-terms/sub-terms 분기는
                         ;; 이 flag 로만 닫힌식을 쓰고, 아니면 sound 한 unroll/interval
                         ;; 경로로 fall through 한다.
                         :homogeneous-index-stride? (apply = strides)
                         :bound (:bound lt-bound)
                         :iterations (:value iterations)}))))))
            (map-indexed vector bindings)))))

(defn- exact-positive-index-sum
  [init stride iterations]
  (when (and (not (neg? init))
             (pos? stride)
             (not (neg? iterations)))
    (if (zero? iterations)
      {:value 0}
      (when-let [twice-init (exact-long-op "multiply" 2 init)]
        (when-let [n-1 (exact-long-op "minus" iterations 1)]
          (when-let [tail (exact-long-op "multiply" (:value n-1) stride)]
            (when-let [inner (exact-long-op "add" (:value twice-init) (:value tail))]
              (when-let [twice-sum (exact-long-op "multiply" iterations (:value inner))]
                {:value (long (quot (:value twice-sum) 2))}))))))))

(defn- exact-positive-factor-power
  [factor iterations]
  (when (and (pos? factor)
             (not (neg? iterations)))
    (loop [n iterations
           acc 1]
      (if (zero? n)
        {:value acc}
        (when-let [acc' (exact-long-op "multiply" acc factor)]
          (recur (- n 1) (:value acc')))))))

(defn- loop-binding-accumulator-range
  [env body recur-exprs idx binding bindings then-recurs else-recurs init-range]
  (when (and (seq recur-exprs)
             (seq then-recurs)
             (empty? else-recurs))
    (when-let [counter (loop-positive-index-iteration-count
                        env
                        body
                        then-recurs
                        else-recurs
                        bindings)]
      (when-not (= idx (:idx counter))
        (let [local-name (:name binding)
              add-strides (keep (fn [exprs]
                                  (add-local-positive-stride
                                   (nth exprs idx nil)
                                   local-name))
                                then-recurs)
              sub-strides (keep (fn [exprs]
                                  (subtract-local-positive-stride
                                   (nth exprs idx nil)
                                   local-name))
                                then-recurs)
              add-terms (keep (fn [exprs]
                                (add-local-variable-term
                                 (nth exprs idx nil)
                                 local-name))
                              then-recurs)
              sub-terms (keep (fn [exprs]
                                (subtract-local-variable-term
                                 (nth exprs idx nil)
                                 local-name))
                              then-recurs)
              mul-factors (keep (fn [exprs]
                                  (multiply-local-positive-factor
                                   (nth exprs idx nil)
                                   local-name))
                                then-recurs)]
          (when (and init-range
                     (= (:min init-range) (:max init-range)))
            (or
             (when (= (count add-strides) (count then-recurs))
               (let [stride (long (reduce max add-strides))]
                 (when-let [delta (exact-long-op
                                   "multiply"
                                   (:iterations counter)
                                   stride)]
                   (when-let [hi (exact-long-op
                                  "add"
                                  (:max init-range)
                                  (:value delta))]
                     {:min (:min init-range)
                      :max (:value hi)}))))
             (when (= (count sub-strides) (count then-recurs))
               (let [stride (long (reduce max sub-strides))]
                 (when-let [delta (exact-long-op
                                   "multiply"
                                   (:iterations counter)
                                   stride)]
                   (when-let [lo (exact-long-op
                                  "minus"
                                  (:min init-range)
                                  (:value delta))]
                     {:min (:value lo)
                      :max (:max init-range)}))))
             (when (and (= (count add-terms) (count then-recurs))
                        (every? #(= (:name counter) %) add-terms)
                        (or (:homogeneous-index-stride? counter)
                            *disable-homogeneous-index-stride-gate*))
               (when-let [delta (exact-positive-index-sum
                                 (:init counter)
                                 (:stride counter)
                                 (:iterations counter))]
                 (when-let [hi (exact-long-op
                                "add"
                                (:max init-range)
                                (:value delta))]
                   {:min (:min init-range)
                    :max (:value hi)})))
             (when (and (= (count sub-terms) (count then-recurs))
                        (every? #(= (:name counter) %) sub-terms)
                        (or (:homogeneous-index-stride? counter)
                            *disable-homogeneous-index-stride-gate*))
               (when-let [delta (exact-positive-index-sum
                                 (:init counter)
                                 (:stride counter)
                                 (:iterations counter))]
                 (when-let [lo (exact-long-op
                                "minus"
                                (:min init-range)
                                (:value delta))]
                   {:min (:value lo)
                    :max (:max init-range)})))
             (when (and (seq then-recurs)
                        (not (neg? (:min init-range)))
                        (= (count mul-factors) (count then-recurs)))
               (let [factor (long (reduce max mul-factors))]
                 (when-let [scale (exact-positive-factor-power
                                   factor
                                   (:iterations counter))]
                   (when-let [hi (exact-long-op
                                  "multiply"
                                  (:max init-range)
                                  (:value scale))]
                     {:min (:min init-range)
                      :max (:value hi)})))))))))))

(defn- loop-binding-bounded-step-range
  [body recur-exprs idx binding then-recurs else-recurs init-range]
  (let [local-name (:name binding)
        lt-bound   (loop-lt-local-bound body)
        gt-bound   (loop-gt-local-bound body)
        add-strides (keep (fn [exprs]
                            (add-local-positive-stride (nth exprs idx nil) local-name))
                          then-recurs)
        sub-strides (keep (fn [exprs]
                            (subtract-local-positive-stride (nth exprs idx nil) local-name))
                          then-recurs)]
    (or
     (when (and (seq recur-exprs)
                (seq then-recurs)
                (empty? else-recurs)
                init-range
                (= (:min init-range) (:max init-range))
                (= local-name (:name lt-bound))
                (<= (:max init-range) (:bound lt-bound))
                (= (count add-strides) (count then-recurs)))
       (let [stride (long (reduce max add-strides))]
         (when-let [extra (exact-long-op "minus" stride 1)]
           (when-let [hi (exact-long-op "add" (:bound lt-bound) (:value extra))]
             {:min (:min init-range)
              :max (:value hi)}))))
     (when (and (seq recur-exprs)
                (seq then-recurs)
                (empty? else-recurs)
                init-range
                (= (:min init-range) (:max init-range))
                (= local-name (:name gt-bound))
                (>= (:min init-range) (:bound gt-bound))
                (= (count sub-strides) (count then-recurs)))
       (let [stride (long (reduce max sub-strides))]
         (when-let [extra (exact-long-op "minus" stride 1)]
           (when-let [lo (exact-long-op "minus" (:bound gt-bound) (:value extra))]
             {:min (:value lo)
              :max (:max init-range)})))))))

(defn- result-of
  "메서드 반환 타입 처리: void → nil, primitive → box, ref → 그대로."
  [^GeneratorAdapter ga ^Class rtype]
  (cond
    (= Void/TYPE rtype) (.visitInsn ga Opcodes/ACONST_NULL)
    (.isPrimitive rtype) (box-value ga (Type/getType rtype))
    :else nil))

(defn- numbers-primitive-param-class
  [^Class c]
  (cond
    (= c Long/TYPE)   Long/TYPE
    (= c Double/TYPE) Double/TYPE
    :else             Object))

(defn- resolve-numbers-method
  ^java.lang.reflect.Method [^Class cls ^String mname arg-classes]
  (when (and (= Numbers cls) (contains? numbers-primitive-methods mname))
    (let [pclasses (mapv numbers-primitive-param-class arg-classes)]
      (when (some #(not= Object %) pclasses)
        (try
          (.getMethod cls mname (into-array Class pclasses))
          (catch Exception _ nil))))))

(def ^:private resolved-method-cache (atom {}))
(def ^:private resolved-ctor-cache (atom {}))

(defn- cached-resolution
  [cache key resolve-f]
  (if-let [hit (find @cache key)]
    (val hit)
    (let [resolved (resolve-f)]
      (get (swap! cache
                  (fn [m]
                    (if (contains? m key)
                      m
                      (assoc m key resolved))))
           key))))

(defn- primitive-return-compatible?
  [^Class from ^Class to]
  (or (= from to)
      (boolean (widening-distance from to))))

(defn- cast-primitive
  [^GeneratorAdapter ga ^Class from ^Class to]
  (when-not (= from to)
    (.cast ga (Type/getType from) (Type/getType to))))

(defn- emit-primitive-const
  [^GeneratorAdapter ga v ^Class pc]
  (cond
    (= pc Long/TYPE)    (if (long-sized-integer? v)
                          (.push ga (long v))
                          (throw (ex-info "unsupported primitive long const target"
                                          {:type :unsupported-op
                                           :target pc
                                           :value v
                                           :class (class v)})))
    (= pc Double/TYPE)  (.push ga (double v))
    (= pc Boolean/TYPE) (.push ga (boolean v))
    :else (throw (ex-info "unsupported primitive const target"
                          {:target pc :value v}))))

(defn- resolve-method-uncached
  "이름+인자타입으로 메서드 reflect. analyzer 가 준 정적 타입으로 overload 후보를
  점수화한다. 정적으로 안전하지 않은 모호 호출은 host fallback 으로 보낸다."
  ^java.lang.reflect.Method [^Class cls ^String mname arg-classes]
  (let [n    (count arg-classes)
        candidates (loop [xs  (seq (.getMethods cls))
                          acc []]
                     (if xs
                       (let [^java.lang.reflect.Method m (first xs)]
                         (recur (next xs)
                                (if (and (= mname (.getName m))
                                         (= n (count (.getParameterTypes m))))
                                  (conj acc m)
                                  acc)))
                       acc))]
    (or (best-executable candidates #(.getParameterTypes ^java.lang.reflect.Method %) arg-classes)
        (best-executable-permissive candidates #(.getParameterTypes ^java.lang.reflect.Method %) arg-classes)
        (throw (ex-info (str "메서드 못찾음/모호함: " (.getName cls) "/" mname "/" n)
                        {:type :unsupported-op
                         :class cls
                         :method mname
                         :arg-classes arg-classes})))))

(defn- resolve-method
  ^java.lang.reflect.Method [^Class cls ^String mname arg-classes]
  (let [arg-classes' (vec arg-classes)]
    (cached-resolution resolved-method-cache
                       [cls mname arg-classes']
                       #(resolve-method-uncached cls mname arg-classes'))))

(defn- resolve-ctor-uncached
  ^java.lang.reflect.Constructor [^Class cls arg-classes]
  (let [n    (count arg-classes)
        candidates (loop [xs  (seq (.getConstructors cls))
                          acc []]
                     (if xs
                       (let [^java.lang.reflect.Constructor ctor (first xs)]
                         (recur (next xs)
                                (if (= n (count (.getParameterTypes ctor)))
                                  (conj acc ctor)
                                  acc)))
                       acc))]
    (or (best-executable candidates #(.getParameterTypes ^java.lang.reflect.Constructor %) arg-classes)
        (best-executable-permissive candidates #(.getParameterTypes ^java.lang.reflect.Constructor %) arg-classes)
        (throw (ex-info (str "생성자 못찾음/모호함: " (.getName cls) "/" n)
                        {:type :unsupported-op
                         :class cls
                         :arg-classes arg-classes})))))

(defn- resolve-ctor
  ^java.lang.reflect.Constructor [^Class cls arg-classes]
  (let [arg-classes' (vec arg-classes)]
    (cached-resolution resolved-ctor-cache
                       [cls arg-classes']
                       #(resolve-ctor-uncached cls arg-classes'))))

(defn- static-call-method
  ^java.lang.reflect.Method [node]
  (let [^Class cls (:class node)
        arg-classes (mapv arg-class (:args node))
        ^java.lang.reflect.Method m
        (or (resolve-numbers-method cls (str (:method node)) arg-classes)
            (resolve-method cls (str (:method node)) arg-classes))]
    m))

(defn- emit-static-call-raw
  "정적 메서드 호출을 box 없이 방출하고 JVM 반환 타입을 돌려준다."
  ^Class [^GeneratorAdapter ga env node ^java.lang.reflect.Method m]
  (let [direct-op (let [ptypes (.getParameterTypes m)]
                    (when (and (= Numbers (.getDeclaringClass m))
                               (= 2 (alength ptypes)))
                      (let [mname (.getName m)
                            ret   (.getReturnType m)
                            p0    (aget ptypes 0)
                            p1    (aget ptypes 1)]
                        (cond
                          (and (= Double/TYPE ret)
                               (= Double/TYPE p0)
                               (= Double/TYPE p1)
                               (or (= "add" mname)
                                   (= "unchecked_add" mname)))
                          Opcodes/DADD

                          (and (= Double/TYPE ret)
                               (= Double/TYPE p0)
                               (= Double/TYPE p1)
                               (or (= "multiply" mname)
                                   (= "unchecked_multiply" mname)))
                          Opcodes/DMUL

                          (and (= Double/TYPE ret)
                               (= Double/TYPE p0)
                               (= Double/TYPE p1)
                               (or (= "minus" mname)
                                   (= "unchecked_minus" mname)))
                          Opcodes/DSUB

	                          (and (= Long/TYPE ret)
	                               (= Long/TYPE p0)
	                               (= Long/TYPE p1)
	                               (or (= "unchecked_add" mname)
	                                   (and (= "add" mname)
	                                        (checked-long-static-no-overflow? env mname (:args node)))))
	                          Opcodes/LADD

	                          (and (= Long/TYPE ret)
	                               (= Long/TYPE p0)
	                               (= Long/TYPE p1)
	                               (or (= "unchecked_multiply" mname)
	                                   (and (= "multiply" mname)
	                                        (checked-long-static-no-overflow? env mname (:args node)))))
	                          Opcodes/LMUL

	                          (and (= Long/TYPE ret)
	                               (= Long/TYPE p0)
	                               (= Long/TYPE p1)
	                               (or (= "unchecked_minus" mname)
	                                   (and (= "minus" mname)
	                                        (checked-long-static-no-overflow? env mname (:args node)))))
	                          Opcodes/LSUB

                          :else nil))))]
    (if direct-op
      (do
        (emit-args-coerced ga env (:args node) (.getParameterTypes m))
        (.visitInsn ga (int direct-op))
        (.getReturnType m))
      (let [^Class cls (:class node)
            ptypes (.getParameterTypes m)]
        (emit-args-coerced ga env (:args node) ptypes)
        (.invokeStatic ga (Type/getType cls) (Method/getMethod m))
        (.getReturnType m)))))

(defn- emit-static-call
  "정적 메서드 호출(예: clojure.core/* inline → Numbers.multiply, 또는 Math/sqrt)."
  [^GeneratorAdapter ga env node]
  (result-of ga (emit-static-call-raw ga env node (static-call-method node))))

(defn- emit-node-as
  "node 를 pc 타입으로 스택에 남긴다. primitive pc 는 가능한 box/unbox 없이 직접 방출한다."
  [^GeneratorAdapter ga env node ^Class pc]
  (cond
    (nil? pc)
    (emit-node ga env node)

    (not (:typed-locals-enabled? env))
    (do (emit-node ga env node)
        (coerce-arg ga pc))

    (not (.isPrimitive pc))
    (do (emit-node ga env node)
        (coerce-arg ga pc))

    (and (= :const (:op node))
         (contains? typed-local-primitives pc))
    (emit-primitive-const ga (:val node) pc)

    (= :local (:op node))
    (let [[kind idx pclass] (get env (:name node))]
      (cond
        (and (= :jvm-arg kind)
             (class? pclass)
             (.isPrimitive ^Class pclass)
             (primitive-return-compatible? pclass pc))
        (do (.loadArg ga (int idx))
            (cast-primitive ga pclass pc))

        (and (= :jvm-arg kind)
             (class? pclass)
             (not (.isPrimitive ^Class pclass)))
        (do (.loadArg ga (int idx))
            (coerce-arg ga pc))

        (and (= :local kind)
             (class? pclass)
             (.isPrimitive ^Class pclass)
             (primitive-return-compatible? pclass pc))
        (do (.loadLocal ga (int idx))
            (cast-primitive ga pclass pc))

        (and (= :field kind)
             (class? pclass)
             (.isPrimitive ^Class pclass)
             (primitive-return-compatible? pclass pc))
        (do (.loadThis ga)
            (.getField ga ^Type (:self-ctype env) ^String idx (Type/getType ^Class pclass))
            (cast-primitive ga pclass pc))

        (= :arg kind)
        (do (.loadArg ga (int idx))
            (coerce-arg ga pc))

        :else
        (do (emit-node ga env node)
            (coerce-arg ga pc))))

    (= :static-call (:op node))
    (let [m (static-call-method node)
          rtype (.getReturnType m)]
      (if (and (.isPrimitive rtype)
               (primitive-return-compatible? rtype pc))
        (do (emit-static-call-raw ga env node m)
            (cast-primitive ga rtype pc))
        (do (result-of ga (emit-static-call-raw ga env node m))
            (coerce-arg ga pc))))

    (= :static-field (:op node))
    (let [^Class cls (:class node)
          f  (.getField cls (str (:field node)))
          ft (.getType f)]
      (if (and (.isPrimitive ft)
               (primitive-return-compatible? ft pc))
        (do
          (.getStatic ga (Type/getType cls) (str (:field node)) (Type/getType ft))
          (cast-primitive ga ft pc))
        (do (emit-node ga env node)
            (coerce-arg ga pc))))

    (= :instance-field (:op node))
    (let [^Class cls (:class node)
          f  (.getField cls (str (:field node)))
          ft (.getType f)
          ct (Type/getType cls)]
      (if (and (.isPrimitive ft)
               (primitive-return-compatible? ft pc))
        (do
          (emit-node ga env (:instance node))
          (.checkCast ga ct)
          (.getField ga ct (str (:field node)) (Type/getType ft))
          (cast-primitive ga ft pc))
        (do (emit-node ga env node)
            (coerce-arg ga pc))))

    :else
    (do (emit-node ga env node)
        (coerce-arg ga pc))))

(defn- emit-instance-call
  "(.method obj args) — 수신자/인자 coerce, invokeVirtual|invokeInterface."
  [^GeneratorAdapter ga env node]
  (let [cls (:class node)]
    (if (nil? cls)
      (do
        (emit-node ga env (:instance node))
        (.push ga (str (:method node)))
        (emit-node-array ga env (:args node))
        (.invokeStatic ga reflector-type reflector-instance-method-method))
      (let [^Class cls cls
            ^java.lang.reflect.Method m
            (resolve-method cls (str (:method node)) (mapv arg-class (:args node)))
            ptypes (.getParameterTypes m)
            ctype  (Type/getType cls)]
        (emit-node ga env (:instance node))
        (.checkCast ga ctype)
        (emit-args-coerced ga env (:args node) ptypes)
        (if (.isInterface cls)
          (.invokeInterface ga ctype (Method/getMethod m))
          (.invokeVirtual ga ctype (Method/getMethod m)))
        (result-of ga (.getReturnType m))))))

(defn- emit-new
  "(ClassName. args) — NEW + dup + 인자 coerce + invokespecial <init>."
  [^GeneratorAdapter ga env node]
  (let [^Class cls (:val (:class node))
        ctor   (resolve-ctor cls (mapv arg-class (:args node)))
        ptypes (.getParameterTypes ctor)
        ctype  (Type/getType cls)
        cm     (Method. "<init>" Type/VOID_TYPE
                        (into-array Type (map #(Type/getType ^Class %) ptypes)))]
    (.newInstance ga ctype)
    (.dup ga)
    (emit-args-coerced ga env (:args node) ptypes)
    (.invokeConstructor ga ctype cm)))

(defn- emit-static-field
  [^GeneratorAdapter ga node]
  (let [^Class cls (:class node)
        f  (.getField cls (str (:field node)))
        ft (.getType f)]
    (.getStatic ga (Type/getType cls) (str (:field node)) (Type/getType ft))
    (when (.isPrimitive ft) (box-value ga (Type/getType ft)))))

(defn- emit-instance-field
  [^GeneratorAdapter ga env node]
  (let [^Class cls (:class node)
        f  (.getField cls (str (:field node)))
        ft (.getType f)
        ct (Type/getType cls)]
    (emit-node ga env (:instance node))
    (.checkCast ga ct)
    (.getField ga ct (str (:field node)) (Type/getType ft))
    (when (.isPrimitive ft) (box-value ga (Type/getType ft)))))

(defn- emit-host-interop
  "정적 타입 미해소 field/0-arg member. Clojure Reflector 의 런타임 의미론을 호출한다."
  [^GeneratorAdapter ga env node]
  (emit-node ga env (:target node))
  (.push ga (str (:m-or-f node)))
  (.push ga (boolean (:field node)))
  (.invokeStatic ga reflector-type reflector-noarg-instance-member-method))

(defn- emit-keyword-invoke
  "(:k target) — Keyword 는 IFn 이므로 직접 IFn.invoke(Object) 로 호출한다."
  [^GeneratorAdapter ga env node]
  (emit-node ga env (:keyword node))
  (.checkCast ga ifn-type)
  (emit-node ga env (:target node))
  (.invokeInterface ga ifn-type (invoke-method 1)))

(defn- emit-instance?
  "(instance? C x) — JVM INSTANCEOF 결과를 Boolean 으로 box."
  [^GeneratorAdapter ga env node]
  (emit-node ga env (:target node))
  (.visitTypeInsn ga Opcodes/INSTANCEOF (.getInternalName (Type/getType ^Class (:class node))))
  (box-value ga Type/BOOLEAN_TYPE))

(defn- if-branch-long-ranges
  [test]
  (when (or (numbers-call? test "lt")
            (numbers-call? test "gt"))
    (let [[a b] (:args test)
          bound (const-long-literal b)]
      (when (and (= :local (:op a))
                 (some? bound))
        (cond
          (numbers-call? test "lt")
          {:name (:name a)
           :then (when-let [hi (exact-long-op "minus" bound 1)]
                   {:min Long/MIN_VALUE
                    :max (:value hi)})
           :else {:min bound
                  :max Long/MAX_VALUE}}

          (numbers-call? test "gt")
          {:name (:name a)
           :then (when-let [lo (exact-long-op "add" bound 1)]
                   {:min (:value lo)
                    :max Long/MAX_VALUE})
           :else {:min Long/MIN_VALUE
                  :max bound}})))))

(defn- if-branch-envs
  [env test]
  (if-let [{:keys [name then else]} (if-branch-long-ranges test)]
    [(if then (narrow-local-long-range env name then) env)
     (if else (narrow-local-long-range env name else) env)]
    [env env]))

(defn- emit-if
  "(if test then else) — test 를 booleanCast 한 뒤 분기. else 는 analyzer 가 항상 채움."
  [^GeneratorAdapter ga env node]
  (let [else-label (.newLabel ga)
        end-label  (.newLabel ga)
        [then-env else-env] (if-branch-envs env (:test node))]
    (emit-node ga env (:test node))
    (.invokeStatic ga rt-type rt-booleancast-method)
    (.ifZCmp ga GeneratorAdapter/EQ else-label)   ; false(0) → else
    (emit-node ga then-env (:then node))
    (.goTo ga end-label)
    (.mark ga else-label)
    (emit-node ga else-env (:else node))
    (.mark ga end-label)))

(defn- emit-let
  "(let* [b init …] body) — 각 init 를 새 지역 슬롯에 저장(순차 = let* 의미)."
  [^GeneratorAdapter ga env node]
  (let [env' (reduce (fn [e b]
	                       (if-let [pc (and (:typed-locals-enabled? e)
	                                        (typed-local-primitive-class (:init b)))]
	                         (do
	                           (emit-node-as ga e (:init b) pc)
	                           (let [slot (.newLocal ga (Type/getType ^Class pc))]
	                             (.storeLocal ga slot)
	                             (assoc e (:name b)
	                                    (ranged-env-entry
	                                     :local
	                                     slot
	                                     pc
	                                     (long-range-of e (:init b))
	                                     (independent-long-range-of e (:init b))))))
	                         (do
	                           (emit-node ga e (:init b))
	                           (let [slot (.newLocal ga obj-type)]
	                             (.storeLocal ga slot)
	                             (assoc e (:name b)
	                                    (ranged-env-entry
	                                     :local
	                                     slot
	                                     nil
	                                     (long-range-of e (:init b))
	                                     (independent-long-range-of e (:init b))))))))
                     env (:bindings node))
        bound-entries (mapv #(get env' (:name %)) (:bindings node))
        result-slot (.newLocal ga obj-type)]
    (emit-node ga env' (:body node))
    (.storeLocal ga result-slot)
    (clear-object-locals! ga bound-entries)
    (.loadLocal ga result-slot)))

;; ── M9b: 원리적 abstract-interpretation loop 변수 range (interval 도메인) ──────────
;; abstract_interval 엔진(interval lattice + Cousot-Cousot widening/narrowing fixpoint,
;; sound over-approximation)으로 loop 변수의 finite long range 를 도출한다. 기존
;; M6ab~M6ai 인식기가 못 잡는 케이스(예: branch-dependent stride `(recur (if c (+ i 1) (+ i 2)))`)
;; 를 *한 패턴씩 추가하지 않고* 엔진으로 흡수한다. emit-loop 의 (or …) 중 마지막이라
;; recognizer 가 우선이고 회귀가 없으며, 도출 range 도 tv/lowering-sound? validator 를
;; 통과해야 raw opcode 가 된다.
;; ★ 정직한 신뢰경계(2026-06-29): tv/lowering-sound? 는 공급 range 와, 있을 때는
;; bounded-unroll 에서 온 독립 operand range 를 join 한 effective range 위에서
;; overflow-freedom 을 검사한다. unroll 이 붙지 않는 VC 는 여전히 range 제공자(엔진)에
;; 상대적이므로 full proof 로 과장하지 않는다.

(defn- ai-translate-expr
  "loop 본문 expr 노드를 abstract-interval interval 로 평가한다. ai-env 는
  {binding-name → ai/interval}. 모르는 형태는 ai/top(보수적 over-approximation)."
  [ai-env loop-vars node]
  (cond
    (some? (const-long-literal node))
    (ai/singleton (const-long-literal node))

    (and (= :local (:op node)) (contains? loop-vars (:name node)))
    (get ai-env (:name node) ai/top)

    (and (= :static-call (:op node))
         (= Numbers (:class node))
         (= 2 (count (:args node))))
    (let [m  (str (:method node))
          op (cond (= "add" m) :+ (= "minus" m) :- (= "multiply" m) :* :else nil)]
      (if op
        (ai/transfer op
                     (ai-translate-expr ai-env loop-vars (first (:args node)))
                     (ai-translate-expr ai-env loop-vars (second (:args node))))
        ai/top))

    (= :if (:op node))
    (ai/join (ai-translate-expr ai-env loop-vars (:then node))
             (ai-translate-expr ai-env loop-vars (:else node)))

    (= :do (:op node))
    (ai-translate-expr ai-env loop-vars (:ret node))

    :else ai/top))

(defn- ai-loop-guard
  "본문 (if (< v c) …) / (if (> v c) …) 의 guard 를 engine spec [op v c] 로. (v=local, c=const long)"
  [body]
  (when (= :if (:op body))
    (or (when-let [g (loop-lt-local-bound body)] [:< (:name g) (:bound g)])
        (when-let [g (loop-gt-local-bound body)] [:> (:name g) (:bound g)]))))

;; ── M9b: conserved linear-quantity refinement (관계 도메인 최소 구현) ────────────
;; interval 도메인만으로는 못 잡는 mixed-sign 누적(예: acc 가 i 와 함께 acc+i 를 보존)을
;; 정수 선형형(linform) 위에서 *정확히* 추론해 좁힌다. ad-hoc 패턴이 아니라 관계
;; 도메인(octagon/linear-invariant)의 최소 구현이다. 정수 선형 대수는 exact 이고, 최종
;; raw opcode 승격은 여전히 tv/lowering-sound? validator 가 overflow 를 차단한다.

(defn- linform-drop-zeros
  [lf]
  (update lf :coeffs (fn [c] (into {} (remove (comp zero? val) c)))))

(defn- linform-add
  [a b]
  (try
    (linform-drop-zeros
     {:coeffs (merge-with (fn [x y] (Math/addExact (long x) (long y)))
                          (:coeffs a) (:coeffs b))
      :const (Math/addExact (long (:const a)) (long (:const b)))})
    (catch ArithmeticException _ nil)))

(defn- linform-scale
  [k a]
  (try
    (linform-drop-zeros
     {:coeffs (into {} (map (fn [[v c]] [v (Math/multiplyExact (long k) (long c))]) (:coeffs a)))
      :const (Math/multiplyExact (long k) (long (:const a)))})
    (catch ArithmeticException _ nil)))

(defn- linform-of-expr
  "AST 노드를 정수 선형형 {:coeffs {name coeff} :const c} 으로. 비선형/미지원이면 nil.
  loop var 만 변수로 취급한다."
  [loop-vars node]
  (cond
    (some? (const-long-literal node))
    {:coeffs {} :const (const-long-literal node)}

    (and (= :local (:op node)) (contains? loop-vars (:name node)))
    {:coeffs {(:name node) 1} :const 0}

    (and (= :static-call (:op node)) (= Numbers (:class node)) (= 2 (count (:args node))))
    (let [m (str (:method node))
          a (linform-of-expr loop-vars (first (:args node)))
          b (linform-of-expr loop-vars (second (:args node)))]
      (when (and a b)
        (cond
          (= "add" m)   (linform-add a b)
          (= "minus" m) (some->> (linform-scale -1 b) (linform-add a))
          (= "multiply" m)
          (cond
            (empty? (:coeffs a)) (linform-scale (:const a) b)
            (empty? (:coeffs b)) (linform-scale (:const b) a)
            :else nil)
          :else nil)))

    :else nil))

(defn- conserved-refine
  "interval fixpoint 결과(head)에서 아직 finite 하지 못한 loop var u 를, 단일 then-recur
  안에서 bounded var b 와 보존되는 선형량 (u + s·b = const, s∈{+1,-1}) 으로 좁힌다.
  보존 판정은 정수 선형형 equality(exact). u = const − s·b → finite interval."
  [head names bindings then-recurs loop-vars]
  (if (not= 1 (count then-recurs))
    head
    (let [exprs      (first then-recurs)
          recur-lf   (into {} (map-indexed
                               (fn [idx nm] [nm (linform-of-expr loop-vars (nth exprs idx nil))])
                               names))
          init-const (into {} (map (fn [b] [(:name b) (const-long-literal (:init b))]) bindings))
          finite?    (fn [h nm] (let [iv (get h nm)] (and (ai/interval? iv) (ai/finite? iv))))]
      (reduce
       (fn [h u]
         (if (finite? h u)
           h
           (or
            (some
             (fn [b]
               (when (and (not= u b) (finite? h b)
                          (recur-lf u) (recur-lf b)
                          (init-const u) (init-const b))
                 (some
                  (fn [s]
                    (let [q-orig  (linform-drop-zeros {:coeffs {u 1 b s} :const 0})
                          q-trans (some->> (linform-scale s (recur-lf b))
                                           (linform-add (recur-lf u)))]
                      (when (and q-trans
                                 (= (:coeffs q-orig) (:coeffs q-trans))
                                 (= (:const q-orig) (:const q-trans)))
                        (try
                          (let [qv   (Math/addExact (long (init-const u))
                                                    (Math/multiplyExact (long s) (long (init-const b))))
                                bv   (get h b)
                                lo   (Math/subtractExact qv (Math/multiplyExact (long s) (long (:max bv))))
                                hi   (Math/subtractExact qv (Math/multiplyExact (long s) (long (:min bv))))]
                            (assoc h u (ai/interval (min lo hi) (max lo hi))))
                          (catch ArithmeticException _ nil)))))
                  [1 -1])))
             names)
            h)))
       head
       names))))

;; ── M9b: bounded geometric acceleration (nonlinear multiplicative recurrence) ────
;; interval/conserved 로도 못 잡는 곱셈 점화식(예: acc'=(* acc -2), 부호 교번 + 지수 증가)을,
;; 양수 stride bounded index loop 의 iteration count N 으로 |init|*|factor|^N magnitude 를
;; *정확히* 계산해 acc∈[-M,M] 으로 bound 한다(geometric acceleration; 최소 nonlinear 도메인).
;; factor/N/overflow 모르면 skip→fallback, 승격은 여전히 validator 통과 필수.

(defn- multiplicative-self-factor
  "node 가 (* u const) 또는 (* const u) (u=local-name) 이면 그 const long factor."
  [node local-name]
  (when (and (= :static-call (:op node))
             (= Numbers (:class node))
             (= "multiply" (str (:method node)))
             (= 2 (count (:args node))))
    (let [[a b] (:args node)]
      (cond
        (and (same-local-name? a local-name) (const-long-literal b)) (const-long-literal b)
        (and (same-local-name? b local-name) (const-long-literal a)) (const-long-literal a)
        :else nil))))

(defn- abs-long-safe
  "Math/abs 는 Long/MIN_VALUE 에서 음수를 반환하므로 그 경우 nil(보수적)."
  [x]
  (let [a (Math/abs (long x))]
    (when-not (neg? a) a)))

(defn- geometric-magnitude-bound
  "|init| * |factor|^iterations, exact long. overflow/degenerate 면 nil."
  [init factor iterations]
  (let [af (abs-long-safe factor)
        ai (abs-long-safe init)]
    (when (and af ai (>= af 1) (>= iterations 0))
      (try
        (loop [acc ai n iterations]
          (if (zero? n)
            acc
            (recur (Math/multiplyExact (long acc) (long af)) (dec n))))
        (catch ArithmeticException _ nil)))))

(defn- geometric-refine
  "아직 finite 하지 못한 var u 의 단일 then-recur 가 (* u factor)(factor const long, |factor|>=1)
  이고 bounded positive-index iteration count N 을 알면, u 를 [-|init_u|*|factor|^N,
  +|init_u|*|factor|^N] 으로 좁힌다(부호 교번 포함 sound symmetric bound)."
  [head names bindings then-recurs loop-vars counter]
  (if (not= 1 (count then-recurs))
    head
    (if counter
      (let [exprs    (first then-recurs)
            N        (:iterations counter)
            idx-name (:name counter)
            finite?  (fn [h nm] (let [iv (get h nm)] (and (ai/interval? iv) (ai/finite? iv))))]
        (reduce
         (fn [h u]
           (if (or (finite? h u) (= u idx-name))
             h
             (let [u-idx   (some (fn [[i b]] (when (= (:name b) u) i))
                                 (map-indexed vector bindings))
                   recur-u (when u-idx (nth exprs u-idx nil))
                   factor  (when recur-u (multiplicative-self-factor recur-u u))
                   init-u  (when u-idx (const-long-literal (:init (nth bindings u-idx))))]
               (if-let [M (and factor init-u
                               (geometric-magnitude-bound init-u factor N))]
                 (assoc h u (ai/interval (- (long M)) (long M)))
                 h))))
         head
         names))
      head)))

;; ── M9b: bounded interval unrolling (일반 nonlinear 점화식의 통일적 처리) ──────────
;; 유계 인덱스로 반복 횟수 N(≤cap)을 알면, widening 대신 interval transfer 를 N 번 그대로
;; 펼쳐 각 step state 를 join 한다. 선형/기하/nonlinear(acc*acc 이중지수 등)을 모두 통일적으로
;; 덮는다 — N 이 작아 magnitude 가 overflow 안 하면 finite bound, overflow 면 top→fallback.
;; N 은 loop-positive-index-iteration-count 의 tight upper bound 라 states 0..N 이 모든
;; loop-head 값을 덮어 sound. guard 는 각 step 전 적용. 최종 승격은 여전히 validator 가 차단.

(def ^:private loop-unroll-cap 256)

(defn- loop-unroll-ranges
  [then-recurs guard-spec loop-vars names bindings counter init-ranges]
  (when counter
    (let [N (long (:iterations counter))]
      (when (<= 0 N loop-unroll-cap)
        (let [init-state (into {}
                               (map (fn [b]
                                      [(:name b)
                                       (if-let [r (get init-ranges (:name b))]
                                         (ai/interval (:min r) (:max r))
                                         ai/top)])
                                    bindings))
              recur-state (fn [guarded exprs]
                            (into {}
                                  (map-indexed
                                   (fn [idx nm]
                                     [nm (ai-translate-expr
                                          guarded loop-vars (nth exprs idx nil))])
                                   names)))
              step       (fn [state]
                           (let [guarded (if guard-spec (ai/guard state guard-spec) state)]
                             (reduce ai/env-join
                                     (mapv #(recur-state guarded %) then-recurs))))
              states     (loop [j 0 st init-state acc [init-state]]
                           (if (>= j N)
                             acc
                             (let [st' (step st)]
                               (recur (inc j) st' (conj acc st')))))
              joined     (reduce (fn [a st]
                                   (into {} (map (fn [nm] [nm (ai/join (get a nm) (get st nm))]) names)))
                                 (first states)
                                 (rest states))]
          (into {}
                (keep (fn [nm]
                        (let [iv (get joined nm)]
                          (when (and (ai/interval? iv) (ai/finite? iv))
                            [nm {:min (long (:min iv)) :max (long (:max iv))}])))
                      names)))))))

(defn- loop-analysis-context
  [env node recur-exprs branch-recurs]
  (let [body      (:body node)
        bindings  (:bindings node)
        names     (mapv :name bindings)
        loop-vars (set names)]
    (when (and (= :if (:op body)) (seq recur-exprs))
      (let [guard-spec  (ai-loop-guard body)
            then-recurs (:then-recurs branch-recurs)
            else-recurs (:else-recurs branch-recurs)]
        (when (and guard-spec
                   (contains? loop-vars (second guard-spec))
                   (seq then-recurs)
                   (empty? else-recurs)
                   (= (count then-recurs) (count recur-exprs)))
          (let [init-ranges (into {}
                                  (map (fn [b]
                                         [(:name b)
                                          (long-range-of env (:init b))])
                                       bindings))
                counter     (loop-positive-index-iteration-count
                             env body then-recurs else-recurs bindings)]
            {:body body
             :bindings bindings
             :names names
             :loop-vars loop-vars
             :guard-spec guard-spec
             :then-recurs then-recurs
             :else-recurs else-recurs
             :init-ranges init-ranges
             :counter counter}))))))

(defn- loop-ai-ranges
  "abstract-interval 엔진으로 loop 변수들의 sound finite long range 를 도출한다.
  안전을 위해 본문이 (if guard then else), guard 가 (< v c)/(> v c)(v=loop var),
  모든 current-loop recur 가 then 분기에 있을 때만 처리한다(else 분기 recur 가 있으면
  guard 가 그 recur 에 성립하지 않으므로 보수적으로 포기 → nil). 반환: {name {:min :max}}."
  [loop-ctx unroll-ranges]
  (when loop-ctx
    (let [{:keys [bindings names loop-vars guard-spec then-recurs
                  init-ranges counter]} loop-ctx
          initial    (into {}
                           (map (fn [b]
                                  [(:name b)
                                   (if-let [r (get init-ranges (:name b))]
                                     (ai/interval (:min r) (:max r))
                                     ai/top)])
                                bindings))
          transfer-f (fn [ai-env]
                       (let [guarded (ai/guard ai-env guard-spec)]
                         (reduce ai/env-join
                                 (mapv (fn [exprs]
                                         (into {}
                                               (map-indexed
                                                (fn [idx nm]
                                                  [nm (ai-translate-expr
                                                       guarded loop-vars (nth exprs idx nil))])
                                                names)))
                                       then-recurs))))
          closure    (ai/loop-closure initial transfer-f
                                      {:widen-after 3 :max-iterations 64})
          ;; interval → 보존 선형량(관계 도메인) → 기하 가속(곱셈 점화식) 순으로
          ;; 못 좁힌 var 를 추가로 좁힌다(전부 sound, 우선순위는 더 정밀한 쪽).
          head       (-> (:state closure)
                         (conserved-refine names bindings then-recurs loop-vars)
                         (geometric-refine names bindings then-recurs loop-vars
                                           counter))
          widen-ranges (into {}
                             (keep (fn [nm]
                                     (let [iv (get head nm)]
                                       (when (and (ai/interval? iv) (ai/finite? iv))
                                         [nm {:min (long (:min iv)) :max (long (:max iv))}])))
                                   names))]
      ;; 위에서 못 잡은 nonlinear var(예: acc*acc)는 유계 구간 언롤링으로 채운다.
      ;; merge 시 widen-ranges 가 이기므로 기존 admitted 케이스는 변하지 않는다(회귀 0).
      (merge unroll-ranges widen-ranges))))

(defn- loop-independent-unroll-ranges
  "compiler admission 에 넘길 독립 2차 range. fast recognizer 와 같은 닫힌식을
  재사용하지 않고 bounded interval unroll 만 실행한다."
  [unroll-ranges]
  (when-not *disable-independent-range-admission*
    unroll-ranges))

(defn- emit-loop
  "(loop* [b init …] body) — 바인딩을 지역 슬롯에 두고, 시작 라벨을 recur 타겟으로.
  recur 는 슬롯을 갱신하고 이 라벨로 goto(비스택 반복)."
  [^GeneratorAdapter ga env node]
  (let [body        (:body node)
        bindings    (:bindings node)
        range-needed? (has-checked-long-candidate? body)
        recur-exprs (if range-needed?
                      (current-loop-recur-exprs body)
                      [])
        branch-recurs (when (and range-needed? (= :if (:op body)))
                        {:then-recurs (current-loop-recur-exprs (:then body))
                         :else-recurs (current-loop-recur-exprs (:else body))})
        then-recurs (or (:then-recurs branch-recurs) [])
        else-recurs (or (:else-recurs branch-recurs) [])
        loop-ctx    (when range-needed?
                      (loop-analysis-context env node recur-exprs branch-recurs))
        unroll-ranges (when loop-ctx
                        (let [{:keys [then-recurs guard-spec loop-vars names
                                      bindings counter init-ranges]} loop-ctx]
                          (loop-unroll-ranges then-recurs guard-spec loop-vars
                                              names bindings counter init-ranges)))
        ;; M9b: recognizer 들이 못 잡는 binding 은 abstract-interpretation 엔진의 sound
        ;; finite range 로 보강(마지막 fallback, 회귀 없음).
        ai-ranges   (loop-ai-ranges loop-ctx unroll-ranges)
        independent-ranges (loop-independent-unroll-ranges unroll-ranges)
        env'    (reduce (fn [e [idx b]]
                          (let [init-range (when range-needed?
                                             (long-range-of e (:init b)))
                                independent-init-range (when range-needed?
                                                         (independent-long-range-of e (:init b)))
                                independent-range (when range-needed?
                                                    (or (get independent-ranges (:name b))
                                                        independent-init-range))
                                r (when range-needed?
                                    (or (when (loop-binding-invariant? recur-exprs idx (:name b))
                                          init-range)
                                        (loop-binding-bounded-step-range
                                         body
                                         recur-exprs
                                         idx
                                         b
                                         then-recurs
                                         else-recurs
                                         init-range)
                                        (loop-binding-accumulator-range
                                         e
                                         body
                                         recur-exprs
                                         idx
                                         b
                                         bindings
                                         then-recurs
                                         else-recurs
                                         init-range)
                                        (get ai-ranges (:name b))))]
                            (if-let [pc (and (:typed-locals-enabled? e)
                                             (typed-local-primitive-class (:init b)))]
                              (do
                                (emit-node-as ga e (:init b) pc)
                                (let [slot (.newLocal ga (Type/getType ^Class pc))]
                                  (.storeLocal ga slot)
                                  (assoc e (:name b)
                                         (ranged-env-entry
                                          :local
                                          slot
                                          pc
                                          r
                                          independent-range))))
                              (do
                                (emit-node ga e (:init b))
                                (let [slot (.newLocal ga obj-type)]
                                  (.storeLocal ga slot)
                                  (assoc e (:name b)
                                         (ranged-env-entry
                                          :local
                                          slot
                                          nil
                                          r
                                          independent-range)))))))
                        env (map-indexed vector bindings))
        start   (.newLabel ga)
        targets (mapv #(get env' (:name %)) bindings)
        env''   (assoc env' :recur-target {:label start :targets targets})
        result-slot (.newLocal ga obj-type)]
    (.mark ga start)
    (emit-node ga env'' (:body node))
    (.storeLocal ga result-slot)
    (clear-object-locals! ga targets)
    (.loadLocal ga result-slot)))

(defn- emit-recur
  "(recur e …) — 모든 e 를 먼저 평가(옛 값 기준)한 뒤 역순으로 타겟 슬롯에 저장,
  그리고 recur 타겟 라벨로 goto. 타겟은 loop 슬롯 또는 fn 인자(:arg)."
  [^GeneratorAdapter ga env node]
  (let [{:keys [label targets]} (:recur-target env)]
    (when (nil? label)
      (throw (ex-info "recur 타겟 없음(loop/fn 밖)" {:node node})))
    (doseq [[target e] (map vector targets (:exprs node))]
      (let [[kind _ pclass] target]
        (if (and (= :local kind)
                 (class? pclass)
                 (.isPrimitive ^Class pclass))
          (emit-node-as ga env e pclass)
          (emit-node ga env e))))
    (doseq [[kind idx] (reverse targets)]
      (condp = kind
        :arg   (.storeArg ga (int idx))
        :local (.storeLocal ga (int idx))
        (throw (ex-info (str "unsupported recur target kind: " kind)
                        {:kind kind :form (:form node)}))))
    (.goTo ga label)))

(defn- emit-throw
  [^GeneratorAdapter ga env node]
  (emit-node ga env (:exception node))
  (.checkCast ga throwable-type)
  (.visitInsn ga Opcodes/ATHROW))

(defn- emit-finally-body
  [^GeneratorAdapter ga env finally-node]
  (emit-node ga env finally-node)
  (.pop ga))

(defn- emit-try
  "(try body (catch C e h) … (finally f)) — try/catch 는 result 슬롯에 값을 모으고,
  finally 는 정상/handled/uncaught 모든 경로에서 실행한다."
  [^GeneratorAdapter ga env node]
  (let [result   (.newLocal ga obj-type)
        thrown   (.newLocal ga throwable-type)
        start    (.newLabel ga)
        end      (.newLabel ga)
        after    (.newLabel ga)
        catches  (:catches node)
        catch-class (fn [c] ^Class (:val (:class c)))
        _        (doseq [c catches]
                   (let [klass (catch-class c)]
                     (when-not (.isAssignableFrom Throwable klass)
                       (throw (ex-info "catch type must extend Throwable"
                                       {:type :unsupported-op
                                        :op :try
                                        :catch-class klass})))))
        handlers (mapv (fn [c]
                         {:catch c
                          :handler (.newLabel ga)
                          :body-start (.newLabel ga)
                          :body-end (.newLabel ga)
                          :uncaught (.newLabel ga)})
                       catches)
        finally-node (:finally node)
        uncaught (.newLabel ga)]
    (.mark ga start)
    (emit-node ga env (:body node))
    (.storeLocal ga result)
    (.mark ga end)
    ;; JVM exception table order matters. Register this try after emitting its body,
    ;; so nested try/finally handlers appear first and run before an outer catch.
    (doseq [{:keys [catch handler]} handlers]
      (.visitTryCatchBlock ga start end handler
                           (.getInternalName (Type/getType ^Class (catch-class catch)))))
    (when finally-node
      (.visitTryCatchBlock ga start end uncaught nil))
    (when finally-node
      (emit-finally-body ga env finally-node))
    (.goTo ga after)
    (doseq [{:keys [catch handler body-start body-end uncaught]} handlers]
      (.mark ga handler)                       ; 예외 객체가 스택 top 에 옴
      (let [slot (.newLocal ga obj-type)]
        (.storeLocal ga slot)                  ; catch 변수 바인딩
        (.mark ga body-start)
        (emit-node ga (assoc env (:name (:local catch)) [:local slot]) (:body catch))
        (.storeLocal ga result)
        (.mark ga body-end)
        (when finally-node
          (.visitTryCatchBlock ga body-start body-end uncaught nil))
        (when finally-node
          (emit-finally-body ga env finally-node))
        (.goTo ga after)
        (when finally-node
          (.mark ga uncaught)
          (.storeLocal ga thrown)
          (emit-finally-body ga env finally-node)
          (.loadLocal ga thrown)
          (.visitInsn ga Opcodes/ATHROW))))
    (when finally-node
      (.mark ga uncaught)
      (.storeLocal ga thrown)
      (emit-finally-body ga env finally-node)
      (.loadLocal ga thrown)
      (.visitInsn ga Opcodes/ATHROW))
    (.mark ga after)
    (.loadLocal ga result)))

(defn- emit-node-array
  "AST 노드들을 평가해 Object[] 를 스택에 만든다(non-const 컬렉션 재료)."
  [^GeneratorAdapter ga env nodes]
  (.push ga (int (count nodes)))
  (.newArray ga obj-type)
  (let [arr-slot (.newLocal ga obj-array-type)
        val-slot (.newLocal ga obj-type)]
    (.storeLocal ga arr-slot)
    (doseq [[i n] (map-indexed vector nodes)]
      ;; try/catch/finally 노드는 exception handler 진입 시 operand stack 이 비워진다.
      ;; 배열/인덱스를 올리기 전에 원소를 먼저 평가해 stackmap 불일치를 피한다.
      (emit-node ga env n)
      (.storeLocal ga val-slot)
      (.loadLocal ga arr-slot)
      (.push ga (int i))
      (.loadLocal ga val-slot)
      (.arrayStore ga obj-type))
    (.loadLocal ga arr-slot)))

(defn- emit-vector
  [^GeneratorAdapter ga env node]
  (emit-node-array ga env (:items node))
  (.invokeStatic ga rt-type rt-vector-method))

(defn- emit-set
  [^GeneratorAdapter ga env node]
  (emit-node-array ga env (:items node))
  (.invokeStatic ga rt-type rt-set-method))

(defn- emit-map
  [^GeneratorAdapter ga env node]
  (emit-node-array ga env (interleave (:keys node) (:vals node)))
  (.invokeStatic ga rt-type rt-map-method))

(defn- emit-set!
  "(set! target val) — Var thread binding 또는 public field 를 갱신하고 val 을 반환한다."
  [^GeneratorAdapter ga env node]
  (let [target (:target node)
        val-slot (.newLocal ga obj-type)]
    (condp = (:op target)
      :var
      (do
        (emit-node ga env (:val node))
        (.storeLocal ga val-slot)
        (load-var ga env (:var target))
        (.loadLocal ga val-slot)
        (.invokeVirtual ga var-type var-set-method))

      :instance-field
      (let [^Class cls (:class target)
            f  (.getField cls (str (:field target)))
            ft (.getType f)
            ct (Type/getType cls)
            inst-slot (.newLocal ga obj-type)]
        (emit-node ga env (:instance target))
        (.storeLocal ga inst-slot)
        (emit-node ga env (:val node))
        (.storeLocal ga val-slot)
        (.loadLocal ga inst-slot)
        (.checkCast ga ct)
        (.loadLocal ga val-slot)
        (coerce-arg ga ft)
        (.putField ga ct (str (:field target)) (Type/getType ft))
        (.loadLocal ga val-slot))

      :static-field
      (let [^Class cls (:class target)
            f  (.getField cls (str (:field target)))
            ft (.getType f)
            ct (Type/getType cls)]
        (emit-node ga env (:val node))
        (.storeLocal ga val-slot)
        (.loadLocal ga val-slot)
        (coerce-arg ga ft)
        (.putStatic ga ct (str (:field target)) (Type/getType ft))
        (.loadLocal ga val-slot))

      :local
      ;; deftype/defrecord mutable field set!: (set! field val) → putfield this.field.
      ;; target 은 [:field fname] env, self-ctype 의 mutable 필드. tag 로 타입 coerce.
      (let [field-entry (get env (:name target))]
        (when-not (and (vector? field-entry) (= :field (first field-entry)))
          (throw (ex-info "unsupported set! :local target (not a deftype field)"
                          {:type :unsupported-op :op :set! :name (:name target)})))
        (let [fname  (second field-entry)
              ^Type ct (:self-ctype env)
              ftag   (:tag target)
              ^Class ft (if (class? ftag) ftag Object)
              ftype  (Type/getType ft)]
          (emit-node ga env (:val node))
          (.storeLocal ga val-slot)
          (.loadThis ga)
          (.loadLocal ga val-slot)
          (coerce-arg ga ft)
          (.putField ga ct ^String fname ftype)
          (.loadLocal ga val-slot)))

      :host-interop
      (let [inst-slot (.newLocal ga obj-type)]
        (emit-node ga env (:target target))
        (.storeLocal ga inst-slot)
        (emit-node ga env (:val node))
        (.storeLocal ga val-slot)
        (.loadLocal ga inst-slot)
        (.push ga (str (:m-or-f target)))
        (.loadLocal ga val-slot)
        (.invokeStatic ga reflector-type reflector-set-instance-field-method))

      (throw (ex-info (str "unsupported set! target op: " (:op target))
                      {:type :unsupported-op :op (:op target) :form (:form node)})))))

(defn- emit-monitor
  [^GeneratorAdapter ga env node opcode]
  (emit-node ga env (:target node))
  (.visitInsn ga opcode)
  (.visitInsn ga Opcodes/ACONST_NULL))

(defn- emit-case-branch-chain
  [^GeneratorAdapter ga env node]
  (let [test-slot (.newLocal ga obj-type)
        end-label (.newLabel ga)]
    (emit-node ga env (:test node))
    (.storeLocal ga test-slot)
    (doseq [[test-node then-node] (map vector (:tests node) (:thens node))]
      (let [next-label (.newLabel ga)]
        (.loadLocal ga test-slot)
        (emit-node ga env (:test test-node))
        (.invokeStatic ga util-type util-equiv-method)
        (.ifZCmp ga GeneratorAdapter/EQ next-label)
        (emit-node ga env (:then then-node))
        (.goTo ga end-label)
        (.mark ga next-label)))
    (emit-node ga env (:default node))
    (.mark ga end-label)))

(defn- case-test-hash
  [test-node]
  (when (and (= :case-test (:op test-node))
             (integer? (:hash test-node))
             (:test test-node))
    (int (:hash test-node))))

(defn- case-switch-buckets
  [node]
  (let [pairs (mapv vector (:tests node) (:thens node))]
    (when (and (seq pairs)
               (every? (comp some? case-test-hash first) pairs))
      (->> pairs
           (group-by (comp case-test-hash first))
           (into (sorted-map))))))

(defn- case-switch-supported?
  [node]
  (contains? #{:int :hash-identity :hash-equiv} (:test-type node)))

(defn- dense-switch?
  [hashes]
  (let [min-h (apply min hashes)
        max-h (apply max hashes)
        span  (inc (- max-h min-h))]
    (<= span (* 2 (count hashes)))))

(defn- emit-switch-dispatch
  [^GeneratorAdapter ga buckets default-label]
  (let [hashes        (vec (keys buckets))
        bucket-labels (into {}
                            (map (fn [h] [h (.newLabel ga)]))
                            hashes)]
    (if (dense-switch? hashes)
      (let [min-h  (apply min hashes)
            max-h  (apply max hashes)
            labels (mapv #(get bucket-labels % default-label)
                         (range min-h (inc max-h)))]
        (.visitTableSwitchInsn ga
                               (int min-h)
                               (int max-h)
                               ^Label default-label
                               (into-array Label labels)))
      (.visitLookupSwitchInsn ga
                              ^Label default-label
                              (int-array hashes)
                              (into-array Label (map bucket-labels hashes))))
    bucket-labels))

(defn- emit-case-int-switch-key
  [^GeneratorAdapter ga test-slot default-label]
  (let [cast-label (.newLabel ga)]
    (.loadLocal ga test-slot)
    (.instanceOf ga number-type)
    (.ifZCmp ga GeneratorAdapter/NE cast-label)
    (.loadLocal ga test-slot)
    (.instanceOf ga character-type)
    (.ifZCmp ga GeneratorAdapter/NE cast-label)
    (.goTo ga default-label)
    (.mark ga cast-label)
    (.loadLocal ga test-slot)
    (.invokeStatic ga rt-type rt-unchecked-intcast-object-method)))

(defn- emit-case-hash-switch-key
  [^GeneratorAdapter ga node test-slot]
  (.loadLocal ga test-slot)
  (.invokeStatic ga util-type util-hash-method)
  (let [shift (long (or (:shift node) 0))
        mask  (long (or (:mask node) 0))]
    (when (pos? shift)
      (.push ga (int shift))
      (.visitInsn ga Opcodes/IUSHR))
    (when (pos? mask)
      (.push ga (int mask))
      (.visitInsn ga Opcodes/IAND))))

(defn- emit-case-switch-key
  [^GeneratorAdapter ga node test-slot default-label]
  (case (:test-type node)
    :int           (emit-case-int-switch-key ga test-slot default-label)
    :hash-identity (emit-case-hash-switch-key ga node test-slot)
    :hash-equiv    (emit-case-hash-switch-key ga node test-slot)))

(defn- emit-case-bucket
  [^GeneratorAdapter ga env test-slot end-label default-label pairs skip-check?]
  (if skip-check?
    (do
      (emit-node ga env (:then (second (first pairs))))
      (.goTo ga end-label))
    (do
      (doseq [[test-node then-node] pairs]
        (let [next-label (.newLabel ga)]
          (.loadLocal ga test-slot)
          (emit-node ga env (:test test-node))
          (.invokeStatic ga util-type util-equiv-method)
          (.ifZCmp ga GeneratorAdapter/EQ next-label)
          (emit-node ga env (:then then-node))
          (.goTo ga end-label)
          (.mark ga next-label)))
      (.goTo ga default-label))))

(defn- emit-case
  [^GeneratorAdapter ga env node]
  (if-let [buckets (and (case-switch-supported? node)
                        (case-switch-buckets node))]
    (let [test-slot    (.newLocal ga obj-type)
          default-label (.newLabel ga)
          end-label     (.newLabel ga)]
      (emit-node ga env (:test node))
      (.storeLocal ga test-slot)
      (emit-case-switch-key ga node test-slot default-label)
      (let [bucket-labels (emit-switch-dispatch ga buckets default-label)
            skip-hashes   (set (:skip-check? node))]
        (doseq [[h pairs] buckets]
          (.mark ga ^Label (get bucket-labels h))
          (emit-case-bucket ga env test-slot end-label default-label pairs
                            (contains? skip-hashes h))))
      (.mark ga default-label)
      (emit-node ga env (:default node))
      (.mark ga end-label))
    (emit-case-branch-chain ga env node)))

(defn- emit-import
  "analyzer :import op `(import* \"java.util.ArrayList\")` 직접 emit.
  host clojure.lang.Compiler.ImportExpr 와 동일한 bytecode 를 낸다:
    (.importClass ^Namespace (.deref RT/CURRENT_NS)
                  (RT/classForNameNonLoading class-name))
  → import 된 Class 를 stack 에 남긴다(:do statement 위치면 pop). 클래스 등록은
  host runtime(Namespace/RT) 위임이지만, 그 호출 bytecode 는 우리가 직접 짠다
  (host Compiler 0회). §13 runtime-lib host 위임 경계와 일치."
  [^GeneratorAdapter ga _env node]
  (.getStatic ga rt-type "CURRENT_NS" var-type)
  (.invokeVirtual ga var-type var-deref-method)
  (.checkCast ga ns-type)
  (.push ga ^String (:class node))
  (.invokeStatic ga rt-type rt-class-for-name-nonloading-method)
  (.invokeVirtual ga ns-type ns-import-class-method))

(defn- emit-node
  [^GeneratorAdapter ga env node]
  (emit-line-number ga node)
  (case (:op node)
    :const     (emit-const ga env node)
    :local     (emit-local ga env node)
    :var         (emit-var ga env node)
    :invoke      (emit-invoke ga env node)
    :prim-invoke (emit-invoke ga env node)  ; 우리 fn 은 Object invoke 만 → 일반 invoke
    :protocol-invoke (emit-protocol-invoke ga env node)
    :static-call (emit-static-call ga env node)
    :if          (emit-if ga env node)
    :let         (emit-let ga env node)
    :letfn       (emit-letfn ga env node)
    :vector      (emit-vector ga env node)
    :map         (emit-map ga env node)
    :set         (emit-set ga env node)
    :quote       (emit-node ga env (:expr node))   ; :expr 은 :const(quoted datum)
    :loop        (emit-loop ga env node)
    :case        (emit-case ga env node)
    :recur       (emit-recur ga env node)
    :throw       (emit-throw ga env node)
    :try         (emit-try ga env node)
    :set!        (emit-set! ga env node)
    :monitor-enter (emit-monitor ga env node Opcodes/MONITORENTER)
    :monitor-exit  (emit-monitor ga env node Opcodes/MONITOREXIT)
    :new            (emit-new ga env node)
    :instance-call  (emit-instance-call ga env node)
    :static-field   (emit-static-field ga node)
    :instance-field (emit-instance-field ga env node)
    :host-interop   (emit-host-interop ga env node)
    :keyword-invoke (emit-keyword-invoke ga env node)
    :instance?      (emit-instance? ga env node)
    :the-var        (emit-the-var ga env node) ; #'x → Var (deref 안 함)
    :fn             (emit-fn ga env node) ; 중첩 fn(클로저 캡처)
    :reify          (emit-reify ga env node)
    :deftype        (emit-deftype ga env node)
    :import         (emit-import ga env node)
    :def            (emit-def ga env node)
    :do        (do
                 (doseq [s (:statements node)]
                   (emit-node ga env s)
                   (.pop ga))                 ; statement 값 버림
                 (emit-node ga env (:ret node)))
    :with-meta (emit-node ga env (:expr node))
    (throw (ex-info (str "unsupported AST op: " (:op node))
                    {:type :unsupported-op :op (:op node) :form (:form node)}))))

;; ---------------------------------------------------------------------------
;; fn 컴파일: AST(:fn) → AFn 서브클래스 bytecode → 로드된 인스턴스
;; ---------------------------------------------------------------------------

;; ── 자유변수(free local) 분석 — 클로저 캡처 대상 ─────────────────────────
;; analyzer 가 :closed-overs 를 주지 않으므로 직접 계산한다.
;; fn 노드가 참조하지만 그 fn 안에서 바인딩되지 않은 지역변수 = 캡처 대상.

(defn- free-locals*
  [node bound acc]
  (when (map? node)
    (condp = (:op node)
      :local     (when-not (contains? bound (:name node))
                   (swap! acc conj (:name node)))
      :fn        (let [b (if-let [l (:local node)] (conj bound (:name l)) bound)]
                   (doseq [mth (:methods node)] (free-locals* mth b acc)))
      :fn-method (free-locals* (:body node)
                               (into bound (map :name (:params node))) acc)
      :let       (let [b (reduce (fn [bb bd]
                                   (free-locals* (:init bd) bb acc)
                                   (conj bb (:name bd)))
                                 bound (:bindings node))]
                   (free-locals* (:body node) b acc))
      :letfn     (let [b (into bound (map :name (:bindings node)))]
                   (doseq [bd (:bindings node)]
                     (free-locals* (:init bd) b acc))
                   (free-locals* (:body node) b acc))
      :reify     (doseq [mth (:methods node)]
                   (free-locals* mth bound acc))
      :deftype   (let [b (into bound (map :name (:fields node)))]
                   (doseq [mth (:methods node)]
                     (free-locals* mth b acc)))
      :method    (let [b (into bound
                                (concat [(some-> node :this :name)]
                                        (map :name (:params node))))]
                   (free-locals* (:body node) (disj b nil) acc))
      :loop      (let [b (reduce (fn [bb bd]
                                   (free-locals* (:init bd) bb acc)
                                   (conj bb (:name bd)))
                                 bound (:bindings node))]
                   (free-locals* (:body node) b acc))
      :catch     (free-locals* (:body node) (conj bound (:name (:local node))) acc)
      ;; default: 모든 child 노드 재귀(같은 bound)
      (doseq [k (:children node)]
        (let [v (get node k)]
          (if (vector? v)
            (doseq [x v] (free-locals* x bound acc))
            (free-locals* v bound acc)))))))

(defn- free-locals
  "fn 노드가 캡처하는 자유 지역변수 이름 집합."
  [fn-node]
  (let [acc (atom #{})]
    (free-locals* fn-node #{} acc)
    (vec (sort @acc))))

;; ---------------------------------------------------------------------------
;; fn 컴파일: AST(:fn) → AFn 서브클래스 bytecode(캡처는 필드+생성자로)
;; ---------------------------------------------------------------------------

(defn- compile-fn-class
  "fn-node 를 AFn(고정 arity, **다중 가능**) 또는 RestFn(variadic 포함) 서브클래스로 emit.
  fvs(캡처)는 필드+생성자 인자로. *dcl* 에 defineClass 하고 {:class :ctype} 반환.
  - 고정 arity 들: 각 arity 마다 invoke(Object * nparams) 메서드
  - variadic: getRequiredArity()=fixed, doInvoke(Object * (fixed+1))
  - variadic + fixed 혼합: RestFn 에 fixed invoke 들도 같이 override"
  [fn-node fvs & [opts]]
  (let [methods   (:methods fn-node)
        typed-locals-enabled? (if (contains? opts :typed-locals-enabled?)
                                (:typed-locals-enabled? opts)
                                *typed-locals-enabled*)
        variadic? (boolean (some :variadic? methods))
        variadic-method (first (filter :variadic? methods))
        nfv       (count fvs)
        cap-name  (fn [i] (str "__cap" i))
        super-type      (if variadic? restfn-type afn-type)
        super-internal  (if variadic? "clojure/lang/RestFn" "clojure/lang/AFunction")
        cname     (next-generated-class-name "Fn")
        internal  (.replace cname \. \/)
        ctype     (Type/getObjectType internal)
        ;; COMPUTE_FRAMES 는 분기 merge 시 공통 슈퍼타입을 Class.forName 으로 찾는데,
        ;; 우리 gen 클래스는 *dcl* 에만 있어 시스템 로더가 못 찾는다(깊은 중첩에서
        ;; ClassNotFound). gen 클래스 쌍은 Object 로 처리해 회피한다.
        cw        (proxy [ClassWriter] [(bit-or ClassWriter/COMPUTE_FRAMES ClassWriter/COMPUTE_MAXS)]
                    (getCommonSuperClass [^String t1 ^String t2]
                      (if (or (.startsWith t1 "pnix/clj_meta/gen/")
                              (.startsWith t2 "pnix/clj_meta/gen/"))
                        "java/lang/Object"
                        (proxy-super getCommonSuperClass t1 t2))))
        capture-fields (into {}
                             (map-indexed (fn [i fv] [fv (cap-name i)]) fvs))
        var-fields   (atom {})                     ; 캐싱 {Var → field}
        const-fields (atom {})                     ; 캐싱 {복합 const 값 → field}
        method-env (fn [method body-start]
        (let [params (:params method)]
                       (-> (into {} (map-indexed
                                     (fn [i b]
                                       (if-let [pc (typed-local-primitive-class b)]
                                         [(:name b) [:arg i pc]]
                                         [(:name b) [:arg i]]))
                                     params))
                           (cond-> (:local fn-node) (assoc (:name (:local fn-node)) [:this]))
                           (into (map-indexed (fn [i fv] [fv [:field (cap-name i)]]) fvs))
                           (assoc :self-ctype ctype
                                  :var-fields var-fields
                                  :const-fields const-fields
                                  :typed-locals-enabled? typed-locals-enabled?
                                  :recur-target {:label body-start
                                                 :targets (vec (map-indexed (fn [i _] [:arg i]) params))}))))
        emit-method (fn [^GeneratorAdapter ga method]
                      (let [body-start (.newLabel ga)]
                        (.mark ga body-start)
                        (binding [*last-emitted-line* (atom nil)]
                          (emit-node ga (method-env method body-start) (:body method)))
                        (.returnValue ga)
                        (.endMethod ga)))]
    (.visit cw Opcodes/V1_8 Opcodes/ACC_PUBLIC internal nil super-internal nil)
    (.visitSource cw (or (node-source fn-node) "NO_SOURCE_PATH") nil)
    ;; 캡처 필드
    (dotimes [i nfv]
      (.visitEnd (.visitField cw (if (:mutable-captures? opts)
                                    Opcodes/ACC_PUBLIC
                                    (bit-or Opcodes/ACC_PUBLIC Opcodes/ACC_FINAL))
                              (cap-name i) (.getDescriptor obj-type) nil nil)))
    ;; 생성자: super() + 캡처 필드 저장
    (let [^Method ctor (Method. "<init>" Type/VOID_TYPE (into-array Type (repeat nfv obj-type)))
          ga   (GeneratorAdapter. Opcodes/ACC_PUBLIC ctor nil nil cw)]
      (.loadThis ga)
      (.invokeConstructor ga super-type init-method)
      (dotimes [i nfv]
        (.loadThis ga)
        (.loadArg ga (int i))
        (.putField ga ctype (cap-name i) obj-type))
      (.returnValue ga)
      (.endMethod ga))
    ;; 각 arity 메서드 (다중 fixed → 여러 invoke; variadic → getRequiredArity+doInvoke)
    (when variadic?
      (let [ga (GeneratorAdapter. Opcodes/ACC_PUBLIC getreqarity-method nil nil cw)]
        (.push ga (int (:fixed-arity variadic-method)))
        (.returnValue ga)
        (.endMethod ga)))
    (doseq [method methods]
      (if (:variadic? method)
        (let [^Method dm (Method. "doInvoke" obj-type
                                  (into-array Type (repeat (count (:params method)) obj-type)))
              ga (GeneratorAdapter. Opcodes/ACC_PUBLIC dm nil nil cw)]
          (emit-method ga method))
        (let [^Method m (invoke-method (count (:params method)))
              ga (GeneratorAdapter. Opcodes/ACC_PUBLIC m nil nil cw)]
          (emit-method ga method))))
    ;; var/const 캐싱: 참조된 Var/복합 const 를 static field 로 두고 <clinit> 1회 초기화
    (let [var-field-rows   (sort-by val @var-fields)
          const-field-rows (sort-by val @const-fields)]
      (doseq [[_ fname] var-field-rows]
        (.visitEnd (.visitField cw (bit-or Opcodes/ACC_PUBLIC Opcodes/ACC_STATIC)
                                fname (.getDescriptor var-type) nil nil)))
      (doseq [[_ fname] const-field-rows]
        (.visitEnd (.visitField cw (bit-or Opcodes/ACC_PUBLIC Opcodes/ACC_STATIC)
                                fname (.getDescriptor obj-type) nil nil)))
      (when (or (seq var-field-rows) (seq const-field-rows))
        (let [ga (GeneratorAdapter. Opcodes/ACC_STATIC (Method/getMethod "void <clinit> ()") nil nil cw)]
          (doseq [[^Var v fname] var-field-rows]
            (let [m (meta v)]
              (.push ga (str (ns-name (:ns m))))
              (.push ga (str (:name m)))
              (.invokeStatic ga rt-type rt-var-method)
              (.putStatic ga ctype fname var-type)))
          (doseq [[val fname] const-field-rows]
            (emit-const-value ga val)                ; 값 1회 생성
            (.putStatic ga ctype fname obj-type))
          (.returnValue ga)
          (.endMethod ga))))
    (.visitEnd cw)
    (let [bytes (.toByteArray cw)]
      (when *emitted* (swap! *emitted* assoc cname bytes))
      (let [klass (define-generated-class cname bytes)]
        (remember-class! klass)
        {:class klass
         :ctype ctype
         :capture-fields capture-fields}))))

(defn- class-internal-name
  [^Class c]
  (.getInternalName (Type/getType c)))

(def ^:private primitive-symbol->class
  {'int Integer/TYPE 'long Long/TYPE 'double Double/TYPE 'float Float/TYPE
   'boolean Boolean/TYPE 'byte Byte/TYPE 'short Short/TYPE 'char Character/TYPE
   'void Void/TYPE})

(defn- reify-class-value
  ^Class [x]
  (cond
    (class? x) x
    (symbol? x) (or (primitive-symbol->class x) (Class/forName (str x)))
    :else (throw (ex-info "reify metadata did not contain a Class"
                          {:type :unsupported-op
                           :op :reify
                           :value x}))))

(defn- normalize-reify-method-target
  [target]
  (cond
    (instance? java.lang.reflect.Method target)
    (let [^java.lang.reflect.Method m target]
      {:name (.getName m)
       :return-type (.getReturnType m)
       :declaring-class (.getDeclaringClass m)
       :parameter-types (vec (.getParameterTypes m))})

    (map? target)
    {:name (name (:name target))
     :return-type (reify-class-value (:return-type target))
     :declaring-class (reify-class-value (:declaring-class target))
     :parameter-types (mapv reify-class-value (:parameter-types target))}

    :else
    (throw (ex-info "unsupported reify method target metadata"
                    {:type :unsupported-op
                     :op :reify
                     :target target}))))

(defn- reify-user-interface-classes
  "사용자가 명시한 인터페이스(IObj 제외 — IObj 는 우리가 auto-generate)."
  [node]
  (->> (concat (:interfaces node)
               (map :declaring-class
                    (map normalize-reify-method-target
                         (mapcat :methods (:methods node)))))
       (filter class?)
       (filter #(.isInterface ^Class %))
       (remove #(or (= clojure.lang.IObj %) (= clojure.lang.IMeta %)))
       (distinct)
       (sort-by #(.getName ^Class %))))

(defn- reify-interface-classes
  "host clojure reify 처럼 항상 IObj 를 구현한다(meta/withMeta auto-generate). 사용자
  인터페이스는 그대로 + IObj 를 추가한다."
  [node]
  (concat (reify-user-interface-classes node) [clojure.lang.IObj]))

(defn- reify-method-target
  [method-node]
  (or (first (sort-by (comp pr-str :name)
                      (map normalize-reify-method-target (:methods method-node))))
      (throw (ex-info "reify method has no reflected target"
                      {:type :unsupported-op
                       :op :reify
                       :method (:name method-node)}))))

(defn- reify-asm-method
  ^Method [target]
  (Method. ^String (:name target)
           (Type/getType ^Class (:return-type target))
           (into-array Type (map #(Type/getType ^Class %)
                                 (:parameter-types target)))))

(defn- iobj-method?
  [method-node ^String method-name arity]
  (boolean
   (some (fn [target]
           (let [target' (normalize-reify-method-target target)]
             (and (= method-name (:name target'))
                  (= arity (count (:parameter-types target')))
                  (let [decl (:declaring-class target')]
                    (or (= clojure.lang.IObj decl)
                        (= clojure.lang.IMeta decl))))))
         (:methods method-node))))

(defn- emit-reify-class
  [node fvs opts]
  (let [typed-locals-enabled? (if (contains? opts :typed-locals-enabled?)
                                (:typed-locals-enabled? opts)
                                *typed-locals-enabled*)
        nfv       (count fvs)
        cap-name  (fn [i] (str "__cap" i))
        cname     (next-generated-class-name "Reify")
        internal  (.replace cname \. \/)
        ctype     (Type/getObjectType internal)
        cw        (proxy [ClassWriter] [(bit-or ClassWriter/COMPUTE_FRAMES ClassWriter/COMPUTE_MAXS)]
                    (getCommonSuperClass [^String t1 ^String t2]
                      (if (or (.startsWith t1 "pnix/clj_meta/gen/")
                              (.startsWith t2 "pnix/clj_meta/gen/"))
                        "java/lang/Object"
                        (proxy-super getCommonSuperClass t1 t2))))
        interfaces (into-array String
                               (map class-internal-name
                                    (reify-interface-classes node)))
        capture-fields (into {}
                             (map-indexed (fn [i fv] [fv (cap-name i)]) fvs))
        var-fields   (atom {})
        const-fields (atom {})
        method-env (fn [method-node]
                     (let [target (reify-method-target method-node)
                           ptypes (vec (:parameter-types target))
                           params (:params method-node)
                           this-name (some-> method-node :this :name)]
                       (when-not (= (count ptypes) (count params))
                         (throw (ex-info "reify method parameter count mismatch"
                                         {:type :unsupported-op
                                          :op :reify
                                          :method (:name target)
                                          :jvm-params (count ptypes)
                                          :ast-params (count params)})))
                       (-> (into {}
                                 (map-indexed
                                  (fn [i b]
                                    [(:name b) [:jvm-arg i (nth ptypes i)]])
                                  params))
                           (cond-> this-name (assoc this-name [:this]))
                           (into (map-indexed
                                  (fn [i fv] [fv [:field (cap-name i)]])
                                  fvs))
                           (assoc :self-ctype ctype
                                  :var-fields var-fields
                                  :const-fields const-fields
                                  :typed-locals-enabled? typed-locals-enabled?))))
        emit-method (fn [method-node]
                      (let [target (reify-method-target method-node)
                            ret (:return-type target)
                            ga  (GeneratorAdapter. Opcodes/ACC_PUBLIC
                                                   (reify-asm-method target)
                                                   nil
                                                   nil
                                                   cw)
                            env (method-env method-node)]
                        (binding [*last-emitted-line* (atom nil)]
                          (emit-line-number ga method-node)
                          (if (= Void/TYPE ret)
                            (do
                              (emit-node ga env (:body method-node))
                              (.pop ga))
                            (emit-node-as ga env (:body method-node) ret)))
                        (.returnValue ga)
                        (.endMethod ga)))
        user-meta? (some #(iobj-method? % "meta" 0) (:methods node))
        user-withmeta? (some #(iobj-method? % "withMeta" 1) (:methods node))]
    (when (or user-meta? user-withmeta?)
      (throw (ex-info "explicit IObj/IMeta reify matches host rejection"
                      {:type :unsupported-op :op :reify})))
    (.visit cw Opcodes/V1_8 Opcodes/ACC_PUBLIC internal nil "java/lang/Object" interfaces)
    (.visitSource cw (or (node-source node) "NO_SOURCE_PATH") nil)
    ;; __meta 필드(IObj 자동 구현) + capture 필드들
    (.visitEnd (.visitField cw (bit-or Opcodes/ACC_PUBLIC Opcodes/ACC_FINAL)
                            "__meta" (.getDescriptor ipm-type) nil nil))
    (dotimes [i nfv]
      (.visitEnd (.visitField cw (bit-or Opcodes/ACC_PUBLIC Opcodes/ACC_FINAL)
                              (cap-name i) (.getDescriptor obj-type) nil nil)))
    ;; 생성자: <init>(IPersistentMap __meta, Object*captures)
    (let [^Method ctor (Method. "<init>" Type/VOID_TYPE
                                (into-array Type (cons ipm-type (repeat nfv obj-type))))
          ga   (GeneratorAdapter. Opcodes/ACC_PUBLIC ctor nil nil cw)]
      (.loadThis ga)
      (.invokeConstructor ga obj-type init-method)
      (.loadThis ga)
      (.loadArg ga (int 0))
      (.putField ga ctype "__meta" ipm-type)
      (dotimes [i nfv]
        (.loadThis ga)
        (.loadArg ga (int (inc i)))
        (.putField ga ctype (cap-name i) obj-type))
      (.returnValue ga)
      (.endMethod ga))
    ;; 자동 IObj 메서드. 명시 IObj/IMeta reify 는 host 도 거부하므로 위에서
    ;; :unsupported-op 로 닫고, 정상 reify 에만 meta/withMeta 를 생성한다.
    (when-not user-meta?
      (let [ga (GeneratorAdapter. Opcodes/ACC_PUBLIC reify-meta-method nil nil cw)]
        (.loadThis ga)
        (.getField ga ctype "__meta" ipm-type)
        (.returnValue ga)
        (.endMethod ga)))
    (when-not user-withmeta?
      (let [ga (GeneratorAdapter. Opcodes/ACC_PUBLIC reify-withmeta-method nil nil cw)]
        (.newInstance ga ctype)
        (.dup ga)
        (.loadArg ga (int 0))
        (dotimes [i nfv]
          (.loadThis ga)
          (.getField ga ctype (cap-name i) obj-type))
        (.invokeConstructor ga ctype
                            (Method. "<init>" Type/VOID_TYPE
                                     (into-array Type (cons ipm-type (repeat nfv obj-type)))))
        (.returnValue ga)
        (.endMethod ga)))
    (doseq [method-node (:methods node)]
      (emit-method method-node))
    (let [var-field-rows   (sort-by val @var-fields)
          const-field-rows (sort-by val @const-fields)]
      (doseq [[_ fname] var-field-rows]
        (.visitEnd (.visitField cw (bit-or Opcodes/ACC_PUBLIC Opcodes/ACC_STATIC)
                                fname (.getDescriptor var-type) nil nil)))
      (doseq [[_ fname] const-field-rows]
        (.visitEnd (.visitField cw (bit-or Opcodes/ACC_PUBLIC Opcodes/ACC_STATIC)
                                fname (.getDescriptor obj-type) nil nil)))
      (when (or (seq var-field-rows) (seq const-field-rows))
        (let [ga (GeneratorAdapter. Opcodes/ACC_STATIC (Method/getMethod "void <clinit> ()") nil nil cw)]
          (doseq [[^Var v fname] var-field-rows]
            (let [m (meta v)]
              (.push ga (str (ns-name (:ns m))))
              (.push ga (str (:name m)))
              (.invokeStatic ga rt-type rt-var-method)
              (.putStatic ga ctype fname var-type)))
          (doseq [[val fname] const-field-rows]
            (emit-const-value ga val)
            (.putStatic ga ctype fname obj-type))
          (.returnValue ga)
          (.endMethod ga))))
    (.visitEnd cw)
    (let [bytes (.toByteArray cw)]
      (when *emitted* (swap! *emitted* assoc cname bytes))
      (let [klass (define-generated-class cname bytes)]
        (remember-class! klass)
        {:class klass
         :ctype ctype
         :capture-fields capture-fields}))))

(defn- emit-reify
  "Direct emit for anonymous reify classes. host clojure reify 처럼 IObj 를 자동 구현해
  metadata(meta/withMeta)를 지원한다. 새로 만든 인스턴스의 __meta 는 null."
  [^GeneratorAdapter ga env node]
  (let [fvs (vec (free-locals node))
        {:keys [ctype]} (try
                          (emit-reify-class
                           node fvs
                           {:typed-locals-enabled?
                            (:typed-locals-enabled? env)})
                          (catch clojure.lang.ExceptionInfo e
                            (if (= :unsupported-op (:type (ex-data e)))
                              (throw e)
                              (throw (ex-info "reify direct emit failed"
                                              {:type :unsupported-op :op :reify}))))
                          (catch Throwable _e
                            (throw (ex-info "reify direct emit failed"
                                            {:type :unsupported-op :op :reify}))))]
    (.newInstance ga ctype)
    (.dup ga)
    (.visitInsn ga Opcodes/ACONST_NULL)        ; __meta = null
    (doseq [fv fvs]
      (emit-local ga env {:name fv}))
    (.invokeConstructor ga ctype
                        (Method. "<init>" Type/VOID_TYPE
                                 (into-array Type (cons ipm-type (repeat (count fvs) obj-type)))))))

;; ── deftype 직접 emit (named type, host Compiler 0회) ───────────────────────────
;; deftype* 의 named 클래스를 우리가 직접 emit 한다. analyzer 가 분석-시점에 정의한 stub 은
;; sibling loader 에 있고, 우리 full 클래스는 *dcl* 에 정의된다. 단일 컴파일 단위 안에서
;; factory(`new Name`)/method 가 *dcl* 의 우리 클래스로 resolve 하므로 type identity 가
;; 일관된다(필드는 reify capture 처럼 `:local :field` → [:field name] env).

(defn- deftype-interface-classes
  [node]
  (->> (:interfaces node)
       (filter class?)
       (filter #(.isInterface ^Class %))
       (distinct)
       (sort-by #(.getName ^Class %))))

(defn- deftype-field-class
  "deftype/defrecord 필드의 선언 타입(:tag) 을 Class 로. 기본 Object."
  ^Class [field]
  (let [tag (:tag field)]
    (cond
      (class? tag) tag
      (symbol? tag) (or (primitive-symbol->class tag)
                        (try (Class/forName (str tag)) (catch Throwable _ Object)))
      :else Object)))

(defn- emit-deftype-class
  [node opts]
  (let [typed-locals-enabled? (if (contains? opts :typed-locals-enabled?)
                                (:typed-locals-enabled? opts)
                                *typed-locals-enabled*)
        cname       (let [cn (:class-name node)]
                      (if (class? cn) (.getName ^Class cn) (str cn)))
        internal    (.replace cname \. \/)
        ctype       (Type/getObjectType internal)
        fields      (vec (:fields node))
        field-names (mapv #(name (:name %)) fields)
        field-classes (mapv deftype-field-class fields)
        field-types (mapv #(Type/getType ^Class %) field-classes)
        field-mutability (mapv :mutable fields)
        nfld        (count fields)
        record?     (boolean (some #(= clojure.lang.IRecord %) (:interfaces node)))
        record-user-field-count (if record?
                                  (let [idx (.indexOf field-names "__meta")]
                                    (if (neg? idx) (max 0 (- nfld 4)) idx))
                                  0)
        cw          (proxy [ClassWriter] [(bit-or ClassWriter/COMPUTE_FRAMES ClassWriter/COMPUTE_MAXS)]
                      (getCommonSuperClass [^String t1 ^String t2]
                        (if (or (.startsWith t1 "pnix/clj_meta/gen/")
                                (.startsWith t2 "pnix/clj_meta/gen/")
                                (= t1 internal) (= t2 internal))
                          "java/lang/Object"
                          (proxy-super getCommonSuperClass t1 t2))))
        interfaces  (into-array String (map class-internal-name (deftype-interface-classes node)))
        var-fields  (atom {})
        const-fields (atom {})
        field-env   (into {} (map (fn [f] [(:name f) [:field (name (:name f)) (deftype-field-class f)]]) fields))
        field-flags (fn [mutability]
                      (cond
                        (= :volatile-mutable mutability)
                        (bit-or Opcodes/ACC_PUBLIC Opcodes/ACC_VOLATILE)

                        mutability
                        Opcodes/ACC_PUBLIC

                        :else
                        (bit-or Opcodes/ACC_PUBLIC Opcodes/ACC_FINAL)))
        method-env  (fn [method-node]
                      (let [target (reify-method-target method-node)
                            ptypes (vec (:parameter-types target))
                            params (:params method-node)
                            this-name (some-> method-node :this :name)]
                        (when-not (= (count ptypes) (count params))
                          (throw (ex-info "deftype method parameter count mismatch"
                                          {:type :unsupported-op :op :deftype
                                           :method (:name target)})))
                        (-> (into {} (map-indexed (fn [i b] [(:name b) [:jvm-arg i (nth ptypes i)]]) params))
                            (cond-> this-name (assoc this-name [:this]))
                            (into field-env)
                            (assoc :self-ctype ctype
                                   :var-fields var-fields
                                   :const-fields const-fields
                                   :typed-locals-enabled? typed-locals-enabled?))))
        emit-method (fn [method-node]
                      (let [target (reify-method-target method-node)
                            ret (:return-type target)
                            ga  (GeneratorAdapter. Opcodes/ACC_PUBLIC (reify-asm-method target) nil nil cw)
                            env (method-env method-node)]
                        (binding [*last-emitted-line* (atom nil)]
                          (emit-line-number ga method-node)
                          (if (= Void/TYPE ret)
                            (do (emit-node ga env (:body method-node)) (.pop ga))
                            (emit-node-as ga env (:body method-node) ret)))
                        (.returnValue ga)
                        (.endMethod ga)))]
    (.visit cw Opcodes/V1_8 Opcodes/ACC_PUBLIC internal nil "java/lang/Object" interfaces)
    (.visitSource cw (or (node-source node) "NO_SOURCE_PATH") nil)
    ;; 필드 선언: 선언 타입 + mutable 은 non-final.
    (dotimes [i nfld]
      (.visitEnd (.visitField cw (field-flags (nth field-mutability i))
                              (nth field-names i)
                              (.getDescriptor ^Type (nth field-types i)) nil nil)))
    ;; full 생성자: 모든 필드(선언 타입). field-mutable 무관하게 set.
    (let [^Method ctor (Method. "<init>" Type/VOID_TYPE (into-array Type field-types))
          ga (GeneratorAdapter. Opcodes/ACC_PUBLIC ctor nil nil cw)]
      (.loadThis ga)
      (.invokeConstructor ga obj-type init-method)
      (dotimes [i nfld]
        (.loadThis ga)
        (.loadArg ga (int i))
        (.putField ga ctype (nth field-names i) ^Type (nth field-types i)))
      (.returnValue ga)
      (.endMethod ga))
    ;; defrecord 규약: full 외에 (nfld-2)-arg(__hash/__hasheq=0) + (nfld-4)-arg(__meta/__extmap=null)
    ;; 생성자를 추가해 factory(->R 2-arg)/withMeta(4-arg)가 resolve 되게 한다.
    (when (and record? (>= nfld 4))
      ;; (nfld-2)-arg: this(...prefix, 0, 0) — __hash/__hasheq 를 int 0 으로
      (let [n2 (- nfld 2)
            ^Method c2 (Method. "<init>" Type/VOID_TYPE (into-array Type (subvec field-types 0 n2)))
            ga (GeneratorAdapter. Opcodes/ACC_PUBLIC c2 nil nil cw)]
        (.loadThis ga)
        (dotimes [i n2] (.loadArg ga (int i)))
        (.push ga (int 0))
        (.push ga (int 0))
        (.invokeConstructor ga ctype (Method. "<init>" Type/VOID_TYPE (into-array Type field-types)))
        (.returnValue ga)
        (.endMethod ga))
      ;; (nfld-4)-arg(=user fields): this(...user, null, null) — __meta/__extmap 를 null 로
      (let [n4 (- nfld 4)
            n2 (- nfld 2)
            ^Method c4 (Method. "<init>" Type/VOID_TYPE (into-array Type (subvec field-types 0 n4)))
            ga (GeneratorAdapter. Opcodes/ACC_PUBLIC c4 nil nil cw)]
        (.loadThis ga)
        (dotimes [i n4] (.loadArg ga (int i)))
        (.visitInsn ga Opcodes/ACONST_NULL)
        (.visitInsn ga Opcodes/ACONST_NULL)
        (.invokeConstructor ga ctype (Method. "<init>" Type/VOID_TYPE (into-array Type (subvec field-types 0 n2))))
        (.returnValue ga)
        (.endMethod ga)))
    ;; defrecord map factory: map->R calls public static R create(IPersistentMap).
    (when (and record? (>= nfld 4))
      (let [n-user record-user-field-count
            n2 (- nfld 2)
            ctor (Method. "<init>" Type/VOID_TYPE (into-array Type (subvec field-types 0 n2)))
            create-method (Method. "create" ctype (into-array Type [ipm-type]))
            ga (GeneratorAdapter. (bit-or Opcodes/ACC_PUBLIC Opcodes/ACC_STATIC)
                                  create-method nil nil cw)
            ext-slot (.newLocal ga ipm-type)]
        (.loadArg ga (int 0))
        (.storeLocal ga ext-slot)
        (dotimes [i n-user]
          (.loadLocal ga ext-slot)
          (.getStatic ga ctype
                      (const-field! const-fields (keyword (nth field-names i)))
                      obj-type)
          (.invokeInterface ga ipm-type ipm-without-method)
          (.storeLocal ga ext-slot))
        (.newInstance ga ctype)
        (.dup ga)
        (dotimes [i n-user]
          (.loadArg ga (int 0))
          (.getStatic ga ctype
                      (const-field! const-fields (keyword (nth field-names i)))
                      obj-type)
          (.invokeInterface ga ilookup-type ilookup-valat-method)
          (coerce-arg ga (nth field-classes i)))
        (.visitInsn ga Opcodes/ACONST_NULL)
        (let [nonempty-label (.newLabel ga)
          done-label (.newLabel ga)]
          (.loadLocal ga ext-slot)
          (.invokeStatic ga rt-type rt-seq-method)
          (.visitJumpInsn ga Opcodes/IFNONNULL nonempty-label)
          (.visitInsn ga Opcodes/ACONST_NULL)
          (.goTo ga done-label)
          (.mark ga nonempty-label)
          (.loadLocal ga ext-slot)
          (.mark ga done-label))
        (.invokeConstructor ga ctype ctor)
        (.returnValue ga)
        (.endMethod ga)))
    (doseq [method-node (:methods node)]
      (emit-method method-node))
    (let [var-field-rows   (sort-by val @var-fields)
          const-field-rows (sort-by val @const-fields)]
      (doseq [[_ fname] var-field-rows]
        (.visitEnd (.visitField cw (bit-or Opcodes/ACC_PUBLIC Opcodes/ACC_STATIC)
                                fname (.getDescriptor var-type) nil nil)))
      (doseq [[_ fname] const-field-rows]
        (.visitEnd (.visitField cw (bit-or Opcodes/ACC_PUBLIC Opcodes/ACC_STATIC)
                                fname (.getDescriptor obj-type) nil nil)))
      (when (or (seq var-field-rows) (seq const-field-rows))
        (let [ga (GeneratorAdapter. Opcodes/ACC_STATIC (Method/getMethod "void <clinit> ()") nil nil cw)]
          (doseq [[^Var v fname] var-field-rows]
            (let [m (meta v)]
              (.push ga (str (ns-name (:ns m))))
              (.push ga (str (:name m)))
              (.invokeStatic ga rt-type rt-var-method)
              (.putStatic ga ctype fname var-type)))
          (doseq [[val fname] const-field-rows]
            (emit-const-value ga val)
            (.putStatic ga ctype fname obj-type))
          (.returnValue ga)
          (.endMethod ga))))
    (.visitEnd cw)
    (let [bytes (.toByteArray cw)]
      (when *emitted* (swap! *emitted* assoc cname bytes))
      (let [klass (define-generated-class cname bytes)]
        (remember-class! klass)
        {:class klass :ctype ctype}))))

(defn- emit-deftype
  "deftype* op: named 클래스를 *dcl* 에 정의(emit-time)하고, 런타임 값은 nil(:do 에서 pop).
  best-effort: 우리가 직접 못 내는 deftype(예: dynamic protocol 메서드 target 미해소)은
  `:unsupported-op` 으로 올려 compile-form 이 host fallback 하게 한다(기능 손실 0)."
  [^GeneratorAdapter ga env node]
  (try
    (emit-deftype-class node {:typed-locals-enabled? (:typed-locals-enabled? env)})
    (catch clojure.lang.ExceptionInfo e
      (if (= :unsupported-op (:type (ex-data e)))
        (throw e)
        (throw (ex-info "deftype direct emit failed" {:type :unsupported-op :op :deftype}))))
    (catch Throwable _e
      (throw (ex-info "deftype direct emit failed" {:type :unsupported-op :op :deftype}))))
  (.visitInsn ga Opcodes/ACONST_NULL))

(defn- emit-fn
  "중첩 fn: 클래스 emit 후 NEW + 캡처값(둘러싼 env)을 생성자로 주입."
  [^GeneratorAdapter ga env node]
  (let [fvs (vec (free-locals node))
        {:keys [ctype]} (compile-fn-class
                         node fvs
                         {:typed-locals-enabled?
                          (:typed-locals-enabled? env)})]
    (.newInstance ga ctype)
    (.dup ga)
    (doseq [fv fvs]
      (emit-local ga env {:name fv}))     ; 캡처값을 둘러싼 env 에서 평가
    (.invokeConstructor ga ctype
                        (Method. "<init>" Type/VOID_TYPE
                                 (into-array Type (repeat (count fvs) obj-type))))))

(defn- emit-letfn
  "letfn 상호재귀 closure emit.
  각 fn 은 먼저 nil letfn capture 로 생성해 local 슬롯에 저장하고, 모든 인스턴스가
  준비된 뒤 서로의 capture field 를 채운다. 외부 capture 는 생성자에서 바로 주입한다."
  [^GeneratorAdapter ga env node]
  (let [names    (set (map :name (:bindings node)))
        compiled (mapv (fn [b]
                         (let [fvs (vec (free-locals (:init b)))]
                           (assoc b
                                  :fvs fvs
                                  :compiled (compile-fn-class
                                             (:init b)
                                             fvs
                                             {:typed-locals-enabled?
                                              (:typed-locals-enabled? env)
                                              :mutable-captures? true}))))
                       (:bindings node))
        env'     (reduce
                  (fn [e {:keys [name fvs compiled]}]
                    (let [{:keys [ctype]} compiled]
                      (.newInstance ga ctype)
                      (.dup ga)
                      (doseq [fv fvs]
                        (if (contains? names fv)
                          (.visitInsn ga Opcodes/ACONST_NULL)
                          (emit-local ga e {:name fv})))
                      (.invokeConstructor
                       ga ctype
                       (Method. "<init>" Type/VOID_TYPE
                                (into-array Type (repeat (count fvs) obj-type))))
                      (let [slot (.newLocal ga obj-type)]
                        (.storeLocal ga slot)
                        (assoc e name [:local slot]))))
                  env
                  compiled)]
    (doseq [{:keys [name fvs compiled]} compiled
            :let [{:keys [ctype capture-fields]} compiled]]
      (doseq [fv fvs
              :when (contains? names fv)]
        (.loadLocal ga (int (second (get env' name))))
        (.checkCast ga ctype)
        (emit-local ga env' {:name fv})
        (.putField ga ctype (get capture-fields fv) obj-type)))
    (emit-node ga env' (:body node))))

(defn- fn-form-syntax?
  [form]
  (and (seq? form)
       (contains? #{'fn 'fn*} (first form))))

(defn- normalize-analyzed-root
  [ast]
  (if (= :with-meta (:op ast)) (:expr ast) ast))

(defn- hoist-expression-tries
  "JVM 예외 핸들러 진입은 피연산자 스택을 비우므로, 호출 인자 등 스택 위에 값이
  있는 표현 위치의 try 를 그대로 emit 하면 VerifyError(Operand stack underflow)가
  된다 — pnix-clj lowering 의 `(force-slot (get (try …) \"k\"))` 가 실전 재현.
  Clojure 본가(Compiler.java TryParser 의 FNONCE 래핑)처럼 try 를 zero-arg fn
  호출로 호이스팅한다: fn 본문은 RETURN 컨텍스트라 스택이 항상 비어 있다.
  위치 추적 없는 over-approximation — tail try 도 감싸지만 의미는 동일하다
  (recur 는 어차피 try 를 건널 수 없다). quote 내부는 건드리지 않는다."
  [form]
  (cond
    (and (seq? form) (= 'quote (first form)))
    form

    (seq? form)
    (let [walked (with-meta (apply list (map hoist-expression-tries form))
                   (meta form))]
      (if (= 'try (first form))
        (list (list 'fn* [] walked))
        walked))

    (vector? form)
    (with-meta (mapv hoist-expression-tries form) (meta form))

    (map? form)
    (with-meta (into {}
                     (map (fn [[k v]] [(hoist-expression-tries k)
                                       (hoist-expression-tries v)]))
                     form)
      (meta form))

    (set? form)
    (with-meta (into #{} (map hoist-expression-tries) form) (meta form))

    :else form))

(defn- analyze-for-compile-form
  [form phase]
  (try
    {:ok true :ast (ana/analyze (hoist-expression-tries form))}
    (catch Throwable e
      (record-compile-form-fallback!
       {:phase phase
        :reason :analyze-failed
        :form (pr-str form)
        :throwable (throwable-diagnostic e)})
      {:ok false :throwable e})))

(defn- compile-analyzed-fn-form
  [form ast]
  (let [fvs (vec (free-locals ast))]
    (when (seq fvs)
      (throw-compile-form-error!
       :analyze
       form
       (ex-info "compile-form: top-level free variables are not host-eval fallback"
                {:type :compile-form-free-vars
                 :free-vars fvs
                 :form form})))
    (let [compiled (try
                     (compile-fn-class ast fvs {:typed-locals-enabled? true})
                     (catch clojure.lang.ExceptionInfo e
                       (if (= :unsupported-op (:type (ex-data e)))
                         (do
                           (record-compile-form-fallback!
                            {:phase :emit
                             :reason :unsupported-op
                             :form (pr-str form)
                             :throwable (throwable-diagnostic e)})
                           (eval-compile-form-fallback :emit form))
                         (throw-compile-form-error! :emit form e)))
                     (catch Throwable t
                       (throw-compile-form-error! :emit form t)))
          direct?  (map? compiled)]
      (if direct?
        (let [f (try
                  (.newInstance ^Class (:class compiled))
                  (catch Throwable t
                    (throw-compile-form-error! :instantiate form t)))]
          (keep-class-unit! @*unit-classes*)
          f)
        compiled))))

(defn- compile-wrapped-top-level-form
  [form]
  (let [wrapper (list 'fn [] form)
        analysis (analyze-for-compile-form wrapper :non-fn-top-level-wrapper)]
    (if (:ok analysis)
      (compile-analyzed-fn-form wrapper (normalize-analyzed-root (:ast analysis)))
      (eval-compile-form-fallback :analyze form wrapper))))

(defn- compile-form-impl
  ^IFn [form]
  (binding [*dcl*         (compiler-session-loader)
            *gen-counter* (atom -1)
            *class-unit-id* (compile-unit-id form)
            *unit-classes* (atom [])
            *typed-locals-enabled* true
            *emit-line-numbers* true]
    ;; analyzer(frontend)가 분석 못 하는 fn form 은 host eval(clojure.lang.Compiler)로
    ;; 위임한다. top-level ns/defprotocol/deftype 조합은 compile-ns incremental 경로에서
    ;; 직접 emit 한다. fallback 원인은 diagnostics atom 에 남기고, fallback 밖으로 새는
    ;; throwable 은 compile-error envelope 로 감싼다.
    (try
      (let [analysis (analyze-for-compile-form form :root)]
        (if (:ok analysis)
          (let [ast (normalize-analyzed-root (:ast analysis))]
            (if (= :fn (:op ast))
              (compile-analyzed-fn-form form ast)
              (compile-wrapped-top-level-form form)))
          (if (fn-form-syntax? form)
            (eval-compile-form-fallback :analyze form)
            (compile-wrapped-top-level-form form))))       ; non-fn top-level → backend wrapper emit
      (catch Throwable t
        (throw-compile-form-error! :emit form t)))))

(defn compile-form*
  "Clojure form 을 컴파일해 `{:fn IFn :mode :direct|:host-fallback :diagnostics [...]}` 를 반환한다.
  우리 emit 가능하면 host Compiler 0회로 bytecode(clojure.asm). 우리가 아직 직접 emit
  하지 못하는 analyzer/emit 실패 form 은 명시적으로 host eval 에 위임(fallback)하고, fallback
  원인은 호출별 diagnostics 에 담는다. opts `:ns` 는 분석/emit/fallback 을 실행할 namespace 를
  지정한다(symbol, string, Namespace)."
  ([form] (compile-form* form {}))
  ([form opts]
   (let [diagnostics (atom [])]
     (binding [*compile-form-local-fallback-diagnostics* diagnostics]
       (with-compile-context
         opts
         #(let [f (compile-form-impl form)
                ds @diagnostics]
            {:fn f
             :mode (if (seq ds) :host-fallback :direct)
             :diagnostics ds}))))))

(defn compile-form
  "Clojure form 을 컴파일해 실행 가능한 IFn 을 반환한다.
  opts `:ns` 로 분석/emit/fallback namespace 를 지정할 수 있다. 호환 wrapper 이며,
  fallback 모드와 호출별 diagnostics 가 필요하면 `compile-form*` 를 사용한다.
  → Clojure 전체 기능 손실 0 (todo §13 host 위임 경계)."
  (^IFn [form]
   (:fn (compile-form* form)))
  (^IFn [form opts]
   (:fn (compile-form* form opts))))

(defn eval-form
  "임의 top-level form 을 컴파일·실행해 그 값을 반환한다(0-arity fn 으로 감싸 invoke).
  def 등 부수효과 포함. 여러 폼은 (do …) 로 묶어 전달. fallback/mode 관측이 필요하면
  `compile-form*` 를 직접 사용한다. opts 는 `compile-form` 과 같다."
  ([form] (eval-form form {}))
  ([form opts]
   (with-compile-context
     opts
     #((compile-form (list 'fn [] form) opts)))))

(defn eval-string
  "Clojure source 문자열의 모든 form 을 읽어 `(do …)` 로 묶고 `eval-form` 으로 실행한다.
  반환값은 마지막 form 값이다. opts 는 `compile-form` 과 같다."
  ([src] (eval-string src {}))
  ([src opts]
   (eval-form (cons 'do (read-source-forms src)) opts)))

(defn- compile-fn-strict
  "`(fn …)` form 을 우리 backend 로만 컴파일해 IFn 인스턴스를 만든다.
  compile-form 과 달리 미구현 op 를 만나면 host eval fallback 없이 throw 한다 →
  이 경로로 만들어진 함수는 host clojure.lang.Compiler 0회가 보장된다(genuine
  direct emit). top-level 자유변수가 있으면 캡처 불가이므로 throw."
  (^IFn [form] (compile-fn-strict form {}))
  (^IFn [form opts]
   (with-compile-context
     opts
     #(binding [*dcl*         (compiler-session-loader)
                *gen-counter* (atom -1)
                *class-unit-id* (compile-unit-id form)
                *unit-classes* (atom [])
                *typed-locals-enabled* true
                *emit-line-numbers* true]
        (let [ast0 (ana/analyze form)
              ast  (if (= :with-meta (:op ast0)) (:expr ast0) ast0)]
          (when-not (= :fn (:op ast))
            (throw (ex-info "compile-fn-strict: (fn …) 만 받는다" {:op (:op ast)})))
          (let [fvs (vec (free-locals ast))]
            (when (seq fvs)
              (throw (ex-info "compile-fn-strict: top-level 자유변수 불가"
                              {:type :unsupported-op :free-vars fvs})))
            (let [f (.newInstance ^Class (:class (compile-fn-class
                                                  ast fvs {:typed-locals-enabled? true})))]
              (keep-class-unit! @*unit-classes*)
              f)))))))

(defn compile-form-strict
  "`(fn …)` form 을 host fallback 없이 우리 backend 로만 컴파일해 IFn 을 반환한다.
  미지원 op, top-level 자유변수, 비-fn form 은 throw 한다. opts `:ns` 로 분석 namespace 를
  지정할 수 있다. 성공 시 host clojure.lang.Compiler emit 0회가 보장된다."
  (^IFn [form]
   (compile-fn-strict form))
  (^IFn [form opts]
   (compile-fn-strict form opts)))

(defn run-ns-form-strict
  "ns form 하나를 host clojure.lang.Compiler 0회로 backend 컴파일·실행한다.
  analyzer 가 ns 를 (do (in-ns …) (refer …) (require …) (import* …)) 로 확장하고 그
  op 들은 모두 직접 emit 가능하다; in-ns/refer/require/import 의 runtime side-effect 만
  host runtime-lib 위임(§13). compile-fn-strict 라 미구현 op 면 fallback 없이 throw 한다.
  부트스트랩(selfaudit)이 ns form 을 host eval 대신 이 경로로 실행해, 'ns-form-backend-
  compiled=true' 가 *별도 witness 가 아니라 실제 부트스트랩 경로*가 되게 한다(H5b 해소)."
  [ns-form]
  ((compile-fn-strict (list 'fn [] ns-form))))

(defn- ns-form?
  [form]
  (and (seq? form)
       (= 'ns (first form))
       (symbol? (second form))))

(defn- simple-ns-form?
  [form]
  (and (ns-form? form)
       (= 2 (count form))))

(defonce ^:private isolated-ns-counter (atom 0))

(defn- isolated-ns-symbol
  [ns-sym]
  (symbol (str ns-sym "__isolated__" (swap! isolated-ns-counter inc))))

(defn- rewrite-ns-form-symbol
  [ns-form ns-sym]
  (when ns-form
    (apply list 'ns ns-sym (nnext ns-form))))

(defn- prepare-simple-ns!
  [ns-sym]
  (let [target (or (find-ns ns-sym) (create-ns ns-sym))]
    (binding [*ns* target]
      (clojure.core/refer 'clojure.core))
    target))

(defn unload-ns!
  "전역 namespace registry 에서 `ns-sym` 을 제거한다. 이미 없으면 false 를 반환한다.
  기본 compile-ns/load-compiled-ns 는 host load 처럼 global ns 를 변형하므로, 장수 호스트가
  명시적으로 정리할 때 사용한다."
  [ns-sym]
  (if (find-ns ns-sym)
    (do (remove-ns ns-sym) true)
    false))

(defn- prepare-compiled-ns!
  [ns-form ns-sym ns-mode file]
  (case ns-mode
    :current-ns
    (or (find-ns ns-sym) (create-ns ns-sym))

    :direct-simple
    (prepare-simple-ns! ns-sym)

    :direct-compiled
    (if ns-form
      (do
        ;; require/import clause 가 있는 ns form 도 우리 backend 로 컴파일해 실행한다.
        ;; analyzer 가 ns 를 in-ns/refer/require/import* 의 (do …) 로 확장하고,
        ;; 그 op 들(:do/:invoke/:if/:import/:static-call/…)은 모두 직접 emit 가능하다.
        ;; in-ns/require/import 의 runtime side-effect 만 host runtime-lib 위임(§13).
        ;; host clojure.lang.Compiler 는 거치지 않는다(compile-fn-strict = no fallback).
        (binding [*ns* *ns*
                  *file* file]
          (run-ns-form-strict ns-form))
        (or (find-ns ns-sym) (create-ns ns-sym)))
      (or (find-ns ns-sym) (create-ns ns-sym)))))

(defn compile-ns
  "Clojure source 문자열을 load 가능한 in-memory 산출물로 준비한다.
  단순 (ns foo) 는 namespace/core refer 를 직접 준비한다. require/import clause 가 있는
  ns form 도 host clojure.lang.Compiler 없이 우리 backend 로 컴파일해 실행한다
  (compile-fn-strict; in-ns/require/import 의 runtime side-effect 만 host runtime-lib 위임).
  body top-level form 은 load-compiled-ns 가 host 처럼 *incremental* 로 컴파일+실행한다
  (form N 의 runtime 효과가 form N+1 컴파일에 보이도록 — defprotocol/deftype 전방참조 지원).
  기본은 host load 처럼 global ns registry 를 변형한다. `{:isolate true}` 는 원본 ns 대신
  임시 ns 에 준비/로드하고 끝나면 제거하므로 장수 호스트의 registry 오염을 피한다."
  ([src] (compile-ns src {}))
  ([src {:keys [file isolate]}]
   (let [forms      (read-source-forms src)
         source-ns-form (when (ns-form? (first forms)) (first forms))
         source-ns-sym  (or (second source-ns-form) (ns-name *ns*))
         ns-sym     (if isolate (isolated-ns-symbol source-ns-sym) source-ns-sym)
         ns-form    (cond
                      (and isolate source-ns-form) (rewrite-ns-form-symbol source-ns-form ns-sym)
                      isolate                     (list 'ns ns-sym)
                      :else                       source-ns-form)
         body-forms (if source-ns-form (subvec forms 1) forms)
         file'      (or file "NO_SOURCE_PATH")
         ns-mode    (cond
                      (nil? ns-form) :current-ns
                      (simple-ns-form? ns-form) :direct-simple
                      :else :direct-compiled)]
     (try
       (prepare-compiled-ns! ns-form ns-sym ns-mode file')
       (finally
         (when isolate
           (unload-ns! ns-sym))))
     {:schema "pnix.clj-meta.compile-ns.v1"
      :namespace ns-sym
      :source-namespace source-ns-sym
      :isolate (boolean isolate)
      :prepared (not (boolean isolate))
      :ns-form ns-form
      :ns-form-mode ns-mode
      :file file'
      :form-count (count forms)
      :body-count (count body-forms)
      :forms forms
      :body-forms body-forms
      :loadable true})))

(defn load-compiled-ns
  "compile-ns 산출물을 현재 JVM 에 로드한다. host load 처럼 각 body form 을 직전에
  컴파일(우리 backend)하고 즉시 실행한다(incremental). form N 의 runtime 효과(var intern,
  protocol/type 정의)가 form N+1 컴파일 때 보여 전방참조가 동작한다. 기본은 host load 처럼
  global ns 를 남기며, isolate 산출물은 로드 후 임시 ns 를 제거한다. 반환: body 결과 벡터 +
  생성된 thunk fn 들."
  [artifact]
  (let [ns-sym (:namespace artifact)
        ns-form (:ns-form artifact)
        ns-mode (or (:ns-form-mode artifact) :current-ns)
        file   (or (:file artifact) "NO_SOURCE_PATH")
        isolate? (boolean (:isolate artifact))
        thunks (atom [])]
    (try
      (let [target  (or (when (:prepared artifact)
                          (find-ns ns-sym))
                        (prepare-compiled-ns! ns-form ns-sym ns-mode file))
            results (binding [*dcl* (compiler-session-loader)
                              *ns* target
                              *file* file]
                      (mapv (fn [form]
                              (let [f (compile-form (list 'fn [] form))]
                                (swap! thunks conj f)
                                (f)))
                            (:body-forms artifact)))]
        {:schema "pnix.clj-meta.loaded-ns.v1"
         :namespace ns-sym
         :source-namespace (:source-namespace artifact)
         :isolate isolate?
         :unloaded (when isolate? true)
         :file file
         :results results
         :thunks @thunks
         :loaded true})
      (finally
        (when isolate?
          (unload-ns! ns-sym))))))

(defn compile-and-load-ns
  "Clojure source 문자열을 `compile-ns` 한 뒤 즉시 `load-compiled-ns` 한다.
  반환 스키마는 `load-compiled-ns` 와 같다."
  ([src] (compile-and-load-ns src {}))
  ([src opts]
   (load-compiled-ns (compile-ns src opts))))

(defn compile-classes
  "(fn …) form 을 컴파일해 생성된 모든 클래스의 {classname byte[]} 를 반환한다.
  실행하지 않으며, 결정적 클래스명 → 같은 소스는 같은 bytecode. opts `:ns` 로 분석 namespace 를
  지정할 수 있다. self-host 고정점 검증용."
  ([form] (compile-classes form {}))
  ([form opts]
   (with-compile-context
     opts
     #(binding [*dcl*         (compiler-session-loader)
                *gen-counter* (atom -1)
                *class-unit-id* (compile-unit-id form)
                *emitted*     (atom {})
                *typed-locals-enabled* true
                *emit-line-numbers* false]
        (let [ast0 (ana/analyze form)
              ast  (if (= :with-meta (:op ast0)) (:expr ast0) ast0)]
          (when-not (= :fn (:op ast))
            (throw (ex-info "compile-classes: (fn …) 만" {:op (:op ast)})))
          (compile-fn-class ast
                            (vec (free-locals ast))
                            {:typed-locals-enabled? true})
          @*emitted*)))))

(defn- top-level-fn-class-name
  [form]
  (str "pnix.clj_meta.gen.Fn__" (compile-unit-id form) "__0"))

(defn- as-file
  [x]
  (if (instance? File x)
    x
    (File. (str x))))

(defn- ensure-output-directory!
  [dir]
  (let [^File f (.getCanonicalFile (as-file dir))]
    (when (and (.exists f) (not (.isDirectory f)))
      (throw (ex-info "compile-to-dir: output path is not a directory"
                      {:path (.getPath f)})))
    (when-not (.exists f)
      (when-not (.mkdirs f)
        (throw (ex-info "compile-to-dir: could not create output directory"
                        {:path (.getPath f)}))))
    f))

(defn- existing-directory!
  [dir]
  (let [^File f (.getCanonicalFile (as-file dir))]
    (when-not (and (.exists f) (.isDirectory f))
      (throw (ex-info "load-from-dir: directory does not exist"
                      {:path (.getPath f)})))
    f))

(defn- class-output-file
  [^File root cname]
  (File. root (str (.replace ^String (str cname) \. File/separatorChar) ".class")))

(defn- write-class-file!
  [^File root cname bytes]
  (let [^File file (class-output-file root cname)
        parent     (.getParentFile file)]
    (when parent
      (when (and (.exists parent) (not (.isDirectory parent)))
        (throw (ex-info "compile-to-dir: class parent path is not a directory"
                        {:path (.getPath parent)
                         :class cname})))
      (when-not (.exists parent)
        (when-not (.mkdirs parent)
          (throw (ex-info "compile-to-dir: could not create class parent directory"
                          {:path (.getPath parent)
                           :class cname})))))
    (with-open [out (FileOutputStream. file)]
      (.write out ^bytes bytes))
    file))

(defn- class-file?
  [^File f]
  (and (.isFile f)
       (.endsWith (.getName f) ".class")))

(defn- class-file->class-name
  [^File root ^File class-file]
  (let [rel       (str (.relativize (.toPath root)
                                    (.toPath (.getCanonicalFile class-file))))
        no-suffix (subs rel 0 (- (count rel) (count ".class")))]
    (-> no-suffix
        (.replace File/separatorChar \.)
        (.replace \/ \.)
        (.replace \\ \.))))

(defn- class-names-in-dir
  [^File root]
  (->> (file-seq root)
       (filter class-file?)
       (map #(class-file->class-name root %))
       sort
       vec))

(defn compile-to-dir
  "(fn …) form 을 `compile-classes` 로 컴파일하고 생성된 class bytes 를
  `out-dir/<internal-name>.class` 로 쓴다. 반환 artifact 는 `load-from-dir` 에 바로 넘길 수 있다.
  opts 는 `compile-classes` 와 같다."
  ([form out-dir] (compile-to-dir form out-dir {}))
  ([form out-dir opts]
   (let [classes    (compile-classes form opts)
         root       (ensure-output-directory! out-dir)
         main-class (with-compile-context opts #(top-level-fn-class-name form))
         rows       (sort-by key classes)
         files      (into {}
                          (map (fn [[cname bytes]]
                                 [cname (.getAbsolutePath
                                         ^File (write-class-file! root cname bytes))])
                               rows))]
     {:schema "pnix.clj-meta.compile-to-dir.v1"
      :directory (.getAbsolutePath root)
      :main-class main-class
      :class-count (count rows)
      :classes (mapv key rows)
      :files files
      :written true})))

(defn load-from-dir
  "compile-to-dir artifact 또는 `(out-dir, main-class)` 에서 class files 를 읽어
  DynamicClassLoader 에 정의하고 main IFn 인스턴스를 반환한다.
  opts/artifact `:classes` 가 있으면 그 목록만 로드하고, 없으면 디렉터리의 모든 `.class` 를 스캔한다."
  ([artifact]
   (if (map? artifact)
     (load-from-dir artifact {})
     (throw (ex-info "load-from-dir: artifact map or directory+main-class required"
                     {:value artifact}))))
  ([x y]
   (if (map? x)
     (load-from-dir (:directory x) (:main-class x) (merge x y))
     (load-from-dir x y {})))
  ([out-dir main-class opts]
   (let [root           (existing-directory! out-dir)
         main-class'    (str main-class)
         configured     (seq (:classes opts))
         class-names    (mapv str (or configured (class-names-in-dir root)))
         class-names'   (if (some #(= main-class' %) class-names)
                          class-names
                          (conj class-names main-class'))]
     (binding [*dcl* (compiler-session-loader)
               *unit-classes* (atom [])]
       (let [loaded (mapv (fn [cname]
                            (let [file  (class-output-file root cname)
                                  bytes (Files/readAllBytes (.toPath file))]
                              (remember-class!
                               (define-generated-class cname bytes))))
                          class-names')
             main   (or (some #(when (= main-class' (.getName ^Class %)) %) loaded)
                        (.loadClass ^DynamicClassLoader *dcl* main-class'))
             f      (try
                      (.newInstance ^Class main)
                      (catch Throwable t
                        (throw (ex-info "load-from-dir: could not instantiate main class"
                                        {:directory (.getAbsolutePath root)
                                         :main-class main-class'}
                                        t))))]
         (when-not (instance? IFn f)
           (throw (ex-info "load-from-dir: main class is not an IFn"
                           {:directory (.getAbsolutePath root)
                            :main-class main-class'
                            :class (.getName (class f))})))
         (keep-class-unit! @*unit-classes*)
         {:schema "pnix.clj-meta.load-from-dir.v1"
          :directory (.getAbsolutePath root)
          :main-class main-class'
          :class-count (count class-names')
          :classes class-names'
          :fn f
          :loaded true})))))

(defn warm-up!
  "첫 실제 요청 전에 analyzer/backend/DynamicClassLoader/JIT 경로를 예열한다.
  반환값은 관측용 receipt 이며, opts 는 `compile-form` 과 같다."
  ([] (warm-up! {}))
  ([opts]
   (let [f (compile-form '(fn [] 42) opts)]
     {:schema "pnix.clj-meta.warm-up.v1"
      :value (f)
      :warmed true})))

(defn with-checked-long-admission-mutant
  "Test-only hook: reintroduce the historical heterogeneous-stride fast-range
  bug and disable the independent unroll admission guard around f."
  [f]
  (binding [*disable-homogeneous-index-stride-gate* true
            *disable-independent-range-admission* true]
    (f)))

;; ---------------------------------------------------------------------------
;; smoke (Phase 2b 첫 조각 자기검증)
;; ---------------------------------------------------------------------------

(def ^:private smoke-cases
  "[설명 form 인자들 기대값]. form 을 우리 컴파일러로 IFn 으로 만들고 인자로 호출."
  [["상수 thunk"            '(fn [] 42)        []      42]
   ["항등(파라미터 통과)"   '(fn [n] n)        [7]     7]
   ["파라미터 무시 상수"    '(fn [n] 100)      [99]    100]
   ["2-arity 중 첫 인자"    '(fn [a b] a)      [11 22] 11]
   ["2-arity 중 둘째 인자"  '(fn [a b] b)      [11 22] 22]
   ["var 호출: 곱셈"        '(fn [n] (* n n))  [9]     81]
   ["var 호출: 덧셈"        '(fn [a b] (+ a b)) [20 22] 42]
   ["지역+상수 혼합"        '(fn [n] (+ n 1))  [41]    42]
   ["중첩 invoke"           '(fn [n] (* (+ n 1) 2)) [20] 42]
   ["if: then 분기"         '(fn [n] (if (< n 5) 1 0)) [3]  1]
   ["if: else 분기"         '(fn [n] (if (< n 5) 1 0)) [9]  0]
   ["case: int dispatch"    '(fn [n] (case n 1 :one 2 :two :other)) [2] :two]
   ["case: out-of-int number dispatch defaults" '(fn [] (case (long 9999999999) 1 :one 2 :two :other)) [] :other]
   ["case: keyword dispatch" '(fn [k] (case k
                                        :a 1 :b 2 :c 3 :d 4
                                        :e 5 :f 6 :g 7 :h 8
                                        0)) [:g] 7]
   ["case: hash collision Aa" '(fn [] (case "Aa" "Aa" 1 "BB" 2 :other)) [] 1]
   ["case: hash collision BB" '(fn [] (case "BB" "Aa" 1 "BB" 2 :other)) [] 2]
   ["let: 지역 슬롯"        '(fn [n] (let [a (* n 2)] (+ a 1))) [10] 21]
   ["let: 순차 바인딩"      '(fn [n] (let [a (+ n 1) b (* a 2)] b)) [20] 42]
   ["typed let: long slot"  '(fn [^long n] (let [a (+ n 1)
                                                  b (* a 2)]
                                              (+ b 3))) [19] 43]
   ["typed let: double slot" '(fn [^double x] (let [y (+ x 0.5)]
                                               (* y 2.0))) [20.5] 42.0]
   ["typed let: primitive static field" '(fn [] (let [^long x Long/MAX_VALUE] x))
    [] Long/MAX_VALUE]
   ["typed let: primitive instance field" '(fn [] (let [p (java.awt.Point. 7 9)
                                                        ^long x (.-x p)]
                                                    (+ x 1)))
    [] 8]
   ["unchecked long add raw opcode" '(fn [^long a ^long b] (unchecked-add a b))
    [Long/MAX_VALUE 1] Long/MIN_VALUE]
   ["unchecked long subtract raw opcode" '(fn [^long a ^long b] (unchecked-subtract a b))
    [Long/MIN_VALUE 1] Long/MAX_VALUE]
   ["unchecked long multiply raw opcode" '(fn [^long a ^long b] (unchecked-multiply a b))
    [Long/MAX_VALUE 2] -2]
   ["checked const long add raw opcode" '(fn [] (+ 20 22)) [] 42]
   ["checked const long subtract raw opcode" '(fn [] (- 50 8)) [] 42]
   ["checked const long multiply raw opcode" '(fn [] (* 6 7)) [] 42]
   ["checked overflow long add stays checked" '(fn [] (let [a Long/MAX_VALUE
                                                            b 1]
                                                        (+ a b))) [] [:throw "long overflow"]]
   ["checked overflow long subtract stays checked" '(fn [] (- Long/MIN_VALUE 1)) [] [:throw "long overflow"]]
   ["checked overflow long multiply stays checked" '(fn [] (* Long/MIN_VALUE -1)) [] [:throw "long overflow"]]
   ["checked let-local long add raw opcode" '(fn [] (let [a 20 b 22] (+ a b))) [] 42]
   ["checked let-local chained long raw opcode" '(fn [] (let [a 20
                                                               b (+ a 1)]
                                                           (+ b 21))) [] 42]
   ["checked let-local multiply raw opcode" '(fn [] (let [a 6 b 7] (* a b))) [] 42]
   ["checked loop-invariant long add raw opcode" '(fn [] (loop [a 20 b 22] (+ a b))) [] 42]
   ["checked loop-invariant long recur raw opcode" '(fn [] (loop [a 20 b 22]
                                                              (if true
                                                                (+ a b)
                                                                (recur a b)))) [] 42]
   ["checked loop-invariant long multiply raw opcode" '(fn [] (loop [a 6 b 7] (* a b))) [] 42]
   ["checked bounded loop-step long raw opcode" '(fn [] (loop [i 0]
                                                          (if (< i 10)
                                                            (recur (+ i 1))
                                                            [(+ i 20)
                                                             (- 30 i)
                                                             (* i 2)]))) [] [30 20 20]]
   ["checked non-unit loop-stride long raw opcode" '(fn [] (loop [i 0]
                                                             (if (< i 10)
                                                               (recur (+ i 3))
                                                               [(+ i 20)
                                                                (- 40 i)
                                                                (* i 2)]))) [] [32 28 24]]
   ["checked decreasing loop-stride long raw opcode" '(fn [] (loop [i 10]
                                                               (if (> i 0)
                                                                 (recur (- i 3))
                                                                 [(+ i 20)
                                                                  (- 40 i)
                                                                  (* i 2)]))) [] [18 42 -4]]
   ["checked accumulator loop range raw opcode" '(fn [] (loop [i 0 acc 0]
                                                         (if (< i 10)
                                                           (recur (+ i 1) (+ acc 2))
                                                           [(+ acc 20)
                                                            (- 50 acc)
                                                            (* acc 2)]))) [] [40 30 40]]
   ["checked fn guard long range raw opcode" '(fn [^long x]
                                                (if (> x -10)
                                                  (if (< x 10)
                                                    [(+ x 1)
                                                     (- 10 x)
                                                     (* x 2)]
                                                    0)
                                                  0)) [7] [8 3 14]]
   ["checked index accumulator loop range raw opcode" '(fn [] (loop [i 0 acc 0]
                                                                (if (< i 10)
                                                                  (recur (+ i 1) (+ acc i))
                                                                  [(+ acc 1)
                                                                   (- 50 acc)
                                                                   (* acc 2)]))) [] [46 5 90]]
   ["checked multiplicative accumulator loop range raw opcode" '(fn [] (loop [i 0 acc 1]
                                                                         (if (< i 5)
                                                                           (recur (+ i 1) (* acc 2))
                                                                           [(+ acc 1)
                                                                            (- 100 acc)
                                                                            (* acc 2)]))) [] [33 68 64]]
   ;; M9b: branch-dependent stride — recognizer 가 (recur (if …)) 를 못 잡고
   ;; abstract-interpretation 엔진이 i∈[0,11] 을 sound 하게 도출 → checked long raw opcode.
   ["M9b branch-dependent loop-stride long raw opcode" '(fn [^long c]
                                                          (loop [i 0]
                                                            (if (< i 10)
                                                              (recur (if (even? c) (+ i 1) (+ i 2)))
                                                              [(+ i 20) (- 40 i) (* i 2)]))) [3] [30 30 20]]
   ;; M9b soundness sentinel: engine range [0,11] 은 sound 라 overflow 하는 exit add 는
   ;; 승격 안 됨 → Numbers fallback 이 overflow 시 throw. 만약 range 가 unsound(너무 narrow)
   ;; 라 ladd 로 잘못 승격되면 wraparound(숫자) 가 되어 이 테스트가 깨진다.
   ["M9b branch-dependent overflow exit falls back" '(fn [^long c]
                                                       (loop [i 0]
                                                         (if (< i 10)
                                                           (recur (if (even? c) (+ i 1) (+ i 2)))
                                                           (+ i Long/MAX_VALUE)))) [3] [:throw "long overflow"]]
   ;; M9b 관계 도메인: acc 는 interval 만으로 [neg-inf,0] 이지만 보존 선형량 acc+i=0 으로
   ;; acc∈[-10,0] 도출 → (* acc 2) checked long raw opcode.
   ["M9b mixed-sign conserved-quantity long raw opcode" '(fn [] (loop [i 0 acc 0]
                                                                  (if (< i 10)
                                                                    (recur (+ i 1) (- acc 1))
                                                                    (* acc 2)))) [] -20]
   ;; soundness sentinel: 보존량이 sound 라 큰 acc 에서 overflow 하는 산술은 fallback→throw.
   ["M9b mixed-sign overflow falls back" '(fn [] (loop [i 0 acc 4000000000]
                                                   (if (< i 10)
                                                     (recur (+ i 1) (- acc 1))
                                                     (* acc acc)))) [] [:throw "long overflow"]]
   ;; M9b 기하 가속: acc'=(* acc -2) 는 nonlinear(부호 교번+지수)이라 interval/conserved 로
   ;; 못 잡지만, bounded index N=5 로 |1|*2^5=32 magnitude 도출 → acc∈[-32,32] → (+ acc 100) raw.
   ["M9b negative-factor geometric long raw opcode" '(fn [] (loop [i 0 acc 1]
                                                              (if (< i 5)
                                                                (recur (+ i 1) (* acc -2))
                                                                (+ acc 100)))) [] 68]
   ;; soundness sentinel: 기하 bound 가 sound 라 큰 acc(2^40)의 (* acc acc)=2^80 은 fallback→throw.
   ["M9b negative-factor overflow falls back" '(fn [] (loop [i 0 acc 1]
                                                        (if (< i 40)
                                                          (recur (+ i 1) (* acc 2))
                                                          (* acc acc)))) [] [:throw "long overflow"]]
   ;; M9b 유계 구간 언롤링: acc'=(* acc acc) 는 non-constant-factor nonlinear(이중지수)지만,
   ;; N=4 unroll 로 acc∈[2,65536] 도출 → (* acc acc) raw lmul + (+ acc 1) raw ladd.
   ["M9b nonlinear unroll long raw opcode" '(fn [] (loop [i 0 acc 2]
                                                     (if (< i 4)
                                                       (recur (+ i 1) (* acc acc))
                                                       (+ acc 1)))) [] 65537]
   ;; soundness sentinel: N=6 이면 acc 가 2^64 로 overflow → unroll 이 top → fallback→throw.
   ["M9b nonlinear unroll overflow falls back" '(fn [] (loop [i 0 acc 2]
                                                         (if (< i 6)
                                                           (recur (+ i 1) (* acc acc))
                                                           acc))) [] [:throw "long overflow"]]
   ["U9 unknown-iteration nonlinear overflow stays checked" '(fn [^long n]
                                                               (loop [i 0 acc 2]
                                                                 (if (< i n)
                                                                   (recur (+ i 1) (* acc acc))
                                                                   acc))) [6] [:throw "long overflow"]]
   ["letfn 상호재귀 direct"  '(fn [n]
                               (letfn [(ev? [x] (if (zero? x) true (od? (dec x))))
                                       (od? [x] (if (zero? x) false (ev? (dec x))))]
                                 (ev? n))) [4] true]
   ["reify Object/toString direct" '(fn [] (str (reify Object
                                                  (toString [_] "rx")))) [] "rx"]
   ["reify Callable/call direct" '(fn [] (.call (reify java.util.concurrent.Callable
                                                  (call [_] "ok")))) [] "ok"]
   ["reify Callable capture direct" '(fn [x] (.call (reify java.util.concurrent.Callable
                                                     (call [_] x)))) ["cap"] "cap"]
   ;; general reify: host 처럼 IObj 자동 구현 → with-meta 로 metadata 부여, withMeta 가 복사본 생성.
   ["reify with-meta metadata direct" '(fn [] (meta (with-meta (reify Object (toString [_] "x")) {:k 9}))) [] {:k 9}]
   ["reify default no-metadata direct" '(fn [] (meta (reify Object (toString [_] "x")))) [] nil]
   ["reify withMeta preserves method direct" '(fn [] (str (with-meta (reify Object (toString [_] "rx")) {:k 1}))) [] "rx"]
   ["reify is IObj direct" '(fn [] (instance? clojure.lang.IObj (reify Object (toString [_] "x")))) [] true]
   ;; deftype/defrecord 직접 emit: named 클래스를 우리 backend 가 emit(host Compiler 0회).
   ;; analyzer 가 만든 stub 은 sibling loader 라 우리 full 클래스가 *dcl* 에서 shadow 한다.
   ["deftype toString direct"   '(fn [] (do (deftype DtA [x y] Object (toString [_] (str x "|" y))) (str (->DtA 3 7)))) [] "3|7"]
   ["deftype field access direct" '(fn [] (do (deftype DtB [x y] Object (toString [_] "")) (.x (->DtB 5 9)))) [] 5]
   ["deftype interface direct"  '(fn [] (do (deftype DtC [v] java.util.concurrent.Callable (call [_] (str "c" v))) (.call (->DtC 7)))) [] "c7"]
   ["deftype primitive-return direct" '(fn [] (do (deftype DtD [a b] Object (hashCode [_] (+ a b)) (toString [_] "")) (.hashCode (->DtD 3 4)))) [] 7]
   ["deftype instance? direct"  '(fn [] (do (deftype DtE [n] Object (toString [_] "")) (instance? DtE (->DtE 1)))) [] true]
   ;; mutable 필드 + set! :local 직접 emit
   ["deftype mutable set! direct" '(fn [] (do (deftype DtF [^:unsynchronized-mutable v] Object (toString [_] (do (set! v 99) (str v)))) (str (->DtF 1)))) [] "99"]
   ["deftype volatile field flag direct" '(fn [] (do (deftype DtVol [^:volatile-mutable v] Object (toString [_] ""))
                                                     (java.lang.reflect.Modifier/isVolatile
                                                      (.getModifiers (.getDeclaredField (class (->DtVol 1)) "v"))))) [] true]
   ;; defrecord 직접 emit: typed/mutable(__hash:int) 필드 + record multi-constructor + set! :local.
   ["defrecord direct"          '(fn [] (do (defrecord RecA [a b]) [(:a (->RecA 1 2)) (= (->RecA 1 2) (->RecA 1 2)) (:c (assoc (->RecA 1 2) :c 3))])) [] [1 true 3]]
   ["defrecord map factory direct" '(fn [] (do (defrecord RecMapA [a b])
                                               [(:a (map->RecMapA {:a 5 :b 6}))
                                                (:c (map->RecMapA {:a 1 :b 2 :c 9}))
                                                (= (map->RecMapA {:a 5 :b 6}) (->RecMapA 5 6))])) [] [5 9 true]]
   ["defrecord keyword/hash semantics direct" '(fn [] (do (defrecord RecSemA [a b])
                                                          (let [r (->RecSemA 1 2)
                                                                r2 (->RecSemA 1 2)
                                                                d (dissoc r :a)
                                                                t (.getLookupThunk r :a)]
                                                            [(.get t r)
                                                             (record? d)
                                                             (:b d)
                                                             (contains? #{r} r2)
                                                             (= (hash r) (hash r2))
                                                             (= (clojure.lang.Util/hasheq r)
                                                                (clojure.lang.Util/hasheq r2))]))) [] [1 false 2 true true true]]
   ;; code-position 컬렉션 리터럴의 quoted 값: host 처럼 quote 한 겹 평가({:k 'sym}→{:k sym}).
   ["quoted value in map literal" '(fn [] {:on 'foo :v ['a 'b] :n 1}) [] '{:on foo :v [a b] :n 1}]
   ["import* direct"        '(fn [] (.getName (clojure.core/import* "java.util.zip.CRC32"))) [] "java.util.zip.CRC32"]
   ["const 벡터"            '(fn [] [1 2 3])    []      [1 2 3]]
   ["동적 벡터(지역)"       '(fn [k] [k (* k 2)]) [21]  [21 42]]
   ["keyword 상수"          '(fn [] :hello)     []      :hello]
   ["quoted symbol"         '(fn [] 'sym)       []      'sym]
   ["char 상수"             '(fn [] \.)         []      \.]
   ["BigInt literal class"   '(fn [] [(class 5N) 5N]) [] [clojure.lang.BigInt 5N]]
   ["BigInt out-of-long literal" '(fn [] [(class 10000000000000000000N)
                                          10000000000000000000N]) [] [clojure.lang.BigInt 10000000000000000000N]]
   ["BigDecimal literal direct" '(fn [] [(class 1.5M) 1.5M]) [] [java.math.BigDecimal 1.5M]]
   ["Ratio literal direct" '(fn [] [(class 1/3) 1/3]) [] [clojure.lang.Ratio 1/3]]
   ["regex literal direct" '(fn [] [(class #"a+") (boolean (re-find #"a+" "aa"))]) [] [java.util.regex.Pattern true]]
   ["quoted list"           '(fn [] '(1 2 sym)) []      '(1 2 sym)]
   ["동적 맵"               '(fn [k] {:a k :b 2}) [9]   {:a 9 :b 2}]
   ["keyword invoke"        '(fn [m] (:a m)) [{:a 42}] 42]
   ["const 집합"            '(fn [] #{1 2 3})   []      #{1 2 3}]
   ["중첩 const 컬렉션"     '(fn [] {:x [1 2] :y :z}) [] {:x [1 2] :y :z}]
   ["loop/recur factorial"  '(fn [n] (loop [i n acc 1] (if (< i 2) acc (recur (- i 1) (* acc i))))) [5] 120]
   ["typed loop/recur: long slots" '(fn [^long n]
                                      (loop [i n acc 0]
                                        (if (< i 1)
                                          acc
                                          (recur (- i 1) (+ acc i)))))
    [10] 55]
   ["typed loop/recur: mixed long/double slots" '(fn [^double x]
                                                  (loop [i 0 acc x]
                                                    (if (< i 3)
                                                      (recur (+ i 1) (+ acc 0.5))
                                                      acc)))
    [40.5] 42.0]
   ["fn-level recur 합"     '(fn [n acc] (if (< n 1) acc (recur (- n 1) (+ acc n)))) [5 0] 15]
   ["throw"                 '(fn [] (throw (ex-info "boom" {}))) []  [:throw "boom"]]
   ["try/catch 정상"        '(fn [x] (try (quot 10 x) (catch ArithmeticException e :divzero))) [2] 5]
   ["try/catch 예외"        '(fn [x] (try (quot 10 x) (catch ArithmeticException e :divzero))) [0] :divzero]
   ["try 다중 catch"        '(fn [] (try (throw (ex-info "b" {})) (catch ArithmeticException e :a) (catch Exception e :caught))) [] :caught]
   ["try/finally 정상"      '(fn [] (let [a (java.util.concurrent.atomic.AtomicInteger. 0)]
                                     (try 10 (finally (.incrementAndGet a)))
                                     (.get a))) [] 1]
   ["try/finally 예외"      '(fn [] (let [a (java.util.concurrent.atomic.AtomicInteger. 0)]
                                     (try
                                       (try (throw (ex-info "b" {}))
                                         (finally (.incrementAndGet a)))
                                       (catch Exception e (.get a))))) [] 1]
   ["try/catch/finally"     '(fn [] (let [a (java.util.concurrent.atomic.AtomicInteger. 0)]
                                     [(try (quot 10 0)
                                        (catch ArithmeticException e :divzero)
                                        (finally (.incrementAndGet a)))
                                      (.get a)])) [] [:divzero 1]]
   ["interop: .length"      '(fn [^String s] (.length s)) ["hello"] 5]
   ["interop: .toUpperCase" '(fn [^String s] (.toUpperCase s)) ["hi"] "HI"]
   ["interop: static-call"  '(fn [] (Math/sqrt 16.0)) []  4.0]
   ["interop: static-field" '(fn [] Long/MAX_VALUE) []  9223372036854775807]
   ["interop: new + .size"  '(fn [] (.size (java.util.ArrayList.))) []  0]
   ["interop: new(arg)"     '(fn [^String s] (.toString (StringBuilder. s))) ["xy"] "xy"]
   ["host-interop: .length" '(fn [x] (.length x)) ["hello"] 5]
   ["host-interop: field read" '(fn [p] (.-x p)) [(java.awt.Point. 7 9)] 7]
   ["host-interop: field set!" '(fn [p] (set! (.-x p) 8) (.-x p)) [(java.awt.Point.)] 8]
   ["set!: field eval order" '(fn [] (let [seen (atom []) p (java.awt.Point.)]
                                       (set! (. (do (swap! seen conj :target) p) x)
                                             (do (swap! seen conj :val) 8))
                                       @seen)) [] [:target :val]]
   ["protocol-invoke: coll-reduce" '(fn [xs] (clojure.core.protocols/coll-reduce xs +)) [[1 2 3]] 6]
   ["protocol-invoke: coll-reduce init" '(fn [xs] (clojure.core.protocols/coll-reduce xs + 10)) [[1 2 3]] 16]
   ["interop overload: static char[]" '(fn [] (String/valueOf (.toCharArray "ab"))) [] "ab"]
   ["interop overload: append char[]" '(fn [] (let [sb (StringBuilder.)]
                                                (.append sb (.toCharArray "ab"))
                                                (.toString sb))) [] "ab"]
   ["interop overload: int arg" '(fn [] (Integer/toString 42)) [] "42"]
   ["interop overload: int ctor" '(fn [] (.size (java.util.ArrayList. 4))) [] 0]
   ["instance?: true"       '(fn [x] (instance? String x)) ["x"] true]
   ["instance?: false"      '(fn [x] (instance? String x)) [42] false]
   ["set!: instance field"  '(fn [] (let [p (java.awt.Point.)] (set! (. p x) 7) (. p x))) [] 7]
   ["binding + set!"        '(fn [] (binding [*print-length* 1]
                                     (set! *print-length* 7)
                                     *print-length*)) [] 7]
   ["def dynamic binding direct" '(fn [] (do (def ^:dynamic *compiler-dyn-smoke* 1)
                                             [(.isDynamic (var *compiler-dyn-smoke*))
                                              (binding [*compiler-dyn-smoke* 2]
                                                *compiler-dyn-smoke*)])) [] [true 2]]
   ["locking/monitor"       '(fn [] (let [sb (StringBuilder.)]
                                     (locking sb (.append sb "x"))
                                     (.toString sb))) [] "x"]
   ["the-var #'+"           '(fn [] (var +)) []  #'clojure.core/+]
   ["클로저: arg 캡처"      '(fn [x] ((fn [y] (+ x y)) 5)) [10] 15]
   ["클로저: let 캡처"      '(fn [n] (let [m (* n 2)] ((fn [] m)))) [21] 42]
   ["클로저: 2개 캡처"      '(fn [a b] ((fn [] (+ a b)))) [20 22] 42]
   ["클로저 ⨉ host map"     '(fn [x] (map (fn [y] (+ x y)) [1 2 3])) [10] '(11 12 13)]
   ["def + use(body)"       '(fn [] (do (def k-sq (fn [n] (* n n))) (k-sq 9))) [] 81]
   ["def 값(body)"          '(fn [] (do (def k-v (+ 40 2)) k-v)) [] 42]
   ["variadic: rest"        '(fn [a & r] r) [1 2 3] '(2 3)]
   ["variadic: 고정"        '(fn [a & r] a) [1 2 3] 1]
   ["variadic: 전부 rest"   '(fn [& r] r) [1 2 3] '(1 2 3)]
   ["variadic: count rest"  '(fn [a & r] (count r)) [1 2 3] 2]
   ["variadic ⨉ apply"      '(fn [f] (apply f 1 [2 3])) [(fn [a & r] (cons a r))] '(1 2 3)]
   ["다중arity: 0"          '(fn ([] 0) ([x] x) ([x y] (+ x y))) []      0]
   ["다중arity: 1"          '(fn ([] 0) ([x] x) ([x y] (+ x y))) [5]     5]
   ["다중arity: 2"          '(fn ([] 0) ([x] x) ([x y] (+ x y))) [20 22] 42]
   ["다중arity ⨉ 캡처(0)"   '(fn [a] ((fn ([] a) ([x] (+ a x))))) [10]   10]
   ["다중arity ⨉ 캡처(1)"   '(fn [a] ((fn ([] a) ([x] (+ a x))) 5)) [10] 15]
   ["혼합 variadic: fixed"  '(fn ([x] [:fixed x]) ([x & r] [:rest x r])) [7] [:fixed 7]]
   ["혼합 variadic: rest"   '(fn ([x] [:fixed x]) ([x & r] [:rest x r])) [7 8 9] [:rest 7 '(8 9)]]
   ["혼합 variadic ⨉ 캡처" '(fn [a] ((fn ([x] (+ a x)) ([x & r] [a x r])) 1 2 3)) [10] [10 1 '(2 3)]]])

(defn- generated-frame
  [^Throwable t]
  (first (filter (fn [^StackTraceElement ste]
                   (.startsWith (.getClassName ste) "pnix.clj_meta.gen.Fn__"))
                 (.getStackTrace t))))

(defn- generated-instance?
  [x]
  (.startsWith (.getName (class x)) "pnix.clj_meta.gen."))

(defn- line-metadata-smoke
  []
  (let [source "line_smoke.clj"
        form   (read-source-form "(fn []\n  (let [x 1]\n    (throw (Exception. \"line-boom\"))))")
        want   {:source source :line 3}
        got    (try
                 (binding [*file* source]
                   ((compile-form form)))
                 :no-throw
                 (catch Throwable t
                   (if-let [^StackTraceElement ste (generated-frame t)]
                     {:source (.getFileName ste) :line (.getLineNumber ste)}
                     :no-generated-frame)))]
    {:desc "source line metadata"
     :want want
     :got got
     :ok (= want got)}))

(defn- compile-form-top-level-smoke
  []
  (clear-compile-form-fallback-diagnostics!)
  (let [f    (compile-form '(+ 1 2))
        got  {:generated? (generated-instance? f)
              :value (f)
              :diagnostics @compile-form-fallback-diagnostics}
        want {:generated? true
              :value 3
              :diagnostics []}]
    {:desc "compile-form non-fn top-level backend wrapper"
     :want want
     :got got
     :ok (= want got)}))

(defn- compile-form-diagnostics-smoke
  []
  (clear-compile-form-fallback-diagnostics!)
  (try
    (compile-form '(fn [] x))
    (catch Throwable _ nil))
  (let [row  (first @compile-form-fallback-diagnostics)
        got  (select-keys row [:phase :reason])
        want {:phase :root :reason :analyze-failed}]
    (clear-compile-form-fallback-diagnostics!)
    {:desc "compile-form fallback diagnostics"
     :want want
     :got got
     :ok (= want got)}))

(defn- compile-form-star-smoke
  []
  (clear-compile-form-fallback-diagnostics!)
  (let [direct        (compile-form* '(+ 1 2))
        fallback-form (read-string "(fn [] #uuid \"00000000-0000-0000-0000-000000000000\")")
        fallback      (compile-form* fallback-form)
        got           {:direct-mode (:mode direct)
                       :direct-value ((:fn direct))
                       :direct-diagnostics (:diagnostics direct)
                       :fallback-mode (:mode fallback)
                       :fallback-value (str ((:fn fallback)))
                       :fallback-diagnostic (select-keys (first (:diagnostics fallback))
                                                         [:phase :reason])}
        want          {:direct-mode :direct
                       :direct-value 3
                       :direct-diagnostics []
                       :fallback-mode :host-fallback
                       :fallback-value "00000000-0000-0000-0000-000000000000"
                       :fallback-diagnostic {:phase :emit :reason :unsupported-op}}]
    (clear-compile-form-fallback-diagnostics!)
    {:desc "compile-form* result diagnostics"
     :want want
     :got got
     :ok (= want got)}))

(defn- compile-form-strict-no-fallback-smoke
  []
  (clear-compile-form-fallback-diagnostics!)
  (let [uuid-text     "00000000-0000-0000-0000-000000000000"
        fallback-form (read-string (str "(fn [] #uuid \"" uuid-text "\")"))
        strict-error  (try
                        (compile-form-strict fallback-form)
                        nil
                        (catch Throwable t t))
        strict-diags  @compile-form-fallback-diagnostics
        fallback      (compile-form* fallback-form)
        got           {:strict-threw? (some? strict-error)
                       :strict-type (:type (ex-data strict-error))
                       :strict-global-diagnostics strict-diags
                       :fallback-mode (:mode fallback)
                       :fallback-value (str ((:fn fallback)))
                       :fallback-diagnostic (select-keys (first (:diagnostics fallback))
                                                         [:phase :reason])}
        want          {:strict-threw? true
                       :strict-type :unsupported-op
                       :strict-global-diagnostics []
                       :fallback-mode :host-fallback
                       :fallback-value uuid-text
                       :fallback-diagnostic {:phase :emit :reason :unsupported-op}}]
    (clear-compile-form-fallback-diagnostics!)
    {:desc "compile-form-strict no fallback contract"
     :want want
     :got got
     :ok (= want got)}))

(defn- compile-form-diagnostics-cap-smoke
  []
  (clear-compile-form-fallback-diagnostics!)
  (binding [*fallback-diagnostics-cap* 2]
    (dotimes [_ 3]
      (try
        (compile-form '(fn [] x))
        (catch Throwable _ nil))))
  (let [rows @compile-form-fallback-diagnostics
        got  {:count (count rows)
              :indexes (mapv :index rows)}
        want {:count 2
              :indexes [1 2]}]
    (clear-compile-form-fallback-diagnostics!)
    {:desc "compile-form fallback diagnostics cap"
     :want want
     :got got
     :ok (= want got)}))

(defn- compile-form-error-envelope-smoke
  []
  (let [summarize (fn [form]
                    (try
                      (compile-form form)
                      :no-throw
                      (catch Throwable t
                        (let [data (ex-data t)]
                          {:type (:type data)
                           :phase (:phase data)
                           :form (:form data)
                           :cause-class (.getName (class (:cause data)))}))))
        bad-catch '(fn [] (try 1 (catch Object o 2)))
        unknown   '(fn [] missing-symbol)
        got       {:bad-catch (summarize bad-catch)
                   :unknown (summarize unknown)
                   :normal ((compile-form '(fn [] 42)))}
        want      {:bad-catch {:type :compile-error
                               :phase :emit
                               :form (pr-str bad-catch)
                               :cause-class "java.lang.VerifyError"}
                   :unknown {:type :compile-error
                             :phase :analyze
                             :form (pr-str unknown)
                             :cause-class "clojure.lang.Compiler$CompilerException"}
                   :normal 42}]
    {:desc "compile-form error envelope"
     :want want
     :got got
     :ok (= want got)}))

(defn- compile-form-class-name-smoke
  []
  (let [form-a     '(fn [] 41)
        form-b     '(fn [] 42)
        reify-form '(fn [] (.getName (class (reify Object (toString [_] "rx")))))
        fa         (compile-form form-a)
        fa2        (compile-form form-a)
        fb         (compile-form form-b)
        fr         (compile-form reify-form)
        unit-a     (compile-unit-id form-a)
        unit-b     (compile-unit-id form-b)
        unit-r     (compile-unit-id reify-form)
        got        {:same-source-name (= (.getName (class fa))
                                         (.getName (class fa2)))
                    :different-source-name (not= (.getName (class fa))
                                                 (.getName (class fb)))
                    :fn-a-name (.getName (class fa))
                    :fn-b-name (.getName (class fb))
                    :reify-name (fr)
                    :values [(fa) (fa2) (fb)]}
        want       {:same-source-name true
                    :different-source-name true
                    :fn-a-name (str "pnix.clj_meta.gen.Fn__" unit-a "__0")
                    :fn-b-name (str "pnix.clj_meta.gen.Fn__" unit-b "__0")
                    :reify-name (str "pnix.clj_meta.gen.Reify__" unit-r "__1")
                    :values [41 41 42]}]
    {:desc "compile-form deterministic source-hash class names"
     :want want
     :got got
     :ok (= want got)}))

(defn- compile-ns-smoke
  []
  (let [ns-sym 'pnix.clj-meta.compile-ns-smoke
        _      (remove-ns ns-sym)
        caller-ns (ns-name *ns*)
        src    "(ns pnix.clj-meta.compile-ns-smoke)\n(def base 40)\n(defn inc2 [x] (+ x 2))\n(def answer (inc2 base))\nanswer"
        artifact (compile-ns src {:file "compile_ns_smoke.clj"})
        loaded   (load-compiled-ns artifact)
        got      {:namespace (:namespace loaded)
                  :ns-form-mode (:ns-form-mode artifact)
                  :loadable (:loadable artifact)
                  :body-count (:body-count artifact)
                  :last-result (last (:results loaded))
                  :answer @(ns-resolve ns-sym 'answer)
                  :caller-ns (ns-name *ns*)}
        want     {:namespace ns-sym
                  :ns-form-mode :direct-simple
                  :loadable true
                  :body-count 4
                  :last-result 42
                  :answer 42
                  :caller-ns caller-ns}]
    {:desc "compile-ns loadable artifact"
     :want want
     :got got
     :ok (= want got)}))

(defn- compile-ns-require-import-smoke
  "require/import clause 가 있는 ns form 도 host Compiler 없이 backend 로 컴파일·실행됨을
  확인한다(:direct-compiled). import 가 실제로 등록되고 require 한 별칭이 동작해야 한다."
  []
  (let [ns-sym 'pnix.clj-meta.compile-ns-ri-smoke
        _      (remove-ns ns-sym)
        caller-ns (ns-name *ns*)
        src    (str "(ns pnix.clj-meta.compile-ns-ri-smoke\n"
                    "  (:require [clojure.string :as s])\n"
                    "  (:import [java.util ArrayList]))\n"
                    "(def joined (s/join \",\" [\"a\" \"b\"]))\n"
                    "(def cls-name (.getName ArrayList))\n"
                    "[joined cls-name]")
        artifact (compile-ns src {:file "compile_ns_ri_smoke.clj"})
        loaded   (load-compiled-ns artifact)
        got      {:ns-form-mode (:ns-form-mode artifact)
                  :last-result (last (:results loaded))
                  :caller-ns (ns-name *ns*)}
        want     {:ns-form-mode :direct-compiled
                  :last-result ["a,b" "java.util.ArrayList"]
                  :caller-ns caller-ns}]
    (remove-ns ns-sym)
    {:desc "compile-ns require/import (direct-compiled)"
     :want want
     :got got
     :ok (= want got)}))

(defn- compile-ns-isolate-smoke
  []
  (let [source-ns 'pnix.clj-meta.compile-ns-isolate-smoke
        default-ns 'pnix.clj-meta.compile-ns-unload-smoke
        _ (do (unload-ns! source-ns)
              (unload-ns! default-ns))
        got (try
              (let [src (str "(ns pnix.clj-meta.compile-ns-isolate-smoke\n"
                             "  (:require [clojure.string :as s]))\n"
                             "(def joined (s/join \",\" [\"i\" \"so\"]))\n"
                             "joined")
                    artifact (compile-ns src {:file "compile_ns_isolate_smoke.clj"
                                              :isolate true})
                    isolated-ns (:namespace artifact)
                    compile-left-ns? (boolean (find-ns isolated-ns))
                    source-left-ns-after-compile? (boolean (find-ns source-ns))
                    loaded (load-compiled-ns artifact)
                    load-left-ns? (boolean (find-ns isolated-ns))
                    default-loaded (compile-and-load-ns
                                    "(ns pnix.clj-meta.compile-ns-unload-smoke)\n(def x 42)\nx"
                                    {:file "compile_ns_unload_smoke.clj"})
                    default-value @(ns-resolve default-ns 'x)
                    unload-removed? (unload-ns! default-ns)
                    default-left-ns? (boolean (find-ns default-ns))]
                {:source-namespace (:source-namespace artifact)
                 :isolated-differs? (not= source-ns isolated-ns)
                 :isolate (:isolate artifact)
                 :ns-form-mode (:ns-form-mode artifact)
                 :compile-left-ns? compile-left-ns?
                 :source-left-ns-after-compile? source-left-ns-after-compile?
                 :last-result (last (:results loaded))
                 :loaded-unloaded (:unloaded loaded)
                 :load-left-ns? load-left-ns?
                 :default-last-result (last (:results default-loaded))
                 :default-value default-value
                 :unload-removed? unload-removed?
                 :default-left-ns? default-left-ns?})
              (finally
                (unload-ns! source-ns)
                (unload-ns! default-ns)))
        want {:source-namespace source-ns
              :isolated-differs? true
              :isolate true
              :ns-form-mode :direct-compiled
              :compile-left-ns? false
              :source-left-ns-after-compile? false
              :last-result "i,so"
              :loaded-unloaded true
              :load-left-ns? false
              :default-last-result 42
              :default-value 42
              :unload-removed? true
              :default-left-ns? false}]
    {:desc "compile-ns isolate and unload-ns!"
     :want want
     :got got
     :ok (= want got)}))

(defn- compile-form-ns-options-smoke
  []
  (let [ns-a 'pnix.clj-meta.compile-form-ns-a-smoke
        ns-b 'pnix.clj-meta.compile-form-ns-b-smoke
        normalize-class-bytes (fn [classes]
                                (into {}
                                      (map (fn [[cname bytes]]
                                             [cname (vec bytes)]))
                                      classes))
        got  (try
               (let [target-a (ensure-compile-namespace! ns-a)
                     target-b (ensure-compile-namespace! ns-b)
                     _        (do (intern target-a 'x 41)
                                  (intern target-b 'x 100))
                     form     (list 'fn [] (list '+ 'x 1))
                     f-a      (compile-form form {:ns ns-a})
                     f-b      (compile-form form {:ns target-b})
                     name-a   (.getName (class f-a))
                     name-a-from-b-caller
                     (binding [*ns* target-b]
                       (.getName (class (compile-form form {:ns ns-a}))))
                     star     (compile-form* (list '+ 'x 2) {:ns (str ns-a)})
                     eval-v   (eval-string "(def y (+ x 1)) y" {:ns ns-a})
                     classes-a (compile-classes form {:ns ns-a})
                     classes-a-from-b-caller
                     (binding [*ns* target-b]
                       (compile-classes form {:ns ns-a}))]
                 {:value-a (f-a)
                  :value-b (f-b)
                  :same-name-from-opts-ns? (= name-a name-a-from-b-caller)
                  :star-mode (:mode star)
                  :star-value ((:fn star))
                  :star-diagnostics (:diagnostics star)
                  :eval-string-value eval-v
                  :eval-string-var @(ns-resolve ns-a 'y)
                  :compile-classes-same-names?
                  (= (sort (keys classes-a))
                     (sort (keys classes-a-from-b-caller)))
                  :compile-classes-same-bytes?
                  (= (normalize-class-bytes classes-a)
                     (normalize-class-bytes classes-a-from-b-caller))})
               (finally
                 (unload-ns! ns-a)
                 (unload-ns! ns-b)))
        want {:value-a 42
              :value-b 101
              :same-name-from-opts-ns? true
              :star-mode :direct
              :star-value 43
              :star-diagnostics []
              :eval-string-value 42
              :eval-string-var 42
              :compile-classes-same-names? true
              :compile-classes-same-bytes? true}]
    {:desc "compile entrypoints :ns opts"
     :want want
     :got got
     :ok (= want got)}))

(defn- compile-concurrency-smoke
  []
  (let [thread-count 8
        iterations 6
        uuid-text "00000000-0000-0000-0000-000000000000"
        fallback-form (read-string (str "(fn [] #uuid \"" uuid-text "\")"))
        ns-syms (mapv #(symbol (str "pnix.clj-meta.compile-concurrency-smoke-" %))
                      (range thread-count))
        got (try
              (doseq [[idx ns-sym] (map-indexed vector ns-syms)
                      :let [target (ensure-compile-namespace! ns-sym)]]
                (intern target 'x idx))
              (let [worker (fn [idx ns-sym]
                             (try
                               (mapv
                                (fn [j]
                                  (let [target (ensure-compile-namespace! ns-sym)
                                        form (list 'fn [] (list '+ 'x j))
                                        expected (+ idx j)
                                        host (binding [*ns* target]
                                               ((eval form)))
                                        compiled ((compile-form form {:ns ns-sym}))
                                        evaled (eval-string (str "(+ x " j ")") {:ns ns-sym})
                                        fallback (compile-form* fallback-form {:ns ns-sym})
                                        fallback-diags
                                        (mapv #(select-keys % [:phase :reason])
                                              (:diagnostics fallback))
                                        fallback-value (str ((:fn fallback)))]
                                    {:ok (and (= expected host compiled evaled)
                                              (= :host-fallback (:mode fallback))
                                              (= [{:phase :emit
                                                   :reason :unsupported-op}]
                                                 fallback-diags)
                                              (= uuid-text fallback-value))}))
                                (range iterations))
                               (catch Throwable t
                                 [{:ok false
                                   :throwable (.getName (class t))
                                   :message (.getMessage t)}])))
                    rows (vec (mapcat deref
                                      (doall
                                       (map-indexed
                                        (fn [idx ns-sym]
                                          (future (worker idx ns-sym)))
                                        ns-syms))))
                    failures (remove :ok rows)]
                {:threads thread-count
                 :iterations iterations
                 :result-count (count rows)
                 :all-ok? (every? :ok rows)
                 :failures (count failures)})
              (finally
                (doseq [ns-sym ns-syms]
                  (unload-ns! ns-sym))))
        want {:threads thread-count
              :iterations iterations
              :result-count (* thread-count iterations)
              :all-ok? true
              :failures 0}]
    {:desc "compile-form/eval-string concurrent ns isolation"
     :want want
     :got got
     :ok (= want got)}))

(defn- public-api-entrypoints-smoke
  []
  (let [ns-sym 'pnix.clj-meta.public-api-entrypoints-smoke
        _      (remove-ns ns-sym)
        fallback-form (read-string "(fn [] #uuid \"00000000-0000-0000-0000-000000000000\")")
        strict-fn (compile-form-strict '(fn [] 42))
        strict-throws? (try
                         (compile-form-strict fallback-form)
                         false
                         (catch Throwable _ true))
        eval-string-result (eval-string "(+ 1 41)")
        loaded (compile-and-load-ns
                "(ns pnix.clj-meta.public-api-entrypoints-smoke)\n(def x 40)\n(+ x 2)"
                {:file "public_api_entrypoints_smoke.clj"})
        got    {:strict-value (strict-fn)
                :strict-unsupported-throws? strict-throws?
                :eval-string eval-string-result
                :compile-and-load-ns {:namespace (:namespace loaded)
                                      :last-result (last (:results loaded))
                                      :loaded (:loaded loaded)}}
        want   {:strict-value 42
                :strict-unsupported-throws? true
                :eval-string 42
                :compile-and-load-ns {:namespace ns-sym
                                      :last-result 42
                                      :loaded true}}]
    (remove-ns ns-sym)
    {:desc "public API entrypoints"
     :want want
     :got got
     :ok (= want got)}))

(defn- classloader-lifecycle-smoke
  []
  (clear-kept-classes!)
  (let [session-got (with-compiler-session
                      #(let [a (compile-form '(fn [] 1))
                             b (compile-form '(fn [] 2))
                             c (compile-form '(fn [] 1))]
                         {:shared-loader? (= (.getClassLoader (class a))
                                             (.getClassLoader (class b))
                                             (.getClassLoader (class c)))
                          :values [(a) (b) (c)]}))
        _           (clear-kept-classes!)
        cap-count   (binding [*kept-class-units-cap* 1]
                      (compile-form '(fn [] 10))
                      (compile-form '(fn [] 20))
                      (count @kept-class-units))
        _           (clear-kept-classes!)
        got         (assoc session-got
                           :cap-count cap-count
                           :clear-count (count @kept-class-units))
        want        {:shared-loader? true
                     :values [1 2 1]
                     :cap-count 1
                     :clear-count 0}]
    {:desc "classloader lifecycle controls"
     :want want
     :got got
     :ok (= want got)}))

(defn- temp-directory!
  [prefix]
  (let [dir (File/createTempFile prefix ".dir")]
    (when-not (.delete dir)
      (throw (ex-info "temp-directory!: could not delete temp file"
                      {:path (.getPath dir)})))
    (when-not (.mkdirs dir)
      (throw (ex-info "temp-directory!: could not create temp directory"
                      {:path (.getPath dir)})))
    dir))

(defn- delete-tree!
  [^File root]
  (when (.exists root)
    (doseq [f (reverse (file-seq root))]
      (.delete ^File f))))

(defn- compile-to-dir-smoke
  []
  (let [dir  (temp-directory! "pnix-clj-meta-aot")
        form '(fn [x] (+ x 2))]
    (try
      (let [artifact (compile-to-dir form dir)
            loaded   (load-from-dir artifact)
            class-file (File. ^String (get (:files artifact) (:main-class artifact)))
            got      {:class-count (:class-count artifact)
                      :class-file-exists? (.exists class-file)
                      :loaded (:loaded loaded)
                      :same-classes? (= (:classes artifact) (:classes loaded))
                      :value ((:fn loaded) 40)}
            want     {:class-count 1
                      :class-file-exists? true
                      :loaded true
                      :same-classes? true
                      :value 42}]
        {:desc "compile-to-dir/load-from-dir roundtrip"
         :want want
         :got got
         :ok (= want got)})
      (finally
        (delete-tree! dir)))))

(defn- warm-up-smoke
  []
  (let [got  (select-keys (warm-up!) [:warmed :value])
        want {:warmed true
              :value 42}]
    {:desc "warm-up! compiles trivial direct form"
     :want want
     :got got
     :ok (= want got)}))

(defn- independent-unroll-admission-smoke
  []
  (let [form '(fn []
                (loop [i 0 acc 0]
                  (if (< i 10)
                    (if (< i 1)
                      (recur (+ i 3) (+ acc i))
                      (recur (+ i 2) (+ acc i)))
                    (+ acc 9223372036854775787))))
        got  (binding [*disable-homogeneous-index-stride-gate* true]
               (try
                 ((compile-form form))
                 (catch Throwable t
                   [:throw (.getMessage t)])))
        want [:throw "long overflow"]]
    {:desc "checked-long admission uses independent bounded-unroll range"
     :want want
     :got got
     :ok (= want got)}))

(defn run-smoke
  []
  (conj
   (mapv (fn [[desc form args want]]
           (let [got (try
                       (let [^IFn f (compile-form form)]
                         (apply f args))
                       (catch Throwable t [:throw (.getMessage t)]))
                 ok  (= got want)]
             {:desc desc :want want :got got :ok ok}))
         smoke-cases)
   (line-metadata-smoke)
   (compile-form-top-level-smoke)
   (compile-form-diagnostics-smoke)
   (compile-form-star-smoke)
   (compile-form-strict-no-fallback-smoke)
   (compile-form-diagnostics-cap-smoke)
   (compile-form-error-envelope-smoke)
   (compile-form-class-name-smoke)
   (compile-ns-smoke)
   (compile-ns-require-import-smoke)
   (compile-ns-isolate-smoke)
   (compile-form-ns-options-smoke)
   (compile-concurrency-smoke)
   (public-api-entrypoints-smoke)
   (classloader-lifecycle-smoke)
   (compile-to-dir-smoke)
   (warm-up-smoke)
   (independent-unroll-admission-smoke)))

(defn run-concurrency-smoke
  []
  [(compile-concurrency-smoke)])

(defn- print-smoke-results!
  [label results]
  (let [ok? (every? :ok results)]
    (doseq [{:keys [desc want got ok]} results]
      (println (format "  [%s] %s  want=%s got=%s"
                       (if ok "OK" "FAIL") desc (pr-str want) (pr-str got))))
    (println (str label ": "
                  (if ok? "ALL OK" "FAILED")
                  " (" (count (filter :ok results)) "/" (count results) ")"))
    ok?))

(defn -main
  [& args]
  (let [concurrency? (contains? #{"concurrency" "concurrency-smoke"} (first args))
        results      (if concurrency? (run-concurrency-smoke) (run-smoke))
        label        (if concurrency?
                       "compiler concurrency smoke"
                       "compiler smoke (host Compiler 0회 emit)")
        ok?          (print-smoke-results! label results)]
    (shutdown-agents)
    (when-not ok?
      (System/exit 1))))
