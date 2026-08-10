(ns pnix.clj-meta.core
  (:require [clojure.edn :as edn]
            [clojure.java.io :as io]
            [clojure.string :as str]))

(def receipt-path "clj-meta/proof/stage-chain.receipt.edn")

(def required-lane-locks
  [:not-pnix-core-owner :not-brain-codec :not-redb-ingest])

(defn file-path
  [root path]
  (.getPath (io/file root path)))

(defn slurp-if-exists
  [root path]
  (let [f (io/file root path)]
    (when (.isFile f)
      (slurp f))))

(defn read-receipt
  [root]
  (edn/read-string (slurp (file-path root receipt-path))))

(defn proof-report
  [root path]
  (let [text (slurp-if-exists root path)
        ok? (boolean (and text
                          (or (str/includes? text "ok=True")
                              (str/includes? text "ok=true"))))
        fail? (boolean (and text
                            (or (str/includes? text "ok=False")
                                (str/includes? text "ok=false"))))]
    {:path path
     :exists (boolean text)
     :ok (and ok? (not fail?))
     :mentions-ok ok?
     :mentions-fail fail?}))

(defn fixed-point-reports
  [root receipt]
  (->> (:stages receipt)
       (keep (fn [{:keys [id fixed-point-against proof]}]
               (when proof
                 (assoc (proof-report root proof)
                        :stage id
                        :against fixed-point-against))))
       vec))

(defn runtime-smoke-reports
  [root receipt]
  (->> (:runtime-smoke receipt)
       (map (fn [[stage path]]
              (let [text (slurp-if-exists root path)]
                {:stage stage
                 :path path
                 :exists (boolean text)
                 :ok (boolean
                       (and text
                            (str/includes? text ":stage")
                            (str/includes? text ":sum")))})))
       (sort-by (comp name :stage))
       vec))

(defn clj-meta-smoke-reports
  [root receipt]
  (->> (:clj-meta-smoke receipt)
       (map (fn [[stage path]]
              (let [text (slurp-if-exists root path)]
                {:stage stage
                 :path path
                 :exists (boolean text)
                 :ok (boolean
                       (and text
                            (case stage
                              :stm (and (str/includes? text ":stm-ref true")
                                        (str/includes? text ":ready true"))
                              :classes (str/includes? text ".class")
                              true)))})))
       (sort-by (comp name :stage))
       vec))

(defn test-reports
  [root receipt]
  (->> (:tests receipt)
       (map (fn [[stage {:keys [summary tests assertions failures errors]}]]
              (let [text (slurp-if-exists root summary)
                    ok-line (boolean (and text (str/includes? text "OK")))]
                {:stage stage
                 :summary summary
                 :exists (boolean text)
                 :tests tests
                 :assertions assertions
                 :failures failures
                 :errors errors
                 :ok (and ok-line (zero? failures) (zero? errors))})))
       (sort-by (comp name :stage))
       vec))

(defn lane-lock-report
  [receipt]
  (let [lane (:lane receipt)]
    {:locks (select-keys lane required-lane-locks)
     :ok (every? true? (map lane required-lane-locks))}))

(defn report
  [root]
  (let [receipt (read-receipt root)
        target-stage (or (get-in receipt [:lane :target-stage]) 7)
        fixed-points (fixed-point-reports root receipt)
        smokes (runtime-smoke-reports root receipt)
        clj-meta-smokes (clj-meta-smoke-reports root receipt)
        tests (test-reports root receipt)
        lane-lock (lane-lock-report receipt)
        ok (and (= target-stage (count (:stages receipt)))
                (every? :ok fixed-points)
                (every? :ok smokes)
                (every? :ok clj-meta-smokes)
                (every? :ok tests)
                (:ok lane-lock))]
    {:schema "pnix.clj-meta.runtime-report.v2"
     :source (:source receipt)
     :lane (:lane receipt)
     :stage-count (count (:stages receipt))
     :fixed-points fixed-points
     :runtime-smokes smokes
     :clj-meta-smokes clj-meta-smokes
     :tests tests
     :lane-lock lane-lock
     :ready ok}))

(defn -main
  [& args]
  (let [root (or (first args) ".")
        result (report root)]
    (prn result)
    (shutdown-agents)
    (when-not (:ready result)
      (System/exit 1))))
