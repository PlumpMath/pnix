(ns pnix.clj-meta.translation-validation
  "M10 translation-validation witness.

  The compiler's checked-long raw opcode admission path consumes this boundary:
  every raw lowering candidate is converted into an explicit overflow-freedom
  VC, the checker re-runs the M9 interval transfer on operand ranges and gates
  on finiteness, and unproven candidates are rejected/fallback rather than
  silently accepted.

  Defense-in-depth boundary (2026-06-29): when a VC carries an independent
  operand derivation (octagon self-test or compiler bounded-unroll interval),
  validation joins the supplied range with that independently derived range and
  checks the wider result. This catches too-tight supplied ranges for covered
  VCs. VCs without an independent operand derivation remain sound relative to
  their range provider and must not be overclaimed as full proof."
  (:require [clojure.java.io :as io]
            [clojure.pprint :as pp]
            [pnix.clj-meta.abstract-interval :as ai]
            [pnix.clj-meta.abstract-octagon :as ao])
  (:import [java.security MessageDigest]))

(def receipt-path "clj-meta/proof/translation-validation.receipt.edn")

(defn- sha256-string
  [s]
  (let [md (MessageDigest/getInstance "SHA-256")]
    (apply str
           (map #(format "%02x" (bit-and 0xff %))
                (.digest md (.getBytes ^String (str s) "UTF-8"))))))

(defn- opcode-for
  [op]
  (case op
    :+ :ladd
    :- :lsub
    :* :lmul))

(defn- interval-ref
  [i]
  (select-keys i [:kind :min :max]))

(defn- vc-digest
  [vc]
  (sha256-string
   (pr-str (select-keys vc
                        [:vc/kind
                         :source/id
                         :op
                         :opcode
                         :lhs
                         :rhs
                         :independent-lhs
                         :independent-rhs
                         :effective-lhs
                         :effective-rhs
                         :claimed-result]))))

(defn- independent-operand-range
  [spec]
  (when spec
    (cond
      (ai/interval? (:interval spec))
      (:interval spec)

      (:domain spec)
      (ao/expr-interval (:domain spec) (:terms spec))

      :else nil)))

(defn- effective-range
  [supplied independent]
  (if independent
    (ai/join supplied independent)
    supplied))

(defn make-vc
  [{:keys [id op lhs rhs opcode independent]}]
  (let [independent-lhs (independent-operand-range (:lhs independent))
        independent-rhs (independent-operand-range (:rhs independent))
        effective-lhs   (effective-range lhs independent-lhs)
        effective-rhs   (effective-range rhs independent-rhs)
        supplied-result (ai/transfer op lhs rhs)
        result          (ai/transfer op effective-lhs effective-rhs)]
    {:vc/kind :overflow-freedom
     :source/id id
     :op op
     :opcode opcode
     :expected-opcode (opcode-for op)
     :lhs lhs
     :rhs rhs
     :supplied-result supplied-result
     :independent-lhs independent-lhs
     :independent-rhs independent-rhs
     :effective-lhs effective-lhs
     :effective-rhs effective-rhs
     :claimed-result result
     :claimed-finite? (ai/finite? result)}))

(defn lowering-sound?
  "raw opcode 승급 후보 VC 가 통과 가능한가. 검사: (1) opcode 가 op 에 맞고,
  (2) 유효 operand range 로 transfer 를 재계산한 값이 claimed-result 와 일치(consistency
  recompute — soundness 검사가 아니라 일관성 검사), (3) 그 결과가 finite(overflow-free).
  independent octagon operand derivation 이 있으면 공급 range 와 독립 range 를 join 한 더 넓은
  range 로 검사해 too-tight supplied range 를 reject 한다."
  [vc]
  (let [computed (ai/transfer (:op vc) (:effective-lhs vc) (:effective-rhs vc))]
    (and (= (:opcode vc) (:expected-opcode vc))
         (= computed (:claimed-result vc))
         (ai/finite? computed))))

(defn validate-vc
  [vc]
  (let [computed (ai/transfer (:op vc) (:effective-lhs vc) (:effective-rhs vc))
        opcode-ok? (= (:opcode vc) (:expected-opcode vc))
        result-ok? (= computed (:claimed-result vc))
        finite-ok? (ai/finite? computed)
        ok?        (and opcode-ok? result-ok? finite-ok?)]
    {:vc/digest (vc-digest vc)
     :computed-result computed
     :opcode-ok? opcode-ok?
     :result-ok? result-ok?
     :finite-ok? finite-ok?
     :lowering-sound? ok?
     :verdict (if ok? :accepted :rejected)
     :reject-reason (when-not ok?
                      (cond
                        (not opcode-ok?) :opcode-mismatch
                        (not result-ok?) :result-mismatch
                        (not finite-ok?) :overflow-freedom-not-proven
                        :else :unknown))}))

