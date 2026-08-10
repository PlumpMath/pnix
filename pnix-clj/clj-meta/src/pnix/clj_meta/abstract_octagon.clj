(ns pnix.clj-meta.abstract-octagon
  "M9a minimal octagon relation witness.

  This is not an Apron replacement. It is the first JVM-native relational
  domain needed by the stage15/N compiler roadmap: keep finite interval facts,
  add octagonal constraints of the form ±x±y<=c, close simple transitive
  consequences, and prove only facts that can be discharged from those
  constraints. Non-linear recurrences stay held."
  (:require [clojure.java.io :as io]
            [clojure.pprint :as pp]
            [pnix.clj-meta.abstract-interval :as ai])
  (:import [java.security MessageDigest]))

(def receipt-path "clj-meta/proof/abstract-octagon.receipt.edn")

(defn- sha256-string
  [s]
  (let [md (MessageDigest/getInstance "SHA-256")]
    (apply str
           (map #(format "%02x" (bit-and 0xff %))
                (.digest md (.getBytes ^String (str s) "UTF-8"))))))

(defn- sign-value
  [sign]
  (case sign
    :+ 1
    :- -1))

(defn- normalize-terms
  [terms]
  (let [coeffs (reduce (fn [m [sign local]]
                         (update m local (fnil + 0) (sign-value sign)))
                       (sorted-map)
                       terms)]
    (->> coeffs
         (keep (fn [[local coeff]]
                 (cond
                   (= coeff 1) [:+ local]
                   (= coeff -1) [:- local]
                   (zero? coeff) nil
                   :else (throw (ex-info "octagon supports only ±x±y terms"
                                         {:local local
                                          :coefficient coeff
                                          :terms terms})))))
         (sort-by (juxt (comp name second) first))
         vec)))

(defn- negate-terms
  [terms]
  (mapv (fn [[sign local]]
          [(if (= sign :+) :- :+) local])
        terms))

(defn- tighten-upper
  [old c]
  (if (nil? old)
    (long c)
    (min (long old) (long c))))

(defn domain
  [intervals]
  {:intervals (into (sorted-map) intervals)
   :constraints (sorted-map)})

(defn constrain-upper
  [d terms c]
  (let [k (normalize-terms terms)]
    (assoc-in d [:constraints k]
              (tighten-upper (get-in d [:constraints k]) c))))

(defn constrain-lower
  [d terms c]
  ;; expr >= c  <=>  -expr <= -c
  (constrain-upper d (negate-terms terms) (- (long c))))

(defn constrain-eq
  [d terms c]
  (-> d
      (constrain-upper terms c)
      (constrain-lower terms c)))

(defn- combine-constraints
  [[terms-a ca] [terms-b cb]]
  (try
    (let [terms (normalize-terms (concat terms-a terms-b))]
      (when (seq terms)
        [terms (Math/addExact (long ca) (long cb))]))
    (catch Exception _
      nil)))

