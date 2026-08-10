(ns pnix.clj-meta.bytecode-witness
  "M6w primitive bytecode witness.

  This proof inspects compiled class bytes with ASM. It deliberately checks the
  bytecode artifact shape, not product file IO or launcher consumption."
  (:require [clojure.java.io :as io]
            [clojure.pprint :as pp]
            [pnix.clj-meta.compiler :as comp])
  (:import [clojure.asm ClassReader ClassVisitor MethodVisitor Opcodes]
           [java.security MessageDigest]))

(def receipt-path "clj-meta/proof/primitive-bytecode-witness.receipt.edn")

(def ^:private asm-api Opcodes/ASM5)

(def ^:private opcode->kw
  {Opcodes/ACONST_NULL :aconst-null
   Opcodes/ALOAD :aload
   Opcodes/ARETURN :areturn
   Opcodes/ASTORE :astore
   Opcodes/ATHROW :athrow
   Opcodes/DADD :dadd
   Opcodes/DCONST_0 :dconst-0
   Opcodes/DCONST_1 :dconst-1
   Opcodes/DLOAD :dload
   Opcodes/DMUL :dmul
   Opcodes/DRETURN :dreturn
   Opcodes/DSTORE :dstore
   Opcodes/DSUB :dsub
   Opcodes/DUP :dup
   Opcodes/DUP_X1 :dup-x1
   Opcodes/DUP_X2 :dup-x2
   Opcodes/GOTO :goto
   Opcodes/IADD :iadd
   Opcodes/IFEQ :ifeq
   Opcodes/IFNE :ifne
   Opcodes/IFNONNULL :ifnonnull
   Opcodes/IFNULL :ifnull
   Opcodes/IRETURN :ireturn
   Opcodes/GETFIELD :getfield
   Opcodes/INVOKEINTERFACE :invokeinterface
   Opcodes/INVOKESPECIAL :invokespecial
   Opcodes/INVOKESTATIC :invokestatic
   Opcodes/INVOKEVIRTUAL :invokevirtual
   Opcodes/LADD :ladd
   Opcodes/LCONST_0 :lconst-0
   Opcodes/LCONST_1 :lconst-1
   Opcodes/LLOAD :lload
   Opcodes/LMUL :lmul
   Opcodes/LRETURN :lreturn
   Opcodes/LSTORE :lstore
   Opcodes/LSUB :lsub
   Opcodes/NOP :nop
   Opcodes/POP :pop
   Opcodes/PUTFIELD :putfield
   Opcodes/RETURN :return
   Opcodes/SWAP :swap})

(defn- opcode-kw
  [opcode]
  (or (get opcode->kw opcode)
      (keyword (str "opcode-" opcode))))

(defn- sha256-string
  [s]
  (let [md (MessageDigest/getInstance "SHA-256")]
    (apply str
           (map #(format "%02x" (bit-and 0xff %))
                (.digest md (.getBytes ^String (str s) "UTF-8"))))))

(defn- method-event
  [class-name method-name method-desc event]
  (merge {:class class-name
          :method method-name
          :method-desc method-desc}
         event))

(defn- method-visitor
  [class-name method-name method-desc events]
  (proxy [MethodVisitor] [asm-api]
    (visitInsn [opcode]
      (swap! events conj
             (method-event class-name method-name method-desc
                           {:op (opcode-kw opcode)})))
    (visitVarInsn [opcode var]
      (swap! events conj
             (method-event class-name method-name method-desc
                           {:op (opcode-kw opcode)
                            :var var})))
    (visitJumpInsn [opcode label]
      (swap! events conj
             (method-event class-name method-name method-desc
                           {:op (opcode-kw opcode)})))
    (visitLookupSwitchInsn [dflt keys labels]
      (swap! events conj
             (method-event class-name method-name method-desc
                           {:op :lookupswitch
                            :keys (vec keys)
                            :label-count (alength labels)})))
    (visitTableSwitchInsn [min max dflt labels]
      (swap! events conj
             (method-event class-name method-name method-desc
                           {:op :tableswitch
                            :min min
                            :max max
                            :label-count (alength labels)})))
    (visitMethodInsn
      ([opcode owner name desc]
       (swap! events conj
              (method-event class-name method-name method-desc
                            {:op (opcode-kw opcode)
                             :owner owner
                             :name name
                             :invoke-desc desc
                             :interface? false})))
      ([opcode owner name desc itf]
       (swap! events conj
              (method-event class-name method-name method-desc
                            {:op (opcode-kw opcode)
                             :owner owner
                             :name name
                             :invoke-desc desc
                             :interface? (boolean itf)}))))
    (visitFieldInsn [opcode owner name desc]
      (swap! events conj
             (method-event class-name method-name method-desc
                           {:op (opcode-kw opcode)
                            :owner owner
                            :name name
                            :field-desc desc})))))

