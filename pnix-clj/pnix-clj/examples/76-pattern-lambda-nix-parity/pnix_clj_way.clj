;;; pnix-clj의 방식 - D19 pattern lambda 의미를 evaluator/clj-meta/px-runtime receipt로 고정한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/76-pattern-lambda-nix-parity/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.core :as pnix]))

(def accepted-cases
  [{:source "({ a ? throw \"x\" }: 1) { }"
    :expected 1
    :note "unused default is lazy"}
   {:source "({ a ? b, b ? 2 }: a) { }"
    :expected 2
    :note "default sees later formal through knot"}
   {:source "({ a, ... }: a) { a = 1; b = 2; }"
    :expected 1
    :note "ellipsis permits extra keys"}
   {:source "({ a ? 5 }@args: args.a or \"absent\") { }"
    :expected "absent"
    :note "@as binds the actual argument only"}])

(def held-cases
  [{:source "({ a }: a) { a = 1; b = 2; }"
    :reason :unexpected-lambda-pattern-arg}
   {:source "({ a }: a) 1"
    :reason :lambda-pattern-arg-not-attrset}
   {:source "({ a ? b, b ? a }: a) { }"
    :reason :infinite-recursion}])

(doseq [{:keys [source expected note]} accepted-cases]
  (let [receipt (pnix/run-source source)]
    (println "accepted:" note "=>" (select-keys receipt [:status :reason])
             (select-keys (:eval-result receipt) [:status :value]))
    (assert (= :accepted (:status receipt)))
    (assert (= :all-lanes-agree (:reason receipt)))
    (assert (= expected (get-in receipt [:eval-result :value])))
    (assert (= expected (get-in receipt [:clj-meta-result :value])))
    (assert (= expected (get-in receipt [:px-runtime :value])))))

(doseq [{:keys [source reason]} held-cases]
  ;; Error cases only need the real evaluator reason here. Some held edges
  ;; intentionally trigger expensive lane failure paths; examples should stay
  ;; short-running while tests keep the full matrix pinned.
  (let [result (pnix/eval-source source)]
    (println "held:" source "=>" (select-keys result [:status :reason]))
    (assert (= :held (:status result)))
    (assert (= reason (:reason result)))))

(println)
(println "결론: pnix-clj는 D19 pattern lambda를 lazy recursive defaults, ellipsis, @as, application-time error receipt로 고정한다.")
(shutdown-agents)
