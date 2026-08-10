;;; pnix-clj의 방식 - projection host crossing에 interop metadata와 witness를 붙인다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/75-clojure-projection-host-crossing/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.clojure-projection.host :as host]))

(let [read-result (host/read-host-value "{:a 1}")
      eval-result (host/host-eval-source :demo "(+ 1 2)")]
  (println "read:" read-result)
  (println "eval:" (select-keys eval-result [:status :value :interop :capability :witness]))

  (assert (= :ok (:status read-result)))
  (assert (= {:a 1} (:value read-result)))
  (assert (= :ok (:status eval-result)))
  (assert (= 3 (:value eval-result)))
  (assert (= :host-eval (get-in eval-result [:witness :effect-class])))
  (assert (= :clojure-projection->host-value
             (get-in eval-result [:witness :direction])))
  (assert (string? (get-in eval-result [:witness :input-hash])))
  (assert (string? (get-in eval-result [:witness :output-hash])))
  (assert (string? (get-in eval-result [:witness :witness-hash]))))

(println)
(println "결론: pnix-clj projection host crossing은 host 값에 effect/capability/witness evidence를 붙인다.")
(shutdown-agents)

