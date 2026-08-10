(ns pnix.clj-meta.form-proof
  "Per-form compile proof receipt consumed by pnix-clj.

  This keeps clj-meta's compile-proof machinery on the clj-meta side. Callers
  provide the already-compiled primary/repeat results and a value normalizer;
  this namespace owns the determinism row, strict row, bytecode artifact row, and
  verified-compile row shape."
  (:require [clojure.java.io :as io]
            [pnix.clj-meta.compiler :as compiler]
            [pnix.clj-meta.verified-compile :as verified-compile])
  (:import [java.io File]
           [java.nio.file Files]
           [java.nio.file.attribute FileAttribute]
           [java.security MessageDigest]))

(defn- hex-digest
  [digest]
  (apply str (map #(format "%02x" (bit-and % 0xff)) digest)))

(defn- sha256-bytes
  [bytes]
  (hex-digest (.digest (MessageDigest/getInstance "SHA-256")
                       ^bytes bytes)))

(defn- sha256
  [s]
  (sha256-bytes (.getBytes (str s) "UTF-8")))

(defn- data-hash
  [x]
  (sha256 (pr-str x)))

(defn- throwable-data
  [^Throwable t]
  {:class (.getName (class t))
   :message (.getMessage t)
   :data (ex-data t)})

(defn- temp-dir
  []
  (.toFile (Files/createTempDirectory "pnix-clj-bytecode-"
                                      (make-array FileAttribute 0))))

(defn- delete-file-tree!
  [^File file]
  (when (and file (.exists file))
    (when (.isDirectory file)
      (doseq [child (.listFiles file)]
        (delete-file-tree! child)))
    (.delete file))
  nil)

(defn- compile-run-row
  [label compiled value]
  (let [diagnostics (vec (:diagnostics compiled))
        fn-class (.getName (class (:fn compiled)))]
    {:label label
     :mode (:mode compiled)
     :diagnostics diagnostics
     :diagnostics-count (count diagnostics)
     :diagnostics-hash (data-hash diagnostics)
     :fn-class fn-class
     :fn-class-hash (sha256 fn-class)
     :value-hash (data-hash value)}))

(declare deterministic-compile-key)

(defn determinism-receipt
  "Build the bounded primary/repeat receipt used by finite projection batches.
  Strict, persisted-bytecode, and verified-compile evidence remains owned by
  compile-receipt and the aggregate compiler gates."
  [form primary primary-value repeat repeat-value]
  (let [primary-row (compile-run-row :primary primary primary-value)
        repeat-row (compile-run-row :repeat repeat repeat-value)
        same-mode? (= (:mode primary-row) (:mode repeat-row))
        same-diagnostics? (= (:diagnostics-hash primary-row)
                             (:diagnostics-hash repeat-row))
        same-class? (= (:fn-class primary-row) (:fn-class repeat-row))
        same-value? (= (:value-hash primary-row) (:value-hash repeat-row))
        deterministic? (and same-mode? same-diagnostics? same-class?
                            same-value?)]
    {:schema :pnix-clj.clj-meta.determinism-receipt.v0
     :proof-owner {:namespace 'pnix.clj-meta.form-proof
                   :scope :primary-repeat-determinism
                   :full-compile-receipt
                   'pnix.clj-meta.form-proof/compile-receipt}
     :deterministic-cache-key (deterministic-compile-key form)
     :form-hash (data-hash form)
     :wrapper-form-hash (data-hash (list 'fn [] form))
     :primary primary-row
     :repeat repeat-row
     :determinism {:status (if deterministic? :ok :failed)
                   :same-mode? same-mode?
                   :same-diagnostics? same-diagnostics?
                   :same-class-name? same-class?
                   :same-value-hash? same-value?}
     :strict {:status :not-requested
              :reason :bounded-determinism-scope}
     :bytecode-artifact {:status :not-requested
                         :reason :bounded-determinism-scope}
     :verified-compile {:status :not-requested
                        :reason :bounded-determinism-scope}}))

(defn deterministic-compile-key
  [form]
  {:schema :pnix-clj.clj-meta.compile-key.v0
   :compiler-symbols ['pnix.clj-meta.compiler/compile-form*
                      'pnix.clj-meta.compiler/compile-form-strict]
   :form-hash (data-hash form)
   :wrapper-form-hash (data-hash (list 'fn [] form))
   :classloader-policy :clj-meta-owned
   :artifact-cache-policy :deterministic-key-required})

(defn- strict-run-row
  [wrapper-form primary-row normalize]
  (try
    (let [strict-fn (compiler/compile-form-strict wrapper-form)
          strict-value (normalize (strict-fn))
          fn-class (.getName (class strict-fn))
          value-hash (data-hash strict-value)]
      {:status :ok
       :mode :strict-direct
       :fn-class fn-class
       :fn-class-hash (sha256 fn-class)
       :value-hash value-hash
       :same-value-as-primary? (= value-hash (:value-hash primary-row))})
    (catch Throwable t
      {:status :failed
       :reason :clj-meta-strict-compile-failed
       :error (throwable-data t)})))

(defn- bytecode-artifact-row
  [wrapper-form]
  (let [dir (temp-dir)]
    (try
      (let [artifact (compiler/compile-to-dir wrapper-form (.getPath dir))
            class-hashes (into (sorted-map)
                               (map (fn [[class-name path]]
                                      [(str class-name)
                                       (sha256-bytes
                                        (Files/readAllBytes
                                         (.toPath (io/file path))))]))
                               (:files artifact))
            artifact-hash (data-hash {:main-class (:main-class artifact)
                                      :class-hashes class-hashes})]
        {:status :ok
         :schema :pnix-clj.clj-meta.bytecode-artifact.v0
         :main-class (:main-class artifact)
         :class-count (count class-hashes)
         :class-hashes class-hashes
         :artifact-hash artifact-hash})
      (catch Throwable t
        {:status :failed
         :reason :clj-meta-bytecode-artifact-unavailable
         :error (throwable-data t)})
      (finally
        (delete-file-tree! dir)))))

(defn- verified-compile-row
  [wrapper-form]
  (try
    (let [result (verified-compile/compile-classes-verified wrapper-form)
          classes (:classes result)
          verification (:verification result)
          class-hashes (into (sorted-map)
                             (map (fn [[class-name bytes]]
                                    [(str class-name)
                                     (sha256-bytes bytes)]))
                             classes)
          artifact-hash (data-hash {:class-hashes class-hashes
                                    :verification-ok? (:ok verification)})]
      {:status (if (:ok verification) :ok :failed)
       :schema :pnix-clj.clj-meta.verified-compile.v0
       :reason (if (:ok verification)
                 :verified-compile-artifact-ok
                 :verified-compile-artifact-failed)
       :class-count (count class-hashes)
       :class-hashes class-hashes
       :artifact-hash artifact-hash
       :verification (select-keys verification
                                  [:ok :class-count :classreader :define
                                   :check-class-adapter])})
    (catch Throwable t
      {:status :failed
       :reason :clj-meta-verified-compile-unavailable
       :error (throwable-data t)})))

(defn compile-receipt
  "Build the per-form compile proof receipt for a caller-owned form evaluation."
  ([form primary primary-value repeat repeat-value]
   (compile-receipt form primary primary-value repeat repeat-value identity))
  ([form primary primary-value repeat repeat-value normalize]
   (let [primary-row (compile-run-row :primary primary primary-value)
         repeat-row (compile-run-row :repeat repeat repeat-value)
         compile-key (deterministic-compile-key form)
         wrapper-form (list 'fn [] form)
         strict-row (strict-run-row wrapper-form primary-row normalize)
         bytecode-row (bytecode-artifact-row wrapper-form)
         verified-row (verified-compile-row wrapper-form)
         same-mode? (= (:mode primary-row) (:mode repeat-row))
         same-diagnostics? (= (:diagnostics-hash primary-row)
                              (:diagnostics-hash repeat-row))
         same-class? (= (:fn-class primary-row) (:fn-class repeat-row))
         same-value? (= (:value-hash primary-row) (:value-hash repeat-row))
         deterministic? (and same-mode? same-diagnostics? same-class?
                             same-value?)
         fallback? (not= :direct (:mode primary-row))]
     {:schema :pnix-clj.clj-meta.compile-receipt.v0
      :proof-owner {:namespace 'pnix.clj-meta.form-proof
                    :determinism :per-form-repeat-compile
                    :bytecode-artifact
                    'pnix.clj-meta.compiler/compile-to-dir
                    :verified-compile
                    'pnix.clj-meta.verified-compile/compile-classes-verified
                    :related-global-proof-apis
                    ['pnix.clj-meta.determinism-policy/run
                     'pnix.clj-meta.bytecode-witness/run
                     'pnix.clj-meta.verified-compile/run]}
      :deterministic-cache-key compile-key
      :compiled-artifact-cache {:status :disabled
                                :reason :preserve-clj-meta-classloader-policy
                                :key compile-key}
      :form-hash (data-hash form)
      :wrapper-form-hash (data-hash (list 'fn [] form))
      :primary primary-row
      :repeat repeat-row
      :strict strict-row
      :bytecode-artifact bytecode-row
      :verified-compile verified-row
      :verified-policy {:acceptance-required? false
                        :reason :verified-compile-attached-as-evidence}
      :strict-policy {:acceptance-required? false
                      :reason :strict-direct-attached-as-evidence}
      :determinism {:status (if deterministic? :ok :failed)
                    :same-mode? same-mode?
                    :same-diagnostics? same-diagnostics?
                    :same-class-name? same-class?
                    :strict-same-value-as-primary?
                    (when (= :ok (:status strict-row))
                      (:same-value-as-primary? strict-row))
                    :same-value-hash? same-value?}
      :fallback {:mode (:mode primary-row)
                 :fallback? fallback?
                 :diagnostics-count (:diagnostics-count primary-row)
                 :diagnostics-hash (:diagnostics-hash primary-row)}})))
