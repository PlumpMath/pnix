(ns pnix.clj-meta.m6aj-framework
  "M6aj absorption witness for the M9 relation/ranking framework.

  M6aj was intentionally not added as another ad-hoc compiler lowering pattern.
  This receipt classifies its three hard cases against the new substrates:
  octagon relations, linear ranking, and fail-closed fallback for non-linear
  recurrence. It still performs no raw opcode admission."
  (:require [clojure.java.io :as io]
            [clojure.pprint :as pp]
            [pnix.clj-meta.abstract-interval :as ai]
            [pnix.clj-meta.abstract-octagon :as ao]
            [pnix.clj-meta.linear-ranking :as ranking]
            [pnix.clj-meta.compiler :as comp])
  (:import [java.security MessageDigest]
           [clojure.asm ClassReader ClassVisitor MethodVisitor Opcodes]))

(def receipt-path "clj-meta/proof/m6aj-framework.receipt.edn")

(defn- sha256-string
  [s]
  (let [md (MessageDigest/getInstance "SHA-256")]
    (apply str
           (map #(format "%02x" (bit-and 0xff %))
                (.digest md (.getBytes ^String (str s) "UTF-8"))))))

(defn- branch-ranking-candidate
  []
  (some (fn [row]
          (when (= :branch-dependent-stride (:id row))
            (:candidate row)))
        (:rows (ranking/run))))

(defn- long-arith-opcodes
  "form 을 clj-meta backend 로 컴파일해 실제 emit 된 long 산술 opcode 종류를 모은다.
  LADD/LSUB/LMUL = engine range 를 validator 가 승인해 raw lowering 된 증거,
  Numbers.add(JJ)J invokestatic = overflow/미증명으로 checked fallback 된 증거."
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
                     (visitMethodInsn [op owner nm desc _itf]
                       (when (and (= op Opcodes/INVOKESTATIC)
                                  (= owner "clojure/lang/Numbers")
                                  (= desc "(JJ)J"))
                         (condp = nm
                           "add"      (swap! ops conj :numbers-add-jj)
                           "multiply" (swap! ops conj :numbers-multiply-jj)
                           nil))))))
               0))
    @ops))

(def ^:private branch-stride-direct-form
  '(fn [^long c]
     (loop [i 0]
       (if (< i 10)
         (recur (if (even? c) (+ i 1) (+ i 2)))
         [(+ i 20) (- 40 i) (* i 2)]))))

(def ^:private branch-stride-overflow-form
  '(fn [^long c]
     (loop [i 0]
       (if (< i 10)
         (recur (if (even? c) (+ i 1) (+ i 2)))
         (+ i Long/MAX_VALUE)))))

(defn- branch-dependent-compiler-admission
  "branch-dependent stride 가 실제로 compiler 에서 engine range 를 통해 raw opcode 로
  승격되는지(그리고 overflow exit 은 fallback 하는지) bytecode 로 확인한다."
  []
  (let [direct   (long-arith-opcodes branch-stride-direct-form)
        overflow (long-arith-opcodes branch-stride-overflow-form)]
    {:direct-opcodes (vec (sort direct))
     :overflow-opcodes (vec (sort overflow))
     :raw-lowered? (boolean (some #{:ladd :lsub :lmul} direct))
     :overflow-falls-back? (contains? overflow :numbers-add-jj)}))

(def ^:private mixed-sign-direct-form
  '(fn [] (loop [i 0 acc 0]
            (if (< i 10)
              (recur (+ i 1) (- acc 1))
              (* acc 2)))))

(def ^:private mixed-sign-overflow-form
  '(fn [] (loop [i 0 acc 4000000000]
            (if (< i 10)
              (recur (+ i 1) (- acc 1))
              (* acc acc)))))

(defn- mixed-sign-compiler-admission
  "mixed-sign 누적이 보존 선형량 acc+i=0 으로 acc∈[-10,0] 을 얻어 (* acc 2) 가 raw lmul
  로 승격되는지, 그리고 큰 acc 의 overflow 산술은 Numbers.multiply fallback 하는지 확인."
  []
  (let [direct   (long-arith-opcodes mixed-sign-direct-form)
        overflow (long-arith-opcodes mixed-sign-overflow-form)]
    {:direct-opcodes (vec (sort direct))
     :overflow-opcodes (vec (sort overflow))
     :raw-lowered? (contains? direct :lmul)
     :overflow-falls-back? (contains? overflow :numbers-multiply-jj)}))

(def ^:private negative-factor-direct-form
  '(fn [] (loop [i 0 acc 1]
            (if (< i 5)
              (recur (+ i 1) (* acc -2))
              (+ acc 100)))))

