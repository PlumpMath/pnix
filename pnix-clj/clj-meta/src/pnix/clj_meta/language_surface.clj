(ns pnix.clj-meta.language-surface
  "M12 language-surface boundary witness.

  Protocol invocation, letfn mutual recursion, simple reify Object/interface
  methods, and simple compile-ns namespace preparation are direct paths.
  Protocol, multimethod, require/import namespace forms, and full JVM type
  definitions remain explicit host-maintained side-effect boundaries until
  fallback-free self-source is closed."
  (:require [clojure.java.io :as io]
            [clojure.pprint :as pp]
            [clojure.tools.analyzer.jvm :as ana]
            [pnix.clj-meta.compiler :as comp])
  (:import [java.security MessageDigest]))

(def receipt-path "clj-meta/proof/language-surface.receipt.edn")

(defn- sha256-bytes
  [^bytes bs]
  (let [md (MessageDigest/getInstance "SHA-256")]
    (apply str
           (map #(format "%02x" (bit-and 0xff %))
                (.digest md bs)))))

(defn- sha256-string
  [s]
  (sha256-bytes (.getBytes ^String (str s) "UTF-8")))

(defn- try-val
  [f]
  (try
    {:ok true
     :value (f)}
    (catch Throwable t
      {:ok false
       :throwable (.getName (class t))
       :message (.getMessage t)})))

(defn- child-nodes
  [node]
  (->> (:children node)
       (mapcat (fn [k]
                 (let [v (get node k)]
                   (cond
                     (vector? v) v
                     (map? v) [v]
                     :else []))))
       (filter map?)))

(defn- ast-nodes
  [ast]
  (tree-seq map? child-nodes ast))

