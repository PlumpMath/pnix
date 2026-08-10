;;; pnix-clj의 방식 - refactor/format 변경에도 canonical AST content-address cache가 hit 된다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/88-refactor-cache-stability/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.cached-eval :as ce]))

(ce/clear-eval-cache!)

(let [original (ce/cached-eval "1 + 2")
      formatted (ce/cached-eval " 1   +   2 ")
      parenthesized (ce/cached-eval "(1 + 2)")
      stats (ce/eval-cache-stats)]
  (println "original:" (select-keys original [:status :value :cache]))
  (println "formatted:" (select-keys formatted [:status :value :cache]))
  (println "parenthesized:" (select-keys parenthesized [:status :value :cache]))
  (println "stats:" stats)

  (assert (= :miss (get-in original [:cache :status])))
  (assert (= :hit (get-in formatted [:cache :status])))
  (assert (= :hit (get-in parenthesized [:cache :status])))
  (assert (= 3 (:value original) (:value formatted) (:value parenthesized)))
  (assert (= (get-in original [:cache :key :content-hash])
             (get-in formatted [:cache :key :content-hash])
             (get-in parenthesized [:cache :key :content-hash])))
  (assert (= {:hits 2 :misses 1 :bypasses 0 :entries 1} stats)))

(println)
(println "결론: pnix-clj cache는 source string이 아니라 position-stripped AST hash로 잡혀, formatting/refactor noise에 강하다.")
(shutdown-agents)