(def ^:private negative-factor-overflow-form
  '(fn [] (loop [i 0 acc 1]
            (if (< i 40)
              (recur (+ i 1) (* acc 2))
              (* acc acc)))))

(def ^:private nonlinear-bounded-form
  '(fn [] (loop [i 0 acc 2]
            (if (< i 4)
              (recur (+ i 1) (* acc acc))
              (+ acc 1)))))

(def ^:private nonlinear-bounded-overflow-form
  '(fn [] (loop [i 0 acc 2]
            (if (< i 6)
              (recur (+ i 1) (* acc acc))
              acc))))

;; iteration count 가 정적으로 미지(param bound)라 어떤 도메인으로도 못 bound → 진짜 held.
(def ^:private general-nonlinear-form
  '(fn [n] (loop [i 0 acc 2]
             (if (< i n)
               (recur (+ i 1) (* acc acc))
               (+ acc 1)))))

(defn- nonlinear-bounded-compiler-admission
  "non-constant-factor nonlinear 점화식(acc*acc)도 유계 구간 언롤링(known N)으로 acc∈[2,65536]
  을 얻어 recur (* acc acc) 와 exit (+ acc 1) 이 raw lmul/ladd 로 승격되는지, N 이 커서
  값이 2^64 로 overflow 하는 경우는 unroll 이 top→Numbers fallback 하는지 확인."
  []
  (let [direct   (long-arith-opcodes nonlinear-bounded-form)
        overflow (long-arith-opcodes nonlinear-bounded-overflow-form)]
    {:direct-opcodes (vec (sort direct))
     :overflow-opcodes (vec (sort overflow))
     :raw-lowered? (boolean (some #{:ladd :lmul} direct))
     :overflow-falls-back? (contains? overflow :numbers-multiply-jj)}))

(defn- negative-factor-compiler-admission
  "negative-factor 곱셈 점화식이 bounded geometric acceleration(|init|*|factor|^N=32)으로
  acc∈[-32,32] 을 얻어 (+ acc 100) 가 raw ladd 로 승격되는지, 큰 acc 의 (* acc acc)=2^80 은
  Numbers.multiply fallback 하는지 확인."
  []
  (let [direct   (long-arith-opcodes negative-factor-direct-form)
        overflow (long-arith-opcodes negative-factor-overflow-form)]
    {:direct-opcodes (vec (sort direct))
     :overflow-opcodes (vec (sort overflow))
     :raw-lowered? (boolean (some #{:ladd :lmul} direct))
     :overflow-falls-back? (contains? overflow :numbers-multiply-jj)}))

(defn- branch-dependent-stride-row
  "M9b 첫 흡수: branch-dependent stride 는 이제 ad-hoc 패턴이 아니라 abstract-interval
  엔진(interval lattice + widening/narrowing fixpoint)이 sound finite range(i∈[0,11])를
  도출하고, M10 tv/lowering-sound? validator 가 승인할 때만 compiler 가 raw long opcode
  로 내린다. overflow exit 은 validator 가 거부해 Numbers fallback 으로 남는다."
  []
  (let [d (-> (ao/domain {:i (ai/interval 0 9)
                          :next (ai/interval 1 11)})
              (ao/constrain-upper [[:+ :next] [:- :i]] 2)
              (ao/constrain-lower [[:+ :next] [:- :i]] 1))
        stride (ao/expr-interval d [[:+ :next] [:- :i]])
        candidate (branch-ranking-candidate)
        admission (branch-dependent-compiler-admission)
        ok? (and (= (ai/interval 1 2) stride)
                 (= {:coeffs {:i -1} :const 9} candidate)
                 (:raw-lowered? admission)
                 (:overflow-falls-back? admission))]
    {:id :branch-dependent-stride
     :kind :engine-admitted-lowering
     :desc "interval engine derives a sound finite range, validator admits raw long opcodes; octagon stride 1..2 and linear ranking corroborate"
     :evidence {:stride-interval stride
                :ranking-candidate candidate
                :compiler-admission admission}
     :gate/verdict :accepted
     :admission-path [:abstract-interval-engine
                      :translation-validation-vc
                      :primitive-bytecode-witness]
     :promotion/allowed? true
     :ok ok?}))

(defn- mixed-sign-index-sum-row
  "M9b 둘째 흡수: mixed-sign 누적은 interval 만으로는 acc 가 unbounded 지만, compiler 가
  보존 선형량 acc+i=0 (정수 선형형 위의 관계 도메인 최소 구현)을 정확히 추론해
  acc∈[-10,0] 을 얻고, validator 승인 시 raw long opcode 로 내린다. octagon 의 acc+i=0
  도 같은 사실을 증명해 교차검증한다."
  []
  (let [d (-> (ao/domain {:i (ai/interval 0 10)
                          :acc (ai/interval -10 0)})
              (ao/constrain-eq [[:+ :acc] [:+ :i]] 0))
        sum-range (ao/expr-interval d [[:+ :acc] [:+ :i]])
        admission (mixed-sign-compiler-admission)
        ok? (and (= (ai/singleton 0) sum-range)
                 (:raw-lowered? admission)
                 (:overflow-falls-back? admission))]
    {:id :mixed-sign-index-sum
     :kind :engine-admitted-lowering
     :desc "conserved linear quantity acc+i=0 bounds acc, validator admits raw long opcode; octagon corroborates the exact sum"
     :evidence {:sum-interval sum-range
                :interval-only (ao/interval-bound d [[:+ :acc] [:+ :i]])
                :compiler-admission admission}
     :gate/verdict :accepted
     :admission-path [:conserved-linear-quantity
                      :abstract-interval-engine
                      :translation-validation-vc
                      :primitive-bytecode-witness]
     :promotion/allowed? true
     :ok ok?}))

(defn- negative-factor-recurrence-row
  "M9b 셋째 흡수: constant-factor 곱셈 점화식(부호 교번 negative factor 포함)은 bounded
  geometric acceleration 으로 |init|*|factor|^N magnitude 를 exact 계산해 symmetric range
  로 bound 한다. interval/octagon 으로는 top 이지만, bounded iteration 의 기하 가속이 최소
  nonlinear 도메인을 제공한다. validator 승인 시 raw opcode, overflow 면 fallback."
  []
  (let [admission (negative-factor-compiler-admission)
        ok? (and (:raw-lowered? admission)
                 (:overflow-falls-back? admission))]
    {:id :negative-factor-recurrence
     :kind :engine-admitted-lowering
     :desc "bounded geometric acceleration bounds |acc| <= |init|*|factor|^N, validator admits raw long opcode; sign alternation handled by the symmetric range"
     :evidence {:domain :bounded-geometric-acceleration
                :compiler-admission admission}
     :gate/verdict :accepted
     :admission-path [:bounded-geometric-acceleration
                      :translation-validation-vc
                      :primitive-bytecode-witness]
     :promotion/allowed? true
     :ok ok?}))

(defn- nonlinear-bounded-unroll-row
  "M9b 넷째 흡수: non-constant-factor nonlinear 점화식(acc'=(* acc acc), 이중지수)도 closed-form
  factor 가 없지만, 유계 인덱스의 known iteration count N 으로 interval transfer 를 그대로 N 번
  unroll 해 각 step 을 join 하면 sound finite range(acc∈[2,65536])를 얻는다. validator 승인 시
  raw opcode, N 이 커서 값이 overflow 하면 unroll 이 top→fallback."
  []
  (let [admission (nonlinear-bounded-compiler-admission)
        ok? (and (:raw-lowered? admission)
                 (:overflow-falls-back? admission))]
    {:id :nonlinear-bounded-unroll
     :kind :engine-admitted-lowering
     :desc "bounded interval unrolling over a known iteration count bounds a double-exponential recurrence, validator admits raw long opcode"
     :evidence {:domain :bounded-interval-unrolling
                :compiler-admission admission}
     :gate/verdict :accepted
     :admission-path [:bounded-interval-unrolling
                      :translation-validation-vc
                      :primitive-bytecode-witness]
     :promotion/allowed? true
     :ok ok?}))

(defn- general-nonlinear-recurrence-row
  "A statically unknown iteration count has no proof in this analyzer, so the
  executable path remains checked instead of claiming an impossible bound."
  []
  (let [ops (long-arith-opcodes general-nonlinear-form)
        ok? (not (some #{:lmul} ops))]
    {:id :general-nonlinear-recurrence
     :kind :checked-fallback-boundary
     :desc "nonlinear recurrence with a statically unknown iteration count has no bound proof in this analyzer and stays checked"
     :evidence {:opcodes (vec (sort ops))
                :raw-lowered? (boolean (some #{:lmul} ops))}
     :required-before-admission
     [:unbounded-nonlinear-recurrence-domain
      :overflow-vc-discharge
      :translation-validation-receipt
      :primitive-bytecode-witness]
     :gate/verdict :accepted
     :boundary/status :checked-fallback
     :boundary/reason :static-bound-not-established
     :promotion/allowed? false
     :fallback :numbers-primitive-descriptor-calls
     :ok ok?}))

(defn run
  []
  (let [rows [(branch-dependent-stride-row)
              (mixed-sign-index-sum-row)
              (negative-factor-recurrence-row)
              (nonlinear-bounded-unroll-row)
              (general-nonlinear-recurrence-row)]
        accepted (filter #(= :accepted (:gate/verdict %)) rows)
        held (filter #(= :held (:gate/verdict %)) rows)
        rejected (filter #(= :rejected (:gate/verdict %)) rows)
        canonical (mapv #(select-keys %
                                      [:id
                                       :kind
                                       :evidence
                                       :required-before-admission
                                       :admission-path
                                       :gate/verdict
                                       :held-reason
                                       :promotion/allowed?
                                       :fallback
                                       :ok])
                        rows)
        row-by      (fn [id] (first (filter #(= id (:id %)) rows)))
        bytecode-admitted? (fn [row]
                             (and (= :accepted (:gate/verdict row))
                                  (get-in row [:evidence :compiler-admission :raw-lowered?])
                                  (get-in row [:evidence :compiler-admission :overflow-falls-back?])))
        invariants (sorted-map
                    :all-m6aj-cases-accounted
                    (= #{:branch-dependent-stride
                         :mixed-sign-index-sum
                         :negative-factor-recurrence
                         :nonlinear-bounded-unroll
                         :general-nonlinear-recurrence}
                       (set (map :id rows)))
                    :all-rows-ok
                    (every? :ok rows)
                    ;; M9b 첫 흡수: branch-dependent stride 는 interval engine+validator 로 admitted.
                    :branch-dependent-stride-engine-admitted
                    (bytecode-admitted? (row-by :branch-dependent-stride))
                    ;; M9b 둘째 흡수: mixed-sign 은 보존 선형량(관계 도메인)+validator 로 admitted.
                    :mixed-sign-conserved-quantity-admitted
                    (bytecode-admitted? (row-by :mixed-sign-index-sum))
                    ;; M9b 셋째 흡수: negative-factor 는 bounded geometric acceleration+validator 로 admitted.
                    :negative-factor-geometric-admitted
                    (bytecode-admitted? (row-by :negative-factor-recurrence))
                    ;; M9b 넷째 흡수: bounded nonlinear(acc*acc) 은 interval unrolling+validator 로 admitted.
                    :nonlinear-bounded-unroll-admitted
                    (bytecode-admitted? (row-by :nonlinear-bounded-unroll))
                    ;; Unknown iteration count keeps checked execution; it does
                    ;; not justify either a raw-opcode promotion or a hold.
                    :unbounded-nonlinear-checked-fallback-active
                    (let [r (row-by :general-nonlinear-recurrence)]
                      (and (= :accepted (:gate/verdict r))
                           (= :checked-fallback (:boundary/status r))
                           (not (get-in r [:evidence :raw-lowered?]))))
                    :no-rejected-rows
                    (empty? rejected)
                    :unbounded-nonlinear-fallback-present
                    (= :numbers-primitive-descriptor-calls
                       (:fallback (row-by :general-nonlinear-recurrence))))
        ok? (every? true? (vals invariants))]
    {:schema "pnix.clj-meta.m6aj-framework.receipt.v1"
     :stage [:M9b :M6aj]
     :target-goal "meta-circular stage15/N Clojure compiler"
     :desc "M6aj hard cases admitted through M9 substrates (interval fixpoint, conserved linear quantity, bounded geometric acceleration, bounded interval unrolling), each validator-gated and bytecode-witnessed; only a nonlinear recurrence with a statically-unknown iteration count stays held"
     :policy {:engine-admission "branch-dependent stride (interval fixpoint), mixed-sign sum (conserved linear quantity), negative-factor (geometric acceleration), and bounded non-constant-factor nonlinear acc*acc (interval unrolling over a known iteration count) each derive a sound finite range and the tv validator admits the raw long opcode"
              :checked-fallback "a nonlinear recurrence with no bound proof in this analyzer stays on Numbers.* checked execution"
              :soundness "every derived range is over-approximating; the validator independently rejects overflow, and overflow sentinels confirm fallback"
              :no-ad-hoc-pattern "no new hand-rolled recognizer was added; the general engine + relational/acceleration/unrolling substrates are the consumers"}
     :rows rows
     :status-counts {:accepted (count accepted)
                     :checked-fallback (count (filter #(= :checked-fallback (:boundary/status %)) accepted))
                     :rejected (count rejected)}
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
    (println (str "M6aj framework: "
                  (if (:ok r) "OK" "FAILED")
                  "  (receipt: " receipt-path
                  ", digest: " (:canonical-receipt-digest r)
                  ")"))
    (shutdown-agents)
    (when-not (:ok r)
      (System/exit 1))))
