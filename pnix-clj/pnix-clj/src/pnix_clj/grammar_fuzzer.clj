(ns pnix-clj.grammar-fuzzer
  "Grammar fuzzer: generates positive (must-parse/eval) and negative (must-reject) pnix sources to stress the parser + evaluator beyond the fixed corpus."
  (:require [pnix-clj.core :as pnix]
            [pnix-clj.hash :as hash]))

(def lane-classification
  {:lane :proof-only
   :scope :bounded-grammar-fuzz-evidence
   :product-runtime :forbidden
   :autonomous-execution :forbidden
   :mutation :forbidden
   :admission :forbidden
   :default-on-failure :report-only
   :allowed-output :fuzz-evidence-report})

(def default-positive-count
  8)

(def default-error-count
  3)

(defn- n
  [seed i offset]
  (inc (mod (+ seed i offset) 17)))

(def positive-templates
  [(fn [seed i]
     (format "%d + %d" (n seed i 0) (n seed i 3)))
   (fn [seed i]
     (format "if %s then %d else %d"
             (if (even? (+ seed i)) "true" "false")
             (n seed i 1)
             (n seed i 2)))
   (fn [seed i]
     (format "let x = %d; y = %d; in x + y"
             (n seed i 4)
             (n seed i 5)))
   (fn [seed i]
     (format "{ a = %d; b = %d; }.%s"
             (n seed i 6)
             (n seed i 7)
             (if (even? i) "a" "b")))
   (fn [seed i]
     (format "[%d %d %d]" (n seed i 1) (n seed i 2) (n seed i 3)))
   (fn [seed i]
     (format "builtins.length [%d %d]" (n seed i 8) (n seed i 9)))
   (fn [seed i]
     (format "builtins.head [%d %d]" (n seed i 10) (n seed i 11)))
   (fn [seed i]
     (format "builtins.elemAt [%d %d %d] 1"
             (n seed i 12)
             (n seed i 13)
             (n seed i 14)))
   (fn [seed i]
     (format "\"s%d\" + \"t%d\"" (n seed i 15) (n seed i 16)))
   (fn [seed i]
     (format "assert true; %d" (n seed i 0)))])

(def error-templates
  [(fn [seed i]
     (format "%d / 0" (n seed i 0)))
   (fn [_ _]
     "builtins.head []")
   (fn [seed i]
     (format "missing_%d" (n seed i 2)))
   (fn [_ _]
     "assert false; 1")
   (fn [_ _]
     "if true then builtins.head [] else 1")])

(defn- generated-case
  [seed class index template]
  (let [source (template seed index)
        id (keyword "grammar-fuzzer" (format "%s-%03d" (name class) index))]
    {:source-id id
     :source source
     :source-hash (hash/sha256 source)
     :fixture-class class
     ;; Error templates are deterministic semantic failures (div0, empty head,
     ;; unbound, assert false). verify-source reports :failed, not a policy
     ;; :held frontier — match that honest shape.
     :expected-status (case class
                        :positive :accepted
                        :error :failed)
     :generator {:schema :pnix-clj.grammar-fuzzer.v0
                 :seed seed
                 :index index
                 :class class}}))

(defn generated-cases
  ([] (generated-cases {}))
  ([{:keys [seed positive-count error-count]
     :or {seed 0
          positive-count default-positive-count
          error-count default-error-count}}]
   (vec
    (concat
     (map (fn [i]
            (generated-case seed
                            :positive
                            i
                            (nth positive-templates
                                 (mod (+ seed i) (count positive-templates)))))
          (range positive-count))
     (map (fn [i]
            (generated-case seed
                            :error
                            i
                            (nth error-templates
                                 (mod (+ seed i) (count error-templates)))))
          (range error-count))))))

(defn- gate-status
  "Row ok iff the multi-lane receipt status matches the fixture expectation."
  [expected actual]
  (if (= expected actual) :ok :failed))

(defn- fuzzer-row
  [case]
  (let [receipt (pnix/verify-source case)
        actual (:status receipt)
        status (gate-status (:expected-status case) actual)]
    (-> case
        (dissoc :source)
        (assoc :status status
               :actual-status actual
               :actual-reason (:reason receipt)
               :lane-summary (:lane-summary receipt)
               :eval-status (get-in receipt [:eval-result :status])
               :eval-reason (get-in receipt [:eval-result :reason])
               :clj-meta-status (get-in receipt [:clj-meta-result :status])
               :px-runtime-status (get-in receipt [:px-runtime :status])
               :pnix-mirror-status (get-in receipt [:pnix-mirror :status])))))

(defn report
  ([] (report {}))
  ([opts]
   (let [cases (generated-cases opts)
         rows (mapv fuzzer-row cases)
         counts (frequencies (map :status rows))
         actual-counts (frequencies (map :actual-status rows))
         failed (filter #(= :failed (:status %)) rows)]
     {:kind :pnix-grammar-fuzzer-report
      :schema :pnix-clj.grammar-fuzzer-report.v0
      :policy :generated-programs-through-verify-source-differential-gate
      :seed (or (:seed opts) 0)
      :source-count (count rows)
      :positive-count (count (filter #(= :positive (:fixture-class %)) rows))
      :error-count (count (filter #(= :error (:fixture-class %)) rows))
      :ok (get counts :ok 0)
      :failed (get counts :failed 0)
      :actual-status-counts actual-counts
      :first-failed (first failed)
      :rows rows})))

(defn -main
  [& [positive-count error-count seed]]
  (let [{:keys [source-count ok failed first-failed] :as report}
        (report {:positive-count (if positive-count
                                   (parse-long positive-count)
                                   default-positive-count)
                 :error-count (if error-count
                                (parse-long error-count)
                                default-error-count)
                 :seed (if seed (parse-long seed) 0)})]
    (println (format "pnix-clj grammar fuzzer: sources=%d ok=%d failed=%d seed=%d"
                     source-count ok failed (:seed report)))
    (when first-failed
      (println "first failed:" (pr-str first-failed)))
    (shutdown-agents)
    (when (pos? failed)
      (System/exit 1))))
