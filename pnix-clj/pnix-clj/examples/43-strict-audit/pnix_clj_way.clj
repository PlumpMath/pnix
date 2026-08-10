;;; pnix-clj의 방식 - strict typing은 behavior change 없이 audit/gate report로 분리한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/43-strict-audit/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.strict-audit :as strict]))

(let [opts {:include-runtime? false}
      audit (strict/report opts)
      gate (strict/strict-gate-report opts)]
  (println "audit sources:" (:source-count audit))
  (println "strict ok/violation/held:" (:strict-ok audit) (:strict-violation audit) (:held audit))
  (println "gate checked/ok/failed:" (:checked gate) (:ok gate) (:failed gate))

  (assert (pos? (:source-count audit)))
  (assert (= (:source-count audit) (:classified-source-count gate)))
  (assert (= (:checked gate) (:ok gate)))
  (assert (zero? (:failed gate))))

(println)
(println "결론: pnix-clj strict audit는 typing 강화 후보를 report로 측정하고 opt-in gate로 재확인한다.")
(shutdown-agents)
