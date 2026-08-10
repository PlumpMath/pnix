(ns pnix-clj.classfile-receipt
  "JVM class-file artifact receipts -- content hashes of the classes clj-meta emits, so a compiled artifact's identity is verifiable and stable."
  (:require [clojure.edn :as edn]
            [clojure.java.io :as io]
            [pnix-clj.clojure-form :as clojure-form]
            [pnix-clj.core :as pnix]
            [pnix-clj.hash :as hash]))

(def lane-classification
  {:lane :core
   :scope :classfile-artifact-receipt
   :role :hash-and-verify-clj-meta-generated-classfiles
   :product-runtime :allowed
   :semantic-authority :artifact-evidence-only
   :mutation :forbidden
   :admission :verified-compile-required
   :determinism :classfile-hash-required
   :allowed-output :deterministic-classfile-report})

(def generated-class-fixtures
  {:deftype :clojure-form/deftype-object-form
   :defrecord :clojure-form/defrecord-map-form
   :reify :clojure-form/reify-callable-form
   :proxy :clojure-form/proxy-runnable-form})

(defn- repo-root
  []
  (.getCanonicalFile (io/file (System/getProperty "user.dir") "..")))

(defn- read-edn-file
  [path]
  (edn/read-string (slurp path)))

(defn- deps-file
  [relative]
  (io/file (repo-root) relative "deps.edn"))

(defn- dep-version
  [deps coord]
  (get-in deps [:deps coord :mvn/version]))

(defn- alias-extra-dep-version
  [deps alias coord]
  (get-in deps [:aliases alias :extra-deps coord :mvn/version]))

(defn dependency-pins
  []
  (let [pnix-deps (read-edn-file (deps-file "pnix-clj"))
        clj-meta-deps (read-edn-file (deps-file "clj-meta"))]
    {:schema :pnix-clj.classfile-dependency-pins.v0
     :asm-util
     [{:owner :pnix-clj
       :location [:deps 'org.ow2.asm/asm-util]
       :mvn/version (dep-version pnix-deps 'org.ow2.asm/asm-util)}
      {:owner :clj-meta
       :location [:aliases :gate :extra-deps 'org.ow2.asm/asm-util]
       :mvn/version (alias-extra-dep-version clj-meta-deps
                                              :gate
                                              'org.ow2.asm/asm-util)}
      {:owner :clj-meta
       :location [:aliases :bytecode-verifier :extra-deps
                  'org.ow2.asm/asm-util]
       :mvn/version (alias-extra-dep-version clj-meta-deps
                                              :bytecode-verifier
                                              'org.ow2.asm/asm-util)}
      {:owner :clj-meta
       :location [:aliases :verified-compile :extra-deps
                  'org.ow2.asm/asm-util]
       :mvn/version (alias-extra-dep-version clj-meta-deps
                                              :verified-compile
                                              'org.ow2.asm/asm-util)}]
     :shaded-clojure-asm
     {:class "clojure.asm.Opcodes"
      :owner :clojure-runtime
      :pin-policy :covered-by-clj-meta-bytecode-determinism-receipts}}))

(defn- class-artifact-summary
  [receipt]
  (let [bytecode (:bytecode-artifact receipt)
        verified (:verified-compile receipt)]
    {:bytecode-status (:status bytecode)
     :bytecode-class-count (:class-count bytecode)
     :bytecode-artifact-hash (:artifact-hash bytecode)
     :bytecode-class-names (vec (keys (:class-hashes bytecode)))
     :verified-status (:status verified)
     :verified-class-count (:class-count verified)
     :verified-artifact-hash (:artifact-hash verified)
     :verified-class-names (vec (keys (:class-hashes verified)))
     :verified-ok? (get-in verified [:verification :ok])}))

(defn- pnix-compile-row
  []
  (let [compiled (pnix/compile-source "42")]
    {:source-id :pnix-compile/literal-42
     :source "42"
     :status (:status compiled)
     :receipt-schema (get-in compiled [:compile-receipt :schema])
     :summary (class-artifact-summary (:compile-receipt compiled))}))

(defn- generated-class-rows
  []
  (let [report (clojure-form/report)
        row-by-id (into {} (map (juxt :source-id identity)
                                (:clojure-form-rows report)))]
    (mapv (fn [[class-kind source-id]]
            (let [row (row-by-id source-id)
                  receipt (get-in row [:clj-meta-result :compile-receipt])]
              {:class-kind class-kind
               :source-id source-id
               :status (:status row)
               :expected-value (:expected-value row)
               :observed-value (get-in row [:clj-meta-result :value])
               :summary (class-artifact-summary receipt)}))
          generated-class-fixtures)))

(defn- pins-complete?
  [pins]
  (every? (comp string? :mvn/version) (:asm-util pins)))

(defn- class-row-ok?
  [row]
  (and (= :accepted (:status row))
       (= :ok (get-in row [:summary :bytecode-status]))
       (= :ok (get-in row [:summary :verified-status]))
       (= true (get-in row [:summary :verified-ok?]))
       (pos? (or (get-in row [:summary :bytecode-class-count]) 0))
       (= (get-in row [:summary :bytecode-class-count])
          (get-in row [:summary :verified-class-count]))))

(defn report
  []
  (let [pins (dependency-pins)
        pnix-row (pnix-compile-row)
        generated-rows (generated-class-rows)
        rows (into [pnix-row] generated-rows)
        ok? (and (pins-complete? pins)
                 (every? class-row-ok? generated-rows)
                 (= :ok (:status pnix-row))
                 (= :ok (get-in pnix-row [:summary :bytecode-status]))
                 (= :ok (get-in pnix-row [:summary :verified-status])))]
    {:kind :pnix-deterministic-classfile-report
     :schema :pnix-clj.deterministic-classfile-report.v0
     :policy :pin-asm-and-enumerate-generated-classfiles
     :status (if ok? :ok :failed)
     :reason (if ok?
               :classfile-receipts-complete
               :classfile-receipts-incomplete)
     :dependency-pins pins
     :pnix-compile-row pnix-row
     :generated-class-kinds (vec (keys generated-class-fixtures))
     :generated-class-rows generated-rows
     :row-count (count rows)
     :receipt-hash (hash/data-hash {:pins pins
                                    :rows (mapv #(dissoc % :source) rows)})}))

(defn -main
  [& _]
  (let [{:keys [status row-count receipt-hash]} (report)]
    (println (format "pnix-clj deterministic classfiles: status=%s rows=%d hash=%s"
                     (name status) row-count receipt-hash))
    (shutdown-agents)
    (when (not= :ok status)
      (System/exit 1))))
