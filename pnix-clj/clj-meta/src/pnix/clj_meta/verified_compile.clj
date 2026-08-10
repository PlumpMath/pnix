(ns pnix.clj-meta.verified-compile
  "Compiler artifact API with verifier hard-fail admission.

  The plain compiler emits class bytes. This boundary wraps that emission with
  M13 verification: ClassReader parse, fresh loader define, and
  CheckClassAdapter. If verifier status is not OK, callers must refuse the
  artifact instead of publishing it as a compiler product."
  (:require [clojure.java.io :as io]
            [clojure.pprint :as pp]
            [pnix.clj-meta.bytecode-verifier :as verifier]
            [pnix.clj-meta.compiler :as comp])
  (:import [java.security MessageDigest]))

(def receipt-path "clj-meta/proof/verified-compile.receipt.edn")

(defn- sha256-bytes
  [^bytes bs]
  (let [md (MessageDigest/getInstance "SHA-256")]
    (apply str
           (map #(format "%02x" (bit-and 0xff %))
                (.digest md bs)))))

(defn- sha256-string
  [s]
  (sha256-bytes (.getBytes ^String (str s) "UTF-8")))

(defn- canonical-value
  [x]
  (cond
    (map? x)
    (into (sorted-map-by (fn [a b] (compare (pr-str a) (pr-str b))))
          (map (fn [[k v]] [k (canonical-value v)]) x))

    (vector? x) (mapv canonical-value x)
    (seq? x)    (mapv canonical-value x)
    (set? x)    (mapv canonical-value (sort-by pr-str x))
    :else       x))

(defn- class-digests
  [classes]
  (into (sorted-map)
        (map (fn [[class-name bytes]]
               [class-name (sha256-bytes bytes)])
             classes)))

(defn compile-classes-verified
  [form]
  (let [classes (comp/compile-classes form)
        verification (verifier/verify-classes classes)]
    (if (:ok verification)
      {:ok true
       :classes classes
       :verification verification}
      (throw (ex-info "compiler artifact verifier rejected class bundle"
                      {:type :verifier-rejected
                       :verification verification})))))

(defn- try-result
  [f]
  (try
    {:ok true
     :value (f)}
    (catch Throwable t
      {:ok false
       :throwable (.getName (class t))
       :message (.getMessage t)
       :data (ex-data t)})))

(defn- accepted-row
  [{:keys [id form]}]
  (let [result (try-result #(compile-classes-verified form))
        classes (get-in result [:value :classes])
        verification (get-in result [:value :verification])
        ok? (boolean
             (and (:ok result)
                  (:ok verification)
                  (seq classes)))]
    {:id id
     :kind :verified-compiler-artifact
     :form (pr-str form)
     :artifact (when classes
                 {:class-count (count classes)
                  :class-digests (class-digests classes)})
     :verification (when verification
                     (select-keys verification
                                  [:ok :class-count :classreader :define
                                   :check-class-adapter]))
     :gate/verdict (if ok? :accepted :rejected)
     :ok ok?}))

(defn- invalid-row
  []
  (let [invalid {"pnix.clj_meta.gen.Invalid" (.getBytes "not-a-class" "UTF-8")}
        verification (verifier/verify-classes invalid)
        refused (try-result
                 #(if (:ok verification)
                    :unexpected-accepted
                    (throw (ex-info "compiler artifact verifier rejected class bundle"
                                    {:type :verifier-rejected
                                     :verification verification}))))]
    {:id :invalid-class-bundle-refused
     :kind :verifier-reject-hard-fail-sentinel
     :artifact {:class-count 1
                :class-digests (class-digests invalid)}
     :verification (select-keys verification
                                [:ok :class-count :classreader :define
                                 :check-class-adapter])
     :refusal (dissoc refused :value)
     :gate/verdict (if (and (false? (:ok verification))
                            (false? (:ok refused))
                            (= :verifier-rejected (get-in refused [:data :type])))
                     :held
                     :rejected)
     :held-reason :verifier-rejected-artifact-refused
     :ok (and (false? (:ok verification))
              (false? (:ok refused))
              (= :verifier-rejected (get-in refused [:data :type])))}))

(defn- fixtures
  []
  [{:id :literal-verified
    :form '(fn [] 42)}
   {:id :closure-verified
    :form '(fn [n]
             (let [f (fn [x] (+ x n))]
               (f 5)))}
   {:id :case-switch-verified
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
               0))}
   {:id :letfn-verified
    :form '(fn [n]
             (letfn [(ev? [x] (if (zero? x) true (od? (dec x))))
                     (od? [x] (if (zero? x) false (ev? (dec x))))]
               (ev? n)))}
   {:id :reify-object-verified
    :form '(fn []
             (str (reify Object
                    (toString [_] "rx"))))}
   {:id :reify-interface-verified
    :form '(fn []
             (.call (reify java.util.concurrent.Callable
                      (call [_] "ok"))))}
   {:id :reify-capture-verified
    :form '(fn [x]
             (.call (reify java.util.concurrent.Callable
                      (call [_] x))))}
   {:id :try-finally-verified
    :form '(fn [n]
             (try
               (/ 10 n)
               (catch ArithmeticException _
                 :divzero)
               (finally
                 (+ 1 2))))}])

(defn run
  []
  (let [rows (conj (mapv accepted-row (fixtures))
                   (invalid-row))
        accepted (filter #(= :accepted (:gate/verdict %)) rows)
        held (filter #(= :held (:gate/verdict %)) rows)
        rejected (filter #(= :rejected (:gate/verdict %)) rows)
        canonical (canonical-value
                   (mapv #(select-keys %
                                         [:id
                                          :kind
                                          :artifact
                                          :verification
                                          :gate/verdict
                                          :held-reason
                                          :ok])
                         rows))
        invariants (sorted-map
                    :all-rows-ok (every? :ok rows)
                    :verified-fixtures-accepted (= 8 (count accepted))
                    :invalid-class-held-refused (= 1 (count held))
                    :no-rejected-rows (empty? rejected)
                    :all-accepted-have-checkclassadapter
                    (every? #(true? (get-in % [:verification
                                               :check-class-adapter
                                               :ok]))
                            accepted))
        ok? (every? true? (vals invariants))]
    {:schema "pnix.clj-meta.verified-compile.receipt.v1"
     :stage [:M13 :compiler-artifact-api]
     :target-goal "meta-circular stage15/N Clojure compiler"
     :desc "compiler class bundle emission wrapped with verifier hard-fail policy"
     :policy {:accepted "compiler artifact must pass ClassReader, fresh define, and CheckClassAdapter"
              :held "verifier reject is refused, never published"
              :not-consumed-by "pnix-clj launcher"}
     :rows rows
     :status-counts {:accepted (count accepted)
                     :held (count held)
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
    (println (str "verified compile: "
                  (if (:ok r) "OK" "FAILED")
                  "  (receipt: " receipt-path
                  ", digest: " (:canonical-receipt-digest r)
                  ")"))
    (shutdown-agents)
    (when-not (:ok r)
      (System/exit 1))))
