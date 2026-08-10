(ns pnix.clj-meta.jarproof
  (:require [clojure.java.io :as io]
            [clojure.set :as set]
            [clojure.string :as str])
  (:import [java.nio.charset StandardCharsets]
           [java.security MessageDigest]
           [java.util Arrays Collections]
           [java.util.zip ZipFile]))

(defn hex
  [bytes]
  (apply str (map #(format "%02x" (bit-and 0xff %)) bytes)))

(defn sha256
  []
  (MessageDigest/getInstance "SHA-256"))

(defn digest-stream
  [input]
  (let [md (sha256)
        buffer (byte-array 32768)]
    (loop []
      (let [n (.read input buffer)]
        (when (pos? n)
          (.update md buffer 0 n)
          (recur))))
    (.digest md)))

(def generated-class-hash-pattern
  #"(pnix[/\.]clj_meta[/\.]gen[/\.](?:Fn|Reify)__)[0-9a-f]{12}(__\d+)")

(defn normalize-generated-class-name
  [s]
  (str/replace s generated-class-hash-pattern
               (fn [[_ prefix suffix]]
                 (str prefix "000000000000" suffix))))

(defn digest-bytes
  [^bytes bytes]
  (let [md (sha256)]
    (.update md bytes)
    (.digest md)))

(defn normalize-generated-class-bytes
  [^bytes bytes]
  (let [text (String. bytes StandardCharsets/ISO_8859_1)]
    (.getBytes (normalize-generated-class-name text)
               StandardCharsets/ISO_8859_1)))

(defn zip-entries
  [jar-path]
  (with-open [zip (ZipFile. (io/file jar-path))]
    (let [entries (Collections/list (.entries zip))]
      (->> entries
           (remove #(.isDirectory %))
           (map #(.getName %))
           sort
           vec))))

(defn entry-digests
  [jar-path]
  (with-open [zip (ZipFile. (io/file jar-path))]
    (mapv (fn [name]
            (with-open [input (.getInputStream zip (.getEntry zip name))]
              [name (digest-stream input)]))
          (zip-entries jar-path))))

(defn normalized-entry-digests
  [jar-path]
  (with-open [zip (ZipFile. (io/file jar-path))]
    (->> (zip-entries jar-path)
         (mapv (fn [name]
                 (with-open [input (.getInputStream zip (.getEntry zip name))]
                   [(normalize-generated-class-name name)
                    (digest-bytes
                     (normalize-generated-class-bytes
                      (.readAllBytes input)))])))
         (sort-by first)
         vec)))

(defn stable-jar-digest
  [jar-path]
  (let [md (sha256)]
    (doseq [[name digest] (entry-digests jar-path)]
      (.update md (.getBytes name "UTF-8"))
      (.update md (byte-array [0]))
      (.update md ^bytes digest)
      (.update md (byte-array [0])))
    (hex (.digest md))))

(defn write-digest-file!
  [out jars]
  (spit out
        (apply str
               (for [jar jars]
                 (str (stable-jar-digest jar) "  " jar "\n")))))

(defn same-entry-digests?
  [left right]
  (let [a (normalized-entry-digests left)
        b (normalized-entry-digests right)]
    {:ok (and (= (map first a) (map first b))
              (every? true? (map (fn [[[_ da] [_ db]]]
                                    (Arrays/equals ^bytes da ^bytes db))
                                  (map vector a b))))
     :left-count (count a)
     :right-count (count b)
     :left a
     :right b}))

(defn diff-lines
  [left right]
  (let [left-map (into {} (map (fn [[name digest]] [name (hex digest)]) left))
        right-map (into {} (map (fn [[name digest]] [name (hex digest)]) right))
        names (sort (set/union (set (keys left-map)) (set (keys right-map))))]
    (->> names
         (keep (fn [name]
                 (let [a (get left-map name)
                       b (get right-map name)]
                   (when (not= a b)
                     (str "  diff " name " left=" (or a "<missing>")
                          " right=" (or b "<missing>"))))))
         (take 80)
         vec)))

(defn compare-pair-line
  [label left right allowed-transition]
  (let [{:keys [ok left-count right-count left right]} (same-entry-digests? left right)
        allowed? (and allowed-transition (= label (:label allowed-transition)))
        accepted? (or ok allowed?)]
    {:ok accepted?
     :text (str label ": ok=" (if accepted? "True" "False")
                " equal=" (if ok "True" "False")
                (when allowed?
                  (str " transition=" (:name allowed-transition)))
                " entries_left=" left-count
                " entries_right=" right-count
                "\n"
                (when-not ok
                  (str (apply str (map #(str % "\n") (diff-lines left right))))))}))

(defn compare-stage!
  ([out triples]
   (compare-stage! out triples nil))
  ([out triples allowed-transition]
  (let [results (mapv (fn [[label left right]]
                        (compare-pair-line label left right allowed-transition))
                      triples)]
    (spit out (apply str (map :text results)))
    (every? :ok results))))

(defn usage!
  []
  (binding [*out* *err*]
    (println "usage:")
    (println "  jarproof digest OUT JAR...")
    (println "  jarproof compare OUT LABEL LEFT RIGHT [LABEL LEFT RIGHT ...]")
    (println "  jarproof compare-allow OUT ALLOW-LABEL TRANSITION LABEL LEFT RIGHT ..."))
  (System/exit 2))

(defn -main
  [& args]
  (let [[cmd out & rest] args]
    (case cmd
      "digest"
      (if (and out (seq rest))
        (write-digest-file! out rest)
        (usage!))

      "compare"
      (if (and out (seq rest) (zero? (mod (count rest) 3)))
        (let [triples (partition 3 rest)]
          (when-not (compare-stage! out triples)
            (System/exit 1)))
        (usage!))

      "compare-allow"
      (let [[allow-label transition & triples*] rest]
        (if (and out allow-label transition (seq triples*) (zero? (mod (count triples*) 3)))
          (let [triples (partition 3 triples*)]
            (when-not (compare-stage! out triples {:label allow-label
                                                   :name transition})
              (System/exit 1)))
          (usage!)))

      (usage!))))