(defn- candidate
  ([id desc op lhs rhs expected-verdict]
   (candidate id desc op lhs rhs (opcode-for op) expected-verdict))
  ([id desc op lhs rhs opcode expected-verdict]
   (candidate id desc op lhs rhs opcode expected-verdict nil))
  ([id desc op lhs rhs opcode expected-verdict extra]
   (merge {:id id
           :desc desc
           :op op
           :lhs lhs
           :rhs rhs
           :opcode opcode
           :expected-verdict expected-verdict}
          extra)))

(defn- candidates
  []
  [(candidate
    :literal-add-vc
    "literal checked-long add candidate discharges overflow-freedom VC"
    :+
    (ai/singleton 20)
    (ai/singleton 22)
    :accepted)
   (candidate
    :literal-sub-vc
    "literal checked-long subtract candidate discharges overflow-freedom VC"
    :-
    (ai/singleton 50)
    (ai/singleton 8)
    :accepted)
   (candidate
    :literal-mul-vc
    "literal checked-long multiply candidate discharges overflow-freedom VC"
    :*
    (ai/singleton 6)
    (ai/singleton 7)
    :accepted)
   (candidate
    :let-local-range-vc
    "let-local interval candidate discharges through the same validator"
    :+
    (ai/interval 20 20)
    (ai/interval 22 22)
    :accepted)
   (candidate
    :guarded-argument-range-vc
    "branch-local guard range candidate remains finite and accepted"
    :*
    (ai/interval -9 9)
    (ai/singleton 2)
    :accepted)
   (candidate
    :loop-exit-range-vc
    "bounded loop exit interval candidate remains finite and accepted"
    :+
    (ai/interval 10 12)
    (ai/singleton 20)
    :accepted)
   (candidate
    :boundary-add-max-zero-vc
    "Long/MAX_VALUE plus zero remains a finite boundary candidate"
    :+
    (ai/singleton Long/MAX_VALUE)
    (ai/singleton 0)
    :accepted)
   (candidate
    :boundary-sub-min-zero-vc
    "Long/MIN_VALUE minus zero remains a finite boundary candidate"
    :-
    (ai/singleton Long/MIN_VALUE)
    (ai/singleton 0)
    :accepted)
   (candidate
    :overflow-add-rejected
    "overflowing add candidate is rejected and must keep checked fallback"
    :+
    (ai/singleton Long/MAX_VALUE)
    (ai/singleton 1)
    :rejected)
   (candidate
    :overflow-sub-rejected
    "overflowing subtract candidate is rejected and must keep checked fallback"
    :-
    (ai/singleton Long/MIN_VALUE)
    (ai/singleton 1)
    :rejected)
   (candidate
    :overflow-mul-rejected
    "overflowing multiply candidate is rejected and must keep checked fallback"
    :*
    (ai/singleton 4611686018427387904)
    (ai/singleton 2)
    :rejected)
   (candidate
    :overflow-min-negate-mul-rejected
    "Long/MIN_VALUE multiplied by -1 is rejected rather than raw-lowered"
    :*
    (ai/singleton Long/MIN_VALUE)
    (ai/singleton -1)
    :rejected)
   (candidate
    :wide-interval-add-rejected
    "wide interval whose upper endpoint can overflow is rejected"
    :+
    (ai/interval 0 Long/MAX_VALUE)
    (ai/singleton 1)
    :rejected)
   (candidate
    :wide-interval-mul-rejected
    "mixed-sign wide interval multiplication that can overflow is rejected"
    :*
    (ai/interval Long/MIN_VALUE Long/MAX_VALUE)
    (ai/singleton -1)
    :rejected)
   (candidate
    :independent-octagon-corroborated-add-vc
    "independent octagon operand derivation agrees with supplied range and stays accepted"
    :+
    (ai/interval 10 12)
    (ai/singleton 1)
    (opcode-for :+)
    :accepted
    {:independent {:lhs {:domain (ao/domain {:x (ai/interval 10 12)})
                         :terms [[:+ :x]]}}})
   (candidate
    :too-narrow-range-rejected-by-independent-domain
    "too-tight supplied range is rejected when independent octagon derivation exposes overflow possibility"
    :+
    (ai/interval 0 10)
    (ai/singleton 1)
    (opcode-for :+)
    :rejected
    {:independent {:lhs {:domain (ao/domain {:x (ai/interval 0 Long/MAX_VALUE)})
                         :terms [[:+ :x]]}}})
   (candidate
    :too-narrow-range-rejected-by-independent-interval
    "too-tight supplied range is rejected when independent bounded-unroll interval exposes overflow possibility"
    :+
    (ai/interval 0 0)
    (ai/singleton Long/MAX_VALUE)
    (opcode-for :+)
    :rejected
    {:independent {:lhs {:interval (ai/interval 0 6)
                         :source :compiler-bounded-unroll}}})
   (candidate
    :opcode-mismatch-rejected
    "wrong raw opcode cannot pass the named lowering-sound? property"
    :+
    (ai/singleton 20)
    (ai/singleton 22)
    :lsub
    :rejected)])

