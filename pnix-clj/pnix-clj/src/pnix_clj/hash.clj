(ns pnix-clj.hash
  (:import [java.nio.charset StandardCharsets]
           [java.security MessageDigest]))

(def lane-classification
  {:lane :core
   :scope :deterministic-hash-primitive
   :role :nix-message-digests-and-edn-data-hash
   :product-runtime :allowed
   :semantic-authority :forbidden
   :mutation :forbidden
   :admission :forbidden
   :determinism :required
   :allowed-output :stable-content-digest})

(defn- hex-digest
  [digest]
  (apply str (map #(format "%02x" (bit-and % 0xff)) digest)))

(def nix-hash-algorithms
  "Nix-compatible, non-experimental hashString algorithms mapped to their JVM
  MessageDigest names. The legacy algorithms remain available here because
  this is a compatibility primitive; security policy belongs in a separate
  profile, not in the Nix language surface."
  (sorted-map "md5" "MD5"
              "sha1" "SHA-1"
              "sha256" "SHA-256"
              "sha512" "SHA-512"))

(defn hash-bytes
  "Hash an exact byte array using a Nix hashString algorithm and return
  lowercase hexadecimal. Throws ex-info for an unsupported algorithm so the
  evaluator can translate it to its structured builtin error vocabulary."
  [algorithm bytes]
  (if-let [jvm-algorithm (get nix-hash-algorithms algorithm)]
    (hex-digest
     (.digest (MessageDigest/getInstance ^String jvm-algorithm)
              ^bytes bytes))
    (throw (ex-info "unsupported Nix hashString algorithm"
                    {:algorithm algorithm
                     :supported-algorithms (vec (keys nix-hash-algorithms))}))))

(defn hash-string
  "Hash the UTF-8 bytes of `s` using a Nix hashString algorithm."
  [algorithm ^String s]
  (hash-bytes algorithm (.getBytes s StandardCharsets/UTF_8)))

(defn sha256-bytes
  [bytes]
  (hash-bytes "sha256" bytes))

(defn sha256
  [s]
  (hash-string "sha256" (str s)))

(defn data-hash
  [x]
  (sha256 (pr-str x)))
