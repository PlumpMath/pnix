;;; pnix-clj의 방식 - run-source receipt에서 mirror facet rows를 읽는다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/72-mirror-facet-rows/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.core :as pnix]))

(let [receipt (pnix/run-source {:source-id :example/mirror
                                :source "1 + 2"})]
  (println "top:" (select-keys receipt [:status :reason]))
  (println "clojure mirror:" (select-keys (:clojure-mirror receipt)
                                          [:kind :clj-meta-status :stage15-control-status]))
  (println "px runtime:" (select-keys (:px-runtime receipt)
                                      [:kind :status :reason :value]))
  (println "pnix mirror:" (select-keys (:pnix-mirror receipt)
                                      [:kind :status :reason :value]))
  (println "cross:" (select-keys (:cross-mirror-verdict receipt)
                                 [:kind :status :reason :equivalence]))

  (assert (= :accepted (:status receipt)))
  (assert (= :clojure-mirror (get-in receipt [:clojure-mirror :kind])))
  (assert (= :ok (get-in receipt [:px-runtime :status])))
  (assert (= :ok (get-in receipt [:pnix-mirror :status])))
  (assert (= :agree (get-in receipt [:cross-mirror-verdict :equivalence]))))

(println)
(println "결론: pnix-clj receipt는 mirror agreement를 facet row 단위로 디버그할 수 있게 한다.")
(shutdown-agents)

