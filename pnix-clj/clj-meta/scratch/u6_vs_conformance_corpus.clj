;; Manual, ad-hoc cross-check -- NOT wired into any gate (bin/clj-meta-gate,
;; -M:conformance, -M:diverse-double-compile all ignore this file).
;;
;; conformance.clj's 116-case corpus was built to validate the PRODUCTION
;; backend (compiler.clj) against real host eval, with U5 (kernel.clj) as an
;; advisory third axis -- U6 (frontend_selfhost.clj/compile-source) was never
;; part of that comparison. This script runs U6 against the same 116 cases to
;; see how much of that corpus the independent witness also covers, and to
;; surface any genuine crashes (as opposed to honest "not supported" gaps).
;;
;; Run from pnix-clj/clj-meta/:
;;   clojure -M -e "(load-file \"scratch/u6_vs_conformance_corpus.clj\")"
;;
;; 2026-08-15/17 run: 85/116 matched before the try/catch-in-subexpression
;; VerifyError fix, 87/116 after (see STATUS.md's 42nd-slice entry). The
;; other ~29 are mostly honest scope gaps (def/top-level side effects,
;; instance?, var, defrecord, StringBuilder and other unregistered classes,
;; clojure.core.protocols/coll-reduce, (set! (. p x) v)-style interop set!,
;; comma-bearing map literals) -- not yet individually triaged as
;; feature-worth-adding vs. intentionally out of scope.
;;
;; Whether to promote this into conformance.clj as a permanent, gated
;; `via-u6` fourth axis is an open decision, not yet made.

(require '[pnix.clj-meta.conformance :as conf])
(require '[pnix.clj-meta.frontend-selfhost :as fsh])

(defn via-u6 [form args]
  (let [{f :fn} (fsh/compile-source (pr-str form))]
    (apply f args)))

(defn try-val [f]
  (try {:val (f)} (catch Throwable t {:err (.getMessage t) :class (class t)})))

(let [cases (#'conf/all-cases)
      results (mapv (fn [[form args]]
                       (let [host (try-val #(apply (eval form) args))
                             u6 (try-val #(via-u6 form args))
                             ok? (and (contains? host :val)
                                      (contains? u6 :val)
                                      (= (:val host) (:val u6)))]
                         {:form form :args args :host host :u6 u6 :ok ok?}))
                     cases)
      total (count results)
      matched (count (filter :ok results))
      unsupported (count (remove :ok results))]
  (println :total total :matched matched :unsupported unsupported)
  (println "---unsupported cases---")
  (doseq [r (remove :ok results)]
    (println (pr-str (:form r)))
    (println "  host:" (:host r))
    (println "  u6:  " (:u6 r))))
