(ns pnix.clj-meta.bytecode-verifier
  "M13 bytecode verifier/loadability witness.

  Generated class bytes must parse with ClassReader, define in a fresh
  DynamicClassLoader, pass ASM CheckClassAdapter.verify with that fresh
  loader, instantiate when applicable, and execute fixture calls without
  VerifyError/ClassFormatError/LinkageError."
  (:require [clojure.java.io :as io]
            [clojure.pprint :as pp]
            [clojure.string :as str]
            [pnix.clj-meta.compiler :as comp])
  (:import [clojure.asm ClassReader ClassVisitor ClassWriter Label MethodVisitor Opcodes]
           [clojure.lang DynamicClassLoader IFn]
           [java.io PrintWriter StringWriter]
           [java.security MessageDigest]))

(def receipt-path "clj-meta/proof/bytecode-verifier.receipt.edn")

(def ^:private asm-api Opcodes/ASM5)

(defn- sha256-string
  [s]
  (let [md (MessageDigest/getInstance "SHA-256")]
    (apply str
           (map #(format "%02x" (bit-and 0xff %))
                (.digest md (.getBytes ^String (str s) "UTF-8"))))))

(defn- sha256-bytes
  [^bytes bs]
  (let [md (MessageDigest/getInstance "SHA-256")]
    (apply str
           (map #(format "%02x" (bit-and 0xff %))
                (.digest md bs)))))

(defn- try-result
  [f]
  (try
    {:ok true
     :value (f)}
    (catch Throwable t
      {:ok false
       :throwable (.getName (class t))
       :message (.getMessage t)})))

(defn- class-summary
  [declared-class-name bytes]
  (let [methods (atom [])
        summary (atom {:declared-class declared-class-name})]
    (.accept (ClassReader. ^bytes bytes)
             (proxy [ClassVisitor] [asm-api]
               (visit [version access name signature super-name interfaces]
                 (swap! summary assoc
                        :version version
                        :class-name (.replace ^String name \/ \.)
                        :super-name (some-> ^String super-name
                                            (.replace \/ \.))
                        :interfaces (mapv #(.replace ^String % \/ \.)
                                          interfaces)))
               (visitMethod [access name desc signature exceptions]
                 (swap! methods conj {:name name
                                      :desc desc
                                      :access access})
                 (proxy [MethodVisitor] [asm-api])))
             ClassReader/SKIP_DEBUG)
    (assoc @summary
           :methods @methods
           :method-count (count @methods)
           :sha256 (sha256-bytes bytes))))

(defn- define-classes
  [classes]
  (let [dcl (DynamicClassLoader.
             (.getContextClassLoader (Thread/currentThread)))]
    {:loader dcl
     :classes (into (sorted-map)
                    (map (fn [[class-name bytes]]
                           [class-name (.defineClass dcl class-name bytes nil)])
                         (sort-by key classes)))}))

(defn- check-class-adapter-available?
  []
  (:ok (try-result
        #(Class/forName "org.objectweb.asm.util.CheckClassAdapter"
                        false
                        (.getContextClassLoader (Thread/currentThread))))))

(defn- check-class-adapter-verify
  [^ClassLoader loader [class-name bytes]]
  (let [sw     (StringWriter.)
        pw     (PrintWriter. sw)
        result (try-result
                #(org.objectweb.asm.util.CheckClassAdapter/verify
                  (org.objectweb.asm.ClassReader. ^bytes bytes)
                  loader
                  false
                  pw))
        _      (.flush pw)
        output (str/trim (str sw))]
    {:class-name class-name
     :ok (and (:ok result) (str/blank? output))
     :verify (dissoc result :value)
     :output output}))

(defn- root-class-name
  [classes]
  (or (first (filter #(not (.contains ^String % "$"))
                     (sort (keys classes))))
      (first (sort (keys classes)))))