(defn- class-events
  [declared-class-name bytes]
  (let [events (atom [])
        cname  (atom declared-class-name)]
    (.accept (ClassReader. ^bytes bytes)
             (proxy [ClassVisitor] [asm-api]
               (visit [version access name signature super-name interfaces]
                 (reset! cname (.replace ^String name \/ \.)))
               (visitMethod [access name desc signature exceptions]
                 (method-visitor @cname name desc events)))
             0)
    @events))

(defn- inspect-classes
  [classes]
  (->> classes
       (sort-by key)
       (mapcat (fn [[cname bytes]] (class-events cname bytes)))
       vec))

(def ^:private case-specs
  [{:id :typed-let-long
    :desc "typed let stores long locals and preserves primitive arithmetic lowering evidence"
    :form '(fn [^long n]
             (let [a (+ n 1)
                   b (* a 2)]
               (+ b 3)))
    :required-opcodes #{:lload :lstore}
    :required-evidence
    [{:id :long-add
      :any [{:op :ladd}
            {:invoke {:owner "clojure/lang/Numbers"
                      :name "add"
                      :desc "(JJ)J"}}]}
     {:id :long-multiply
      :any [{:op :lmul}
            {:invoke {:owner "clojure/lang/Numbers"
                      :name "multiply"
                      :desc "(JJ)J"}}]}]}
   {:id :typed-let-double-direct
    :desc "typed let lowers safe double arithmetic to direct JVM opcodes"
    :form '(fn [^double x]
             (let [y (+ x 0.5)]
               (* y 2.0)))
    :required-opcodes #{:dload :dstore :dadd :dmul}
    :required-evidence
    [{:id :direct-double-add
      :any [{:op :dadd}]}
     {:id :direct-double-multiply
      :any [{:op :dmul}]}]}
   {:id :case-direct-table-switch
    :desc "dense numeric case* lowers to RT.uncheckedIntCast tableswitch plus Util.equiv guards"
    :form '(fn [n]
             (case n
               1 :one
               2 :two
               :other))
    :required-opcodes #{:tableswitch :goto :ifeq}
    :required-evidence
    [{:id :case-rt-unchecked-int-cast
      :any [{:invoke {:owner "clojure/lang/RT"
                      :name "uncheckedIntCast"
                      :desc "(Ljava/lang/Object;)I"}}]}
     {:id :case-util-equiv
      :any [{:invoke {:owner "clojure/lang/Util"
                      :name "equiv"
                      :desc "(Ljava/lang/Object;Ljava/lang/Object;)Z"}}]}]}
   {:id :case-direct-lookup-switch
    :desc "sparse keyword case* lowers to Util.hash lookupswitch plus Util.equiv guards"
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
    :required-opcodes #{:lookupswitch :goto :ifeq}
    :required-evidence
    [{:id :case-util-hash
      :any [{:invoke {:owner "clojure/lang/Util"
                      :name "hash"
                      :desc "(Ljava/lang/Object;)I"}}]}
     {:id :case-util-equiv
      :any [{:invoke {:owner "clojure/lang/Util"
                      :name "equiv"
                      :desc "(Ljava/lang/Object;Ljava/lang/Object;)Z"}}]}]}
   {:id :letfn-mutual-recursion-direct
    :desc "letfn mutual recursion lowers to generated fn classes with cyclic capture field patching"
    :form '(fn [n]
             (letfn [(ev? [x] (if (zero? x) true (od? (dec x))))
                     (od? [x] (if (zero? x) false (ev? (dec x))))]
               (ev? n)))
    :required-opcodes #{:getfield :putfield :invokeinterface}
    :required-evidence
    [{:id :letfn-capture-field-patch
      :any [{:op :putfield}]}
     {:id :letfn-peer-field-read
      :any [{:op :getfield}]}
     {:id :letfn-peer-invoke
      :any [{:op :invokeinterface}]}]}
   {:id :object-locals-clearing-direct
    :desc "object let/loop locals are cleared with null stores at scope exit while preserving the returned value"
    :form '(fn []
             [(let [xs (range 3)
                    y (first xs)]
                y)
              (loop [xs [1 2 3]]
                (if (seq xs)
                  (recur (rest xs))
                  (count xs)))])
    :required-opcodes #{:aconst-null :astore}
    :required-evidence
    [{:id :object-local-clear-null
      :any [{:op :aconst-null}]}
     {:id :object-local-clear-store
      :any [{:op :astore}]}]}
   {:id :unchecked-long-direct
    :desc "explicit unchecked long arithmetic lowers to direct JVM opcodes"
    :form '(fn [^long a ^long b]
             [(unchecked-add a b)
              (unchecked-subtract a b)
              (unchecked-multiply a b)])
    :required-opcodes #{:ladd :lsub :lmul}
    :required-evidence
    [{:id :direct-unchecked-long-add
      :any [{:op :ladd}]}
     {:id :direct-unchecked-long-subtract
      :any [{:op :lsub}]}
     {:id :direct-unchecked-long-multiply
      :any [{:op :lmul}]}]}
   {:id :checked-long-constant-no-overflow-direct
    :desc "checked long arithmetic with literal no-overflow proof lowers to direct JVM opcodes"
    :form '(fn []
             [(+ 20 22)
              (- 50 8)
              (* 6 7)])
    :required-opcodes #{:ladd :lsub :lmul}
    :required-evidence
    [{:id :direct-checked-long-add-constant-proof
      :any [{:op :ladd}]}
     {:id :direct-checked-long-subtract-constant-proof
      :any [{:op :lsub}]}
     {:id :direct-checked-long-multiply-constant-proof
      :any [{:op :lmul}]}]}
   {:id :checked-long-overflow-fallback
    :desc "overflowing checked long candidate is rejected by validator and remains on Numbers.add fallback"
    :form '(fn []
             (let [a Long/MAX_VALUE
                   b 1]
               (+ a b)))
    :required-opcodes #{}
    :forbidden-opcodes #{:ladd}
    :required-evidence
    [{:id :checked-long-overflow-add-fallback
      :any [{:invoke {:owner "clojure/lang/Numbers"
                      :name "add"
                      :desc "(JJ)J"}}]}]}
   {:id :checked-long-let-local-range-direct
    :desc "checked long arithmetic with let-local range proof lowers to direct JVM opcodes"
    :form '(fn []
             (let [a 20
                   b 22
                   c (+ a b)]
               [(+ a b)
                (- c a)
                (* b 2)]))
    :required-opcodes #{:ladd :lsub :lmul :lload :lstore}
    :required-evidence
    [{:id :direct-checked-long-add-let-range-proof
      :any [{:op :ladd}]}
     {:id :direct-checked-long-subtract-let-range-proof
      :any [{:op :lsub}]}
     {:id :direct-checked-long-multiply-let-range-proof
      :any [{:op :lmul}]}]}
   {:id :checked-long-loop-invariant-range-direct
    :desc "checked long arithmetic with invariant loop-local range proof lowers to direct JVM opcodes"
    :form '(fn []
             (loop [a 20
                    b 22]
               [(+ a b)
                (- b a)
                (* a 2)]))
    :required-opcodes #{:ladd :lsub :lmul :lload :lstore}
    :required-evidence
    [{:id :direct-checked-long-add-loop-invariant-proof
      :any [{:op :ladd}]}
     {:id :direct-checked-long-subtract-loop-invariant-proof
      :any [{:op :lsub}]}
     {:id :direct-checked-long-multiply-loop-invariant-proof
      :any [{:op :lmul}]}]}
   {:id :checked-long-loop-bounded-step-range-direct
    :desc "checked long arithmetic with bounded loop recurrence range proof lowers to direct JVM opcodes"
    :form '(fn []
             (loop [i 0]
               (if (< i 10)
                 (recur (+ i 1))
                 [(+ i 20)
                  (- 30 i)
                  (* i 2)])))
    :required-opcodes #{:goto :ladd :lload :lmul :lstore :lsub}
    :required-evidence
    [{:id :direct-checked-long-add-loop-bounded-step-proof
      :any [{:op :ladd}]}
     {:id :direct-checked-long-subtract-loop-bounded-step-proof
      :any [{:op :lsub}]}
     {:id :direct-checked-long-multiply-loop-bounded-step-proof
      :any [{:op :lmul}]}]}
   {:id :checked-long-loop-non-unit-stride-range-direct
    :desc "checked long arithmetic with positive non-unit loop stride range proof lowers to direct JVM opcodes"
    :form '(fn []
             (loop [i 0]
               (if (< i 10)
                 (recur (+ i 3))
                 [(+ i 20)
                  (- 40 i)
                  (* i 2)])))
    :required-opcodes #{:goto :ladd :lload :lmul :lstore :lsub}
    :required-evidence
    [{:id :direct-checked-long-add-loop-non-unit-stride-proof
      :any [{:op :ladd}]}
     {:id :direct-checked-long-subtract-loop-non-unit-stride-proof
      :any [{:op :lsub}]}
     {:id :direct-checked-long-multiply-loop-non-unit-stride-proof
      :any [{:op :lmul}]}]}
   {:id :checked-long-loop-decreasing-stride-range-direct
    :desc "checked long arithmetic with decreasing loop stride range proof lowers to direct JVM opcodes"
    :form '(fn []
             (loop [i 10]
               (if (> i 0)
                 (recur (- i 3))
                 [(+ i 20)
                  (- 40 i)
                  (* i 2)])))
    :required-opcodes #{:goto :ladd :lload :lmul :lstore :lsub}
    :required-evidence
    [{:id :direct-checked-long-add-loop-decreasing-stride-proof
      :any [{:op :ladd}]}
     {:id :direct-checked-long-subtract-loop-decreasing-stride-proof
      :any [{:op :lsub}]}
     {:id :direct-checked-long-multiply-loop-decreasing-stride-proof
      :any [{:op :lmul}]}]}
   {:id :checked-long-loop-accumulator-range-direct
    :desc "checked long arithmetic with bounded accumulator recurrence range proof lowers to direct JVM opcodes"
    :form '(fn []
             (loop [i 0
                    acc 0]
               (if (< i 10)
                 (recur (+ i 1) (+ acc 2))
                 [(+ acc 20)
                  (- 50 acc)
                  (* acc 2)])))
    :required-opcodes #{:goto :ladd :lload :lmul :lstore :lsub}
    :required-evidence
    [{:id :direct-checked-long-add-loop-accumulator-proof
      :any [{:op :ladd}]}
     {:id :direct-checked-long-subtract-loop-accumulator-proof
      :any [{:op :lsub}]}
     {:id :direct-checked-long-multiply-loop-accumulator-proof
      :any [{:op :lmul}]}]}
   {:id :checked-long-fn-argument-guard-range-direct
    :desc "checked long arithmetic with fn argument guard precondition range proof lowers to direct JVM opcodes"
    :form '(fn [^long x]
             (if (> x -10)
               (if (< x 10)
                 [(+ x 1)
                  (- 10 x)
                  (* x 2)]
                 0)
               0))
    :required-opcodes #{:ladd :lmul :lsub}
    :required-evidence
    [{:id :direct-checked-long-add-fn-argument-guard-proof
      :any [{:op :ladd}]}
     {:id :direct-checked-long-subtract-fn-argument-guard-proof
      :any [{:op :lsub}]}
     {:id :direct-checked-long-multiply-fn-argument-guard-proof
      :any [{:op :lmul}]}]}
   {:id :checked-long-loop-index-accumulator-range-direct
    :desc "checked long arithmetic with bounded index-dependent accumulator recurrence proof lowers to direct JVM opcodes"
    :form '(fn []
             (loop [i 0
                    acc 0]
               (if (< i 10)
                 (recur (+ i 1) (+ acc i))
                 [(+ acc 1)
                  (- 50 acc)
                  (* acc 2)])))
    :required-opcodes #{:goto :ladd :lload :lmul :lstore :lsub}
    :required-evidence
    [{:id :direct-checked-long-add-loop-index-accumulator-proof
      :any [{:op :ladd}]}
     {:id :direct-checked-long-subtract-loop-index-accumulator-proof
      :any [{:op :lsub}]}
     {:id :direct-checked-long-multiply-loop-index-accumulator-proof
      :any [{:op :lmul}]}]}
   {:id :checked-long-loop-multiplicative-accumulator-range-direct
    :desc "checked long arithmetic with bounded multiplicative accumulator recurrence proof lowers to direct JVM opcodes"
    :form '(fn []
             (loop [i 0
                    acc 1]
               (if (< i 5)
                 (recur (+ i 1) (* acc 2))
                 [(+ acc 1)
                  (- 100 acc)
                  (* acc 2)])))
    :required-opcodes #{:goto :ladd :lload :lmul :lstore :lsub}
    :required-evidence
    [{:id :direct-checked-long-add-loop-multiplicative-accumulator-proof
      :any [{:op :ladd}]}
     {:id :direct-checked-long-subtract-loop-multiplicative-accumulator-proof
      :any [{:op :lsub}]}
     {:id :direct-checked-long-multiply-loop-multiplicative-accumulator-proof
      :any [{:op :lmul}]}]}
   {:id :typed-loop-long
    :desc "typed loop/recur uses long local slots and primitive long recurrence evidence"
    :form '(fn [^long n]
             (loop [i n acc 0]
               (if (< i 1)
                 acc
                 (recur (- i 1) (+ acc i)))))
    :required-opcodes #{:lload :lstore :goto}
    :required-evidence
    [{:id :long-subtract
      :any [{:op :lsub}
            {:invoke {:owner "clojure/lang/Numbers"
                      :name "minus"
                      :desc "(JJ)J"}}]}
     {:id :long-add
      :any [{:op :ladd}
            {:invoke {:owner "clojure/lang/Numbers"
                      :name "add"
                      :desc "(JJ)J"}}]}
     {:id :long-lt
      :any [{:invoke {:owner "clojure/lang/Numbers"
                      :name "lt"
                      :desc "(JJ)Z"}}]}]}
   {:id :typed-loop-mixed-long-double
    :desc "typed loop/recur keeps long index and double accumulator primitive slots"
    :form '(fn [^double x]
             (loop [i 0 acc x]
               (if (< i 3)
                 (recur (+ i 1) (+ acc 0.5))
                 acc)))
    :required-opcodes #{:lload :lstore :dload :dstore :goto}
    :required-evidence
    [{:id :long-add
      :any [{:op :ladd}
            {:invoke {:owner "clojure/lang/Numbers"
                      :name "add"
                      :desc "(JJ)J"}}]}
     {:id :double-add
      :any [{:op :dadd}]}
     {:id :long-lt
      :any [{:invoke {:owner "clojure/lang/Numbers"
                      :name "lt"
                      :desc "(JJ)Z"}}]}]}
   {:id :m9b-branch-dependent-stride-range-direct
    :desc "branch-dependent loop stride range proved by the abstract-interpretation engine (interval fixpoint) lowers checked long arithmetic to direct opcodes; recognizers cannot, only the M9 engine derives i in [0,11]"
    :form '(fn [^long c]
             (loop [i 0]
               (if (< i 10)
                 (recur (if (even? c) (+ i 1) (+ i 2)))
                 [(+ i 20) (- 40 i) (* i 2)])))
    :required-opcodes #{:ladd :lsub :lmul :lload :lstore :goto}
    :required-evidence
    [{:id :m9b-engine-long-add :any [{:op :ladd}]}
     {:id :m9b-engine-long-subtract :any [{:op :lsub}]}
     {:id :m9b-engine-long-multiply :any [{:op :lmul}]}]}
   {:id :m9b-branch-dependent-overflow-fallback
    :desc "branch-dependent loop whose exit arithmetic overflows the engine-derived range stays on Numbers.add (JJ)J fallback; the sound engine range [0,11] does not let the overflowing exit add lower (recur increments still lower, exit add does not)"
    :form '(fn [^long c]
             (loop [i 0]
               (if (< i 10)
                 (recur (if (even? c) (+ i 1) (+ i 2)))
                 (+ i Long/MAX_VALUE))))
    :required-opcodes #{}
    :required-evidence
    [{:id :m9b-overflow-exit-add-fallback
      :any [{:invoke {:owner "clojure/lang/Numbers"
                      :name "add"
                      :desc "(JJ)J"}}]}]}
   {:id :m9b-mixed-sign-conserved-range-direct
    :desc "mixed-sign accumulator (acc decreases as i increases) is unbounded under intervals but the conserved linear quantity acc+i=0 (relational substrate) proves acc in [-10,0], so checked long multiply lowers directly"
    :form '(fn [] (loop [i 0 acc 0]
                    (if (< i 10)
                      (recur (+ i 1) (- acc 1))
                      (* acc 2))))
    :required-opcodes #{:lmul :lload :lstore :goto}
    :required-evidence
    [{:id :m9b-conserved-long-multiply :any [{:op :lmul}]}]}
   {:id :m9b-negative-factor-geometric-range-direct
    :desc "negative-factor multiplicative recurrence acc'=(* acc -2) is nonlinear (sign alternating, exponential); bounded geometric acceleration proves |acc| <= |init|*|factor|^N = 32, so acc in [-32,32] and the exit checked long add lowers directly (recur multiply also lowers)"
    :form '(fn [] (loop [i 0 acc 1]
                    (if (< i 5)
                      (recur (+ i 1) (* acc -2))
                      (+ acc 100))))
    :required-opcodes #{:ladd :lmul :lload :lstore :goto}
    :required-evidence
    [{:id :m9b-geometric-exit-long-add :any [{:op :ladd}]}
     {:id :m9b-geometric-recur-long-multiply :any [{:op :lmul}]}]}
   {:id :m9b-nonlinear-unroll-range-direct
    :desc "non-constant-factor nonlinear recurrence acc'=(* acc acc) (double-exponential) has no closed-form factor, but bounded interval unrolling over the known iteration count N=4 derives acc in [2,65536], so both the recur multiply and the exit add lower to direct opcodes"
    :form '(fn [] (loop [i 0 acc 2]
                    (if (< i 4)
                      (recur (+ i 1) (* acc acc))
                      (+ acc 1))))
    :required-opcodes #{:lmul :ladd :lload :lstore :goto}
    :required-evidence
    [{:id :m9b-unroll-recur-long-multiply :any [{:op :lmul}]}
     {:id :m9b-unroll-exit-long-add :any [{:op :ladd}]}]}
   {:id :u9-unknown-iteration-nonlinear-checked-fallback
    :desc "param-bound nonlinear recurrence cannot be statically bounded and must keep checked Numbers.multiply fallback rather than raw LMUL"
    :form '(fn [^long n]
             (loop [i 0 acc 2]
               (if (< i n)
                 (recur (+ i 1) (* acc acc))
                 acc)))
    :required-opcodes #{:goto :lload :lstore}
    :forbidden-opcodes #{:lmul}
    :required-evidence
    [{:id :u9-unknown-nonlinear-multiply-fallback
      :any [{:invoke {:owner "clojure/lang/Numbers"
                      :name "multiply"
                      :desc "(JJ)J"}}]}]}
   {:id :import-direct
    :desc "import* lowers to RT.classForNameNonLoading + Namespace.importClass via direct emit (host ImportExpr 동일 경로, 0 host Compiler)"
    :form '(fn [] (clojure.core/import* "java.util.zip.CRC32"))
    :required-opcodes #{}
    :required-evidence
    [{:id :import-class-for-name-nonloading
      :any [{:invoke {:owner "clojure/lang/RT"
                      :name "classForNameNonLoading"
                      :desc "(Ljava/lang/String;)Ljava/lang/Class;"}}]}
     {:id :import-namespace-import-class
      :any [{:invoke {:op :invokevirtual
                      :owner "clojure/lang/Namespace"
                      :name "importClass"
                      :desc "(Ljava/lang/Class;)Ljava/lang/Class;"}}]}]}])

