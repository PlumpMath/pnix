(ns pnix.clj-meta.abstract-interval
  "M9 abstract-interpretation interval domain.

  This is the first reusable range-proof substrate for checked-long lowering:
  an over-approximating interval lattice, arithmetic transfer functions, a
  Var->Interval environment, and a small widening/narrowing fixpoint witness.
  The compiler still owns admission of raw opcodes; this namespace only proves
  facts that later lowering validators can consume."
  (:require [clojure.java.io :as io]
            [clojure.pprint :as pp])
  (:import [java.security MessageDigest]))

(def receipt-path "clj-meta/proof/abstract-interval.receipt.edn")

(def neg-inf :neg-inf)
(def pos-inf :pos-inf)
(def bottom {:kind :bottom})
(def top {:kind :interval :min neg-inf :max pos-inf})

(defn bottom?
  [i]
  (= (:kind i) :bottom))

(defn interval?
  [i]
  (= (:kind i) :interval))

(defn finite-bound?
  [x]
  (and (integer? x)
       (<= Long/MIN_VALUE x Long/MAX_VALUE)))

(defn finite?
  [i]
  (and (interval? i)
       (finite-bound? (:min i))
       (finite-bound? (:max i))))

(defn top?
  [i]
  (= i top))

(defn- lower<=
  [a b]
  (cond
    (= a b) true
    (= a neg-inf) true
    (= b pos-inf) true
    (= a pos-inf) false
    (= b neg-inf) false
    :else (<= a b)))

(defn- bound-min
  [a b]
  (if (lower<= a b) a b))

(defn- bound-max
  [a b]
  (if (lower<= a b) b a))

(defn interval
  [lo hi]
  (if (lower<= lo hi)
    {:kind :interval :min lo :max hi}
    bottom))

(defn singleton
  [x]
  (interval (long x) (long x)))

(defn join
  [a b]
  (cond
    (bottom? a) b
    (bottom? b) a
    :else (interval (bound-min (:min a) (:min b))
                    (bound-max (:max a) (:max b)))))

(defn meet
  [a b]
  (cond
    (bottom? a) bottom
    (bottom? b) bottom
    :else (interval (bound-max (:min a) (:min b))
                    (bound-min (:max a) (:max b)))))

(defn widen
  [old new]
  (cond
    (bottom? old) new
    (bottom? new) old
    :else (interval (if (lower<= (:min new) (:min old))
                      (if (= (:min new) (:min old)) (:min old) neg-inf)
                      (:min old))
                    (if (lower<= (:max old) (:max new))
                      (if (= (:max old) (:max new)) (:max old) pos-inf)
                      (:max old)))))

(defn narrow
  [old new]
  (cond
    (bottom? old) bottom
    (bottom? new) bottom
    :else (interval (if (and (= (:min old) neg-inf)
                             (not= (:min new) neg-inf))
                      (:min new)
                      (:min old))
                    (if (and (= (:max old) pos-inf)
                             (not= (:max new) pos-inf))
                      (:max new)
                      (:max old)))))

(defn- exact-long-op
  [op a b]
  (try
    (case op
      :+ (Math/addExact (long a) (long b))
      :- (Math/subtractExact (long a) (long b))
      :* (Math/multiplyExact (long a) (long b)))
    (catch ArithmeticException _
      nil)))

(defn- bound-negate
  [b]
  (cond
    (= b neg-inf) pos-inf
    (= b pos-inf) neg-inf
    (= b Long/MIN_VALUE) pos-inf
    :else (- (long b))))

(defn- bound-add
  [a b]
  (cond
    (and (= a neg-inf) (= b pos-inf)) nil
    (and (= a pos-inf) (= b neg-inf)) nil
    (or (= a neg-inf) (= b neg-inf)) neg-inf
    (or (= a pos-inf) (= b pos-inf)) pos-inf
    :else (exact-long-op :+ a b)))

(defn- bound-sub
  [a b]
  (bound-add a (bound-negate b)))

(defn- bound-sign
  [b]
  (cond
    (= b neg-inf) :neg
    (= b pos-inf) :pos
    (neg? b) :neg
    (pos? b) :pos
    :else :zero))

(defn- sign-mul
  [a b]
  (let [sa (bound-sign a)
        sb (bound-sign b)]
    (cond
      (or (= sa :zero) (= sb :zero)) :zero
      (= sa sb) :pos
      :else :neg)))

(defn- bound-mul
  [a b]
  (cond
    (and (finite-bound? a) (finite-bound? b))
    (exact-long-op :* a b)

    (or (= (bound-sign a) :zero)
        (= (bound-sign b) :zero))
    0

    :else
    (case (sign-mul a b)
      :pos pos-inf
      :neg neg-inf
      :zero 0)))

