(ns pnix-clj.lane-policy-test
  (:require [clojure.java.io :as io]
            [clojure.test :refer [deftest is testing]]))

(def allowed-lanes
  #{:core :proof-only :experimental})

(def required-common-keys
  #{:lane :scope :allowed-output})

(def required-core-keys
  #{:product-runtime :semantic-authority})

(def forbidden-proof-only-values
  {:product-runtime :allowed
   :admission :admitted
   :semantic-authority :core-evaluator})

(def forbidden-experimental-values
  {:product-runtime :allowed
   :admission :admitted})

(defn src-files []
  (->> (.listFiles (io/file "src/pnix_clj"))
       (filter #(.isFile %))
       (filter #(= ".clj"
                   (subs (.getName %) (max 0 (- (count (.getName %)) 4)))))))

(defn lane-block [text]
  (let [start (.indexOf text "(def lane-classification")]
    (when (not (neg? start))
      (let [end (.indexOf text "\n\n" start)]
        (subs text start (if (neg? end) (count text) end))))))

(defn read-lane-form [path]
  (let [text (slurp path)
        block (lane-block text)]
    (when block
      (try
        (binding [*read-eval* false]
          (let [form (read-string block)]
            (nth form 2)))
        (catch Throwable t
          {:parse-error (.getMessage t)})))))

(defn file-row [f]
  (let [path (.getPath f)
        m (read-lane-form f)]
    {:file path
     :lane-map m
     :lane (:lane m)}))

(defn missing-keys [m ks]
  (vec (sort (remove #(contains? m %) ks))))

(defn forbidden-value-hits [m forbidden]
  (vec
   (for [[k v] forbidden
         :when (= v (get m k))]
     [k v])))

(deftest every-source-file-has-valid-lane-classification
  (testing "every src/pnix_clj/*.clj file has a readable lane-classification map"
    (let [rows (mapv file-row (src-files))
          missing (filterv #(nil? (:lane-map %)) rows)
          parse-errors (filterv #(contains? (:lane-map %) :parse-error) rows)
          invalid-lanes (filterv #(not (contains? allowed-lanes (:lane %))) rows)]
      (is (= [] missing))
      (is (= [] parse-errors))
      (is (= [] invalid-lanes)))))

(deftest lane-classification-has-required-common-keys
  (testing "all lane classifications expose the common policy surface"
    (let [bad (vec
               (for [{:keys [file lane-map]} (map file-row (src-files))
                     :let [missing (missing-keys lane-map required-common-keys)]
                     :when (seq missing)]
                 {:file file :missing missing}))]
      (is (= [] bad)))))

(deftest core-lanes-carry-runtime-and-authority-boundaries
  (testing "core lanes must explicitly state runtime and semantic authority"
    (let [bad (vec
               (for [{:keys [file lane-map lane]} (map file-row (src-files))
                     :when (= :core lane)
                     :let [missing (missing-keys lane-map required-core-keys)]
                     :when (seq missing)]
                 {:file file :missing missing}))]
      (is (= [] bad)))))

(deftest proof-only-lanes-cannot-claim-product-or-core-authority
  (testing "proof-only lanes remain evidence/corpus lanes, not product runtime"
    (let [bad (vec
               (for [{:keys [file lane-map lane]} (map file-row (src-files))
                     :when (= :proof-only lane)
                     :let [hits (forbidden-value-hits lane-map forbidden-proof-only-values)]
                     :when (seq hits)]
                 {:file file :forbidden hits}))]
      (is (= [] bad)))))

(deftest experimental-lanes-cannot-claim-product-admission
  (testing "experimental lanes cannot silently become product/admission paths"
    (let [bad (vec
               (for [{:keys [file lane-map lane]} (map file-row (src-files))
                     :when (= :experimental lane)
                     :let [hits (forbidden-value-hits lane-map forbidden-experimental-values)]
                     :when (seq hits)]
                 {:file file :forbidden hits}))]
      (is (= [] bad)))))