(defn- invoke-match?
  "owner/name/desc 일치 검사. invoke 종류는 기본 invokestatic 이며, spec 에 명시적
  `:op`(예: :invokevirtual)을 주면 그 종류로 매칭한다(import* 의 importClass 등)."
  [event {:keys [owner name desc op]}]
  (and (= (or op :invokestatic) (:op event))
       (= owner (:owner event))
       (= name (:name event))
       (= desc (:invoke-desc event))))

(defn- evidence-match
  [events alt]
  (cond
    (:op alt)
    (some #(when (= (:op alt) (:op %))
             {:op (:op alt)})
          events)

    (:invoke alt)
    (some #(when (invoke-match? % (:invoke alt))
             {:invoke (:invoke alt)})
          events)

    :else nil))

(defn- evidence-result
  [events {:keys [id any]}]
  (let [matched (some #(evidence-match events %) any)]
    {:id id
     :ok (boolean matched)
     :matched matched
     :accepted-any any}))

(defn- opcode-counts
  [events]
  (into (sorted-map)
        (frequencies (map :op events))))

(defn- invoke-counts
  [events]
  (->> events
       (filter #(contains? #{:invokestatic :invokevirtual :invokeinterface :invokespecial}
                           (:op %)))
       (map (fn [{:keys [op owner name invoke-desc]}]
              {:op op :owner owner :name name :desc invoke-desc}))
       frequencies
       (map (fn [[k n]] (assoc k :count n)))
       (sort-by (juxt :owner :name :desc :op))
       vec))

(defn- relevant-event?
  [{:keys [op owner]}]
  (or (contains? #{:lload :lstore :dload :dstore
                   :aconst-null :astore
                   :ladd :lmul :lsub :dadd :dmul :dsub
                   :goto :ifeq :tableswitch :lookupswitch
                   :getfield :putfield :invokeinterface}
                 op)
      (= "clojure/lang/Numbers" owner)
      (= "clojure/lang/RT" owner)
      (= "clojure/lang/Util" owner)
      (= "java/lang/Long" owner)
      (= "java/lang/Double" owner)))

