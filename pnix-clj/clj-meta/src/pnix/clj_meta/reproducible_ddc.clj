(ns pnix.clj-meta.reproducible-ddc
  "M11e reproducible-build lane as DDC evidence.

  This does not consume clj-meta receipts from pnix-clj. It only reclassifies
  the existing stock Clojure stage7 reproducible-build lane as independent
  toolchain evidence for the clj-meta trust boundary."
  (:require [clojure.edn :as edn]
            [clojure.java.io :as io]
            [clojure.pprint :as pp]
            [clojure.string :as str])
  (:import [java.security MessageDigest]))

(def receipt-path "clj-meta/proof/reproducible-ddc.receipt.edn")

(defn- sha256-bytes
  [^bytes bs]
  (let [md (MessageDigest/getInstance "SHA-256")]
    (apply str
           (map #(format "%02x" (bit-and 0xff %))
                (.digest md bs)))))

(defn- sha256-string
  [s]
  (sha256-bytes (.getBytes ^String (str s) "UTF-8")))

(defn- cwd
  []
  (.getCanonicalFile (io/file ".")))

(defn- repo-root
  []
  (let [here (cwd)]
    (cond
      (.isFile (io/file here "clj-meta/proof/stage-chain.receipt.edn"))
      here

      (.isFile (io/file here "proof/stage-chain.receipt.edn"))
      (.getCanonicalFile (.getParentFile here))

      :else here)))

(defn- file-status
  [root path]
  (let [f (io/file root path)]
    {:path path
     :exists (.isFile f)
     :sha256 (when (.isFile f)
               (sha256-string (slurp f)))}))

(defn- read-repro-receipt
  [root]
  (let [f (io/file root "clj-meta/proof/stage-chain.receipt.edn")]
    (if (.isFile f)
      (edn/read-string (slurp f))
      ;; Honest absent evidence: the reproducible-build lane has not been run.
      {:schema "pnix.clj-meta.stage-chain.receipt.missing"
       :stages []
       :lane {:name "reproducible-build"
              :kind "reproducible-build"
              :target-stage 7
              :not-pnix-core-owner true
              :not-brain-codec true
              :not-redb-ingest true}
       :source {}
       :runtime-smoke {}
       :clj-meta-smoke {}
       :missing-stage-chain-receipt true})))

(defn- proof-ok?
  [root path]
  (let [f (io/file root path)
        text (when (.isFile f) (slurp f))
        mentions-ok? (boolean (and text
                                   (or (str/includes? text "ok=True")
                                       (str/includes? text "ok=true"))))
        mentions-fail? (boolean (and text
                                     (or (str/includes? text "ok=False")
                                         (str/includes? text "ok=false"))))]
    {:path path
     :exists (boolean text)
     :mentions-ok mentions-ok?
     :mentions-fail mentions-fail?
     :ok (and mentions-ok? (not mentions-fail?))}))

(defn- smoke-ok?
  [root [stage path]]
  (let [f (io/file root path)
        text (when (.isFile f) (slurp f))]
    {:stage stage
     :path path
     :exists (boolean text)
     :ok (boolean (and text
                       (str/includes? text ":stage")
                       (str/includes? text ":sum")))}))

(defn- clj-meta-smoke-ok?
  [root [kind path]]
  (let [f (io/file root path)
        text (when (.isFile f) (slurp f))]
    {:kind kind
     :path path
     :exists (boolean text)
     :ok (boolean
          (and text
               (cond
                 (= kind :stm)
                 (and (str/includes? text ":stm-ref true")
                      (str/includes? text ":ready true"))

                 (= kind :classes)
                 (str/includes? text ".class")

                 :else true)))}))

(defn run
  []
  (let [root (repo-root)
        repro (read-repro-receipt root)
        stages (:stages repro)
        fixed-proofs (mapv #(proof-ok? root (:proof %))
                           (filter :proof stages))
        digest-files (mapv #(file-status root (:jar-digest %)) stages)
        runtime-smokes (mapv #(smoke-ok? root %)
                             (sort-by (comp name key) (:runtime-smoke repro)))
        meta-smokes (mapv #(clj-meta-smoke-ok? root %)
                          (sort-by (comp name key) (:clj-meta-smoke repro)))
        lane (:lane repro)
        canonical {:schema (:schema repro)
                   :source (select-keys (:source repro)
                                        [:name :version :artifact-id :scm-tag])
                   :lane (select-keys lane
                                      [:name
                                       :kind
                                       :target-stage
                                       :not-pnix-core-owner
                                       :not-brain-codec
                                       :not-redb-ingest])
                   :stage-ids (mapv :id stages)
                   :fixed-proofs fixed-proofs
                   :digest-files digest-files
                   :runtime-smokes runtime-smokes
                   :clj-meta-smokes meta-smokes}
        invariants (sorted-map
                    :schema-v2 (= "pnix.clj-meta.stage-chain.receipt.v2"
                                  (:schema repro))
                    :reproducible-build-lane (= "reproducible-build"
                                                (:kind lane))
                    :target-stage-seven (= 7 (:target-stage lane))
                    :seven-stages (= 7 (count stages))
                    :lane-not-product-owner
                    (and (:not-pnix-core-owner lane)
                         (:not-brain-codec lane)
                         (:not-redb-ingest lane))
                    :all-digest-files-present (every? :exists digest-files)
                    :all-fixed-point-proofs-ok (every? :ok fixed-proofs)
                    :all-runtime-smokes-ok (every? :ok runtime-smokes)
                    :all-clj-meta-smokes-ok (every? :ok meta-smokes))
        ok? (every? true? (vals invariants))]
    {:schema "pnix.clj-meta.reproducible-ddc.receipt.v1"
     :stage [:M11e]
     :target-goal "meta-circular stage15/N Clojure compiler"
     :desc "stock Clojure reproducible-build lane reclassified as DDC independent-toolchain evidence"
     :trust/status :evidence-only
     :promotion/allowed? false
     :ddc-role :independent-toolchain-evidence
     :policy {:accepted "all reproducible-build lane files and fixed-point proofs are present and OK"
              :not-accepted "this does not admit pnix-clj launcher consumption or bit-identical DDC"}
     :root (.getPath root)
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
    (println (str "reproducible DDC lane: "
                  (if (:ok r) "OK" "FAILED")
                  "  (receipt: " receipt-path
                  ", digest: " (:canonical-receipt-digest r)
                  ")"))
    (shutdown-agents)
    (when-not (:ok r)
      (System/exit 1))))
