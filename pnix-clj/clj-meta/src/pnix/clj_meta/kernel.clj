(ns pnix.clj-meta.kernel
  "축 B-1: 메타순환 인터프리터 커널.

  *해석 대상 프로그램(target form)* 을 호스트 clojure.lang.Compiler / eval 로
  컴파일·실행하지 않고, 커널의 평가 규칙(k-eval)으로 한 폼씩 직접 해석한다
  (SICP식 metacircular evaluator의 Clojure 버전, hy-meta kernel.hy 대응).

  ── 정직한 경계(이 셋은 host 에 기댄다. hy-meta 와 동일한 경계이며 의도된 설계다) ──
  1. 커널 자신(kernel.clj)은 stage1 에서 host 로 컴파일되는 seed 다.
     (hy-meta stage1 도 host Python 으로 실행됨 — 정상.)
  2. 런타임 라이브러리(+ - * = < first map …)는 host clojure.core 위임이다.
     커널이 소유하는 것은 *언어 의미론* — special form 평가 규칙과 함수 적용뿐.
  3. 커널 클로저는 host fn 으로 표현하되, 그 본문은 컴파일된 게 아니라 호출
     시마다 k-eval 로 재해석된다(eval-fn 참조). 그래서 target 함수의 *의미* 는
     여전히 커널이 결정한다.
  - read 도 host reader 위임(여기서는 호출자가 이미 읽은 폼을 받음).

  주의: 이 경계는 축 A self-host(stage2~7)에서 결정적이 된다. stage2 가
  'k-eval 로 kernel.clj 를 해석' 하려면 위 host 위임(atom/swap!/.indexOf/
  ns-resolve/host fn)을 커널이 해석할 수 있어야 하고, 고정점 비교 기준(값/효과)을
  정의해야 한다. → todo.md Phase 2 참조.

  special form(1a~1d 범위): quote / if / do / let / letfn / fn / def / loop / recur /
  when / cond / and / or / -> / ->> / try / throw / set! / binding / locking /
  var / case / 기본 host interop
  env 표현:  {:locals <불변맵 sym->val> :globals <atom sym->Var|val>}")

;; ---------------------------------------------------------------------------
;; 환경(environment)
;; ---------------------------------------------------------------------------

(defn fresh-env
  "새 평가 환경. locals 는 비었고 globals 는 공유 atom."
  []
  {:locals {} :globals (atom {}) :macros (atom {})})

(declare resolve-class)

(def ^:private kernel-instance-key ::instance)
(def ^:private kernel-record-key ::record)
(def ^:private kernel-type-key ::type)
(def ^:private kernel-field-key ::field)

(defn- kernel-instance?
  [x]
  (and (map? x) (true? (get x kernel-instance-key))))

(defn- kernel-record?
  [x]
  (and (kernel-instance? x) (true? (get x kernel-record-key))))

(defn- public-field-keys
  [x]
  (remove #{kernel-instance-key kernel-record-key kernel-type-key} (keys x)))

(defn- plain-kernel-map
  [x]
  (into {} (map (fn [k] [k (get x k)])) (public-field-keys x)))

(defn- fake-class
  [type-name fields volatile-fields]
  {::class true
   :type-name type-name
   :fields (vec fields)
   :volatile-fields (set volatile-fields)})

(defn- fake-class?
  [x]
  (and (map? x) (true? (::class x))))

(defn- fake-field?
  [x]
  (and (map? x) (true? (get x kernel-field-key))))

(defn- kernel-class-of
  [x]
  (if (kernel-instance? x)
    (get x kernel-type-key)
    (clojure.core/class x)))

(defn- kernel-record?-fn
  [x]
  (kernel-record? x))

(defn- kernel-dissoc
  [m & ks]
  (if (kernel-record? m)
    (apply dissoc (plain-kernel-map m) ks)
    (apply dissoc m ks)))

(defn- ensure-kernel-type-overrides!
  [env]
  (swap! (:globals env)
         #(-> %
              (assoc 'class kernel-class-of)
              (assoc 'record? kernel-record?-fn)
              (assoc 'dissoc kernel-dissoc)))
  env)

(defn- static-field-value
  [sym]
  (when (namespace sym)
    (try
      {:value (clojure.lang.Reflector/getStaticField
               (resolve-class (symbol (namespace sym))) (name sym))}
      (catch Exception _
        nil))))

(defn- host-resolve
  "커널 환경에 없는 심볼을 호스트(런타임 라이브러리)에서 찾는다.
  namespaced 면 그대로 resolve, 아니면 clojure.core 우선."
  [sym]
  (if-let [v (if (namespace sym)
               (resolve sym)
               (or (ns-resolve 'clojure.core sym)
                   (resolve sym)))]
    (cond
      (and (var? v) (:macro (meta v)))
      (throw (ex-info (str "Unsupported macro: " sym) {:sym sym}))
      (var? v)   (deref v)
      (class? v) v
      :else      v)
    (if-let [field (static-field-value sym)]
      (:value field)
      (throw (ex-info (str "Unable to resolve symbol: " sym) {:sym sym})))))

(defn- resolve-var
  [sym]
  (let [v (if (namespace sym)
            (resolve sym)
            (or (ns-resolve 'clojure.core sym)
                (resolve sym)))]
    (if (var? v)
      v
      (throw (ex-info (str "Unable to resolve var: " sym) {:sym sym})))))

(defn- class-name
  [sym]
  (if (namespace sym)
    (str (namespace sym) "." (name sym))
    (name sym)))

(defn- resolve-class
  [sym]
  (cond
    (class? sym) sym
    (symbol? sym)
    (let [n (class-name sym)
          resolved (resolve (symbol n))]
      (cond
        (class? resolved) resolved
        :else (try
                (Class/forName n)
                (catch ClassNotFoundException _
                  (Class/forName (str "java.lang." n))))))
    :else
    (throw (ex-info "Expected class symbol" {:value sym}))))

(defn- eval-symbol
  [sym env]
  (let [locals (:locals env)]
    (if (contains? locals sym)
      (get locals sym)
      (let [g @(:globals env)]
        (if (contains? g sym)
          (let [v (get g sym)]
            (if (var? v) (deref v) v))
          (host-resolve sym))))))

(defn- kernel-var
  [sym env]
  (let [g @(:globals env)]
    (when (contains? g sym)
      (let [v (get g sym)]
        (when (var? v) v)))))

;; ---------------------------------------------------------------------------
;; k-eval (전방 선언 — special form 처리기들이 서로 참조)
;; ---------------------------------------------------------------------------

(declare k-eval)

(defn- eval-body
  "do 스타일: 폼들을 순차 평가하고 마지막 값을 반환(빈 본문이면 nil)."
  [body env]
  (reduce (fn [_ f] (k-eval f env)) nil body))

(defn- eval-if
  [form env]
  (if (k-eval (nth form 1) env)
    (k-eval (nth form 2) env)
    (when (>= (count form) 4)
      (k-eval (nth form 3) env))))

(defn- eval-let
  "(let [s1 v1 s2 v2 ...] body...) — 순차 바인딩(let* 의미)."
  [form env]
  (loop [bs  (seq (partition 2 (second form)))
         env env]
    (if bs
      (let [[sym vexpr] (first bs)
            v (k-eval vexpr env)]
        (recur (next bs) (assoc-in env [:locals sym] v)))
      (eval-body (nnext form) env))))

(defn- case-test-match?
  [v test]
  (if (seq? test)
    (boolean (some #(= v %) test))
    (= v test)))

(defn- eval-case
  [form env]
  (let [v        (k-eval (second form) env)
        clauses  (vec (nnext form))
        default? (odd? (count clauses))
        pairs    (if default? (butlast clauses) clauses)
        default  (when default? (peek clauses))]
    (loop [xs (seq (partition 2 pairs))]
      (if xs
        (let [[test expr] (first xs)]
          (if (case-test-match? v test)
            (k-eval expr env)
            (recur (next xs))))
        (if default?
          (k-eval default env)
          (throw (IllegalArgumentException.
                  (str "No matching clause: " (pr-str v)))))))))

(defn- eval-when
  [form env]
  (when (k-eval (second form) env)
    (eval-body (nnext form) env)))

(defn- eval-cond
  [form env]
  (loop [clauses (rest form)]
    (when (seq clauses)
      (when-not (next clauses)
        (throw (ex-info "cond requires an even number of forms" {:form form})))
      (let [[test expr] clauses]
        (if (k-eval test env)
          (k-eval expr env)
          (recur (nnext clauses)))))))

(defn- eval-and
  [form env]
  (loop [xs (rest form)
         ret true]
    (if (seq xs)
      (let [v (k-eval (first xs) env)]
        (if v
          (recur (next xs) v)
          v))
      ret)))

(defn- eval-or
  [form env]
  (loop [xs (rest form)]
    (when (seq xs)
      (let [v (k-eval (first xs) env)]
        (if (or v (nil? (next xs)))
          v
          (recur (next xs)))))))

(defn- thread-first-form
  [x form]
  (if (seq? form)
    (with-meta (cons (first form) (cons x (rest form))) (meta form))
    (list form x)))

(defn- thread-last-form
  [x form]
  (if (seq? form)
    (with-meta (concat (list (first form)) (rest form) (list x)) (meta form))
    (list form x)))

(defn- eval-thread-first
  [form env]
  (k-eval (reduce thread-first-form (second form) (nnext form)) env))

(defn- eval-thread-last
  [form env]
  (k-eval (reduce thread-last-form (second form) (nnext form)) env))

(defn- amp-index
  [params]
  (.indexOf ^java.util.List params '&))

(defn- variadic-params?
  [params]
  (not (neg? (amp-index params))))

(defn- arity-match?
  [params argc]
  (let [amp (amp-index params)]
    (if (neg? amp)
      (= argc (count params))
      (>= argc amp))))

(defn- bind-params
  "파라미터 벡터에 인자를 바인딩. '& 가변인자 지원."
  [params args env]
  (let [amp (amp-index params)]
    (if (neg? amp)
      (update env :locals merge (zipmap params args))
      (let [fixed   (subvec params 0 amp)
            restsym (nth params (inc amp))
            n       (count fixed)]
        (update env :locals merge
                (assoc (zipmap fixed (take n args))
                       restsym (seq (drop n args))))))))

(defn- recur-signal
  [values]
  (ex-info "recur" {:pnix.clj-meta.kernel/type ::recur :values values}))

(defn- recur-signal?
  [t]
  (= ::recur (:pnix.clj-meta.kernel/type (ex-data t))))

(defn- fn-clauses
  [form]
  (let [named? (symbol? (second form))
        name   (when named? (second form))
        parts  (if named? (nnext form) (rest form))]
    {:name name
     :clauses (if (vector? (first parts))
                [{:params (first parts) :body (rest parts)}]
                (mapv (fn [clause] {:params (first clause) :body (rest clause)}) parts))}))

(defn- invoke-kernel-fn
  [closure-env clauses args]
  (let [{:keys [params body]} (or (first (filter #(arity-match? (:params %) (count args)) clauses))
                                  (throw (ex-info "Wrong number of args"
                                                  {:argc (count args)
                                                   :arities (mapv :params clauses)})))]
    (loop [args args]
      (let [[tag value] (try
                          [:value (eval-body body (bind-params params args closure-env))]
                          (catch clojure.lang.ExceptionInfo e
                            (if (recur-signal? e)
                              [:recur (:values (ex-data e))]
                              (throw e))))]
        (if (= tag :recur)
          (recur value)
          value)))))

(defn- eval-fn
  "(fn name? ([params...] body...)...) — 다중 arity/가변인자/named self-ref 클로저.
  본문은 호스트 컴파일러가 아니라 k-eval 로 해석된다. 정의 시점 env 를 캡처한다."
  [form env]
  (let [{:keys [name clauses]} (fn-clauses form)
        self (atom nil)]
    (letfn [(f [& args]
              (let [closure-env (cond-> env name (assoc-in [:locals name] @self))]
                (invoke-kernel-fn closure-env clauses args)))]
      (reset! self f)
      f)))

(defn- letfn-spec->fn-form
  [spec]
  (let [name (first spec)]
    (cons 'fn (cons name (rest spec)))))

(defn- letfn-placeholder
  [cell name]
  (fn [& args]
    (if-let [f @cell]
      (apply f args)
      (throw (ex-info "Uninitialized letfn binding" {:name name})))))

(defn- eval-letfn
  [form env]
  (let [specs (second form)
        names (mapv first specs)
        cells (zipmap names (repeatedly #(atom nil)))
        env'  (update env :locals merge
                      (into {}
                            (map (fn [name]
                                   [name (letfn-placeholder (get cells name) name)]))
                            names))]
    (doseq [spec specs]
      (reset! (get cells (first spec))
              (k-eval (letfn-spec->fn-form spec) env')))
    (eval-body (nnext form) env')))

(defn- eval-loop
  "(loop [s1 v1 ...] body...) — recur 신호를 잡아 같은 스택 프레임에서 재평가한다."
  [form env]
  (let [pairs (partition 2 (second form))
        syms  (mapv first pairs)
        env0  (reduce (fn [e [sym vexpr]]
                        (assoc-in e [:locals sym] (k-eval vexpr e)))
                      env pairs)]
    (loop [env' env0]
      (let [[tag value] (try
                          [:value (eval-body (nnext form) env')]
                          (catch clojure.lang.ExceptionInfo e
                            (if (recur-signal? e)
                              [:recur (:values (ex-data e))]
                              (throw e))))]
        (if (= tag :recur)
          (do
            (when-not (= (count syms) (count value))
              (throw (ex-info "recur arity mismatch"
                              {:expected (count syms) :actual (count value)})))
            (recur (update env :locals merge (zipmap syms value))))
          value)))))

(defn- eval-recur
  [form env]
  (throw (recur-signal (mapv #(k-eval % env) (rest form)))))

(defn- try-clause?
  [x]
  (and (seq? x) (#{'catch 'finally} (first x))))

(defn- parse-try
  [form]
  (let [[body clauses] (split-with (complement try-clause?) (rest form))
        [catches tail] (split-with #(and (seq? %) (= 'catch (first %))) clauses)
        finally-clause (first tail)]
    (when (seq (rest tail))
      (throw (ex-info "try supports at most one finally clause" {:form form})))
    (when (and finally-clause (not= 'finally (first finally-clause)))
      (throw (ex-info "try clauses must be catch* then optional finally" {:form form})))
    {:body body :catches catches :finally finally-clause}))

(defn- matching-catch
  [^Throwable t catches]
  (some (fn [clause]
          (let [klass (resolve-class (second clause))]
            (when (.isInstance klass t)
              clause)))
        catches))

(defn- eval-try
  [form env]
  (let [{:keys [body catches finally]} (parse-try form)]
    (try
      (try
        (eval-body body env)
        (catch Throwable t
          (if (recur-signal? t)
            (throw t)
            (if-let [clause (matching-catch t catches)]
              (eval-body (nnext clause) (assoc-in env [:locals (nth clause 2)] t))
              (throw t)))))
      (finally
        (when finally
          (eval-body (rest finally) env))))))

(defn- eval-throw
  [form env]
  (throw (k-eval (second form) env)))

(defn- eval-var-form
  [form env]
  (let [sym (second form)]
    (or (kernel-var sym env)
        (resolve-var sym))))

(defn- constructor-symbol?
  [sym]
  (and (symbol? sym) (.endsWith (name sym) ".")))

(defn- constructor-class-symbol
  [sym]
  (let [n (name sym)]
    (if (namespace sym)
      (symbol (namespace sym) (subs n 0 (dec (count n))))
      (symbol (subs n 0 (dec (count n)))))))

(defn- eval-new
  [form env]
  (let [klass (resolve-class (constructor-class-symbol (first form)))
        args  (mapv #(k-eval % env) (rest form))]
    (clojure.lang.Reflector/invokeConstructor klass (object-array args))))

(defn- instance-member-symbol?
  [sym]
  (and (symbol? sym) (.startsWith (name sym) ".")))

(defn- eval-instance-member
  [form env]
  (let [member (name (first form))
        target (k-eval (second form) env)
        args   (mapv #(k-eval % env) (nnext form))]
    (cond
      (and (.startsWith member ".-")
           (kernel-instance? target))
      (get target (keyword (subs member 2)))

      (.startsWith member ".-")
      (clojure.lang.Reflector/getInstanceField target (subs member 2))

      (and (= member ".getLookupThunk")
           (kernel-record? target)
           (= 1 (count args)))
      (let [k (first args)]
        (reify clojure.lang.ILookupThunk
          (get [_ target']
            (clojure.core/get target' k))))

      (and (= member ".getDeclaredField")
           (fake-class? target)
           (= 1 (count args)))
      (let [field-name (str (first args))]
        {kernel-field-key true
         :field-name field-name
         :modifiers (if (contains? (:volatile-fields target)
                                    (symbol field-name))
                      java.lang.reflect.Modifier/VOLATILE
                      0)})

      (and (= member ".getModifiers")
           (fake-field? target)
           (empty? args))
      (:modifiers target)

      :else
      (clojure.lang.Reflector/invokeInstanceMethod
       target (subs member 1) (object-array args)))))

(defn- eval-dot
  [form env]
  (let [target (k-eval (second form) env)
        member (nth form 2)
        args   (mapv #(k-eval % env) (drop 3 form))
        name'  (name member)]
    (cond
      (and (kernel-instance? target)
           (empty? args))
      (get target (keyword name'))

      (and (= name' "getDeclaredField")
           (fake-class? target)
           (= 1 (count args)))
      (let [field-name (str (first args))]
        {kernel-field-key true
         :field-name field-name
         :modifiers (if (contains? (:volatile-fields target)
                                    (symbol field-name))
                      java.lang.reflect.Modifier/VOLATILE
                      0)})

      (and (= name' "getModifiers")
           (fake-field? target)
           (empty? args))
      (:modifiers target)

      (seq args)
      (clojure.lang.Reflector/invokeInstanceMethod target name' (object-array args))

      :else
      (clojure.lang.Reflector/getInstanceField target name'))))

(defn- static-member-symbol?
  [sym]
  (and (symbol? sym) (namespace sym) (nil? (resolve sym))))

(defn- eval-static-member
  [form env]
  (let [sym    (first form)
        klass  (resolve-class (symbol (namespace sym)))
        member (name sym)
        args   (mapv #(k-eval % env) (rest form))]
    (try
      (clojure.lang.Reflector/invokeStaticMethod klass member (object-array args))
      (catch IllegalArgumentException e
        (if (empty? args)
          (clojure.lang.Reflector/getStaticField klass member)
          (throw e))))))

(defn- instance-field-target
  [target]
  (cond
    (and (seq? target)
         (symbol? (first target))
         (.startsWith (name (first target)) ".-"))
    {:object (second target)
     :field  (subs (name (first target)) 2)}

    (and (seq? target)
         (= '. (first target))
         (= 3 (count target))
         (symbol? (nth target 2)))
    {:object (second target)
     :field  (name (nth target 2))}

    :else nil))

(defn- eval-set!
  [form env]
  (let [target (second form)
        vexpr  (nth form 2)]
    (cond
      (symbol? target)
      (let [^clojure.lang.Var v (or (kernel-var target env)
                                    (resolve-var target))
            value (k-eval vexpr env)]
        (.set v value))

      (instance-field-target target)
      (let [{:keys [object field]} (instance-field-target target)
            target-value (k-eval object env)
            value        (k-eval vexpr env)]
        (clojure.lang.Reflector/setInstanceField target-value field value))

      :else
      (throw (ex-info "Unsupported set! target" {:target target :form form})))))

(defn- eval-binding
  [form env]
  (let [pairs    (partition 2 (second form))
        bindings (into {}
                       (map (fn [[sym vexpr]]
                              [(or (kernel-var sym env)
                                   (resolve-var sym))
                               (k-eval vexpr env)]))
                       pairs)]
    (clojure.lang.Var/pushThreadBindings bindings)
    (try
      (eval-body (nnext form) env)
      (finally
        (clojure.lang.Var/popThreadBindings)))))

(defn- eval-locking
  [form env]
  (let [lock (k-eval (second form) env)]
    (locking lock
      (eval-body (nnext form) env))))

(defn- eval-def
  "(def sym) / (def sym val) — host 와 같은 Var 를 만들고 globals 에 등록한다."
  [form env]
  (let [sym       (second form)
        sym'      (with-meta sym nil)
        md        (meta sym)
        has-init? (>= (count form) 3)
        v         (when has-init? (k-eval (nth form 2) env))
        ^clojure.lang.Var var (intern *ns* sym')]
    (when (:dynamic md)
      (.setDynamic var))
    (when (seq md)
      (alter-meta! var merge md))
    (when has-init?
      (.bindRoot var v))
    (swap! (:globals env) assoc sym' var)
    var))

(defn- field-symbol
  [f]
  (with-meta f nil))

(defn- field-key
  [f]
  (keyword (name (field-symbol f))))

(defn- field-volatile?
  [f]
  (boolean (:volatile-mutable (meta f))))

(defn- kernel-type-instance
  [fake-class record? fields values ext]
  (merge {kernel-instance-key true
          kernel-record-key (boolean record?)
          kernel-type-key fake-class}
         (zipmap (map field-key fields) values)
         ext))

(defn- install-kernel-type!
  [env type-name fields {:keys [record?]}]
  (ensure-kernel-type-overrides! env)
  (let [fake (fake-class type-name
                         (mapv field-symbol fields)
                         (keep #(when (field-volatile? %) (field-symbol %)) fields))
        ctor-name (symbol (str "->" (name type-name)))
        map-ctor-name (symbol (str "map->" (name type-name)))
        ctor (fn [& values]
               (kernel-type-instance fake record? fields values nil))
        map-ctor (fn [m]
                   (let [field-keys (mapv field-key fields)
                         field-values (mapv #(get m %) field-keys)
                         ext (apply dissoc m field-keys)]
                     (kernel-type-instance fake record? fields field-values ext)))]
    (swap! (:globals env) assoc
           type-name fake
           ctor-name ctor)
    (when record?
      (swap! (:globals env) assoc map-ctor-name map-ctor))
    fake))

(defn- eval-deftype
  [form env]
  (let [[_ type-name fields & _methods] form]
    (install-kernel-type! env type-name fields {:record? false})))

(defn- eval-defrecord
  [form env]
  (let [[_ type-name fields & _methods] form]
    (install-kernel-type! env type-name fields {:record? true})))

(defn- eval-defmacro
  "(defmacro name [raw-args...] body...) — expansion 함수는 raw form args 를 받는다."
  [form env]
  (let [name (second form)
        params (nth form 2)
        body (drop 3 form)
        closure-env env
        macro-fn (fn [& raw-args]
                   (eval-body body (bind-params params raw-args closure-env)))]
    (swap! (:macros env) assoc name macro-fn)
    name))

(defn- eval-apply
  [form env]
  (let [f    (k-eval (first form) env)
        args (mapv #(k-eval % env) (rest form))]
    (if (ifn? f)
      (apply f args)
      (throw (ex-info (str "Not a function: " (pr-str f)) {:form form})))))

(defn k-eval
  "폼 하나를 환경 env 에서 평가한다."
  [form env]
  (cond
    (symbol? form) (eval-symbol form env)

    (seq? form)
    (if (empty? form)
      form
      (let [head (when (symbol? (first form)) (first form))]
        (if-let [macro-fn (and head (get @(:macros env) head))]
          (k-eval (apply macro-fn (rest form)) env)
          (case head
            quote (second form)
            if    (eval-if form env)
            do    (eval-body (rest form) env)
            let   (eval-let form env)
            let*  (eval-let form env)
            letfn (eval-letfn form env)
            case  (eval-case form env)
            when  (eval-when form env)
            cond  (eval-cond form env)
            and   (eval-and form env)
            or    (eval-or form env)
            ->    (eval-thread-first form env)
            ->>   (eval-thread-last form env)
            loop  (eval-loop form env)
            recur (eval-recur form env)
            try   (eval-try form env)
            throw (eval-throw form env)
            set!  (eval-set! form env)
            binding (eval-binding form env)
            locking (eval-locking form env)
            var   (eval-var-form form env)
            .     (eval-dot form env)
            fn    (eval-fn form env)
            def   (eval-def form env)
            deftype (eval-deftype form env)
            defrecord (eval-defrecord form env)
            defmacro (eval-defmacro form env)
            (cond
              (constructor-symbol? (first form))   (eval-new form env)
              (instance-member-symbol? (first form)) (eval-instance-member form env)
              (static-member-symbol? (first form)) (eval-static-member form env)
              :else                                (eval-apply form env))))))

    (vector? form) (mapv #(k-eval % env) form)
    (set? form)    (set (map #(k-eval % env) form))
    (map? form)    (into {} (map (fn [[k v]] [(k-eval k env) (k-eval v env)]) form))

    ;; self-evaluating: 숫자/문자열/키워드/bool/nil/char
    :else form))

;; toplevel 헬퍼: globals 를 공유하며 폼들을 순차 평가
(defn k-eval-forms
  [forms]
  (let [env (fresh-env)]
    (reduce (fn [_ f] (k-eval f env)) nil forms)))

;; ---------------------------------------------------------------------------
;; smoke (Phase 1a 자기검증)
;; ---------------------------------------------------------------------------

(def ^:private smoke-cases
  "[설명 폼들 기대값]. 폼들은 하나의 env 에서 순차 평가."
  [["let 순차 바인딩"          '[(let [x 21] (+ x x))]                            42]
   ["fn 적용(클로저)"          '[((fn [x] (* x x)) 7)]                            49]
   ["if 진리값"                '[(if (< 1 2) :yes :no)]                           :yes]
   ["def 후 호출"              '[(def sq (fn [n] (* n n))) (sq 8)]                64]
   ["커널 fn ⨉ host map 혼용"  '[(map (fn [n] (* n 10)) [1 2 3])]                 '(10 20 30)]
   ["가변인자 & rest"          '[((fn [a & r] [a r]) 1 2 3)]                      [1 '(2 3)]]
   ["재귀 factorial"           '[(def fact (fn [n] (if (< n 2) 1 (* n (fact (- n 1)))))) (fact 5)] 120]
   ["named fn self-ref"        '[((fn fact [n] (if (< n 2) 1 (* n (fact (- n 1))))) 5)] 120]
   ["다중 arity"               '[((fn ([] 0) ([x] x) ([x y] (+ x y))) 20 22)]      42]
   ["loop/recur"               '[(loop [i 5 acc 0] (if (< i 1) acc (recur (- i 1) (+ acc i))))] 15]
   ["core macros: when/cond"   '[(let [x 3] (when (and (< 1 x) (or nil true)) (cond (< x 2) :small :else :ok)))] :ok]
   ["core macros: thread"      '[(->> [1 2 3] (map (fn [x] (* x 2))) (reduce +))] 12]
   ["try/catch/throw"          '[(try (throw (ex-info "boom" {:k 1}))
                                  (catch clojure.lang.ExceptionInfo e (:k (ex-data e))))] 1]
   ["try/finally"              '[(let [a (atom 0)]
                                  (try 10 (finally (swap! a inc)))
                                  @a)] 1]
   ["host interop"             '[(let [sb (StringBuilder.)]
                                  (.append sb "x")
                                  (.append sb (String/valueOf 2))
                                  [(Math/sqrt 16.0) (.toString sb) Long/MAX_VALUE])] [4.0 "x2" Long/MAX_VALUE]]
   ["set!: instance field"     '[(let [p (java.awt.Point.)] (set! (.-x p) 8) (.-x p))] 8]
   ["set!: field eval order"   '[(let [seen (atom []) p (java.awt.Point.)]
                                  (set! (. (do (swap! seen conj :target) p) x)
                                        (do (swap! seen conj :val) 8))
                                  @seen)] [:target :val]]
   ["binding + set!"           '[(binding [*print-length* 1]
                                  (set! *print-length* 7)
                                  *print-length*)] 7]
   ["def dynamic binding"      '[(def ^:dynamic *kernel-dyn-smoke* 1)
                                  [(.isDynamic (var *kernel-dyn-smoke*))
                                   (binding [*kernel-dyn-smoke* 2]
                                     *kernel-dyn-smoke*)]] [true 2]]
   ["locking"                  '[(let [sb (StringBuilder.)]
                                  (locking sb (.append sb "x"))
                                  (.toString sb))] "x"]
   ["var form"                 '[(var +)] #'clojure.core/+]
   ["case"                     '[(case 2 1 :one 2 :two :other)] :two]
   ["case grouped"             '[(case 3 (1 2) :small (3 4) :mid :other)] :mid]
   ["letfn mutual recursion"   '[(letfn [(ev? [x] (if (zero? x) true (od? (dec x))))
                                          (od? [x] (if (zero? x) false (ev? (dec x))))]
                                  (ev? 4))] true]
   ["custom defmacro"          '[(defmacro unless [test then else]
                                   (list 'if test else then))
                                  (unless false 42 0)] 42]])

(defn run-smoke
  []
  (mapv (fn [[desc forms want]]
          (let [got (try (k-eval-forms forms) (catch Throwable t [:throw (.getMessage t)]))
                ok  (= got want)]
            {:desc desc :want want :got got :ok ok}))
        smoke-cases))

(defn -main
  [& _]
  (let [results (run-smoke)
        ok?     (every? :ok results)]
    (doseq [{:keys [desc want got ok]} results]
      (println (format "  [%s] %s  want=%s got=%s"
                       (if ok "OK" "FAIL") desc (pr-str want) (pr-str got))))
    (println (str "kernel smoke: " (if ok? "ALL OK" "FAILED")
                  " (" (count (filter :ok results)) "/" (count results) ")"))
    (when-not ok?
      (System/exit 1))))
