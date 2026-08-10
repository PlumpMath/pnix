;;; pnix-clj의 방식 - D20 dynamic attr key collision/type error를 real Nix처럼 held 처리한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/77-dynamic-attr-key-strictness/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.core :as pnix]))

(def held-cases
  [{:source "{ a = 1; \"${\"a\"}\" = 2; }.a"
    :reason :duplicate-attr}
   {:source "{ \"${\"a\"}\" = 1; \"${\"a\"}\" = 2; }.a"
    :reason :duplicate-attr}
   {:source "{ ${\"a\"} = 1; a = 2; }.a"
    :reason :duplicate-attr}
   {:source "{ }.${1} or \"d\""
    :reason :dynamic-attr-key-not-string}])

(def accepted-cases
  [{:source "let k = \"x\"; in { \"${k}\" = 5; }.x"
    :expected 5}
   {:source "{ ${\"x\"} = 1; ${\"y\"} = 2; }.y"
    :expected 2}
   {:source "let s = { a = 1; ${\"a\"} = 2; }; in 1"
    :expected 1}])

(doseq [{:keys [source reason]} held-cases]
  (let [r (pnix/eval-source source)]
    (println "held:" source "=>" (select-keys r [:status :reason]))
    (assert (= :held (:status r)))
    (assert (= reason (:reason r)))))

(doseq [{:keys [source expected]} accepted-cases]
  (let [receipt (pnix/run-source source)]
    (println "accepted:" source "=>" (select-keys receipt [:status :reason])
             (select-keys (:eval-result receipt) [:status :value]))
    (assert (= :accepted (:status receipt)))
    (assert (= :all-lanes-agree (:reason receipt)))
    (assert (= expected (get-in receipt [:eval-result :value])))))

(println)
(println "결론: pnix-clj는 dynamic attr key collision과 non-string key를 silent overwrite/coercion 없이 held reason으로 남긴다.")
(shutdown-agents)
