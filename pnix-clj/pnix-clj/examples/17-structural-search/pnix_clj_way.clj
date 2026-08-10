;;; pnix-clj의 방식 - search가 content hash, skeleton, free-vars,
;;; structural distance, event index를 분리해 후보를 제안한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/17-structural-search/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.cas :as cas]
            [pnix-clj.parser :as parser]
            [pnix-clj.search :as search]
            [pnix-clj.store :as store]))

(defn ast [source]
  (:ast (parser/parse-source source)))

(let [a (ast "1 + 2")
      b (ast "3 + 4")
      c (ast "x: x + y")
      similar (search/similar-terms a [b (ast "1 * 2") a] 0.5)
      log (store/open-store)]
  (store/append! log :eval/run {:source-hash "s1" :result-hash "r1"})
  (store/append! log :eval/run {:source-hash "s2" :result-hash "r2"})

  (println "same skeleton 1+2 / 3+4?:" (= (search/skeleton a) (search/skeleton b)))
  (println "confirmed equivalent?:" (cas/structurally-equivalent? a b))
  (println "free vars in x: x + y:" (search/free-vars c))
  (println "open summary:" (search/open-term-summary c))
  (println "similar candidates:" similar)
  (println "events s1:" (search/search-events log :eval/run :source-hash "s1"))

  (assert (= true (= (search/skeleton a) (search/skeleton b))))
  (assert (= false (cas/structurally-equivalent? a b)))
  (assert (= #{"y"} (search/free-vars c)))
  ;; structural-distance는 op histogram 기반 heuristic이라 binary operator가
  ;; 달라도 후보가 될 수 있다. proof는 confirmed-equivalent?가 따로 맡는다.
  (assert (= 3 (count similar)))
  (assert (= 2 (count (filter :same-skeleton? similar))))
  (assert (= 1 (count (filter :confirmed-equivalent? similar))))
  (assert (= 1 (count (search/search-events log :eval/run :source-hash "s1")))))

(println)
(println "결론: pnix-clj search는 similarity를 proof로 착각하지 않고 후보와 confirmation을 분리한다.")