(defn- transfer2
  [op a b]
  (cond
    (bottom? a) bottom
    (bottom? b) bottom
    :else
    (let [points (case op
                   :+ [(bound-add (:min a) (:min b))
                       (bound-add (:max a) (:max b))]
                   :- [(bound-sub (:min a) (:max b))
                       (bound-sub (:max a) (:min b))]
                   :* [(bound-mul (:min a) (:min b))
                       (bound-mul (:min a) (:max b))
                       (bound-mul (:max a) (:min b))
                       (bound-mul (:max a) (:max b))])]
      (if (some nil? points)
        top
        (interval (reduce bound-min points)
                  (reduce bound-max points))))))

(defn add
  [a b]
  (transfer2 :+ a b))

(defn subtract
  [a b]
  (transfer2 :- a b))

(defn multiply
  [a b]
  (transfer2 :* a b))

(defn transfer
  [op a b]
  (case op
    :+ (add a b)
    :- (subtract a b)
    :* (multiply a b)))

(defn safe-long-transfer?
  [op a b]
  (finite? (transfer op a b)))

(defn- merge-env-with
  [f a b]
  (into (sorted-map)
        (map (fn [k] [k (f (get a k bottom)
                           (get b k bottom))])
             (sort-by str (distinct (concat (keys a) (keys b)))))))

(defn env-join
  [a b]
  (merge-env-with join a b))

(defn env-meet
  [a b]
  (merge-env-with meet a b))

(defn env-widen
  [old new]
  (merge-env-with widen old new))

(defn env-narrow
  [old new]
  (merge-env-with narrow old new))

(declare eval-expr)

(defn eval-expr
  [env expr]
  (cond
    (keyword? expr) (get env expr top)
    (symbol? expr) (get env expr top)
    (integer? expr) (singleton expr)
    (and (map? expr) (contains? expr :min) (contains? expr :max))
    (interval (:min expr) (:max expr))
    (and (vector? expr) (= 3 (count expr)))
    (let [[op a b] expr]
      (transfer op (eval-expr env a) (eval-expr env b)))
    :else top))

(defn assign
  [env local expr]
  (assoc env local (eval-expr env expr)))

(defn- dec-bound
  [x]
  (when-not (= x Long/MIN_VALUE)
    (exact-long-op :- x 1)))

(defn- inc-bound
  [x]
  (when-not (= x Long/MAX_VALUE)
    (exact-long-op :+ x 1)))

(defn guard
  [env [op local bound]]
  (let [current (get env local top)
        restricted (case op
                     :<  (if-let [hi (dec-bound bound)]
                           (meet current (interval neg-inf hi))
                           bottom)
                     :<= (meet current (interval neg-inf (long bound)))
                     :>  (if-let [lo (inc-bound bound)]
                           (meet current (interval lo pos-inf))
                           bottom)
                     :>= (meet current (interval (long bound) pos-inf)))]
    (assoc env local restricted)))

(defn- fixpoint-step
  [initial transfer-f current]
  (env-join initial (transfer-f current)))

(defn fixpoint
  [initial transfer-f opts]
  (let [max-iterations (or (:max-iterations opts) 64)
        widen-after    (or (:widen-after opts) 4)]
    (loop [n 0
           current initial
           rows []]
      (let [next    (fixpoint-step initial transfer-f current)
            widened (if (>= n widen-after)
                      (env-widen current next)
                      next)
            row     {:iteration n
                     :state current
                     :next next
                     :widened widened}]
        (if (or (= current widened)
                (>= n max-iterations))
          {:state widened
           :iterations n
           :rows (conj rows row)
           :terminated? (= current widened)}
          (recur (inc n) widened (conj rows row)))))))

(defn narrow-fixpoint
  [initial transfer-f widened opts]
  (let [max-iterations (or (:max-iterations opts) 16)]
    (loop [n 0
           current widened
           rows []]
      (let [next     (fixpoint-step initial transfer-f current)
            narrowed (env-narrow current next)
            row      {:iteration n
                      :state current
                      :next next
                      :narrowed narrowed}]
        (if (or (= current narrowed)
                (>= n max-iterations))
          {:state narrowed
           :iterations n
           :rows (conj rows row)
           :terminated? (= current narrowed)}
          (recur (inc n) narrowed (conj rows row)))))))

(defn loop-closure
  [initial transfer-f opts]
  (let [widened  (fixpoint initial transfer-f opts)
        narrowed (narrow-fixpoint initial
                                  transfer-f
                                  (:state widened)
                                  opts)]
    {:widening widened
     :narrowing narrowed
     :state (:state narrowed)}))

(defn- sha256-string
  [s]
  (let [md (MessageDigest/getInstance "SHA-256")]
    (apply str
           (map #(format "%02x" (bit-and 0xff %))
                (.digest md (.getBytes ^String (str s) "UTF-8"))))))