(defn- compact-event
  [event]
  (select-keys event
               [:class :method :method-desc :op :var
                :owner :name :invoke-desc :interface?
                :field-desc :keys :min :max :label-count]))

(defn- run-case
  [{:keys [id desc form required-opcodes forbidden-opcodes required-evidence]
    :or {forbidden-opcodes #{}}}]
  (let [classes          (comp/compile-classes form)
        events           (inspect-classes classes)
        counts           (opcode-counts events)
        opcode-results   (mapv (fn [op]
                                  {:op op
                                   :count (get counts op 0)
                                   :ok (pos? (get counts op 0))})
                                (sort-by name required-opcodes))
        forbidden-results (mapv (fn [op]
                                  {:op op
                                   :count (get counts op 0)
                                   :ok (zero? (get counts op 0))})
                                (sort-by name forbidden-opcodes))
        evidence-results (mapv #(evidence-result events %) required-evidence)
        ok?              (and (every? :ok opcode-results)
                              (every? :ok forbidden-results)
                              (every? :ok evidence-results))]
    {:id id
     :desc desc
     :form (binding [*print-meta* true] (pr-str form))
     :class-count (count classes)
     :class-names (vec (sort (keys classes)))
     :opcode-counts counts
     :invokes (invoke-counts events)
     :required-opcodes opcode-results
     :forbidden-opcodes forbidden-results
     :required-evidence evidence-results
     :normalized-events (->> events
                             (filter relevant-event?)
                             (mapv compact-event))
     :ok ok?}))

(defn run
  []
  (let [cases     (mapv run-case case-specs)
        held      []
        canonical (mapv #(select-keys %
                                      [:id
                                       :class-names
                                       :opcode-counts
                                       :invokes
                                       :required-opcodes
                                       :forbidden-opcodes
                                       :required-evidence
                                       :ok])
                         cases)
        ok?       (every? :ok cases)]
    {:schema "pnix.clj-meta.primitive-bytecode-witness.receipt.v1"
     :stage [:M6w :M6x :M6y :M6z :M6aa :M6ab :M6ac :M6ad :M6ae :M6af :M6ag :M6ah :M6ai :M10 :M12]
     :target-goal "meta-circular stage15/N Clojure compiler"
     :desc "primitive bytecode witness/disasm receipt and safe raw arithmetic opcode lowering"
     :lowering-policy {:accepted [:primitive-local-slots
                                  :primitive-numbers-descriptor-calls
                                  :direct-double-arithmetic-opcodes
                                  :direct-unchecked-long-arithmetic-opcodes
                                  :direct-checked-long-constant-no-overflow-opcodes
                                  :direct-checked-long-let-local-range-opcodes
                                  :direct-checked-long-loop-invariant-range-opcodes
                                  :direct-checked-long-loop-bounded-step-range-opcodes
                                  :direct-checked-long-loop-positive-non-unit-stride-range-opcodes
                                  :direct-checked-long-loop-decreasing-stride-range-opcodes
                                  :direct-checked-long-loop-accumulator-range-opcodes
                                  :direct-checked-long-fn-argument-guard-range-opcodes
                                  :direct-checked-long-loop-index-accumulator-range-opcodes
                                  :direct-checked-long-loop-multiplicative-accumulator-range-opcodes
                                  :direct-checked-long-branch-dependent-stride-range-opcodes
                                  :direct-checked-long-mixed-sign-conserved-range-opcodes
                                  :direct-checked-long-negative-factor-geometric-range-opcodes
                                  :direct-checked-long-nonlinear-unroll-range-opcodes
                                  :direct-case-hash-table-switch
                                  :direct-case-hash-lookup-switch
                                  :direct-letfn-mutual-recursion-fields
                                  :object-let-loop-locals-clearing]
                       :rejected-fallback [:overflowing-checked-long-candidates-stay-on-numbers-fallback]
                       :checked-fallback [:statically-unknown-iteration-nonlinear-recurrence]
                       :held held
                       :current-note "checked long arithmetic lowers directly only after M10 lowering-sound? VC validation over literal/let-local/invariant-loop/bounded monotonic-stride, constant/index/multiplicative accumulator loop, branch-dependent stride, mixed-sign conserved, negative-factor geometric, nonlinear unroll, and branch-local fn argument guard ranges; overflowing candidates and statically-unknown-iteration nonlinear recurrences stay on Numbers fallback; letfn mutual recursion is direct via generated fn classes plus cyclic capture field patching; object let/loop locals are cleared at scope exit with null stores"}
     :cases cases
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
    (pp/pprint r)
    (println (str "\nprimitive bytecode witness: "
                  (if (:ok r) "OK" "FAILED")
                  "  (receipt: " receipt-path
                  ", digest: " (:canonical-receipt-digest r)
                  ")"))
    (shutdown-agents)
    (when-not (:ok r)
      (System/exit 1))))
