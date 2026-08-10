(ns pnix.clj-meta.runtime-selfhost
  "Runtime self-host boundary witness.

  This lane compiles a few small runtime-helper functions with the clj-meta
  backend and records the remaining host runtime boundary honestly.  It does
  not claim an independent clojure.core or clojure.lang replacement."
  (:require [clojure.java.io :as io]
            [clojure.pprint :as pp]
            [pnix.clj-meta.compiler :as comp])
  (:import [clojure.asm ClassReader ClassVisitor MethodVisitor Opcodes]
           [java.security MessageDigest]))

(def receipt-path "clj-meta/proof/runtime-selfhost.receipt.edn")

(defn- sha256-string
  [s]
  (let [md (MessageDigest/getInstance "SHA-256")]
    (apply str
           (map #(format "%02x" (bit-and 0xff %))
                (.digest md (.getBytes ^String (str s) "UTF-8"))))))

(defn- try-val
  [f]
  (try
    {:ok true :value (f)}
    (catch Throwable t
      {:ok false
       :throwable (.getName (class t))
       :message (.getMessage t)
       :data (ex-data t)})))

(defn- generated-fn?
  [x]
  (.startsWith (.getName (class x)) "pnix.clj_meta.gen."))

(defn- sorted-keywords
  [xs]
  (vec (sort-by name xs)))