(defn- case-result
  [id desc got expected]
  {:id id
   :desc desc
   :got got
   :expected expected
   :ok (= got expected)})

(defn run
  []
  (let [counter-step (fn [env]
                       (assign (guard env [:< :i 5])
                               :i
                               [:+ :i 1]))
        closure      (loop-closure {:i (singleton 0)}
                                   counter-step
                                   {:widen-after 2
                                    :max-iterations 16})
        body-state   (guard (:state closure) [:< :i 5])
        cases        [(case-result
                       :lattice-join-meet
                       "bottom/top plus finite join and meet"
                       {:join (join (interval 1 3) (interval 2 5))
                        :meet (meet (interval 1 3) (interval 2 5))
                        :empty (meet (interval 1 2) (interval 3 4))}
                       {:join (interval 1 5)
                        :meet (interval 2 3)
                        :empty bottom})
                      (case-result
                       :finite-long-transfer
                       "finite + - * transfer preserves exact long endpoints"
                       {:add (add (singleton 20) (singleton 22))
                        :sub (subtract (singleton 50) (singleton 8))
                        :mul (multiply (interval 6 7) (interval 2 3))}
                       {:add (singleton 42)
                        :sub (singleton 42)
                        :mul (interval 12 21)})
                      (case-result
                       :overflow-becomes-top
                       "endpoint overflow becomes conservative top, not a raw-lowering proof"
                       {:result (add (singleton Long/MAX_VALUE) (singleton 1))
                        :safe? (safe-long-transfer?
                                :+
                                (singleton Long/MAX_VALUE)
                                (singleton 1))}
                       {:result top
                        :safe? false})
                      (case-result
                       :environment-transfer
                       "Var->Interval environment transfer assigns derived range"
                       (assign {:a (singleton 20)
                                :b (singleton 22)}
                               :c
                               [:+ :a :b])
                       {:a (singleton 20)
                        :b (singleton 22)
                        :c (singleton 42)})
                      (case-result
                       :guard-narrowing
                       "integer guards narrow top through interval meet"
                       (-> {:i top}
                           (guard [:>= :i 0])
                           (guard [:< :i 5])
                           :i)
                       (interval 0 4))
                      (case-result
                       :widening-narrowing-counter
                       "loop head fixpoint widens, then guard narrowing recovers a finite counter range"
                       {:widened (get-in closure [:widening :state :i])
                        :head (get-in closure [:state :i])
                        :body (:i body-state)
                        :after-step (:i (counter-step (:state closure)))
                        :terminated? (and (get-in closure [:widening :terminated?])
                                          (get-in closure [:narrowing :terminated?]))}
                       {:widened (interval 0 pos-inf)
                        :head (interval 0 5)
                        :body (interval 0 4)
                        :after-step (interval 1 5)
                        :terminated? true})]
        canonical    (mapv #(select-keys % [:id :expected :got :ok]) cases)
        invariants   (sorted-map
                      :all-cases-ok (every? :ok cases)
                      :bottom-absorbs-transfer (bottom? (add bottom (singleton 1)))
                      :finite-transfer-proves-not-may
                      (safe-long-transfer? :* (interval 2 3) (interval 4 5))
                      :overflow-is-not-accepted
                      (false? (safe-long-transfer?
                               :+
                               (singleton Long/MAX_VALUE)
                               (singleton 1)))
                      :widening-and-narrowing-terminate
                      (and (get-in closure [:widening :terminated?])
                           (get-in closure [:narrowing :terminated?])))
        ok?          (and (every? :ok cases)
                          (every? true? (vals invariants)))]
    {:schema "pnix.clj-meta.abstract-interval.receipt.v1"
     :stage [:M9]
     :target-goal "meta-circular stage15/N Clojure compiler"
     :desc "interval lattice, arithmetic transfer, environment transfer, and widening/narrowing witness"
     :domain {:bottom bottom
              :top top
              :bounds [neg-inf pos-inf]
              :operations [:join :meet :widen :narrow :+ :- :*]}
     :policy {:safe-result "finite transfer result may become a raw-lowering proof input"
              :unsafe-result "top or bottom-free unknown stays checked/fallback"
              :compiler-admission "not wired into raw opcode emission yet"}
     :cases cases
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
    (doseq [row (:cases r)]
      (println (str "  [" (if (:ok row) "OK" "FAIL") "] "
                    (name (:id row)))))
    (println (str "abstract interval: "
                  (if (:ok r) "OK" "FAILED")
                  "  (receipt: " receipt-path
                  ", digest: " (:canonical-receipt-digest r)
                  ")"))
    (shutdown-agents)
    (when-not (:ok r)
      (System/exit 1))))
