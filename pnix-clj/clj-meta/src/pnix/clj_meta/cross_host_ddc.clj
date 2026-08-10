(ns pnix.clj-meta.cross-host-ddc
  "M11 cross-host independent-toolchain DDC evidence lane.

  Diverse Double-Compiling (Wheeler) defends against Trusting Trust by compiling
  with a *diverse* toolchain and comparing. Full Wheeler DDC over the compiler
  binary stays held for clj-meta (host Compiler and our backend are not
  bit-identical targets). This lane proves a weaker but genuine diversity fact:

    our backend (compiler.clj), when hosted by two INDEPENDENT clojure.lang.Compiler
    versions (1.11.1 and 1.12.0, separately developed/released), emits a
    BIT-IDENTICAL fixed-target bytecode bundle.

  A Trusting-Trust backdoor that altered our target output would have to be
  replicated identically in two independent Clojure releases to escape detection.
  This is partial DDC evidence (cross-host emit determinism), not the full
  compiler-binary DDC — which remains held.

  The lane spawns subprocesses (needs the clojure CLI + Maven), so it is an
  evidence-only lane, not part of the fast in-process gate."
  (:require [clojure.java.io :as io]
            [clojure.java.shell :as shell]
            [clojure.pprint :as pp]
            [clojure.string :as str]
            [pnix.clj-meta.compiler :as comp])
  (:import [java.security MessageDigest]))

(def receipt-path "clj-meta/proof/cross-host-ddc.receipt.edn")

(def ^:private analyzer-version "1.2.3")
(def ^:private host-versions ["1.11.1" "1.12.0"])

;; 고정 target: 분기/재귀/loop/컬렉션/interop 를 섞은 결정적 프로그램.
;; emit 결정성(gensym-free, 결정적 class name)으로 host 버전과 무관하게 같은 bytecode.
(def ^:private target-form
  '(fn [n]
     (loop [i 0 acc 0]
       (if (< i n)
         (recur (+ i 1) (+ acc (* i i)))
         [acc (str "n=" n) (mapv inc [1 2 3])]))))

(defn- sha256-hex
  [^bytes bs]
  (let [md (MessageDigest/getInstance "SHA-256")]
    (apply str (map #(format "%02x" (bit-and 0xff %)) (.digest md bs)))))

(defn target-digest
  "고정 target 을 우리 backend 로 compile-classes 한 결과의 결정적 SHA-256."
  []
  (let [classes (comp/compile-classes target-form)
        md (MessageDigest/getInstance "SHA-256")]
    (doseq [[cn bytes] (sort-by key classes)]
      (.update md (.getBytes ^String cn "UTF-8"))
      (.update md ^bytes bytes))
    (apply str (map #(format "%02x" (bit-and 0xff %)) (.digest md)))))

(defn- deps-edn-for
  [clj-version]
  (str "{:deps {org.clojure/clojure {:mvn/version \"" clj-version "\"} "
       "org.clojure/tools.analyzer.jvm {:mvn/version \"" analyzer-version "\"}} "
       ":paths [\"src\"]}"))

(defn- child-digest-under
  "subprocess 에서 clj-version 으로 backend 를 로드해 target digest 를 얻는다."
  [clj-version]
  (let [{:keys [exit out err]}
        (shell/sh "clojure" "-Sdeps" (deps-edn-for clj-version)
                  "-M" "-m" "pnix.clj-meta.cross-host-ddc" "child"
                  :dir (or (System/getenv "CLJ_META_ROOT") "."))
        line (some->> (str/split-lines (str out))
                      (filter #(str/starts-with? % "DIGEST="))
                      first)]
    {:clojure-version clj-version
     :exit exit
     :digest (when line (subs line (count "DIGEST=")))
     :error (when (or (not (zero? exit)) (nil? line))
              (str "exit=" exit " err=" (when err (subs err 0 (min 400 (count err))))))}))

(defn run
  []
  (let [results (mapv child-digest-under host-versions)
        digests (mapv :digest results)
        all-present? (every? some? digests)
        bit-identical? (and all-present? (apply = digests))
        ok? bit-identical?
        canonical {:target (pr-str target-form)
                   :analyzer-version analyzer-version
                   :host-versions host-versions
                   :digests (mapv #(select-keys % [:clojure-version :digest]) results)
                   :bit-identical? bit-identical?}
        invariants (sorted-map
                    :all-host-runs-succeeded all-present?
                    :cross-host-emit-bit-identical bit-identical?
                    ;; full Wheeler compiler-binary DDC 는 여전히 held (이 레인은 partial).
                    :full-compiler-binary-ddc-not-claimed true)]
    {:schema "pnix.clj-meta.cross-host-ddc.receipt.v1"
     :stage [:M11 :cross-host-ddc]
     :target-goal "meta-circular stage15/N Clojure compiler"
     :desc "our backend emits bit-identical target bytecode under two independent clojure.lang.Compiler versions"
     :policy {:proves "cross-host emit determinism: backend output independent of the hosting Clojure version (partial Trusting-Trust evidence)"
              :held "full Wheeler DDC over the compiler binary (independent compiler producing a bit-identical compiler artifact) remains held"
              :evidence-only "spawns subprocesses (clojure CLI + Maven); not part of the in-process gate"}
     :results results
     :canonical-receipt canonical
     :canonical-receipt-digest (sha256-hex (.getBytes (pr-str canonical) "UTF-8"))
     :ok ok?}))

(defn write-receipt!
  [r]
  (io/make-parents receipt-path)
  (spit receipt-path (with-out-str (pp/pprint r)))
  r)

(defn -main
  [& args]
  (if (= (first args) "child")
    (do
      (println (str "DIGEST=" (target-digest)))
      (flush)
      (shutdown-agents))
    (let [r (write-receipt! (run))]
      (doseq [row (:results r)]
        (println (str "  [" (if (:digest row) "OK" "FAIL") "] clojure-"
                      (:clojure-version row) " -> "
                      (or (:digest row) (:error row)))))
      (println (str "cross-host DDC: "
                    (if (:ok r) "BIT-IDENTICAL ✅" "FAILED/DIVERGED")
                    "  (receipt: " receipt-path
                    ", digest: " (:canonical-receipt-digest r) ")"))
      (shutdown-agents)
      (when-not (:ok r)
        (System/exit 1)))))