(defn- bytecode-summary
  [classes]
  (let [core-var-refs (atom [])
        numbers-calls (atom #{})
        rt-calls (atom #{})
        raw-long-opcodes (atom #{})]
    (doseq [[class-name bytes] classes]
      (.accept (ClassReader. ^bytes bytes)
               (proxy [ClassVisitor] [Opcodes/ASM5]
                 (visitMethod [_access method-name method-desc _signature _exceptions]
                   (proxy [MethodVisitor] [Opcodes/ASM5]
                     (visitInsn [op]
                       (condp = op
                         Opcodes/LADD (swap! raw-long-opcodes conj :ladd)
                         Opcodes/LSUB (swap! raw-long-opcodes conj :lsub)
                         Opcodes/LMUL (swap! raw-long-opcodes conj :lmul)
                         nil))
                     (visitFieldInsn [op owner field-name field-desc]
                       (when (or (= owner "clojure/lang/Var")
                                 (.startsWith ^String owner "clojure/core"))
                         (swap! core-var-refs conj
                                {:class class-name
                                 :method method-name
                                 :op op
                                 :owner owner
                                 :name field-name
                                 :desc field-desc})))
                     (visitMethodInsn
                       ([op owner method-name desc]
                        (cond
                          (= owner "clojure/lang/Numbers")
                          (swap! numbers-calls conj (keyword method-name))

                          (= owner "clojure/lang/RT")
                          (swap! rt-calls conj (keyword method-name))

                          (or (= owner "clojure/lang/Var")
                              (.startsWith ^String owner "clojure/core"))
                          (swap! core-var-refs conj
                                 {:class class-name
                                  :method method-name
                                  :op op
                                  :owner owner
                                  :name method-name
                                  :desc desc})))
                       ([op owner method-name desc _itf]
                        (cond
                          (= owner "clojure/lang/Numbers")
                          (swap! numbers-calls conj (keyword method-name))

                          (= owner "clojure/lang/RT")
                          (swap! rt-calls conj (keyword method-name))

                          (or (= owner "clojure/lang/Var")
                              (.startsWith ^String owner "clojure/core"))
                          (swap! core-var-refs conj
                                 {:class class-name
                                  :method method-name
                                  :op op
                                  :owner owner
                                  :name method-name
                                  :desc desc})))))))
               0))
    {:class-count (count classes)
     :clojure-core-var-deref-count (count @core-var-refs)
     :clojure-core-var-refs (vec @core-var-refs)
     :numbers-calls (sorted-keywords @numbers-calls)
     :rt-calls (sorted-keywords @rt-calls)
     :raw-long-opcodes (sorted-keywords @raw-long-opcodes)}))

(defn- specs
  []
  [{:id :rt-unchecked-inc-long-leaf
    :desc "leaf arithmetic runtime fragment emitted as raw bytecode with clojure.core Var deref 0"
    :form '(fn [^long x] (unchecked-add x 1))
    :args [41]
    :expected 42
    :runtime-boundary [:jvm :clojure-lang-afunction]
    :requires-host-core-free-fragment true
    :required-raw-opcodes #{:ladd}}
   {:id :rt-unchecked-mul-long-leaf
    :desc "leaf multiply runtime fragment emitted as raw LMUL with clojure.core Var deref 0"
    :form '(fn [^long a ^long b] (unchecked-multiply a b))
    :args [6 7]
    :expected 42
    :runtime-boundary [:jvm :clojure-lang-afunction]
    :requires-host-core-free-fragment true
    :required-raw-opcodes #{:lmul}}
   {:id :rt-unchecked-sub-long-leaf
    :desc "leaf subtract runtime fragment emitted as raw LSUB with clojure.core Var deref 0"
    :form '(fn [^long a ^long b] (unchecked-subtract a b))
    :args [50 8]
    :expected 42
    :runtime-boundary [:jvm :clojure-lang-afunction]
    :requires-host-core-free-fragment true
    :required-raw-opcodes #{:lsub}}
   {:id :rt-inc-long
    :desc "small arithmetic helper emitted as a generated fn"
    :form '(fn [^long x] (+ x 1))
    :args [41]
    :expected 42}
   {:id :rt-second-seq
    :desc "sequence helper body is our bytecode; first/next remain host runtime vars"
    :form '(fn [xs] (first (next xs)))
    :args [[10 42 99]]
    :expected 42}
   {:id :rt-assoc-keyword-lookup
    :desc "map helper body is our bytecode; persistent map implementation remains host runtime"
    :form '(fn [m] (:answer (assoc m :answer 42)))
    :args [{}]
    :expected 42}
   {:id :rt-get-after-assoc
    :desc "get/assoc helper body is our bytecode; map lookup implementation remains host runtime"
    :form '(fn [m] (get (assoc m :answer 42) :answer))
    :args [{}]
    :expected 42}
   {:id :rt-reduce-sum
    :desc "reduce helper body is our bytecode; reduce and sequence traversal remain host runtime vars"
    :form '(fn [xs] (reduce + 0 xs))
    :args [[10 20 12]]
    :expected 42}
   {:id :rt-conj-vector
    :desc "vector materialization/conj helper body is our bytecode; persistent vector runtime remains host"
    :form '(fn [xs] (conj (vec xs) 42))
    :args [[1 2]]
    :expected [1 2 42]}
   {:id :rt-map-inc-materialize
    :desc "higher-order map helper body and nested fn are emitted; lazy seq/vector runtime remains host"
    :form '(fn [xs] (vec (map (fn [x] (+ x 1)) xs)))
    :args [[40 41]]
    :expected [41 42]}
   {:id :rt-count-loop
    :desc "loop/recur helper emitted by clj-meta while seq/next are host runtime vars"
    :form '(fn [xs]
             (loop [s xs n 0]
               (if (seq s)
                 (recur (next s) (+ n 1))
                 n)))
    :args [[1 2 3 4 5]]
    :expected 5}])

(defn- row
  [{:keys [id desc form args expected runtime-boundary
           requires-host-core-free-fragment required-raw-opcodes]}]
  (comp/clear-compile-form-fallback-diagnostics!)
  (let [compiled (try-val #(comp/compile-form form))
        got (when (:ok compiled)
              (try-val #(apply (:value compiled) args)))
        classes (try-val #(comp/compile-classes form))
        bytecode (when (:ok classes)
                   (bytecode-summary (:value classes)))
        diagnostics @comp/compile-form-fallback-diagnostics
        direct? (and (:ok compiled)
                     (generated-fn? (:value compiled))
                     (empty? diagnostics))
        host-core-free? (and bytecode
                             (zero? (:clojure-core-var-deref-count bytecode)))
        required-opcodes-present?
        (or (empty? required-raw-opcodes)
            (every? (set (:raw-long-opcodes bytecode))
                    required-raw-opcodes))
        ok? (and direct?
                 (:ok got)
                 (:ok classes)
                 (= expected (:value got))
                 (or (not requires-host-core-free-fragment)
                     host-core-free?)
                 required-opcodes-present?)]
    (comp/clear-compile-form-fallback-diagnostics!)
    {:id id
     :kind (if requires-host-core-free-fragment
             :host-core-free-runtime-fragment
             :direct-emitted-runtime-helper)
     :desc desc
     :form (pr-str form)
     :args args
     :expected expected
     :compiled (if (:ok compiled)
                 {:ok true
                  :generated-class (.getName (class (:value compiled)))
                  :direct-emit-no-fallback direct?}
                 compiled)
     :bytecode (if (:ok classes)
                 bytecode
                 classes)
     :got got
     :fallback-diagnostics diagnostics
     :runtime-boundary (or runtime-boundary
                           [:clojure-core-vars
                            :clojure-lang-data-structures
                            :jvm])
     :gate/verdict (if ok? :accepted :rejected)
     :ok ok?}))

(defn- boundary-rows
  []
  [{:id :full-clojure-core-runtime-boundary
    :kind :runtime-frontier-boundary
    :gate/verdict :accepted
    :boundary/status :trusted-dependency
    :boundary/reason :clojure-core-and-clojure-lang-not-reimplemented
    :not-claimed [:independent-clojure-core :independent-persistent-collections]
    :ok true}
   {:id :manual-jvm-runtime-boundary
    :kind :runtime-frontier-boundary
    :gate/verdict :accepted
    :boundary/status :trusted-dependency
    :boundary/reason :jvm-and-standard-library-remain-trusted-runtime
    :not-claimed [:independent-jvm :independent-java-standard-library]
    :ok true}])

(defn run
  []
  (let [accepted-rows (mapv row (specs))
        boundaries (boundary-rows)
        rows (into accepted-rows boundaries)
        accepted (filter #(= :accepted (:gate/verdict %)) rows)
        rejected (filter #(= :rejected (:gate/verdict %)) rows)
        canonical (mapv #(select-keys %
                                      [:id :kind :form :args :expected :compiled
                                       :bytecode :got :runtime-boundary :gate/verdict
                                       :boundary/status :boundary/reason
                                       :not-claimed :ok])
                        rows)
        invariants (sorted-map
                    :helpers-direct-emitted
                    (every? #(true? (get-in % [:compiled :direct-emit-no-fallback]))
                            accepted-rows)
                    :helper-results-match
                    (every? #(= (:expected %) (get-in % [:got :value]))
                            accepted-rows)
                    :runtime-frontier-declared
                    (every? #(and (= :accepted (:gate/verdict %))
                                  (= :trusted-dependency (:boundary/status %)))
                            boundaries)
                    :host-core-free-leaf-fragment
                    (let [row (first (filter #(= :rt-unchecked-inc-long-leaf
                                                 (:id %))
                                             accepted-rows))]
                      (and (= :accepted (:gate/verdict row))
                           (zero? (get-in row [:bytecode
                                               :clojure-core-var-deref-count]))
                           (contains? (set (get-in row [:bytecode
                                                        :raw-long-opcodes]))
                                      :ladd)))
                    :host-core-free-fragments-all-clean
                    (let [frag-rows (filter #(#{:rt-unchecked-inc-long-leaf
                                                :rt-unchecked-mul-long-leaf
                                                :rt-unchecked-sub-long-leaf}
                                              (:id %))
                                            accepted-rows)]
                      (and (= 3 (count frag-rows))
                           (every? #(and (= :accepted (:gate/verdict %))
                                         (zero? (get-in % [:bytecode
                                                           :clojure-core-var-deref-count])))
                                   frag-rows)))
                    :no-rejected-rows (empty? rejected)
                    :all-rows-ok (every? :ok rows))
        ok? (every? true? (vals invariants))]
    {:schema "pnix.clj-meta.runtime-selfhost.receipt.v1"
     :stage [:U7 :R6 :runtime-selfhost]
     :target-goal "meta-circular stage15/N Clojure compiler"
     :desc "small runtime helpers are direct-emitted; clojure.core, clojure.lang, the JVM, and the Java standard library remain declared trusted dependencies"
     :status-counts {:accepted (count accepted)
                     :trusted-boundary (count boundaries)
                     :rejected (count rejected)}
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
    (println (str "runtime selfhost: "
                  (if (:ok r) "OK" "FAILED")
                  "  (receipt: " receipt-path
                  ", digest: " (:canonical-receipt-digest r)
                  ")"))
    (shutdown-agents)
    (when-not (:ok r)
      (System/exit 1))))
