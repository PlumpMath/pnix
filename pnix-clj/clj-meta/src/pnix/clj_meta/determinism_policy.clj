(ns pnix.clj-meta.determinism-policy
  "Deterministic generated-name policy witness.

  The compiler uses a deterministic namespace+source hash plus a per-compilation
  *gen-counter* starting at -1, producing
  pnix.clj_meta.gen.Fn__<source-sha12>__0, __1, ... without gensym or
  time/random state. This receipt checks that repeated compilation of nested
  forms has stable class names and byte digests."
  (:require [clojure.java.io :as io]
            [clojure.pprint :as pp]
            [clojure.string :as str]
            [pnix.clj-meta.compiler :as comp])
  (:import [java.security MessageDigest]))

(def receipt-path "clj-meta/proof/determinism-policy.receipt.edn")

(defn- sha256-bytes
  [^bytes bs]
  (let [md (MessageDigest/getInstance "SHA-256")]
    (apply str
           (map #(format "%02x" (bit-and 0xff %))
                (.digest md bs)))))

(defn- sha256-string
  [s]
  (sha256-bytes (.getBytes ^String (str s) "UTF-8")))

(defn- class-digests
  [classes]
  (into (sorted-map)
        (map (fn [[class-name bytes]]
               [class-name (sha256-bytes bytes)])
             classes)))

(defn- suffix
  [class-name]
  (when-let [s (second (re-find #"Fn__(?:[0-9a-f]{12}__)?(\d+)$" class-name))]
    (Long/parseLong s)))

(defn- source-hash
  [class-name]
  (second (re-find #"Fn__([0-9a-f]{12})__\d+$" class-name)))

(defn- contiguous-suffixes?
  [class-names]
  (let [suffixes (vec (sort (keep suffix class-names)))]
    (= suffixes (vec (range (count suffixes))))))

(defn- attempt
  [form]
  (let [classes (comp/compile-classes form)
        names (vec (sort (keys classes)))]
    {:class-names names
     :class-digests (class-digests classes)
     :class-count (count classes)
     :contiguous-suffixes? (contiguous-suffixes? names)
     :single-source-hash? (= 1 (count (set (keep source-hash names))))
     :all-generated-prefix?
     (every? #(str/starts-with? % "pnix.clj_meta.gen.Fn__") names)}))

(defn- stable-attempts?
  [attempts]
  (every? #(= (first attempts) %) attempts))

(defn- case-row
  [{:keys [id desc form attempts]}]
  (let [attempts (mapv (fn [_] (attempt form))
                       (range (or attempts 4)))
        ok? (and (stable-attempts? attempts)
                 (every? :contiguous-suffixes? attempts)
                 (every? :single-source-hash? attempts)
                 (every? :all-generated-prefix? attempts))]
    {:id id
     :kind :deterministic-generated-name-policy
     :desc desc
     :form (pr-str form)
     :attempts attempts
     :policy {:counter-reset :per-compilation-unit
              :initial-value -1
              :source-id :sha256-ns-plus-pr-str-prefix-12
              :prefix "pnix.clj_meta.gen.Fn__"
              :shape "pnix.clj_meta.gen.Fn__<source-sha12>__<counter>"
              :forbidden [:gensym :random :time]}
     :gate/verdict (if ok? :accepted :rejected)
     :ok ok?}))

(defn- specs
  []
  [{:id :nested-closure-names
    :desc "nested closure compilation reuses Fn__<source-sha12>__0..N deterministically"
    :form '(fn [x]
             (let [f (fn [y] (+ x y))
                   g (fn [z] (f (+ z 1)))]
               (g 2)))}
   {:id :try-finally-names
    :desc "try/finally artifact names and bytes are repeat-stable"
    :form '(fn []
             (try
               (+ 20 22)
               (finally
                 (+ 1 2))))}
   {:id :case-direct-names
    :desc "case direct branch-chain artifact names and bytes are repeat-stable"
    :form '(fn [n]
             (case n
               1 :one
               2 :two
               :other))}
   {:id :letfn-mutual-recursion-names
    :desc "letfn mutual-recursion generated classes and cyclic capture fields are repeat-stable"
    :form '(fn [n]
             (letfn [(ev? [x] (if (zero? x) true (od? (dec x))))
                     (od? [x] (if (zero? x) false (ev? (dec x))))]
               (ev? n)))}])

(defn run
  []
  (let [rows (mapv case-row (specs))
        accepted (filter #(= :accepted (:gate/verdict %)) rows)
        rejected (filter #(= :rejected (:gate/verdict %)) rows)
        canonical (mapv #(select-keys %
                                      [:id
                                       :kind
                                       :attempts
                                       :policy
                                       :gate/verdict
                                       :ok])
                        rows)
        invariants (sorted-map
                    :all-rows-ok (every? :ok rows)
                    :no-rejected-rows (empty? rejected)
                    :all-accepted (= (count rows) (count accepted))
                    :counter-policy-recorded
                    (every? #(= -1 (get-in % [:policy :initial-value])) rows)
                    :forbids-gensym-random-time
                    (every? #(= [:gensym :random :time]
                                (get-in % [:policy :forbidden]))
                            rows))
        ok? (every? true? (vals invariants))]
    {:schema "pnix.clj-meta.determinism-policy.receipt.v1"
     :stage [:M5 :determinism]
     :target-goal "meta-circular stage15/N Clojure compiler"
     :desc "repeat-stable generated class name and byte digest policy"
     :rows rows
     :status-counts {:accepted (count accepted)
                     :held 0
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
    (println (str "determinism policy: "
                  (if (:ok r) "OK" "FAILED")
                  "  (receipt: " receipt-path
                  ", digest: " (:canonical-receipt-digest r)
                  ")"))
    (shutdown-agents)
    (when-not (:ok r)
      (System/exit 1))))