(defn- verifier-case
  [{:keys [id desc form args expected]}]
  (let [compiled     (try-result #(comp/compile-classes form))
        classes      (:value compiled)
        summaries    (when (:ok compiled)
                       (try-result
                        #(mapv (fn [[class-name bytes]]
                                  (class-summary class-name bytes))
                                (sort-by key classes))))
        defined      (when (:ok compiled)
                       (try-result #(define-classes classes)))
        root-name    (when (:ok defined)
                       (root-class-name classes))
        full-verify  (when (:ok defined)
                       (try-result
                        #(mapv (partial check-class-adapter-verify
                                        (get-in defined [:value :loader]))
                                (sort-by key classes))))
        invocation   (when (and (:ok defined) root-name)
                       (try-result
                        #(let [^Class root-class (get-in defined
                                                         [:value :classes root-name])
                               ^IFn f (.newInstance root-class)]
                           (apply f args))))
        ok?          (and (:ok compiled)
                          (:ok summaries)
                          (:ok defined)
                          (:ok full-verify)
                          (every? :ok (:value full-verify))
                          (:ok invocation)
                          (= expected (:value invocation)))]
    {:id id
     :desc desc
     :form form
     :args args
     :expected expected
     :class-count (if (:ok compiled) (count classes) 0)
     :root-class root-name
     :class-summaries (if (:ok summaries) (:value summaries) [])
     :compile (dissoc compiled :value)
     :classreader (dissoc (or summaries {:ok false}) :value)
     :define (dissoc (or defined {:ok false}) :value)
     :check-class-adapter (cond-> (dissoc (or full-verify {:ok false}) :value)
                            (:ok full-verify)
                            (assoc :classes (:value full-verify)))
     :invoke (cond-> (dissoc (or invocation {:ok false}) :value)
               (:ok invocation) (assoc :value (:value invocation)))
     :ok ok?}))

(defn- asm-backward-jump-widening-bytes
  []
  (let [owner "pnix/clj_meta/gen/AsmBackwardJumpWidening"
        cw    (ClassWriter. (bit-or ClassWriter/COMPUTE_FRAMES
                                    ClassWriter/COMPUTE_MAXS))]
    (.visit cw Opcodes/V1_8 Opcodes/ACC_PUBLIC owner nil "java/lang/Object" nil)
    (let [mv (.visitMethod cw Opcodes/ACC_PUBLIC "<init>" "()V" nil nil)]
      (.visitCode mv)
      (.visitVarInsn mv Opcodes/ALOAD 0)
      (.visitMethodInsn mv Opcodes/INVOKESPECIAL "java/lang/Object" "<init>" "()V" false)
      (.visitInsn mv Opcodes/RETURN)
      (.visitMaxs mv 0 0)
      (.visitEnd mv))
    (let [mv   (.visitMethod cw (bit-or Opcodes/ACC_PUBLIC Opcodes/ACC_STATIC)
                             "run" "(I)I" nil nil)
          loop (Label.)
          exit (Label.)]
      (.visitCode mv)
      (.visitLabel mv loop)
      (.visitVarInsn mv Opcodes/ILOAD 0)
      (.visitJumpInsn mv Opcodes/IFLE exit)
      (dotimes [_ 40000]
        (.visitInsn mv Opcodes/NOP))
      (.visitIincInsn mv 0 -1)
      (.visitVarInsn mv Opcodes/ILOAD 0)
      (.visitJumpInsn mv Opcodes/IFGT loop)
      (.visitLabel mv exit)
      (.visitVarInsn mv Opcodes/ILOAD 0)
      (.visitInsn mv Opcodes/IRETURN)
      (.visitMaxs mv 0 0)
      (.visitEnd mv))
    (.visitEnd cw)
    (.toByteArray cw)))