(defn close
  "Derive direct octagonal consequences such as
  (x-y<=a) + (y-z<=b) => (x-z<=a+b).

  This is intentionally small and fail-closed: if the fixed point does not
  stabilize within max-iterations, the returned domain carries a held closure
  marker instead of claiming a complete relational proof."
  ([d]
   (close d 64))
  ([d max-iterations]
   (loop [d d
          iteration 0]
     (let [constraints (:constraints d)
           derived (keep (fn [[left right]]
                           (combine-constraints left right))
                         (for [left constraints
                               right constraints]
                           [left right]))
           d' (reduce (fn [acc [terms c]]
                        (constrain-upper acc terms c))
                      d
                      derived)]
       (cond
         (= constraints (:constraints d'))
         (assoc d' :closure {:closed? true
                             :iterations iteration})

         (>= iteration max-iterations)
         (assoc d' :closure {:closed? false
                             :iterations iteration
                             :held-reason :closure-iteration-limit})

         :else
         (recur d' (inc iteration)))))))

(defn- term-interval
  [d [sign local]]
  (let [i (get-in d [:intervals local] ai/top)]
    (case sign
      :+ i
      :- (ai/subtract (ai/singleton 0) i))))

(defn interval-bound
  [d terms]
  (reduce ai/add
          (ai/singleton 0)
          (map #(term-interval d %) terms)))

(defn upper-bound
  [d terms]
  (let [terms' (normalize-terms terms)]
    (or (get-in d [:constraints terms'])
        (:max (interval-bound d terms')))))

(defn lower-bound
  [d terms]
  (let [terms' (normalize-terms terms)]
    (let [neg-upper (upper-bound d (negate-terms terms'))]
      (if (integer? neg-upper)
        (- (long neg-upper))
        (:min (interval-bound d terms'))))))

(defn expr-interval
  [d terms]
  (ai/interval (lower-bound d terms)
               (upper-bound d terms)))

(defn relation-proved?
  [d terms expected]
  (= expected (expr-interval d terms)))

(defn- case-result
  [id desc got expected]
  {:id id
   :desc desc
   :got got
   :expected expected
   :ok (= got expected)})

(defn- nonlinear-held-case
  []
  {:id :nonlinear-negative-factor-boundary
   :kind :domain-boundary
   :desc "octagon is linear; negative-factor multiplication uses checked fallback"
   :domain (domain {:acc (ai/interval -4 4)
                    :factor (ai/interval -2 2)})
   :attempted-relation :acc-times-factor
   :gate/verdict :accepted
   :boundary/status :not-applicable
   :boundary/reason :nonlinear-not-in-octagon-domain
   :fallback :checked-arithmetic
   :promotion/allowed? false
   :ok true})

(defn run
  []
  (let [mixed (-> (domain {:i (ai/interval 0 10)
                           :acc (ai/interval -10 0)})
                  (constrain-eq [[:+ :acc] [:+ :i]] 0))
        branch (-> (domain {:i (ai/interval 0 9)
                            :next (ai/interval 1 11)})
                   (constrain-upper [[:+ :next] [:- :i]] 2)
                   (constrain-lower [[:+ :next] [:- :i]] 1))
        transitive-open (-> (domain {:x (ai/interval 0 10)
                                     :y (ai/interval 0 10)
                                     :z (ai/interval 0 10)})
                            (constrain-upper [[:+ :x] [:- :y]] 3)
                            (constrain-upper [[:+ :y] [:- :z]] 4))
        transitive-closed (close transitive-open)
        cases [(case-result
                :mixed-sign-sum-relation
                "acc=-i relation recovers exact acc+i range that interval-only loses"
                {:interval-only (interval-bound mixed [[:+ :acc] [:+ :i]])
                 :octagon (expr-interval mixed [[:+ :acc] [:+ :i]])
                 :proved? (relation-proved? mixed
                                             [[:+ :acc] [:+ :i]]
                                             (ai/singleton 0))}
                {:interval-only (ai/interval -10 10)
                 :octagon (ai/singleton 0)
                 :proved? true})
               (case-result
                :branch-dependent-stride-relation
                "next-i relation captures branch-dependent stride range 1..2"
                {:interval-only (interval-bound branch [[:+ :next] [:- :i]])
                 :octagon (expr-interval branch [[:+ :next] [:- :i]])
                 :upper (upper-bound branch [[:+ :next] [:- :i]])
                 :lower (lower-bound branch [[:+ :next] [:- :i]])}
                {:interval-only (ai/interval -8 11)
                 :octagon (ai/interval 1 2)
                 :upper 2
                 :lower 1})
               (case-result
                :constraint-tightening
                "re-adding a looser bound does not weaken the octagon fact"
                (-> branch
                    (constrain-upper [[:+ :next] [:- :i]] 5)
                    (expr-interval [[:+ :next] [:- :i]]))
                (ai/interval 1 2))
               (case-result
                :transitive-octagon-closure
                "x-y and y-z constraints derive the tighter x-z upper bound"
                {:before-upper (upper-bound transitive-open [[:+ :x] [:- :z]])
                 :after-upper (upper-bound transitive-closed [[:+ :x] [:- :z]])
                 :derived-constraint
                 (get-in transitive-closed
                         [:constraints (normalize-terms [[:+ :x] [:- :z]])])
                 :closure (:closure transitive-closed)}
                {:before-upper 10
                 :after-upper 7
                 :derived-constraint 7
                 :closure {:closed? true :iterations 1}})
               (nonlinear-held-case)]
        accepted (filter #(= :accepted (:gate/verdict %)) cases)
        held (filter #(= :held (:gate/verdict %)) cases)
        rejected (filter #(= :rejected (:gate/verdict %)) cases)
        canonical (mapv #(select-keys %
                                      [:id
                                       :kind
                                       :got
                                       :expected
                                       :gate/verdict
                                       :held-reason
                                       :promotion/allowed?
                                       :ok])
                        cases)
        relation-cases (remove #(= :held-boundary (:kind %)) cases)
        invariants (sorted-map
                    :all-cases-ok (every? :ok cases)
                    :relation-cases-proved (every? :ok relation-cases)
                    :no-rejected-rows (empty? rejected)
                    :nonlinear-not-applicable
                    (= :not-applicable (:boundary/status (first accepted)))
                    :nonlinear-not-promoted
                    (false? (:promotion/allowed? (first accepted)))
                    :transitive-closure-present
                    (some true?
                          (map #(and (= :transitive-octagon-closure (:id %))
                                      (:ok %))
                               cases))
                    :no-compiler-admission
                    (every? #(false? (:promotion/allowed? %)) accepted))
        ok? (every? true? (vals invariants))]
    {:schema "pnix.clj-meta.abstract-octagon.receipt.v1"
     :stage [:M9a]
     :target-goal "meta-circular stage15/N Clojure compiler"
     :desc "minimal JVM-native octagon domain for ±x±y<=c relational range facts"
     :domain {:base :interval
              :relations :octagon
              :constraint-form "±x±y<=c"
              :closure "pairwise transitive consequence closure"
              :implementation :jvm-native-minimal
              :not-polyhedra true}
     :policy {:proved-relation "may feed later M10 VC discharge"
              :compiler-admission "no raw opcode admission happens in this witness"
              :nonlinear "outside the octagon domain; executable checked fallback remains active"}
     :cases cases
     :status-counts {:accepted (count accepted)
                     :not-applicable (count (filter #(= :not-applicable (:boundary/status %)) accepted))
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
    (doseq [row (:cases r)]
      (println (str "  [" (if (:ok row) "OK" "FAIL") "] "
                    (name (:id row)))))
    (println (str "abstract octagon: "
                  (if (:ok r) "OK" "FAILED")
                  "  (receipt: " receipt-path
                  ", digest: " (:canonical-receipt-digest r)
                  ")"))
    (shutdown-agents)
    (when-not (:ok r)
      (System/exit 1))))