(defn- ast-ops
  [ast]
  (->> (ast-nodes ast)
       (map :op)
       (remove nil?)
       frequencies
       (into (sorted-map-by #(compare (name %1) (name %2))))))

(defn- analyze-form
  [ns-sym form]
  (remove-ns ns-sym)
  (create-ns ns-sym)
  (try
    (binding [*ns* (the-ns ns-sym)
              *file* "language_surface_fixture.clj"]
      (clojure.core/refer 'clojure.core)
      (ana/analyze form))
    (finally
      (remove-ns ns-sym))))

(defn- contains-ops?
  [ops expected]
  (every? #(contains? ops %) expected))

(defn- sorted-ops
  [ops]
  (vec (sort-by name ops)))

(defn- protocol-invoke-row
  []
  (let [form '(fn [xs]
                (clojure.core.protocols/coll-reduce xs + 10))
        host-result (try-val #(apply (eval form) [[1 2 3]]))
        backend-result (try-val #(apply (comp/compile-form form) [[1 2 3]]))
        ok? (and (:ok host-result)
                 (:ok backend-result)
                 (= 16 (:value host-result))
                 (= (:value host-result) (:value backend-result)))]
    {:id :protocol-invoke-direct
     :kind :direct-emit
     :form (pr-str form)
     :expected 16
     :host-result host-result
     :backend-result backend-result
     :gate/verdict (if ok? :accepted :rejected)
     :boundary :none-direct-bytecode-emit
     :ok ok?}))

(defn- letfn-direct-row
  []
  (let [form '(fn [n]
                (letfn [(ev? [x] (if (zero? x) true (od? (dec x))))
                        (od? [x] (if (zero? x) false (ev? (dec x))))]
                  (ev? n)))
        host-result (try-val #(apply (eval form) [4]))
        backend-result (try-val #(apply (comp/compile-form form) [4]))
        backend-classes (try-val #(comp/compile-classes form))
        artifact-digest (when (:ok backend-classes)
                          (sha256-string
                           (pr-str
                            (mapv (fn [[class-name class-bytes]]
                                    [class-name (sha256-bytes class-bytes)])
                                  (sort-by key (:value backend-classes))))))
        ok? (and (:ok host-result)
                 (:ok backend-result)
                 (:ok backend-classes)
                 (= true (:value host-result))
                 (= (:value host-result) (:value backend-result)))]
    {:id :letfn-mutual-recursion-direct
     :kind :direct-emit
     :form (pr-str form)
     :required-ops [:letfn :fn :invoke]
     :expected true
     :host-result host-result
     :backend-result backend-result
     :backend-artifact
     (if (:ok backend-classes)
       {:ok true
        :class-count (count (:value backend-classes))
        :digest artifact-digest}
       backend-classes)
     :gate/verdict (if ok? :accepted :rejected)
     :boundary :none-direct-bytecode-emit
     :ok ok?}))

(defn- reify-object-direct-row
  []
  (let [form '(fn []
                (str (reify Object
                       (toString [_] "rx"))))
        host-result (try-val #(apply (eval form) []))
        backend-result (try-val #(apply (comp/compile-form form) []))
        backend-classes (try-val #(comp/compile-classes form))
        artifact-digest (when (:ok backend-classes)
                          (sha256-string
                           (pr-str
                            (mapv (fn [[class-name class-bytes]]
                                    [class-name (sha256-bytes class-bytes)])
                                  (sort-by key (:value backend-classes))))))
        ok? (and (:ok host-result)
                 (:ok backend-result)
                 (:ok backend-classes)
                 (= "rx" (:value host-result))
                 (= (:value host-result) (:value backend-result))
                 (= 2 (count (:value backend-classes))))]
    {:id :reify-object-method-direct
     :kind :direct-emit
     :form (pr-str form)
     :required-ops [:reify :method :fn]
     :expected "rx"
     :host-result host-result
     :backend-result backend-result
     :backend-artifact
     (if (:ok backend-classes)
       {:ok true
        :class-count (count (:value backend-classes))
        :digest artifact-digest}
       backend-classes)
     :gate/verdict (if ok? :accepted :rejected)
     :boundary :none-simple-object-method-bytecode-emit
     :ok ok?}))

(defn- defmulti-direct-row
  "defmulti/defmethod 직접 emit: Var 상수(#'global-hierarchy) 지원 + AFunction base 로
  host Compiler 0회 직접 emit. host(load-string)≡backend(compile-ns, incremental) 등가 +
  defmulti form 이 compile-classes 로 fallback 없이 컴파일됨(direct emit 증명)."
  []
  (let [ns-sym 'pnix.clj-meta.language-surface.defmulti-direct
        src (str "(ns pnix.clj-meta.language-surface.defmulti-direct)\n"
                 "(defmulti area :shape)\n"
                 "(defmethod area :circle [_] :C)\n"
                 "(defmethod area :default [_] :D)\n"
                 "[(area {:shape :circle}) (area {:shape :x})]")
        expected [:C :D]]
    (try
      (remove-ns ns-sym)
      (let [host-result (try-val #(do (remove-ns ns-sym)
                                      (let [r (load-string src)] (remove-ns ns-sym) r)))
            backend-art (try-val #(do (remove-ns ns-sym) (comp/compile-ns src)))
            backend-loaded (if (:ok backend-art)
                             (try-val #(comp/load-compiled-ns (:value backend-art)))
                             {:ok false})
            backend-result (when (:ok backend-loaded) (last (:results (:value backend-loaded))))
            direct? (:ok (try-val #(comp/compile-classes '(fn [] (defmulti lsfoo :x)))))
            ok? (and (:ok host-result) (:ok backend-art) (:ok backend-loaded)
                     (= expected (:value host-result))
                     (= (:value host-result) backend-result)
                     direct?)]
        {:id :defmulti-direct
         :kind :direct-emit
         :source src
         :required-ops [:def :new :invoke]
         :expected expected
         :host-result host-result
         :backend-result {:ok (:ok backend-loaded) :value backend-result}
         :backend-artifact {:direct-emit-no-fallback direct?}
         :gate/verdict (if ok? :accepted :rejected)
         :boundary :multimethod-backend-bytecode-emit
         :ok ok?})
      (finally (remove-ns ns-sym)))))

(defn- defprotocol-direct-row
  "defprotocol 직접 emit: 정의 + extend-protocol + reify-구현 + dispatch 가 host Compiler 0회로
  emit 되고 host(load-string)≡backend(compile-ns incremental) 등가. {:k 'sym} const quote-strip
  수정으로 protocol map :on 이 host(Symbol)와 일치 → reify-가-protocol-구현도 동작."
  []
  (let [ns-sym 'pnix.clj-meta.language-surface.defprotocol-direct
        src (str "(ns pnix.clj-meta.language-surface.defprotocol-direct)\n"
                 "(defprotocol Sized (sz [x]))\n"
                 "(extend-protocol Sized String (sz [s] (count s)))\n"
                 "[(sz \"hello\") (sz (reify Sized (sz [_] 99)))]")
        expected [5 99]]
    (try
      (remove-ns ns-sym)
      (let [host-result (try-val #(do (remove-ns ns-sym)
                                      (let [r (load-string src)] (remove-ns ns-sym) r)))
            backend-art (try-val #(do (remove-ns ns-sym) (comp/compile-ns src)))
            backend-loaded (if (:ok backend-art)
                             (try-val #(comp/load-compiled-ns (:value backend-art)))
                             {:ok false})
            backend-result (when (:ok backend-loaded) (last (:results (:value backend-loaded))))
            direct? (:ok (try-val #(comp/compile-classes '(fn [] (defprotocol LsSized (lssz [x]))))))
            ok? (and (:ok host-result) (:ok backend-art) (:ok backend-loaded)
                     (= expected (:value host-result))
                     (= (:value host-result) backend-result)
                     direct?)]
        {:id :defprotocol-direct
         :kind :direct-emit
         :source src
         :required-ops [:def :set! :invoke :reify]
         :expected expected
         :host-result host-result
         :backend-result {:ok (:ok backend-loaded) :value backend-result}
         :backend-artifact {:direct-emit-no-fallback direct?}
         :gate/verdict (if ok? :accepted :rejected)
         :boundary :protocol-definition-backend-bytecode-emit
         :ok ok?})
      (finally (remove-ns ns-sym)))))

(defn- defprotocol-deftype-direct-row
  "U4: 사용자 defprotocol 을 구현하는 deftype 조합이 analyzer 실패/host eval fallback 없이
  compile-ns incremental backend 경로에서 동작함을 고정한다."
  []
  (let [ns-sym 'pnix.clj-meta.language-surface.defprotocol-deftype-direct
        src (str "(ns pnix.clj-meta.language-surface.defprotocol-deftype-direct)\n"
                 "(defprotocol SizedType (st-size [x]))\n"
                 "(deftype SizedBox [v] SizedType (st-size [_] v))\n"
                 "(st-size (->SizedBox 42))")
        expected 42]
    (try
      (remove-ns ns-sym)
      (let [host-result (try-val #(do (remove-ns ns-sym)
                                      (let [r (load-string src)] (remove-ns ns-sym) r)))
            _ (comp/clear-compile-form-fallback-diagnostics!)
            backend-art (try-val #(do (remove-ns ns-sym) (comp/compile-ns src)))
            backend-loaded (if (:ok backend-art)
                             (try-val #(comp/load-compiled-ns (:value backend-art)))
                             {:ok false :message "compile-ns artifact unavailable"})
            diagnostics @comp/compile-form-fallback-diagnostics
            backend-results (when (:ok backend-loaded) (:results (:value backend-loaded)))
            backend-result (last backend-results)
            named-class (second backend-results)
            direct? (and (empty? diagnostics)
                         (class? named-class)
                         (= "pnix.clj_meta.language_surface.defprotocol_deftype_direct.SizedBox"
                            (.getName ^Class named-class)))
            ok? (and (:ok host-result)
                     (:ok backend-art)
                     (:ok backend-loaded)
                     (= expected (:value host-result))
                     (= (:value host-result) backend-result)
                     direct?)]
        {:id :defprotocol-deftype-direct
         :kind :direct-emit
         :source src
         :required-ops [:def :deftype :method :new :invoke]
         :expected expected
         :host-result host-result
         :backend-result {:ok (:ok backend-loaded) :value backend-result}
         :backend-artifact {:direct-emit-no-fallback direct?
                            :fallback-diagnostics diagnostics
                            :named-class (when (class? named-class)
                                           (.getName ^Class named-class))}
         :gate/verdict (if ok? :accepted :rejected)
         :boundary :protocol-implementation-named-type-backend-bytecode-emit
         :ok ok?})
      (finally
        (comp/clear-compile-form-fallback-diagnostics!)
        (remove-ns ns-sym)))))

(defn- deftype-direct-row
  "deftype/defrecord 직접 emit: named 클래스를 우리 backend 가 emit(host Compiler 0회).
  analyzer 가 만든 stub 은 sibling loader 라 우리 full 클래스가 *dcl* 에서 shadow 한다.
  host-valid 한 top-level forms(compile-ns)로 host(load-string)≡backend(compile-ns) 등가를
  비교하고, deftype-form 을 compile-classes 해 우리가 named 클래스 bytecode 를 냈는지 확인한다."
  [id ns-sym src deftype-form class-pat expected]
  (try
    (remove-ns ns-sym)
    (let [host-result (try-val #(do (remove-ns ns-sym)
                                    (let [r (load-string src)] (remove-ns ns-sym) r)))
          backend-art (try-val #(do (remove-ns ns-sym) (comp/compile-ns src)))
          backend-loaded (if (:ok backend-art)
                           (try-val #(comp/load-compiled-ns (:value backend-art)))
                           {:ok false :message "compile-ns artifact unavailable"})
          backend-result (when (:ok backend-loaded) (last (:results (:value backend-loaded))))
          named-emitted? (let [r (try-val #(comp/compile-classes (list 'fn [] deftype-form)))]
                           (and (:ok r)
                                (boolean (some #(re-find class-pat (str %)) (keys (:value r))))))
          functional? (and (:ok host-result)
                           (:ok backend-art)
                           (:ok backend-loaded)
                           (= expected (:value host-result))
                           (= (:value host-result) backend-result))
          verdict (cond
                    (not functional?) :rejected
                    named-emitted? :accepted    ; 우리 backend 가 named 클래스 bytecode 직접 emit
                    :else :held)]               ; host≡backend 로 동작하나 backend 가 host fallback
      {:id id
       :kind (if named-emitted? :direct-emit :host-fallback-boundary)
       :source src
       :required-ops [:deftype :method :new :def]
       :expected expected
       :host-result host-result
       :backend-result {:ok (:ok backend-loaded) :value backend-result}
       :backend-artifact (if (:ok backend-art)
                           {:ok true
                            :ns-form-mode (:ns-form-mode (:value backend-art))
                            :body-count (:body-count (:value backend-art))
                            :named-class-emitted named-emitted?}
                           backend-art)
       :gate/verdict verdict
       :held-reason (when (= verdict :held) :functions-via-host-fallback-not-direct-emit)
       :promotion/allowed? false
       :boundary :named-type-generation-backend-bytecode-emit
       :ok (not= verdict :rejected)})
    (finally (remove-ns ns-sym))))

(defn- reify-iobj-general-direct-row
  "general reify: host clojure 처럼 reify 가 IObj 를 자동 구현한다(compiler 가 __meta 필드 +
  meta/withMeta 생성). with-meta 가 metadata 를 옮긴 복사본을 만들고 사용자 메서드는 보존된다.
  host(eval)≡backend(compile-form) 등가로 확인한다."
  []
  (let [form '(fn []
                (let [r (with-meta (reify Object (toString [_] "rx")) {:k 9})]
                  [(meta r) (str r) (instance? clojure.lang.IObj r)]))
        host-result (try-val #(apply (eval form) []))
        backend-result (try-val #(apply (comp/compile-form form) []))
        backend-classes (try-val #(comp/compile-classes form))
        artifact-digest (when (:ok backend-classes)
                          (sha256-string
                           (pr-str
                            (mapv (fn [[class-name class-bytes]]
                                    [class-name (sha256-bytes class-bytes)])
                                  (sort-by key (:value backend-classes))))))
        ok? (and (:ok host-result)
                 (:ok backend-result)
                 (:ok backend-classes)
                 (= [{:k 9} "rx" true] (:value host-result))
                 (= (:value host-result) (:value backend-result))
                 (<= 2 (count (:value backend-classes))))]
    {:id :reify-iobj-general-direct
     :kind :direct-emit
     :form (pr-str form)
     :required-ops [:reify :method :fn]
     :expected [{:k 9} "rx" true]
     :host-result host-result
     :backend-result backend-result
     :backend-artifact
     (if (:ok backend-classes)
       {:ok true
        :class-count (count (:value backend-classes))
        :digest artifact-digest}
       backend-classes)
     :gate/verdict (if ok? :accepted :rejected)
     :boundary :general-reify-auto-iobj-metadata-bytecode-emit
     :ok ok?}))

(defn- reify-interface-direct-row
  []
  (let [form '(fn []
                (.call (reify java.util.concurrent.Callable
                         (call [_] "ok"))))
        host-result (try-val #(apply (eval form) []))
        backend-result (try-val #(apply (comp/compile-form form) []))
        backend-classes (try-val #(comp/compile-classes form))
        artifact-digest (when (:ok backend-classes)
                          (sha256-string
                           (pr-str
                            (mapv (fn [[class-name class-bytes]]
                                    [class-name (sha256-bytes class-bytes)])
                                  (sort-by key (:value backend-classes))))))
        ok? (and (:ok host-result)
                 (:ok backend-result)
                 (:ok backend-classes)
                 (= "ok" (:value host-result))
                 (= (:value host-result) (:value backend-result))
                 (= 2 (count (:value backend-classes))))]
    {:id :reify-interface-method-direct
     :kind :direct-emit
     :form (pr-str form)
     :required-ops [:reify :method :fn :instance-call]
     :expected "ok"
     :host-result host-result
     :backend-result backend-result
     :backend-artifact
     (if (:ok backend-classes)
       {:ok true
        :class-count (count (:value backend-classes))
        :digest artifact-digest}
       backend-classes)
     :gate/verdict (if ok? :accepted :rejected)
     :boundary :none-simple-interface-method-bytecode-emit
     :ok ok?}))

(defn- reify-capture-direct-row
  []
  (let [form '(fn [x]
                (.call (reify java.util.concurrent.Callable
                         (call [_] x))))
        host-result (try-val #(apply (eval form) ["cap"]))
        backend-result (try-val #(apply (comp/compile-form form) ["cap"]))
        backend-classes (try-val #(comp/compile-classes form))
        artifact-digest (when (:ok backend-classes)
                          (sha256-string
                           (pr-str
                            (mapv (fn [[class-name class-bytes]]
                                    [class-name (sha256-bytes class-bytes)])
                                  (sort-by key (:value backend-classes))))))
        ok? (and (:ok host-result)
                 (:ok backend-result)
                 (:ok backend-classes)
                 (= "cap" (:value host-result))
                 (= (:value host-result) (:value backend-result))
                 (= 2 (count (:value backend-classes))))]
    {:id :reify-capture-method-direct
     :kind :direct-emit
     :form (pr-str form)
     :required-ops [:reify :method :fn :local]
     :expected "cap"
     :host-result host-result
     :backend-result backend-result
     :backend-artifact
     (if (:ok backend-classes)
       {:ok true
        :class-count (count (:value backend-classes))
        :digest artifact-digest}
       backend-classes)
     :gate/verdict (if ok? :accepted :rejected)
     :boundary :none-reify-capture-field-bytecode-emit
     :ok ok?}))

(defn- simple-ns-direct-row
  []
  (let [ns-sym 'pnix.clj-meta.language-surface.simple-ns-direct
        src "(ns pnix.clj-meta.language-surface.simple-ns-direct)\n(def x 40)\n(defn inc2 [n] (+ n 2))\n(def answer (inc2 x))\nanswer"]
    (try
      (remove-ns ns-sym)
      (let [artifact-result (try-val #(comp/compile-ns src {:file "language_surface_simple_ns.clj"}))
            loaded-result   (if (:ok artifact-result)
                              (try-val #(comp/load-compiled-ns (:value artifact-result)))
                              {:ok false
                               :message "compile-ns artifact unavailable"})
            artifact        (:value artifact-result)
            loaded          (:value loaded-result)
            ok?             (and (:ok artifact-result)
                                  (:ok loaded-result)
                                  (= :direct-simple (:ns-form-mode artifact))
                                  (= 42 (last (:results loaded)))
                                  (= 42 @(ns-resolve ns-sym 'answer)))]
        {:id :simple-ns-direct
         :kind :direct-namespace-preparation
         :source src
         :backend-artifact (if (:ok artifact-result)
                             {:ok true
                              :ns-form-mode (:ns-form-mode artifact)
                              :body-count (:body-count artifact)
                              :loadable (:loadable artifact)}
                             (dissoc artifact-result :value))
         :backend-result (if (:ok loaded-result)
                           {:ok true
                            :last-result (last (:results loaded))}
                           loaded-result)
         :gate/verdict (if ok? :accepted :rejected)
         :boundary :none-simple-ns-no-require-import
         :ok ok?})
      (finally
        (remove-ns ns-sym)))))

(defn- ns-require-import-direct-row
  "require/import clause 가 있는 ns form 도 host clojure.lang.Compiler 0회로 backend
  컴파일된다(`compile-ns` :direct-compiled). :import op 직접 emit + in-ns/refer/require
  의 analyzer expansion 을 backend 가 emit 하고, 클래스 등록/네임스페이스 로드의 runtime
  side-effect 만 host runtime-lib 위임(§13). import 등록 + require 별칭 동작을 확인한다."
  []
  (let [ns-sym 'pnix.clj-meta.language-surface.ns-ri-direct
        src (str "(ns pnix.clj-meta.language-surface.ns-ri-direct\n"
                 "  (:require [clojure.string :as s])\n"
                 "  (:import [java.util ArrayList]))\n"
                 "(def joined (s/join \"-\" [\"x\" \"y\"]))\n"
                 "(def cls (.getName ArrayList))\n"
                 "[joined cls]")]
    (try
      (remove-ns ns-sym)
      (let [artifact-result (try-val #(comp/compile-ns src {:file "language_surface_ns_ri.clj"}))
            loaded-result   (if (:ok artifact-result)
                              (try-val #(comp/load-compiled-ns (:value artifact-result)))
                              {:ok false
                               :message "compile-ns artifact unavailable"})
            artifact        (:value artifact-result)
            loaded          (:value loaded-result)
            ok?             (and (:ok artifact-result)
                                  (:ok loaded-result)
                                  (= :direct-compiled (:ns-form-mode artifact))
                                  (= ["x-y" "java.util.ArrayList"]
                                     (last (:results loaded))))]
        {:id :ns-require-import-direct
         :kind :direct-namespace-preparation
         :source src
         :backend-artifact (if (:ok artifact-result)
                             {:ok true
                              :ns-form-mode (:ns-form-mode artifact)
                              :body-count (:body-count artifact)
                              :loadable (:loadable artifact)}
                             (dissoc artifact-result :value))
         :backend-result (if (:ok loaded-result)
                           {:ok true
                            :last-result (last (:results loaded))}
                           loaded-result)
         :gate/verdict (if ok? :accepted :rejected)
         :boundary :host-runtime-lib-namespace-side-effect
         :ok ok?})
      (finally
        (remove-ns ns-sym)))))

(defn- boundary-row
  [{:keys [id form ns-sym required-ops held-reason]}]
  (let [analysis (try-val #(analyze-form ns-sym form))
        ops (when (:ok analysis)
              (ast-ops (:value analysis)))
        ok? (and (:ok analysis)
                 (contains-ops? ops required-ops))]
    {:id id
     :kind :host-side-effect-boundary
     :form (pr-str form)
     :required-ops (sorted-ops required-ops)
     :observed-ops (or ops {})
     :analysis (if (:ok analysis)
                 {:ok true}
                 (dissoc analysis :value))
     :gate/verdict (if ok? :held :rejected)
     :held-reason held-reason
     :promotion/allowed? false
     :ok ok?}))

(defn- boundary-fixtures
  []
  ;; 모든 named-type/definition 표면(reify/deftype/defrecord/defmulti/defprotocol)이 직접 emit.
  ;; 남은 held-boundary 없음(빈 목록). 새 boundary 가 생기면 여기에 추가한다.
  [])

(defn run
  []
  (let [direct-ids #{:protocol-invoke-direct
                     :letfn-mutual-recursion-direct
                     :reify-object-method-direct
                     :reify-interface-method-direct
                     :reify-capture-method-direct
                     :reify-iobj-general-direct
                     :deftype-direct
                     :defrecord-direct
                     :defmulti-direct
                     :defprotocol-direct
                     :defprotocol-deftype-direct
                     :simple-ns-direct
                     :ns-require-import-direct}
        rows (into [(protocol-invoke-row)
                    (letfn-direct-row)
                    (reify-object-direct-row)
                    (reify-interface-direct-row)
                    (reify-capture-direct-row)
                    (reify-iobj-general-direct-row)
                    (deftype-direct-row
                     :deftype-direct
                     'pnix.clj-meta.language-surface.deftype-surface
                     (str "(ns pnix.clj-meta.language-surface.deftype-surface)\n"
                          "(deftype SurfaceType [x y] Object (toString [_] (str x \"/\" y)))\n"
                          "[(str (->SurfaceType 3 7)) (.x (->SurfaceType 5 9)) (instance? SurfaceType (->SurfaceType 1 2))]")
                     '(deftype SurfaceType [x y] Object (toString [_] (str x "/" y)))
                     #"SurfaceType"
                     ["3/7" 5 true])
                    (deftype-direct-row
                     :defrecord-direct
                     'pnix.clj-meta.language-surface.defrecord-surface
                     (str "(ns pnix.clj-meta.language-surface.defrecord-surface)\n"
                          "(defrecord SurfaceRec [a b])\n"
                          "[(:a (->SurfaceRec 1 2)) (= (->SurfaceRec 1 2) (->SurfaceRec 1 2)) "
                          "(:c (map->SurfaceRec {:a 1 :b 2 :c 9}))]")
                     '(defrecord SurfaceRec [a b])
                     #"SurfaceRec"
                    [1 true 9])
                    (defmulti-direct-row)
                    (defprotocol-direct-row)
                    (defprotocol-deftype-direct-row)
                    (simple-ns-direct-row)
                    (ns-require-import-direct-row)]
                   (mapv boundary-row (boundary-fixtures)))
        accepted (filter #(= :accepted (:gate/verdict %)) rows)
        held (filter #(= :held (:gate/verdict %)) rows)
        rejected (filter #(= :rejected (:gate/verdict %)) rows)
        canonical (mapv #(select-keys %
                                      [:id
                                       :kind
                                       :required-ops
                                       :observed-ops
                                       :expected
                                       :host-result
                                       :backend-result
                                       :backend-artifact
                                       :gate/verdict
                                       :held-reason
                                       :promotion/allowed?
                                       :ok])
                        rows)
        invariants (sorted-map
                    :protocol-invoke-direct-accepted
                    (= :accepted
                       (:gate/verdict
                        (first (filter #(= :protocol-invoke-direct (:id %))
                                       rows))))
                    :letfn-direct-accepted
                    (= :accepted
                       (:gate/verdict
                        (first (filter #(= :letfn-mutual-recursion-direct (:id %))
                                       rows))))
                    :reify-object-method-direct-accepted
                    (= :accepted
                       (:gate/verdict
                        (first (filter #(= :reify-object-method-direct (:id %))
                                       rows))))
                    :reify-interface-method-direct-accepted
                    (= :accepted
                       (:gate/verdict
                        (first (filter #(= :reify-interface-method-direct (:id %))
                                       rows))))
                    :reify-capture-method-direct-accepted
                    (= :accepted
                       (:gate/verdict
                        (first (filter #(= :reify-capture-method-direct (:id %))
                                       rows))))
                    :reify-iobj-general-direct-accepted
                    (= :accepted
                       (:gate/verdict
                        (first (filter #(= :reify-iobj-general-direct (:id %))
                                       rows))))
                    :deftype-direct-accepted
                    (let [row (first (filter #(= :deftype-direct (:id %)) rows))]
                      (and (= :accepted (:gate/verdict row))
                           (true? (get-in row [:backend-artifact :named-class-emitted]))))
                    ;; defrecord 도 set! :local + typed/mutable 필드 + record multi-constructor 로 직접 emit.
                    :defrecord-direct-accepted
                    (let [row (first (filter #(= :defrecord-direct (:id %)) rows))]
                      (and (= :accepted (:gate/verdict row))
                           (true? (get-in row [:backend-artifact :named-class-emitted]))))
                    :defmulti-direct-accepted
                    (let [row (first (filter #(= :defmulti-direct (:id %)) rows))]
                      (and (= :accepted (:gate/verdict row))
                           (true? (get-in row [:backend-artifact :direct-emit-no-fallback]))))
                    :defprotocol-direct-accepted
                    (let [row (first (filter #(= :defprotocol-direct (:id %)) rows))]
                      (and (= :accepted (:gate/verdict row))
                           (true? (get-in row [:backend-artifact :direct-emit-no-fallback]))))
                    :defprotocol-deftype-direct-accepted
                    (let [row (first (filter #(= :defprotocol-deftype-direct (:id %)) rows))]
                      (and (= :accepted (:gate/verdict row))
                           (true? (get-in row [:backend-artifact :direct-emit-no-fallback]))))
                    :simple-ns-direct-accepted
                    (= :accepted
                       (:gate/verdict
                        (first (filter #(= :simple-ns-direct (:id %))
                                       rows))))
                    :ns-require-import-direct-accepted
                    (= :accepted
                       (:gate/verdict
                        (first (filter #(= :ns-require-import-direct (:id %))
                                       rows))))
                    :definition-boundaries-held
                    (every? #(= :held (:gate/verdict %))
                            (remove #(contains? direct-ids (:id %)) rows))
                    :definition-boundaries-evidence-only
                    (every? false?
                            (map :promotion/allowed?
                                 (remove #(contains? direct-ids (:id %)) rows)))
                    :no-rejected-rows
                    (empty? rejected)
                    :all-rows-ok
                    (every? :ok rows))
        ok? (every? true? (vals invariants))]
    {:schema "pnix.clj-meta.language-surface.receipt.v1"
     :stage [:M12]
     :target-goal "meta-circular stage15/N Clojure compiler"
     :desc "direct protocol invocation, letfn, simple + general-IObj reify, deftype/defrecord named-type emit including user protocol implementation, simple + require/import namespace preparation, and held definition boundaries"
     :status-counts {:accepted (count accepted)
                     :held (count held)
                     :rejected (count rejected)}
     :policy {:direct "protocol invocation, letfn mutual recursion bytecode, reify (simple Object/interface-method with capture fields AND general reify implementing IObj via auto meta/withMeta), and compile-ns namespace preparation (simple AND require/import via :direct-compiled, 0 host clojure.lang.Compiler) may be emitted/prepared directly"
              :runtime-boundary "require/import/in-ns runtime side-effects use host runtime-lib (§13, like clojure.core); the namespace form itself is backend-compiled, not a host Compiler fallback"
              :deftype "deftype AND defrecord named classes are emitted by the clj-meta backend (host Compiler 0회): declared (typed/mutable) fields, full + record multi-constructors, user methods, and set! on mutable fields. The analyzer-defined stub is shadowed by our full class in *dcl*. The analyzer (frontend, host-delegated) still creates that stub during analysis, like macroexpansion"
              :defprotocol-defmulti "defprotocol (definition + extend-protocol + reify/deftype implementation + dispatch) and defmulti/defmethod are emitted directly: AFunction fn base (__methodImplCache), Var constants (#'global-hierarchy), incremental compile-ns (forward refs), and {:k 'sym} const quote-strip (protocol map :on parity with host)"
              :held "none — all named-type/definition surface (reify/deftype/defrecord/defmulti/defprotocol) is direct-emit"
              :not-accepted "definition side effects never become accepted compiler lowering without a separate direct emitter"}
     :rows rows
     :invariants invariants
     :canonical-receipt canonical
     :canonical-receipt-digest (sha256-string (pr-str canonical))
     :ok ok?}))

(defn write-receipt!
  [r]
  (io/make-parents receipt-path)
  (spit receipt-path (with-out-str (pp/pprint r)))
  r)

(defn -main
  [& _]
  (let [r (write-receipt! (run))]
    (doseq [row (:rows r)]
      (println (str "  [" (if (:ok row) "OK" "FAIL") "] "
                    (name (:id row))
                    " -> "
                    (name (:gate/verdict row)))))
    (println (str "language surface: "
                  (if (:ok r) "OK" "FAILED")
                  "  (receipt: " receipt-path
                  ", digest: " (:canonical-receipt-digest r)
                  ")"))
    (shutdown-agents)
    (when-not (:ok r)
      (System/exit 1))))
