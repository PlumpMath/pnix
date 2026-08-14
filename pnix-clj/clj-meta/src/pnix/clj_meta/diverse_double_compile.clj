(ns pnix.clj-meta.diverse-double-compile
  "M11 Diverse Double-Compiling trust witness.

  This is a practical DDC boundary for the current project shape. Host Clojure
  and the clj-meta bytecode backend are intentionally different implementations,
  so byte-identical artifacts are not expected. The first closed claim is
  behavior equivalence for a focused compiler fixture set, with our backend
  artifact digests, the full-source self-compiler transcript, and an explicit
  TCB ledger. Bit-for-bit DDC remains held."
  (:require [clojure.java.io :as io]
            [clojure.pprint :as pp]
            [pnix.clj-meta.compiler :as comp]
            [pnix.clj-meta.conformance :as conf]
            [pnix.clj-meta.frontend-selfhost :as frontend-selfhost]
            [pnix.clj-meta.selfaudit :as audit])
  (:import [java.security MessageDigest]))

(def receipt-path "clj-meta/proof/diverse-double-compile.receipt.edn")

(defn- read-edn-file
  [path]
  (binding [*data-readers* {'object (fn [x] {:unreadable-object x})}]
    (read-string (slurp path))))

(declare full-source-transcript-row)

(defn- sha256-bytes
  [^bytes bs]
  (let [md (MessageDigest/getInstance "SHA-256")]
    (apply str
           (map #(format "%02x" (bit-and 0xff %))
                (.digest md bs)))))

(defn- sha256-string
  [s]
  (sha256-bytes (.getBytes ^String (str s) "UTF-8")))

(defn- class-map-digest
  [classes]
  (sha256-string
   (pr-str
    (mapv (fn [[class-name class-bytes]]
            [class-name (sha256-bytes class-bytes)])
          (sort-by key classes)))))

(defn- try-val
  [f]
  (try
    {:ok true
     :value (f)}
    (catch Throwable t
      {:ok false
       :throwable (.getName (class t))
       :message (.getMessage t)})))

(defn- fixtures
  []
  [{:id :literal
    :form '(fn [] 42)
    :args []
    :expected 42
    :expected-verdict :accepted}
   {:id :checked-long-direct-proof
    :form '(fn [] [(+ 20 22) (- 50 8) (* 6 7)])
    :args []
    :expected [42 42 42]
    :expected-verdict :accepted}
   {:id :loop-accumulator
    :form '(fn []
             (loop [i 0 acc 0]
               (if (< i 10)
                 (recur (+ i 1) (+ acc i))
                 [(+ acc 1) (- 50 acc) (* acc 2)])))
    :args []
    :expected [46 5 90]
    :expected-verdict :accepted}
   {:id :closure-capture
    :form '(fn [n]
             (let [f (fn [x] (+ x n))]
               (f 5)))
    :args [37]
    :expected 42
    :expected-verdict :accepted}
   {:id :case-table-switch
    :form '(fn [n]
             (case n
               1 :one
               2 :two
               :other))
    :args [2]
    :expected :two
    :expected-verdict :accepted}
   {:id :case-lookup-switch
    :form '(fn [k]
             (case k
               :a 1
               :b 2
               :c 3
               :d 4
               :e 5
               :f 6
               :g 7
               :h 8
               0))
    :args [:g]
    :expected 7
    :expected-verdict :accepted}
   {:id :letfn-mutual-recursion
    :form '(fn [n]
             (letfn [(ev? [x] (if (zero? x) true (od? (dec x))))
                     (od? [x] (if (zero? x) false (ev? (dec x))))]
               (ev? n)))
    :args [4]
    :expected true
    :expected-verdict :accepted}])

