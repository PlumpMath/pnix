;;; pnix-clj의 방식 - D19 pattern application error는 tryEval로 catch되지 않는다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/82-tryeval-pattern-uncatchable/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.core :as pnix]))

(let [caught-throw (pnix/eval-source "(builtins.tryEval (throw \"t\")).success")
      caught-assert (pnix/eval-source "(builtins.tryEval (assert false; 1)).success")
      pattern-extra (pnix/eval-source "(builtins.tryEval (({ a }: a) { a = 1; b = 2; })).success")
      pattern-not-attr (pnix/eval-source "(builtins.tryEval (({ a }: a) 1)).success")]
  (println "throw caught:" (select-keys caught-throw [:status :value :reason]))
  (println "assert caught:" (select-keys caught-assert [:status :value :reason]))
  (println "pattern extra uncatchable:" (select-keys pattern-extra [:status :value :reason]))
  (println "pattern not attrset uncatchable:" (select-keys pattern-not-attr [:status :value :reason]))

  (assert (= :ok (:status caught-throw)))
  (assert (= false (:value caught-throw)))
  (assert (= :ok (:status caught-assert)))
  (assert (= false (:value caught-assert)))

  (assert (= :held (:status pattern-extra)))
  (assert (= :unexpected-lambda-pattern-arg (:reason pattern-extra)))
  (assert (= :held (:status pattern-not-attr)))
  (assert (= :lambda-pattern-arg-not-attrset (:reason pattern-not-attr))))

(println)
(println "결론: pnix-clj tryEval은 throw/assert만 catch하고, D19 pattern application type errors는 real Nix처럼 uncatchable held로 둔다.")
(shutdown-agents)
