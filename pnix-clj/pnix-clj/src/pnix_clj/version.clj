(ns pnix-clj.version)

(def lane-classification
  {:lane :core
   :scope :runtime-version-metadata
   :role :nix-compatible-version-and-system-metadata
   :product-runtime :allowed
   :semantic-authority :metadata-only
   :mutation :forbidden
   :admission :forbidden
   :determinism :required
   :allowed-output :version-comparison-or-runtime-metadata})

(def current-system
  "x86_64-darwin")

(def nix-version
  "2.18.0-pnix")

(def store-dir
  "/nix/store")

(def lang-version
  6)

(defn split-version
  [s]
  (vec (re-seq #"[A-Za-z]+|[0-9]+" (str s))))

(defn- numeric-part?
  [part]
  (boolean (re-matches #"[0-9]+" part)))

(defn- signum
  [n]
  (cond (neg? n) -1 (pos? n) 1 :else 0))

(defn- compare-component
  "Compare two version components the way Nix does. Missing components compare
  as the empty string. The empty component is older than any real component
  except that it is NEWER than \"pre\"; \"pre\" is older than anything else; a
  numeric component is newer than a non-numeric one; otherwise the comparison is
  numeric (both numeric) or lexicographic."
  [c1 c2]
  (cond
    (= c1 c2) 0
    (= c1 "") (if (= c2 "pre") 1 -1)
    (= c2 "") (if (= c1 "pre") -1 1)
    (= c1 "pre") -1
    (= c2 "pre") 1
    (and (numeric-part? c1) (numeric-part? c2)) (signum (compare (bigint c1) (bigint c2)))
    (numeric-part? c1) 1
    (numeric-part? c2) -1
    :else (signum (compare c1 c2))))

(defn compare-versions
  [left right]
  (loop [left-parts (seq (split-version left))
         right-parts (seq (split-version right))]
    (if (and (nil? left-parts) (nil? right-parts))
      0
      ;; A shorter version is padded with empty components so that, e.g., a
      ;; trailing "pre" component still compares as a pre-release.
      (let [c1 (if left-parts (first left-parts) "")
            c2 (if right-parts (first right-parts) "")
            part-result (compare-component c1 c2)]
        (if (zero? part-result)
          (recur (next left-parts) (next right-parts))
          part-result)))))

(defn parse-drv-name
  [s]
  ;; Nix splits at the FIRST '-' followed by an ASCII digit. A greedy regex
  ;; incorrectly chose the last such hyphen (`a-1-b-2` -> `a-1-b`/`2`) and
  ;; could not represent the empty name in `-1`.
  (let [s (str s)
        n (count s)
        split-at (loop [i 0]
                   (cond
                     (>= (inc i) n) nil
                     (and (= \- (.charAt s i))
                          (let [c (.charAt s (inc i))]
                            (<= (int \0) (int c) (int \9)))) i
                     :else (recur (inc i))))]
    (if (some? split-at)
      {"name" (subs s 0 split-at)
       "version" (subs s (inc split-at))}
      {"name" s
       "version" ""})))