(defn- asm-backward-jump-widening-case
  []
  (let [class-name "pnix.clj_meta.gen.AsmBackwardJumpWidening"
        bytes      (asm-backward-jump-widening-bytes)
        classes    {class-name bytes}
        summaries  (try-result #(mapv (fn [[name bs]] (class-summary name bs))
                                      (sort-by key classes)))
        defined    (try-result #(define-classes classes))
        full-verify (when (:ok defined)
                      (try-result
                       #(mapv (partial check-class-adapter-verify
                                       (get-in defined [:value :loader]))
                              (sort-by key classes))))
        invocation (when (:ok defined)
                     (try-result
                      #(let [^Class klass (get-in defined [:value :classes class-name])
                             method (.getMethod klass "run" (into-array Class [Integer/TYPE]))]
                         (.invoke method nil (object-array [(int 3)])))))
        ok?       (and (:ok summaries)
                       (:ok defined)
                       (:ok full-verify)
                       (every? :ok (:value full-verify))
                       (:ok invocation)
                       (= 0 (:value invocation)))]
    {:id :asm-backward-jump-widening
     :desc "vendored clojure.asm widens a conditional backward jump beyond short range without verifier failure"
     :form :asm-generated-class
     :args [3]
     :expected 0
     :class-count 1
     :root-class class-name
     :class-summaries (if (:ok summaries) (:value summaries) [])
     :compile {:ok true}
     :classreader (dissoc summaries :value)
     :define (dissoc defined :value)
     :check-class-adapter (cond-> (dissoc (or full-verify {:ok false}) :value)
                            (:ok full-verify)
                            (assoc :classes (:value full-verify)))
     :invoke (cond-> (dissoc (or invocation {:ok false}) :value)
               (:ok invocation) (assoc :value (:value invocation)))
     :ok ok?}))

(defn- check-class-adapter-status
  []
  (let [loader (.getContextClassLoader (Thread/currentThread))
        result (try-result
                #(Class/forName "org.objectweb.asm.util.CheckClassAdapter"
                                false
                                loader))]
    (if (:ok result)
      {:status :available
       :ok true
       :class-name (.getName ^Class (:value result))}
      {:status :unavailable
       :ok true
       :availability-reason :check-class-adapter-not-on-classpath})))

(defn verify-classes
  "Verify a generated class bundle without executing it.

  This is the reusable M13 boundary for compiler artifact APIs: parse all class
  bytes, define them in a fresh DynamicClassLoader, and run ASM
  CheckClassAdapter with that same loader. Any parse/define/verifier failure
  returns :ok false and must be treated as emit-refuse by callers."
  [classes]
  (let [summaries   (try-result
                    #(mapv (fn [[class-name bytes]]
                              (class-summary class-name bytes))
                            (sort-by key classes)))
        defined     (when (:ok summaries)
                      (try-result #(define-classes classes)))
        full-verify (when (:ok defined)
                      (try-result
                       #(mapv (partial check-class-adapter-verify
                                       (get-in defined [:value :loader]))
                               (sort-by key classes))))
        ok?         (and (:ok summaries)
                         (:ok defined)
                         (:ok full-verify)
                         (every? :ok (:value full-verify)))]
    {:ok (boolean ok?)
     :class-count (count classes)
     :class-summaries (if (:ok summaries) (:value summaries) [])
     :classreader (dissoc (or summaries {:ok false}) :value)
     :define (dissoc (or defined {:ok false}) :value)
     :check-class-adapter (cond-> (dissoc (or full-verify {:ok false}) :value)
                            (:ok full-verify)
                            (assoc :classes (:value full-verify)))}))