(defn- replay-fixture
  [{:keys [id form args expected expected-verdict]}]
  (let [host-result     (try-val #(apply (eval form) args))
        backend-result  (try-val #(apply (comp/compile-form form) args))
        backend-classes (try-val #(comp/compile-classes form))
        behavior-ok?    (and (:ok host-result)
                             (:ok backend-result)
                             (= expected (:value host-result))
                             (= (:value host-result)
                                (:value backend-result)))
        artifact-digest (when (:ok backend-classes)
                          (class-map-digest (:value backend-classes)))
        verdict         (cond
                          (not behavior-ok?) :rejected
                          (:ok backend-classes) :accepted
                          :else :held)]
    {:id id
     :kind :behavior-equivalence
     :form (pr-str form)
     :args args
     :expected expected
     :host-result host-result
     :backend-result backend-result
     :backend-artifact
     (if (:ok backend-classes)
       {:ok true
        :class-count (count (:value backend-classes))
        :digest artifact-digest}
       backend-classes)
     :bit-identical-artifact? :not-applicable-different-backends
     :gate/verdict verdict
     :held-reason (when (= :held verdict)
                    :host-fallback-no-backend-artifact)
     :ok (and behavior-ok?
              (= verdict expected-verdict))}))

(defn- drift-sentinel
  []
  {:id :synthetic-behavior-drift
   :kind :drift-sentinel
   :host-result {:ok true :value 42}
   :backend-result {:ok true :value 43}
   :gate/verdict :rejected
   :rejection-reason :behavior-drift-would-fail-closed
   :ok true})

(defn- bit-identical-boundary
  []
  {:id :bit-identical-artifact-ddc
   :kind :trust-boundary
   :gate/verdict :accepted
   :boundary/status :not-applicable
   :boundary/reason :host-and-clj-meta-backends-are-not-bit-identical-targets
   :required-before-accepted
   [:stable-host-artifact-capture
    :same-source-executable-correspondence-proof
    :independent-toolchain-transcript]
   :ok true})

(defn- independent-kernel-supported-row
  "U5/R3: kernel.clj 는 별도 bytecode compiler 가 아니라 tree-walking value-semantics
  모델이다. host clojure.core 를 공유하고 deftype/defrecord/reflection 은 맵 모델로
  JVM 규칙을 모델링한다. conformance corpus 중 지원 행에서 host≡compiler≡kernel 을 고정한다."
  [results]
  (let [checked (filter #(boolean? (:k-ok %)) results)
        held (filter #(= :unsupported (:k-ok %) ) results)
        failed (filter #(false? (:k-ok %)) checked)
        ok? (boolean (and (every? :hc-ok results)
                          (empty? failed)
                          (seq checked)))]
    {:id :independent-kernel-evaluator-supported-corpus
     :kind :partial-independent-value-semantics-model
     :backend :pnix.clj-meta.kernel/k-eval
     :scope "tree-walking value-semantics model; host clojure.core shared; not a separate bytecode compiler"
     :model-boundary
     {:deftype-defrecord "modeled as maps/fake class fields for conformance semantics"
      :reflection "getModifiers/isVolatile/getLookupThunk results are JVM-rule models"
      :independence :partial-value-semantics}
     :not-claimed [:independent-jvm-type-generation
                   :full-bytecode-second-compiler
                   :full-wheeler-ddc]
     :evidence {:host-compiler-total (count results)
                :kernel-checked (count checked)
                :kernel-pass (count (filter #(true? (:k-ok %)) checked))
                :kernel-held (count held)
                :held-forms (mapv #(pr-str (:form %)) held)}
     :gate/verdict (if ok? :accepted :rejected)
     :ok ok?}))

(defn- independent-kernel-typegen-boundary-row
  [results]
  (let [held (filter #(= :unsupported (:k-ok %)) results)]
    {:id :independent-kernel-evaluator-typegen-gap
     :kind :partial-kernel-model-boundary
     :backend :pnix.clj-meta.kernel/k-eval
     :gate/verdict :accepted
     :boundary/status :out-of-scope
     :boundary/reason :kernel-is-tree-walking-map-model-not-full-jvm-type-generation
     :held-count (count held)
     :held-forms (mapv #(pr-str (:form %)) held)
     :not-claimed [:general-independent-deftype-runtime
                   :general-independent-defrecord-runtime
                   :separate-bytecode-compiler
                   :full-jvm-type-generation-second-backend]
     :ok true}))

(defn- mini-backend-ddc-fixtures
  []
  [{:id :mini-backend-arithmetic
    :source "(fn [x] (* (+ x 1) 2))"
    :args [20]
    :expected 42}
   {:id :mini-backend-let-branch
    :source "(fn [x] (let [y (+ x 1)] (if (< y 50) y 0)))"
    :args [41]
    :expected 42}
   {:id :mini-backend-loop-recur
    :source "(fn [] (loop [i 0 acc 0] (if (< i 6) (recur (+ i 1) (+ acc 7)) acc)))"
    :args []
    :expected 42}
   {:id :mini-backend-macro-thread-first
    :source "(fn [x] (-> x (+ 1) (* 2)))"
    :args [20]
    :expected 42}
   {:id :mini-backend-macro-thread-last
    :source "(fn [x] (->> x (+ 1) (* 2)))"
    :args [20]
    :expected 42}
   {:id :mini-backend-macro-cond
    :source "(fn [x] (cond (< x 0) :neg (< x 10) :small :else :big))"
    :args [5]
    :expected :small}
   {:id :mini-backend-macro-if-let
    :source "(fn [x] (if-let [y (+ x 1)] y 0))"
    :args [41]
    :expected 42}
   {:id :mini-backend-macro-when-not-not
    :source "(fn [x] (when-not (not (< x 50)) (+ x 1)))"
    :args [41]
    :expected 42}
   {:id :mini-backend-op-comparisons
    :source "(fn [x] (if (and (> x 40) (>= x 41) (<= x 41)) 42 0))"
    :args [41]
    :expected 42}
   {:id :mini-backend-op-quot-rem
    :source "(fn [] (+ (quot 80 2) (rem 102 100)))"
    :args []
    :expected 42}
   {:id :mini-backend-op-unary
    :source "(fn [x] (if (pos? x) (dec (inc (inc x))) 0))"
    :args [41]
    :expected 42}
   {:id :mini-backend-seq-ops
    :source "(fn [v] (+ (first v) (first (next v))))"
    :args [[20 22 99]]
    :expected 42}
   {:id :mini-backend-get
    :source "(fn [m] (get m :answer))"
    :args [{:answer 42}]
    :expected 42}
   {:id :mini-backend-destructure
    :source "(fn [v] (let [[a [b c]] v] (+ a (+ b c))))"
    :args [[10 [20 12]]]
    :expected 42}
   {:id :mini-backend-do-body
    :source "(fn [] (do (+ 1 2) (* 6 7)))"
    :args []
    :expected 42}
   {:id :mini-backend-let-shadowing
    :source "(fn [x] (let [x (+ x 1)] (if (< x 10) (* x 6) (- x 1))))"
    :args [6]
    :expected 42}
   {:id :mini-backend-boolean-const-if
    :source "(fn [] (if true 42 0))"
    :args []
    :expected 42}
   {:id :mini-backend-nil-falsey
    :source "(fn [] (if nil 0 42))"
    :args []
    :expected 42}
   {:id :mini-backend-equality-branch
    :source "(fn [x] (if (= x 7) 42 0))"
    :args [7]
    :expected 42}
   {:id :mini-backend-vector-literal
    :source "(fn [x] [x (+ x 1) true nil])"
    :args [40]
    :expected [40 41 true nil]}
   {:id :mini-backend-string-keyword-vector
    :source "(fn [] [\"ok\" :answer])"
    :args []
    :expected ["ok" :answer]}
   {:id :mini-backend-map-literal
    :source "(fn [x] {:answer x :label \"ok\" :flag true})"
    :args [42]
    :expected {:answer 42 :label "ok" :flag true}}
   {:id :mini-backend-set-literal
    :source "(fn [x] #{x 7 42})"
    :args [41]
    :expected #{41 7 42}}
   {:id :mini-backend-quoted-symbol
    :source "(fn [] (quote answer))"
    :args []
    :expected 'answer}
   {:id :mini-backend-quoted-list
    :source "(fn [] (quote (+ 1 2)))"
    :args []
    :expected '(+ 1 2)}
   {:id :mini-backend-quoted-nested-data
    :source "(fn [] (quote {:op answer :xs [1 2] :call (+ 1 2)}))"
    :args []
    :expected '{:op answer :xs [1 2] :call (+ 1 2)}}
   {:id :mini-backend-macro-when
    :source "(fn [x] (when (< x 50) (+ x 1)))"
    :args [41]
    :expected 42}
   {:id :mini-backend-macro-and
    :source "(fn [x] (and true (< x 50) (+ x 1)))"
    :args [41]
    :expected 42}
   {:id :mini-backend-macro-or
    :source "(fn [x] (or nil false (+ x 1)))"
    :args [41]
    :expected 42}
   {:id :mini-backend-macro-not
    :source "(fn [x] (if (not (< x 0)) (+ x 1) 0))"
    :args [41]
    :expected 42}
   {:id :mini-backend-macro-nil?
    :source "(fn [x] (if (nil? x) 0 (+ x 1)))"
    :args [41]
    :expected 42}
   {:id :mini-backend-macro-when-let
    :source "(fn [x] (when-let [y (+ x 1)] y))"
    :args [41]
    :expected 42}
   {:id :mini-backend-macro-if-not
    :source "(fn [x] (if-not (< x 0) (+ x 1) 0))"
    :args [41]
    :expected 42}
   {:id :mini-backend-macro-as->
    :source "(fn [x] (as-> x v (+ v 1) (* v 2)))"
    :args [20]
    :expected 42}
   {:id :mini-backend-macro-cond->
    :source "(fn [x] (cond-> x (< x 50) (+ 1) (> x 100) (* 2)))"
    :args [41]
    :expected 42}
   {:id :mini-backend-macro-cond->>
    :source "(fn [x] (cond->> x (< x 50) (+ 1)))"
    :args [41]
    :expected 42}
   {:id :mini-backend-macro-some->
    :source "(fn [x] (some-> x (+ 1) (* 2)))"
    :args [20]
    :expected 42}
   {:id :mini-backend-macro-some->-nil
    :source "(fn [x] (some-> x (+ 1)))"
    :args [nil]
    :expected nil}
   {:id :mini-backend-macro-some->>
    :source "(fn [x] (some->> x (+ 1)))"
    :args [41]
    :expected 42}
   {:id :mini-backend-destructure-plain
    :source "(fn [v] (let [[a b] v] (+ a b)))"
    :args [[20 22]]
    :expected 42}
   {:id :mini-backend-destructure-rest-positions
    :source "(fn [v] (let [[a b c] v] (if (nil? c) (+ a b) (+ a (+ b c)))))"
    :args [[20 22]]
    :expected 42}
   {:id :mini-backend-op-zero?
    :source "(fn [x] (if (zero? x) 42 0))"
    :args [0]
    :expected 42}
   {:id :mini-backend-op-neg?
    :source "(fn [x] (if (neg? x) 0 42))"
    :args [5]
    :expected 42}
   {:id :mini-backend-branch-two-arg
    :source "(fn [x y] (if (< x y) (* (+ x 1) y) (- x y)))"
    :args [5 7]
    :expected 42}
   {:id :mini-backend-let-sequential
    :source "(fn [x] (let [a (+ x 1) b (* a 2)] b))"
    :args [20]
    :expected 42}
   {:id :mini-backend-macro-when-not
    :source "(fn [x] (when-not (< x 0) (+ x 1)))"
    :args [41]
    :expected 42}
   {:id :mini-backend-multi-arity-one-arg
    :source "(fn ([x] x) ([x y] (+ x y)))"
    :args [42]
    :expected 42}
   {:id :mini-backend-multi-arity-two-arg
    :source "(fn ([x] x) ([x y] (+ x y)))"
    :args [20 22]
    :expected 42}
   {:id :mini-backend-multi-arity-three-way-zero-arg
    :source "(fn ([] 42) ([x] x) ([x y] (+ x y)))"
    :args []
    :expected 42}
   {:id :mini-backend-multi-arity-three-way-two-arg
    :source "(fn ([] 42) ([x] x) ([x y] (+ x y)))"
    :args [20 22]
    :expected 42}
   {:id :mini-backend-variadic-one-fixed
    :source "(fn [a & r] r)"
    :args [1 2 3]
    :expected '(2 3)}
   {:id :mini-backend-variadic-two-fixed
    :source "(fn [a b & r] [a b r])"
    :args [1 2 3 4]
    :expected [1 2 '(3 4)]}
   {:id :mini-backend-op-count-variadic
    :source "(fn [& r] (count r))"
    :args [1 2 3]
    :expected 3}
   {:id :mini-backend-macro-case-int
    :source "(fn [n] (case n 1 :one 2 :two :other))"
    :args [2]
    :expected :two}
   {:id :mini-backend-macro-case-keyword
    :source "(fn [k] (case k :a 1 :b 2 :c 3 :d 4 :e 5 :f 6 :g 7 :h 8 0))"
    :args [:g]
    :expected 7}
   {:id :mini-backend-try-catch-no-throw
    :source "(fn [x] (try (quot 10 x) (catch ArithmeticException e :divzero)))"
    :args [2]
    :expected 5}
   {:id :mini-backend-try-catch-caught
    :source "(fn [x] (try (quot 10 x) (catch ArithmeticException e :divzero)))"
    :args [0]
    :expected :divzero}
   {:id :mini-backend-throw-one-arg-constructor
    :source "(fn [] (try (throw (IllegalArgumentException. \"boom\")) (catch IllegalArgumentException e (nil? e))))"
    :args []
    :expected false}
   {:id :mini-backend-macro-case-no-default-throws
    :source "(fn [n] (try (case n 1 :one 2 :two) (catch IllegalArgumentException e :no-match)))"
    :args [99]
    :expected :no-match}
   {:id :mini-backend-interop-string-length
    :source "(fn [s] (.length s))"
    :args ["hello"]
    :expected 5}
   {:id :mini-backend-interop-equals
    :source "(fn [a b] (.equals a b))"
    :args [1 2]
    :expected false}
   {:id :mini-backend-static-interop-math-sqrt
    :source "(fn [x] (Math/sqrt x))"
    :args [16.0]
    :expected 4.0}
   {:id :mini-backend-static-interop-ambiguous-rejected
    :source "(fn [a b] (try (Math/max a b) (catch IllegalArgumentException e :ambiguous)))"
    :args [1 2.0]
    :expected :ambiguous}
   ;; Only the pure-value try/finally fixtures are wired here, not the
   ;; AtomicInteger-mutation ones from frontend_selfhost.clj's own fixture
   ;; set: this row applies the SAME `args` vector to all three legs
   ;; (host/compiler/mini) in sequence, so a shared mutable arg would
   ;; accumulate cross-leg mutation and make the comparison meaningless
   ;; rather than a real check.
   {:id :mini-backend-try-finally-normal-path-value
    :source "(fn [a] (try a (finally 99)))"
    :args [42]
    :expected 42}
   {:id :mini-backend-try-finally-nested-in-try-catch-exceptional
    :source "(fn [x] (try (try (quot 10 x) (finally :ignored)) (catch ArithmeticException e :caught)))"
    :args [0]
    :expected :caught}
   ;; Constant, side-effect-free finally bodies here (not the mutable
   ;; AtomicInteger fixtures from frontend_selfhost.clj's own set) for the
   ;; same reason noted above the try-finally rows: this row applies the
   ;; same `args` vector to all three legs in sequence, so mutable state
   ;; shared across legs would make the comparison meaningless.
   {:id :mini-backend-try-catch-finally-normal-path
    :source "(fn [x] (try (quot 10 x) (catch ArithmeticException e :divzero) (finally 99)))"
    :args [2]
    :expected 5}
   {:id :mini-backend-try-catch-finally-caught-path
    :source "(fn [x] (try (quot 10 x) (catch ArithmeticException e :divzero) (finally 99)))"
    :args [0]
    :expected :divzero}
   {:id :mini-backend-str-two-args
    :source "(fn [a b] (str a b))"
    :args ["hello" "world"]
    :expected "helloworld"}
   {:id :mini-backend-str-nil-arg-is-empty
    :source "(fn [a b] (str a b))"
    :args [nil "x"]
    :expected "x"}
   ;; Read-only field access, not the set!/mutation fixture from
   ;; frontend_selfhost.clj's own set -- same shared-mutable-arg-across-legs
   ;; reason as the try/finally rows above.
   {:id :mini-backend-field-get
    :source "(fn [p] (.-x p))"
    :args [(java.awt.Point. 7 9)]
    :expected 7}
   ;; Exceptional-path only, not the normal-path fixture from
   ;; frontend_selfhost.clj's own set: normal-path uses a StringBuilder
   ;; mutated by .append, and (same reason as the other shared-mutable-arg
   ;; rows above) applying that same builder across all three sequential
   ;; legs would accumulate appended text and break the comparison. This
   ;; row's arg is a plain Object with no observed mutation, so it is safe.
   {:id :mini-backend-locking-exceptional-path
    :source "(fn [lock x] (try (locking lock (quot 10 x)) (catch ArithmeticException e :caught)))"
    :args [(Object.) 0]
    :expected :caught}
   {:id :mini-backend-try-multi-catch-first-clause-triggered
    :source "(fn [x] (try (quot 10 x) (catch ArithmeticException e :divzero) (catch IllegalArgumentException e :bad-arg)))"
    :args [0]
    :expected :divzero}
   {:id :mini-backend-try-multi-catch-second-clause-triggered
    :source "(fn [] (try (throw (IllegalArgumentException. \"bad\")) (catch ArithmeticException e :divzero) (catch IllegalArgumentException e :bad-arg)))"
    :args []
    :expected :bad-arg}
   ;; Constant, side-effect-free finally bodies here, not the mutable
   ;; AtomicInteger fixtures from frontend_selfhost.clj's own set -- same
   ;; shared-mutable-arg-across-legs reason as the other try/finally rows.
   {:id :mini-backend-try-multi-catch-finally-normal-path
    :source "(fn [x] (try (quot 10 x) (catch ArithmeticException e :divzero) (catch IllegalArgumentException e :bad-arg) (finally 99)))"
    :args [2]
    :expected 5}
   {:id :mini-backend-try-multi-catch-finally-second-clause
    :source "(fn [] (try (throw (IllegalArgumentException. \"bad\")) (catch ArithmeticException e :divzero) (catch IllegalArgumentException e :bad-arg) (finally 99)))"
    :args []
    :expected :bad-arg}
   {:id :mini-backend-general-new-unique-arity
    :source "(fn [x y] (.-x (java.awt.Point. x y)))"
    :args [7 9]
    :expected 7}
   {:id :mini-backend-general-new-ambiguous-arity
    :source "(fn [n] (.size (java.util.ArrayList. n)))"
    :args [4]
    :expected 0}
   {:id :mini-backend-general-static-interop-digit
    :source "(fn [c] (java.lang.Character/isDigit c))"
    :args [\5]
    :expected true}
   {:id :mini-backend-bigint-literal-beyond-long-range
    :source "(fn [] 10000000000000000000N)"
    :args []
    :expected 10000000000000000000N}
   {:id :mini-backend-bigint-arithmetic-beyond-long-overflow
    :source "(fn [] (+ 9223372036854775807N 1N))"
    :args []
    :expected 9223372036854775808N}
   {:id :mini-backend-bigdec-arithmetic
    :source "(fn [] (* 1.5M 2))"
    :args []
    :expected 3.0M}
   {:id :mini-backend-regex-literal-pattern-source
    :source "(fn [] (.pattern #\"a+\"))"
    :args []
    :expected "a+"}
   {:id :mini-backend-ratio-literal
    :source "(fn [] 1/3)"
    :args []
    :expected 1/3}
   {:id :mini-backend-ratio-arithmetic
    :source "(fn [] (+ 1/3 1/3))"
    :args []
    :expected 2/3}
   {:id :mini-backend-binding-value-inside
    :source "(fn [] (binding [pnix.clj-meta.frontend-selfhost/*tiny-dynamic-var* 42] pnix.clj-meta.frontend-selfhost/*tiny-dynamic-var*))"
    :args []
    :expected 42}
   {:id :mini-backend-binding-reverts-after-normal-exit
    :source "(fn [] (do (binding [pnix.clj-meta.frontend-selfhost/*tiny-dynamic-var* 42] nil) pnix.clj-meta.frontend-selfhost/*tiny-dynamic-var*))"
    :args []
    :expected :tiny-dynamic-var-root}
   {:id :mini-backend-binding-reverts-after-exceptional-exit
    :source "(fn [] (do (try (binding [pnix.clj-meta.frontend-selfhost/*tiny-dynamic-var* 99] (throw (RuntimeException.))) (catch RuntimeException e nil)) pnix.clj-meta.frontend-selfhost/*tiny-dynamic-var*))"
    :args []
    :expected :tiny-dynamic-var-root}])

(defn- mini-backend-case-row
  [{:keys [id source args expected]}]
  (let [form           (read-string source)
        host-result    (try-val #(apply (eval form) args))
        backend-result (try-val #(apply (comp/compile-form form) args))
        mini-artifact  (try-val #(frontend-selfhost/compile-source source))
        mini-result    (when (:ok mini-artifact)
                         (try-val #(apply (:fn (:value mini-artifact)) args)))
        mini-class     (when (:ok mini-artifact)
                         (select-keys (:artifact (:value mini-artifact))
                                      [:class-name :digest]))
        ok?            (and (:ok host-result)
                            (:ok backend-result)
                            (:ok mini-result)
                            (= expected (:value host-result))
                            (= (:value host-result) (:value backend-result))
                            (= (:value host-result) (:value mini-result)))]
    {:id id
     :source source
     :source-hash (sha256-string source)
     :args args
     :expected expected
     :host-result host-result
     :backend-result backend-result
     :mini-backend-result mini-result
     :mini-backend-artifact mini-class
     :ok ok?}))

(defn- independent-mini-backend-row
  "R4 slice: frontend_selfhost 의 tiny compiler 는 compiler.clj 의 analyzer/range
  recognizer/emit helper 를 공유하지 않는 별도 reader+macroexpander+analyzer+ASM emitter 다.
  subset 3-way behavior(host≡compiler.clj backend≡mini backend)를 accepted 로 올리되,
  full Wheeler DDC 는 계속 held 로 남긴다."
  []
  (frontend-selfhost/reset-compiler-state!)
  (let [case-rows (mapv mini-backend-case-row (mini-backend-ddc-fixtures))
        ok?       (and (seq case-rows) (every? :ok case-rows))]
    {:id :independent-mini-backend-subset
     :kind :independent-compiler-subset-ddc
     :backend :pnix.clj-meta.frontend-selfhost/compile-source
     :scope "self-owned reader + rewrite macroexpander + analyzer + direct ASM emitter; no compiler.clj range recognizer or emit helper reuse"
     :claim "host reference, production clj-meta backend, and independent mini backend agree on the covered subset"
     :evidence {:fixture-count (count case-rows)
                :fixtures case-rows
                :independence
                {:uses-compiler-clj-backend false
                 :uses-tools-analyzer-jvm false
                 :uses-clojure-reader false
                 :uses-host-macroexpand false
                 :shared-runtime [:jvm :clojure-lang-numbers-rt]}}
     :not-claimed [:full-wheeler-ddc
                   :compiler-binary-ddc
                   :production-frontend-replacement
                   :full-clojure-runtime-replacement]
     :gate/verdict (if ok? :accepted :rejected)
     :ok ok?}))

(defn- cached-full-source-transcript-row
  []
  (try
    (let [f (io/file receipt-path)]
      (when (.exists f)
        (when-let [row (first (filter #(= :full-source-self-compiler-transcript
                                          (:id %))
                                      (:rows (read-edn-file receipt-path))))]
          (assoc row
                 :evidence-source :cached-diverse-double-compile-receipt
                 :cache-source receipt-path))))
    (catch Throwable _
      nil)))

(defn- standalone-full-source-transcript-row
  []
  (or (cached-full-source-transcript-row)
      (assoc (full-source-transcript-row (audit/run))
             :evidence-source :fresh-audit-run)))

(defn- compile-ns-source
  [ns-sym]
  (str "(ns " ns-sym ")\n"
       "(def base 40)\n"
       "(defn inc2 [x] (+ x 2))\n"
       "(def answer (inc2 base))\n"
       "answer"))

(defn- host-load-string-result
  [ns-sym src]
  (remove-ns ns-sym)
  (let [last-result (load-string src)]
    {:last-result last-result
     :answer @(ns-resolve ns-sym 'answer)}))

(defn- backend-compile-ns-result
  [ns-sym src]
  (remove-ns ns-sym)
  (let [artifact (comp/compile-ns src {:file "ddc_compile_ns_direct_simple.clj"})
        loaded (comp/load-compiled-ns artifact)]
    {:last-result (last (:results loaded))
     :answer @(ns-resolve ns-sym 'answer)
     :artifact {:ns-form-mode (:ns-form-mode artifact)
                :body-count (:body-count artifact)
                :loadable (:loadable artifact)}}))

(defn- compile-ns-direct-simple-row
  []
  (let [host-ns 'pnix.clj-meta.ddc.compile-ns-host
        backend-ns 'pnix.clj-meta.ddc.compile-ns-backend
        host-src (compile-ns-source host-ns)
        backend-src (compile-ns-source backend-ns)
        host-result (try-val #(host-load-string-result host-ns host-src))
        backend-result (try-val #(backend-compile-ns-result backend-ns backend-src))
        ok? (and (:ok host-result)
                 (:ok backend-result)
                 (= {:last-result 42 :answer 42} (:value host-result))
                 (= (:value host-result)
                    (select-keys (:value backend-result) [:last-result :answer]))
                 (= :direct-simple
                    (get-in backend-result [:value :artifact :ns-form-mode])))]
    {:id :compile-ns-direct-simple-transcript
     :kind :namespace-artifact-equivalence
     :source-hash (sha256-string host-src)
     :host-result host-result
     :backend-result backend-result
     :backend-artifact (when (:ok backend-result)
                         (:artifact (:value backend-result)))
     :gate/verdict (if ok? :accepted :rejected)
     :ok ok?}))

(defn- require-import-ns-source
  [ns-sym]
  (str "(ns " ns-sym "\n"
       "  (:require [clojure.string :as s])\n"
       "  (:import [java.util ArrayList]))\n"
       "(def answer (count (s/split (.getName ArrayList) #\"\\.\")))\n"
       "answer"))

(defn- backend-compile-ns-require-import-result
  [ns-sym src]
  (remove-ns ns-sym)
  (let [artifact (comp/compile-ns src {:file "ddc_compile_ns_require_import.clj"})
        loaded (comp/load-compiled-ns artifact)]
    {:last-result (last (:results loaded))
     :answer @(ns-resolve ns-sym 'answer)
     :artifact {:ns-form-mode (:ns-form-mode artifact)
                :body-count (:body-count artifact)
                :loadable (:loadable artifact)}}))

(defn- compile-ns-require-import-row
  "require/import clause 가 있는 ns artifact 의 host(load-string) ≡ clj-meta backend
  (compile-ns :direct-compiled) behavior-equivalence. ns form 도 0 host Compiler 로
  backend 컴파일되며 require 별칭/import 클래스가 동작함을 host 결과와 일치로 확인한다."
  []
  (let [host-ns 'pnix.clj-meta.ddc.compile-ns-ri-host
        backend-ns 'pnix.clj-meta.ddc.compile-ns-ri-backend
        host-src (require-import-ns-source host-ns)
        backend-src (require-import-ns-source backend-ns)
        host-result (try-val #(host-load-string-result host-ns host-src))
        backend-result (try-val #(backend-compile-ns-require-import-result backend-ns backend-src))
        ok? (and (:ok host-result)
                 (:ok backend-result)
                 (= {:last-result 3 :answer 3} (:value host-result))
                 (= (:value host-result)
                    (select-keys (:value backend-result) [:last-result :answer]))
                 (= :direct-compiled
                    (get-in backend-result [:value :artifact :ns-form-mode])))]
    {:id :compile-ns-require-import-transcript
     :kind :namespace-artifact-equivalence
     :source-hash (sha256-string host-src)
     :host-result host-result
     :backend-result backend-result
     :backend-artifact (when (:ok backend-result)
                         (:artifact (:value backend-result)))
     :gate/verdict (if ok? :accepted :rejected)
     :ok ok?}))

(defn- smoke-by-desc
  [audit-report desc]
  (first (filter #(= desc (:desc %)) (:compiled-execution-smokes audit-report))))

(defn- full-source-transcript-row
  [audit-report]
  (let [stage-target (get audit-report :bytecode-stage-target)
        impl         (smoke-by-desc audit-report
                                    "compiled self-source compiler implementation vars")
        full         (smoke-by-desc audit-report
                                    "compiled full namespace artifact/load")
        chain        (get audit-report :generated-stage-chain)
        disk         (get audit-report :generated-stage-chain-disk-proof)
        stage9       (get audit-report :stage9-clean-process)
        stage10      (get audit-report :stage10-isolated-matrix)
        ok?          (every? :ok [audit-report
                                  stage-target
                                  impl
                                  full
                                  chain
                                  disk
                                  stage9
                                  stage10])]
    {:id :full-source-self-compiler-transcript
     :kind :full-source-ddc-evidence
     :claim "compiler.clj self-source is replayed through generated compiler vars, disk reload, clean process, and isolated cwd/classpath matrix"
     :evidence
     {:source (:source audit-report)
      :top-level-forms (:top-level-forms audit-report)
      :analyzed-forms (:analyzed-forms audit-report)
      :host-maintained-present (:host-maintained-present audit-report)
      :stage-target
      {:ok (:ok stage-target)
       :classes (:classes stage-target)
       :full-source-forms (:full-source-forms stage-target)
       :stage-target-safe-forms (:stage-target-safe-forms stage-target)
       :host-side-effect-forms (:host-side-effect-forms stage-target)}
      :compiled-impl
      {:ok (:ok impl)
       :same-result? (:same-result? impl)
       :same-bytecode? (:same-bytecode? impl)
       :generated-vars? (:generated-vars? impl)}
      :compiled-full-namespace
      {:ok (:ok full)
       :same-result? (:same-result? full)
       :same-bytecode? (:same-bytecode? full)
       :generated? (:generated? full)}
      :stage-chain
      {:ok (:ok chain)
       :stages (:stages chain)
       :fixed-point (:fixed-point chain)
       :matches-host-reference? (:matches-host-reference? chain)
       :fresh-stage8-exact? (:fresh-stage8-exact? chain)}
      :disk-reload
      {:ok (:ok disk)
       :compare-ok? (:compare-ok? disk)
       :fresh-stage8-exact? (:fresh-stage8-exact? disk)}
      :clean-process
      {:ok (:ok stage9)
       :canonical-receipt-digest (:canonical-receipt-digest stage9)
       :artifact-match? (:artifact-match? stage9)}
      :isolated-matrix
      {:ok (:ok stage10)
       :cwd-policy (:cwd-policy stage10)
       :variants (count (:rows stage10))
       :canonical-fixed? (:canonical-fixed? stage10)
       :artifact-fixed? (:artifact-fixed? stage10)}}
     :does-not-close [:bit-identical-artifact-ddc
                      :self-perpetuating-compiler-backdoor
                      :independent-toolchain-transcript]
     :gate/verdict (if ok? :accepted :held)
     :held-reason (when-not ok?
                    :full-source-self-compiler-transcript-incomplete)
     :ok ok?}))

(defn- cross-host-ddc-row
  "cross-host-ddc evidence lane(별도 subprocess)의 receipt 를 읽어, 두 독립 clojure.lang.
  Compiler 버전(1.11.1/1.12.0)이 우리 backend 로 bit-identical target bytecode 를 냈는지
  기록한다. 이는 genuine 한 cross-host emit determinism(부분 Trusting-Trust 증거)다. lane
  미실행이면 held(not-run; gate 를 막지 않음), 실행됐고 diverged 면 rejected(실패)."
  []
  (let [path "clj-meta/proof/cross-host-ddc.receipt.edn"
        receipt (try
                  (when (.exists (io/file path))
                    (read-string (slurp path)))
                  (catch Throwable _ nil))]
    (cond
      (nil? receipt)
      {:id :cross-host-emit-ddc
       :kind :cross-host-emit-determinism
       :evidence {:lane-run? false}
       :gate/verdict :unavailable
       :unavailable-reason :cross-host-ddc-lane-not-run
       :ok true}

      (:ok receipt)
      {:id :cross-host-emit-ddc
       :kind :cross-host-emit-determinism
       :evidence {:lane-run? true
                  :bit-identical? true
                  :host-versions (get-in receipt [:canonical-receipt :host-versions])
                  :cross-host-ddc-digest (:canonical-receipt-digest receipt)}
       :gate/verdict :accepted
       :ok true}

      :else
      {:id :cross-host-emit-ddc
       :kind :cross-host-emit-determinism
       :evidence {:lane-run? true :bit-identical? false}
       :gate/verdict :rejected
       :held-reason :cross-host-emit-diverged
       :ok false})))

(defn- trust-gap-ledger
  []
  [{:id :behavior-equivalence-fixtures
    :claim "host eval/Compiler reference and clj-meta backend agree on the focused fixture corpus"
    :closes [:fixture-behavior-drift :backend-result-regression]
    :does-not-close [:full-source-executable-correspondence
                     :self-perpetuating-compiler-backdoor
                     :bit-identical-artifact-ddc]
    :status :partial}
   {:id :backend-artifact-digests
    :claim "accepted backend fixture classes, including case switch artifacts, have stable artifact digests"
    :closes [:unexpected-bytecode-shape-change-in-fixtures]
    :does-not-close [:host-backend-bit-identity
                     :source-level-ddc-correspondence]
    :status :partial}
   {:id :full-source-self-compiler-transcript
    :claim "compiler.clj self-source is replayed by generated compiler vars through stage8/stage9/stage10 receipts"
    :closes [:self-source-replay-transcript
             :fresh-reload-compiler-artifact-evidence
             :clean-process-compiler-replay-evidence]
    :does-not-close [:independent-toolchain-transcript
                     :bit-identical-artifact-ddc
                     :trusting-trust-gap]
    :status :evidence-only}
   {:id :cross-host-emit-ddc
    :claim "our backend emits bit-identical fixed-target bytecode under two independent clojure.lang.Compiler versions (1.11.1, 1.12.0)"
    :closes [:cross-host-emit-determinism
             :host-version-independent-codegen]
    :does-not-close [:full-compiler-binary-ddc
                     :trusting-trust-gap]
    :status :partial}
   {:id :bit-identical-ddc-open
    :claim "host Compiler and clj-meta backend are intentionally different targets, so full compiler-binary bit identity is not claimed; cross-host lane gives only partial (cross-version) independent-toolchain evidence"
    :closes []
    :does-not-close [:trusting-trust-gap]
    :required-before-closed
    [:stable-host-artifact-capture
     :fully-independent-compiler-binary-transcript
     :same-source-executable-correspondence-proof]
    :status :open}
   {:id :independent-kernel-evaluator
    :claim "kernel.clj is a partially independent tree-walking value-semantics model for the supported conformance subset; it shares host clojure.core and models deftype/defrecord/reflection with maps, not a separate bytecode compiler"
    :closes [:supported-corpus-independent-behavior-crosscheck]
    :does-not-close [:jvm-type-generation-second-backend
                     :separate-bytecode-compiler
                     :full-compiler-binary-ddc
                     :trusting-trust-gap]
    :status :partial}
   {:id :independent-mini-backend-subset
    :claim "frontend_selfhost is an independent tiny compiler for a covered source subset, checked by host≡compiler.clj backend≡mini backend behavior"
    :closes [:subset-independent-second-compiler-evidence]
    :does-not-close [:full-wheeler-ddc
                     :compiler-binary-ddc
                     :production-frontend-replacement
                     :trusting-trust-gap]
    :status :partial}
   {:id :reproducible-build-lane
    :claim "stock Clojure reproducible-build lane is independent toolchain evidence only"
    :closes [:deterministic-stock-clojure-stage-chain]
    :does-not-close [:clj-meta-backend-bit-identity
                     :pnix-clj-launcher-admission]
    :status :evidence-only}])

(defn- tcb-ledger
  []
  {:jvm true
   :dynamic-class-loader true
   :tools-analyzer-jvm true
   :host-reader true
   :host-clojure-core true
   :host-compiler-reference true
   :clj-meta-bytecode-backend true
   :frontend-selfhost-tiny-compiler true
   :outside-logic true})

(defn run
  ([]
   (run nil (standalone-full-source-transcript-row)))
  ([audit-report]
   (run audit-report nil))
  ([audit-report cached-full-source-row]
  (let [conf-results (conf/run)
        full-row    (or cached-full-source-row
                        (assoc (full-source-transcript-row audit-report)
                               :evidence-source :provided-audit-report))
        rows       (into (mapv replay-fixture (fixtures))
                         [(compile-ns-direct-simple-row)
                          (compile-ns-require-import-row)
                          full-row
                          (cross-host-ddc-row)
                          (independent-kernel-supported-row conf-results)
                          (independent-kernel-typegen-boundary-row conf-results)
                          (independent-mini-backend-row)
                          (drift-sentinel)
                          (bit-identical-boundary)])
        gap-ledger (trust-gap-ledger)
        accepted   (filter #(= :accepted (:gate/verdict %)) rows)
        behavior-accepted (filter #(= :behavior-equivalence (:kind %))
                                  accepted)
        unavailable (filter #(= :unavailable (:gate/verdict %)) rows)
        rejected   (filter #(= :rejected (:gate/verdict %)) rows)
        canonical-rows (mapv #(select-keys %
                                           [:id
                                            :kind
                                            :backend
                                            :expected
                                            :host-result
                                            :backend-result
                                            :backend-artifact
                                            :evidence
                                            :scope
                                            :claim
                                            :gate/verdict
                                            :held-reason
                                            :held-count
                                            :held-forms
                                            :not-claimed
                                            :evidence-source
                                            :cache-source
                                            :ok])
                             rows)
        canonical  {:rows canonical-rows
                    :trust-gap-ledger gap-ledger}
        invariants (sorted-map
                    :all-rows-ok (every? :ok rows)
                    :accepted-behavior-equivalent
                    (every? #(= (:host-result %) (:backend-result %))
                            behavior-accepted)
                    :backend-artifact-digests-present
                    (every? #(string? (get-in % [:backend-artifact :digest]))
                            behavior-accepted)
                    :full-source-transcript-accepted
                    (= :accepted
                       (:gate/verdict
                        (first (filter #(= :full-source-self-compiler-transcript
                                           (:id %))
                                       rows))))
                    :bit-identical-boundary-not-applicable
                    (let [row (first (filter #(= :bit-identical-artifact-ddc (:id %))
                                             rows))]
                      (and (= :accepted (:gate/verdict row))
                           (= :not-applicable (:boundary/status row))))
                    :drift-sentinel-rejected
                    (= :rejected
                       (:gate/verdict
                        (first (filter #(= :synthetic-behavior-drift (:id %))
                                       rows))))
                    :case-switch-fixtures-accepted-with-artifacts
                    (every? #(and (= :accepted (:gate/verdict %))
                                  (string? (get-in % [:backend-artifact :digest])))
                            (filter #(contains? #{:case-table-switch
                                                  :case-lookup-switch}
                                                (:id %))
                                    rows))
                    :letfn-fixture-accepted-with-artifact
                    (let [row (first (filter #(= :letfn-mutual-recursion (:id %))
                                             rows))]
                      (and (= :accepted (:gate/verdict row))
                           (string? (get-in row [:backend-artifact :digest]))))
                    :compile-ns-direct-simple-accepted
                    (let [row (first (filter #(= :compile-ns-direct-simple-transcript
                                                 (:id %))
                                             rows))]
                      (and (= :accepted (:gate/verdict row))
                           (= :direct-simple
                              (get-in row [:backend-artifact :ns-form-mode]))))
                    :compile-ns-require-import-accepted
                    (let [row (first (filter #(= :compile-ns-require-import-transcript
                                                 (:id %))
                                             rows))]
                      (and (= :accepted (:gate/verdict row))
                           (= :direct-compiled
                              (get-in row [:backend-artifact :ns-form-mode]))))
                    :independent-kernel-supported-corpus-accepted
                    (let [row (first (filter #(= :independent-kernel-evaluator-supported-corpus
                                                 (:id %))
                                             rows))]
                      (and (= :accepted (:gate/verdict row))
                           (pos? (get-in row [:evidence :kernel-checked]))
                           (= (get-in row [:evidence :kernel-checked])
                              (get-in row [:evidence :kernel-pass]))))
                    :independent-kernel-typegen-boundary-declared
                    (let [row (first (filter #(= :independent-kernel-evaluator-typegen-gap
                                                 (:id %))
                                             rows))]
                      (and (= :accepted (:gate/verdict row))
                           (= :out-of-scope (:boundary/status row))))
                    :independent-mini-backend-subset-accepted
                    (let [row (first (filter #(= :independent-mini-backend-subset
                                                 (:id %))
                                             rows))]
                      (and (= :accepted (:gate/verdict row))
                           (pos? (get-in row [:evidence :fixture-count]))
                           (every? :ok (get-in row [:evidence :fixtures]))))
                    :trusting-trust-gap-not-claimed-closed
                    (= :open
                       (:status
                        (first (filter #(= :bit-identical-ddc-open (:id %))
                                       gap-ledger))))
                    :realistic-ddc-scope-explicit
                    (= #{:partial :open :evidence-only}
                       (set (map :status gap-ledger)))
                    :only-drift-sentinel-rejected
                    (= #{:synthetic-behavior-drift} (set (map :id rejected))))
        ok?        (and (every? :ok rows)
                        (every? true? (vals invariants)))]
    {:schema "pnix.clj-meta.diverse-double-compile.receipt.v1"
     :stage [:M11]
     :target-goal "meta-circular stage15/N Clojure compiler"
     :desc "behavior-equivalence DDC boundary with explicit TCB and held bit-identical claim"
     :audit-input {:full-source-transcript-source (:evidence-source full-row)
                   :cache-source (:cache-source full-row)
                   :audit-ok (:ok audit-report)}
     :trust-base (tcb-ledger)
     :trust-gap-ledger gap-ledger
     :status-counts {:accepted (count accepted)
                     :unavailable (count unavailable)
                     :rejected (count rejected)}
     :rows rows
     :invariants invariants
     :canonical-receipt canonical
     :canonical-receipt-digest (sha256-string (pr-str canonical))
     :ok ok?})))

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
    (println (str "diverse double compile: "
                  (if (:ok r) "OK" "FAILED")
                  "  (receipt: " receipt-path
                  ", digest: " (:canonical-receipt-digest r)
                  ")"))
    (shutdown-agents)
    (when-not (:ok r)
      (System/exit 1))))