(defn- row
  [candidate]
  (let [vc         (make-vc candidate)
        validation (validate-vc vc)
        verdict    (:verdict validation)
        fallback?  (= :rejected verdict)]
    {:id (:id candidate)
     :desc (:desc candidate)
     :candidate (select-keys candidate [:op :opcode :expected-verdict :independent])
     :vc (assoc (select-keys vc
                             [:vc/kind
                              :source/id
                              :op
                              :opcode
                              :expected-opcode
                              :claimed-finite?])
                :lhs (interval-ref (:lhs vc))
                :rhs (interval-ref (:rhs vc))
                :supplied-result (interval-ref (:supplied-result vc))
                :independent-lhs (some-> (:independent-lhs vc) interval-ref)
                :independent-rhs (some-> (:independent-rhs vc) interval-ref)
                :effective-lhs (interval-ref (:effective-lhs vc))
                :effective-rhs (interval-ref (:effective-rhs vc))
                :claimed-result (interval-ref (:claimed-result vc))
                :digest (vc-digest vc))
     :validation validation
     :gate/verdict verdict
     :fallback fallback?
     :ok (= verdict (:expected-verdict candidate))}))

(defn run
  []
  (let [rows       (mapv row (candidates))
        accepted   (filter #(= :accepted (:gate/verdict %)) rows)
        rejected   (filter #(= :rejected (:gate/verdict %)) rows)
        canonical  (mapv #(select-keys %
                                       [:id
                                        :candidate
                                        :vc
                                        :gate/verdict
                                        :fallback
                                        :ok])
                         rows)
        invariants (sorted-map
                    :all-rows-ok (every? :ok rows)
                    :accepted-have-sound-lowering
                    (every? #(true? (get-in % [:validation :lowering-sound?]))
                            accepted)
                    :emit-or-refuse
                    (not-any? #(and (= :rejected (:gate/verdict %))
                                    (false? (:fallback %)))
                              rows)
                    :overflow-candidates-rejected
                    (every? #(= :rejected (:gate/verdict %))
                            (filter #(#{:overflow-add-rejected
                                        :overflow-sub-rejected
                                        :overflow-mul-rejected
                                        :overflow-min-negate-mul-rejected
                                        :wide-interval-add-rejected
                                        :wide-interval-mul-rejected
                                        :too-narrow-range-rejected-by-independent-domain
                                        :too-narrow-range-rejected-by-independent-interval}
                                      (:id %))
                                    rows))
                    :independent-domain-corroborated-accepted
                    (= :accepted
                       (:gate/verdict
                        (first (filter #(= :independent-octagon-corroborated-add-vc
                                           (:id %))
                                       rows))))
                    :too-narrow-range-caught-by-independent-domain
                    (let [row (first (filter #(= :too-narrow-range-rejected-by-independent-domain
                                                 (:id %))
                                             rows))]
                      (and (= :rejected (:gate/verdict row))
                           (= :overflow-freedom-not-proven
                              (get-in row [:validation :reject-reason]))))
                    :too-narrow-range-caught-by-independent-interval
                    (let [row (first (filter #(= :too-narrow-range-rejected-by-independent-interval
                                                 (:id %))
                                             rows))]
                      (and (= :rejected (:gate/verdict row))
                           (= :overflow-freedom-not-proven
                              (get-in row [:validation :reject-reason]))))
                    :opcode-mismatch-rejected
                    (= :rejected
                       (:gate/verdict
                        (first (filter #(= :opcode-mismatch-rejected (:id %))
                                       rows))))
                    :vc-digests-present
                    (every? #(string? (get-in % [:vc :digest])) rows))
        status     {:accepted (count accepted)
                    :rejected (count rejected)}
        ok?        (and (every? :ok rows)
                        (every? true? (vals invariants)))]
    {:schema "pnix.clj-meta.translation-validation.receipt.v1"
     :stage [:M10]
     :target-goal "meta-circular stage15/N Clojure compiler"
     :desc "overflow-freedom VC schema, finiteness validator with optional independent operand-range corroboration, and emit-or-refuse witness"
     :named-correctness-property :lowering-sound?
     :trust-boundary "validator joins supplied operand ranges with independent operand ranges when present (octagon self-tests or compiler bounded-unroll intervals); VCs without independent derivation remain relative to the range provider"
     :policy {:accepted "raw opcode candidate may be admitted only after validator proves finite interval result over effective ranges"
              :rejected "candidate must fall back to checked Numbers path or fail closed"
              :compiler-admission "compiler checked-long raw emit calls lowering-sound? before LADD/LSUB/LMUL admission"}
     :status-counts status
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
    (println (str "translation validation: "
                  (if (:ok r) "OK" "FAILED")
                  "  (receipt: " receipt-path
                  ", digest: " (:canonical-receipt-digest r)
                  ")"))
    (shutdown-agents)
    (when-not (:ok r)
      (System/exit 1))))