(defn- fixtures
  []
  [{:id :literal-constant
    :desc "simple generated fn parses, defines, instantiates, and invokes"
    :form '(fn [] 42)
    :args []
    :expected 42}
   {:id :primitive-long-loop
    :desc "typed loop/recur primitive locals remain JVM-loadable"
    :form '(fn [^long n]
             (loop [i n
                    acc 0]
               (if (< i 1)
                 acc
                 (recur (- i 1) (+ acc i)))))
    :args [10]
    :expected 55}
   {:id :try-catch-finally
    :desc "exception table and finally bytecode remain JVM-loadable"
    :form '(fn [n]
             (let [a (atom 0)]
               (try
                 (/ 1 n)
                 (catch ArithmeticException _
                   :divzero)
                 (finally
                   (swap! a inc)))))
    :args [0]
    :expected :divzero}
   {:id :case-table-switch
    :desc "dense numeric case tableswitch bytecode remains verifier-clean"
    :form '(fn [n]
             (case n
               1 :one
               2 :two
               :other))
    :args [2]
    :expected :two}
   {:id :case-lookup-switch
    :desc "sparse keyword case lookupswitch bytecode remains verifier-clean"
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
    :expected 7}
   {:id :letfn-mutual-recursion
    :desc "letfn mutual recursion generated class bundle remains verifier-clean"
   :form '(fn [n]
             (letfn [(ev? [x] (if (zero? x) true (od? (dec x))))
                     (od? [x] (if (zero? x) false (ev? (dec x))))]
               (ev? n)))
    :args [4]
    :expected true}
   {:id :reify-object-method
    :desc "simple reify Object method generated class bundle remains verifier-clean"
   :form '(fn []
             (str (reify Object
                    (toString [_] "rx"))))
    :args []
    :expected "rx"}
   {:id :reify-interface-method
    :desc "simple reify interface method generated class bundle remains verifier-clean"
   :form '(fn []
             (.call (reify java.util.concurrent.Callable
                      (call [_] "ok"))))
    :args []
    :expected "ok"}
   {:id :reify-capture-method
    :desc "reify method capture field generated class bundle remains verifier-clean"
    :form '(fn [x]
             (.call (reify java.util.concurrent.Callable
                      (call [_] x))))
    :args ["cap"]
    :expected "cap"}
   {:id :reify-iobj-general
    :desc "reify auto-implements IObj (compiler-generated __meta field + meta/withMeta); with-meta produces a verifier-clean copy carrying the metadata while preserving user methods"
    :form '(fn []
             (let [r (with-meta (reify Object (toString [_] "rx")) {:x 1})]
               [(meta r) (str r) (instance? clojure.lang.IObj r)]))
    :args []
    :expected [{:x 1} "rx" true]}
   {:id :closure-capture
    :desc "nested generated classes with captured locals load in a fresh loader"
    :form '(fn [n]
             (let [f (fn [x] (+ x n))]
               (f 5)))
    :args [37]
    :expected 42}])

(defn run
  []
  (let [cases       (conj (mapv verifier-case (fixtures))
                          (asm-backward-jump-widening-case))
        adapter     (check-class-adapter-status)
        canonical   {:check-class-adapter
                     (select-keys adapter [:status :class-name])
                     :cases
                     (mapv #(select-keys %
                                         [:id
                                          :class-count
                                          :root-class
                                          :class-summaries
                                          :check-class-adapter
                                          :invoke
                                          :ok])
                           cases)}
        invariants  (sorted-map
                     :all-cases-ok (every? :ok cases)
                     :all-classes-parsed
                     (every? #(true? (get-in % [:classreader :ok])) cases)
                     :all-classes-defined
                     (every? #(true? (get-in % [:define :ok])) cases)
                     :all-classes-checkclassadapter-verified
                     (every? #(and (true? (get-in % [:check-class-adapter :ok]))
                                   (every? :ok (get-in % [:check-class-adapter
                                                          :classes])))
                             cases)
                     :all-fixtures-invoked
                     (every? #(true? (get-in % [:invoke :ok])) cases)
                     :check-class-adapter-available
                     (= :available (:status adapter)))
        ok?         (and (every? :ok cases)
                         (:ok adapter)
                         (check-class-adapter-available?)
                         (every? true? (vals invariants)))]
    {:schema "pnix.clj-meta.bytecode-verifier.receipt.v1"
     :stage [:M13]
     :target-goal "meta-circular stage15/N Clojure compiler"
     :desc "ClassReader parse + fresh DynamicClassLoader define + ASM CheckClassAdapter verify + invoke witness"
     :policy {:verifier-reject "ClassFormatError/VerifyError/LinkageError fails the receipt"
              :check-class-adapter "ASM util must be present and every generated class must verify"
              :compiler-admission "bytecode generation unchanged in this pass"}
     :check-class-adapter adapter
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
    (println (str "bytecode verifier: "
                  (if (:ok r) "OK" "FAILED")
                  "  (receipt: " receipt-path
                  ", digest: " (:canonical-receipt-digest r)
                  ")"))
    (shutdown-agents)
    (when-not (:ok r)
      (System/exit 1))))
