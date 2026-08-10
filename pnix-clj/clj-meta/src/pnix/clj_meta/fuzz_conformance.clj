(ns pnix.clj-meta.fuzz-conformance
  "Deterministic fuzz/PBT-style host/compiler differential conformance lane.

  This is evidence-strengthening, not a full language-correctness proof.  It
  generates a bounded supported subset, compiles each generated function through
  clj-meta, and compares host Clojure results against compiler results over many
  random inputs."
  (:require [clojure.java.io :as io]
            [clojure.pprint :as pp]
            [pnix.clj-meta.compiler :as comp])
  (:import [clojure.asm ClassReader ClassVisitor MethodVisitor Opcodes]
           [java.security MessageDigest]
           [java.util Random]))

(def receipt-path "clj-meta/proof/fuzz-conformance.receipt.edn")

(def ^:private default-program-count 250)
(def ^:private default-inputs-per-program 40)
(def ^:private default-seed 1729176329)
(def ^:private default-minimum-comparisons 10000)
(def ^:private base-vars ['x 'y 'z])
(def ^:private edge-longs
  [0 1 -1
   Long/MAX_VALUE Long/MIN_VALUE
   (- Long/MAX_VALUE 1) (- Long/MAX_VALUE 7) (- Long/MAX_VALUE 20)
   (+ Long/MIN_VALUE 1) (+ Long/MIN_VALUE 7)
   1099511627776 -1099511627776
   3037000499 3037000500 -3037000500])

(defn- sha256-string
  [s]
  (let [md (MessageDigest/getInstance "SHA-256")]
    (apply str
           (map #(format "%02x" (bit-and 0xff %))
                (.digest md (.getBytes ^String (str s) "UTF-8"))))))

(defn- rand-long
  [^Random r lo hi]
  (+ lo (.nextInt r (inc (- hi lo)))))

(defn- choose
  [^Random r xs]
  (nth xs (.nextInt r (count xs))))

(defn- sorted-keywords
  [xs]
  (vec (sort-by name xs)))

(defn- typed-param
  [sym]
  (with-meta sym {:tag 'long}))

(defn- gen-long
  [^Random r]
  (case (.nextInt r 5)
    0 (choose r edge-longs)
    1 (rand-long r -100000 100000)
    2 (* (choose r [-1 1]) (bit-shift-left 1 (+ 32 (.nextInt r 20))))
    (rand-long r -20 20)))

(declare gen-expr)

(defn- gen-let
  [^Random r env depth]
  (let [d (dec depth)
        a (symbol (str "l" depth "_0"))
        b (symbol (str "l" depth "_1"))
        a-expr (gen-expr r env d)
        b-expr (gen-expr r (conj env a) d)]
    (list 'let [a a-expr
                b b-expr]
          (gen-expr r (conj env a b) d))))

(defn- gen-loop
  [^Random r depth]
  (let [i (symbol (str "i" depth))
        acc (symbol (str "acc" depth))
        bound (choose r [0 1 2 3 4 5 6 7 8 10])
        step (choose r [1 2 3])
        init (if (zero? (.nextInt r 4))
               (choose r [Long/MAX_VALUE Long/MIN_VALUE
                          (- Long/MAX_VALUE 20)
                          1099511627776 -1099511627776])
               (rand-long r -10 10))
        update (case (.nextInt r 5)
                 0 (list '+ acc (rand-long r -5 5))
                 1 (list '- acc (rand-long r -5 5))
                 2 (list '+ acc i)
                 3 (list '- acc i)
                 4 (list '* acc (choose r [-2 -1 0 1 2])))]
    (list 'loop [i 0
                 acc init]
          (list 'if (list '< i bound)
                (list 'recur (list '+ i step) update)
                acc))))

(defn- gen-leaf
  [^Random r env]
  (if (and (seq env) (zero? (.nextInt r 2)))
    (choose r env)
    (gen-long r)))

(defn- gen-expr
  [^Random r env depth]
  (if (<= depth 0)
    (gen-leaf r env)
    (let [d (dec depth)]
      (case (.nextInt r 9)
        0 (gen-leaf r env)
        1 (gen-leaf r env)
        2 (list '+ (gen-expr r env d) (gen-expr r env d))
        3 (list '- (gen-expr r env d) (gen-expr r env d))
        4 (list '* (gen-expr r env d) (gen-expr r env d))
        5 (list 'if (list '< (gen-expr r env d) (gen-expr r env d))
                (gen-expr r env d)
                (gen-expr r env d))
        6 (gen-let r env d)
        7 (gen-loop r d)
        8 (list '+ (gen-loop r d) (gen-expr r env d))))))

(defn- gen-program
  [^Random r index]
  (let [arity (.nextInt r (inc (count base-vars)))
        params (vec (take arity base-vars))
        body (gen-expr r params 4)]
    {:program-index index
     :arity arity
     :form (list 'fn (vec (map typed-param params)) body)}))

(defn- gen-args
  [^Random r arity]
  (vec (repeatedly arity #(gen-long r))))

(defn- try-value
  [f]
  (try
    {:status :value
     :value (f)}
    (catch Throwable t
      {:status :error
       :class (.getName (class t))
       :message (.getMessage t)})))

(defn- equivalent-result?
  [host compiler]
  (cond
    (and (= :value (:status host))
         (= :value (:status compiler)))
    (= (:value host) (:value compiler))

    (and (= :error (:status host))
         (= :error (:status compiler)))
    (= (:class host) (:class compiler))

    :else false))

(defn- long-overflow-result?
  [x]
  (and (= :error (:status x))
       (= "java.lang.ArithmeticException" (:class x))
       (= "long overflow" (:message x))))

(defn- finding-b-form
  []
  '(fn []
     (loop [i 0 acc 0]
       (if (< i 10)
         (if (< i 1)
           (recur (+ i 3) (+ acc i))
           (recur (+ i 2) (+ acc i)))
         (+ acc 9223372036854775787)))))

(defn- sentinel-programs
  []
  [{:program-index :sentinel-overflow-add-arg
    :sentinel :overflow
    :arity 1
    :fixed-inputs [[Long/MAX_VALUE]]
    :form '(fn [^long x] (+ x 1))}
   {:program-index :sentinel-overflow-sub-arg
    :sentinel :overflow
    :arity 1
    :fixed-inputs [[Long/MIN_VALUE]]
    :form '(fn [^long x] (- x 1))}
   {:program-index :sentinel-overflow-mul-arg
    :sentinel :overflow
    :arity 1
    :fixed-inputs [[3037000500]]
    :form '(fn [^long x] (* x x))}
   {:program-index :sentinel-finding-b-heterogeneous-stride
    :sentinel :finding-b
    :arity 0
    :fixed-inputs [[]]
    :form (finding-b-form)}
   {:program-index :sentinel-raw-boundary-safe
    :sentinel :raw-reachable
    :arity 0
    :fixed-inputs [[]]
    :form '(fn [] (+ Long/MAX_VALUE 0))}
   {:program-index :sentinel-raw-sub-boundary-safe
    :sentinel :raw-reachable
    :arity 0
    :fixed-inputs [[]]
    :form '(fn [] (- 42 0))}
   {:program-index :sentinel-raw-nonlinear-unroll
    :sentinel :raw-reachable
    :arity 0
    :fixed-inputs [[]]
    :form '(fn []
             (loop [i 0 acc 2]
               (if (< i 4)
                 (recur (+ i 1) (* acc acc))
                 (+ acc 1))))}])

(defn- long-arith-opcodes
  [form]
  (let [classes (comp/compile-classes form)
        ops     (atom #{})]
    (doseq [[_ bytes] classes]
      (.accept (ClassReader. ^bytes bytes)
               (proxy [ClassVisitor] [Opcodes/ASM5]
                 (visitMethod [_a _n _d _s _e]
                   (proxy [MethodVisitor] [Opcodes/ASM5]
                     (visitInsn [op]
                       (condp = op
                         Opcodes/LADD (swap! ops conj :ladd)
                         Opcodes/LSUB (swap! ops conj :lsub)
                         Opcodes/LMUL (swap! ops conj :lmul)
                         nil))
                     (visitMethodInsn
                       ([op owner nm desc]
                        (when (and (= op Opcodes/INVOKESTATIC)
                                   (= owner "clojure/lang/Numbers")
                                   (= desc "(JJ)J"))
                          (swap! ops conj (keyword (str "numbers-" nm "-jj")))))
                       ([op owner nm desc _itf]
                        (when (and (= op Opcodes/INVOKESTATIC)
                                   (= owner "clojure/lang/Numbers")
                                   (= desc "(JJ)J"))
                          (swap! ops conj (keyword (str "numbers-" nm "-jj")))))))))
               0))
    @ops))

(defn- opcode-evidence
  []
  (let [sentinels (filter #(= :raw-reachable (:sentinel %)) (sentinel-programs))
        rows      (mapv (fn [{:keys [program-index form]}]
                          {:program-index program-index
                           :opcodes (vec (sort (long-arith-opcodes form)))})
                        sentinels)
        all-ops   (set (mapcat :opcodes rows))
        raw?      (boolean (some (fn [{:keys [opcodes]}]
                                   (some #{:ladd :lsub :lmul} opcodes))
                                 rows))]
    {:rows rows
     :raw-opcodes (sorted-keywords all-ops)
     :raw-opcode-reached? raw?
     :raw-sub-opcode-reached? (contains? all-ops :lsub)}))

(defn- mutant-regression-row
  []
  (let [form     (finding-b-form)
        host     (try-value #(apply (eval form) []))
        compiler (try-value
                  #(comp/with-checked-long-admission-mutant
                     (fn []
                       (apply (comp/compile-form form) []))))
        would-fail? (not (equivalent-result? host compiler))]
    {:program-index :sentinel-finding-b-heterogeneous-stride
     :mutant [:disable-homogeneous-index-stride-gate
              :disable-independent-range-admission]
     :host host
     :compiler-mutant compiler
     :would-fail? would-fail?}))

(defn- compile-pair
  [form]
  (let [host (try-value #(eval form))
        compiler (try-value #(comp/compile-form form))]
    {:host host
     :compiler compiler
     :ok (and (= :value (:status host))
              (= :value (:status compiler)))}))

(defn- run-program
  [^Random r {:keys [program-index form arity] :as program} inputs-per-program]
  (let [{host-fn :host compiler-fn :compiler compile-ok? :ok} (compile-pair form)]
    (if-not compile-ok?
      {:program program
       :compile-ok false
       :comparisons 0
       :failures [{:program-index program-index
                   :form (pr-str form)
                   :phase :compile
                   :host host-fn
                   :compiler compiler-fn}]}
      (let [rows (mapv (fn [input-index]
                         (let [args (if-let [fixed (:fixed-inputs program)]
                                      (nth fixed input-index)
                                      (gen-args r arity))
                               host (try-value #(apply (:value host-fn) args))
                               compiler (try-value #(apply (:value compiler-fn) args))
                               ok? (equivalent-result? host compiler)]
                           {:program-index program-index
                            :input-index input-index
                            :args args
                            :host host
                            :compiler compiler
                            :ok ok?}))
                       (range (if-let [fixed (:fixed-inputs program)]
                                (count fixed)
                                inputs-per-program)))
            failures (filterv (complement :ok) rows)
            overflow-comparisons
            (count (filter #(and (long-overflow-result? (:host %))
                                 (long-overflow-result? (:compiler %)))
                           rows))]
        {:program program
         :compile-ok true
         :comparisons (count rows)
         :overflow-comparisons overflow-comparisons
         :failures failures}))))

(defn- run*
  [program-count inputs-per-program seed minimum-comparisons]
  (let [r (Random. (long seed))
        random-programs (mapv #(gen-program r %) (range program-count))
        sentinels (sentinel-programs)
        programs (vec (concat sentinels random-programs))
        results (mapv #(run-program r % inputs-per-program) programs)
        failures (vec (mapcat :failures results))
        comparison-count (reduce + (map :comparisons results))
        overflow-comparison-count (reduce + (map #(or (:overflow-comparisons %) 0)
                                                 results))
        opcode-witness (opcode-evidence)
        mutant-regression (mutant-regression-row)
        policy {:claim "evidence-strengthening only"
                :not-claimed :full-language-correctness-proof
                :generator-subset
                [:long-params :primitive-loop-slots-from-long-inits
                 :integer-literals :edge-long-literals
                 :+ :- :* :< :if :let :bounded-loop-recur
                 :overflow-sentinels :raw-opcode-sentinels]}
        generated-digest (sha256-string
                          (pr-str
                           (mapv (fn [{:keys [program]}]
                                   (select-keys program
                                                [:program-index
                                                 :sentinel
                                                 :arity
                                                 :fixed-inputs
                                                 :form]))
                                 results)))
        failure-digest (sha256-string (pr-str failures))
        sample-programs (mapv (fn [{:keys [program]}]
                                (update program :form pr-str))
                              (take 12 results))
        invariants (sorted-map
                    :all-generated-programs-compiled
                    (every? :compile-ok results)
                    :host-compiler-equivalent
                    (empty? failures)
                    :minimum-comparisons-met
                    (>= comparison-count minimum-comparisons)
                    :overflow-path-reached
                    (pos? overflow-comparison-count)
                    :raw-opcode-path-reached
                    (:raw-opcode-reached? opcode-witness)
                    :raw-sub-opcode-path-reached
                    (:raw-sub-opcode-reached? opcode-witness)
                    :finding-b-mutant-regression-detected
                    (:would-fail? mutant-regression)
                    :evidence-not-proof-claim
                    (and (= "evidence-strengthening only" (:claim policy))
                         (= :full-language-correctness-proof
                            (:not-claimed policy)))
                    :deterministic-seed-recorded
                    (integer? seed))
        canonical {:seed seed
                   :policy policy
                   :random-program-count program-count
                   :sentinel-count (count sentinels)
                   :program-count (count programs)
                   :inputs-per-program inputs-per-program
                   :comparison-count comparison-count
                   :overflow-comparison-count overflow-comparison-count
                   :minimum-comparisons minimum-comparisons
                   :generated-program-digest generated-digest
                   :failure-digest failure-digest
                   :failure-count (count failures)
                   :invariants invariants}
        ok? (every? true? (vals invariants))]
    {:schema "pnix.clj-meta.fuzz-conformance.receipt.v1"
     :stage [:U8 :language-correctness-evidence]
     :target-goal "meta-circular stage15/N Clojure compiler"
     :desc "deterministic generated-program differential testing over host Clojure and clj-meta compiler"
     :policy policy
     :seed seed
     :random-program-count program-count
     :sentinel-count (count sentinels)
     :program-count (count programs)
     :inputs-per-program inputs-per-program
     :comparison-count comparison-count
     :overflow-comparison-count overflow-comparison-count
     :minimum-comparisons minimum-comparisons
     :sample-programs sample-programs
     :opcode-evidence opcode-witness
     :mutant-regression mutant-regression
     :failures (vec (take 20 failures))
     :failure-count (count failures)
     :generated-program-digest generated-digest
     :failure-digest failure-digest
     :invariants invariants
     :canonical-receipt canonical
     :canonical-receipt-digest (sha256-string (pr-str canonical))
     :ok ok?}))

(defn run
  ([] (run default-program-count
           default-inputs-per-program
           default-seed
           default-minimum-comparisons))
  ([program-count inputs-per-program seed minimum-comparisons]
   (run* program-count inputs-per-program seed minimum-comparisons)))

(defn write-receipt!
  [receipt]
  (io/make-parents receipt-path)
  (spit receipt-path (with-out-str (pp/pprint receipt)))
  receipt)

(defn- parse-long-or
  [s default]
  (if (some? s)
    (Long/parseLong s)
    default))

(defn -main
  [& args]
  (let [program-count (parse-long-or (nth args 0 nil) default-program-count)
        inputs-per-program (parse-long-or (nth args 1 nil) default-inputs-per-program)
        seed (parse-long-or (nth args 2 nil) default-seed)
        minimum-comparisons (parse-long-or (nth args 3 nil)
                                           default-minimum-comparisons)
        receipt (write-receipt!
                 (run program-count
                      inputs-per-program
                      seed
                      minimum-comparisons))]
    (println (str "fuzz conformance: "
                  (if (:ok receipt) "OK" "FAILED")
                  "  (programs=" (:program-count receipt)
                  ", inputs/program=" (:inputs-per-program receipt)
                  ", comparisons=" (:comparison-count receipt)
                  ", failures=" (:failure-count receipt)
                  ", seed=" (:seed receipt)
                  ", receipt: " receipt-path
                  ", digest: " (:canonical-receipt-digest receipt)
                  ")"))
    (when (seq (:failures receipt))
      (doseq [failure (:failures receipt)]
        (println (str "  [FAIL] " (pr-str failure)))))
    (shutdown-agents)
    (when-not (:ok receipt)
      (System/exit 1))))
