;;; pnix-clj의 방식 - CAS가 위치 제거, binder alpha-quotient,
;;; order-independent canonical form을 만든 뒤 content address로 저장한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/13-canonical-term-store/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.cas :as cas]
            [pnix-clj.parser :as parser]))

(defn ast [source]
  (:ast (parser/parse-source source)))

(cas/clear-store!)

(let [a (ast "x: x")
      b (ast "y: y")
      c (ast "{ a = 1; }")
      d (ast "{ z = 1; }")
      first-put (cas/put-term! a)
      second-put (cas/put-term! b)]
  (println "lambda alpha-equivalent?:" (cas/structurally-equivalent? a b))
  (println "lambda hashes:" (cas/term-hash a) (cas/term-hash b))
  (println "store results:" first-put second-put)
  (println "attr labels equivalent?:" (cas/structurally-equivalent? c d))
  (println "term count:" (cas/term-count))

  (assert (= true (cas/structurally-equivalent? a b)))
  (assert (= (cas/term-hash a) (cas/term-hash b)))
  (assert (= :stored (:status first-put)))
  (assert (= :hit (:status second-put)))
  (assert (= false (cas/structurally-equivalent? c d)))
  (assert (= 1 (cas/term-count))))

(println)
(println "결론: pnix-clj CAS는 hash를 proof로 믿지 않고 canonical form과 structural confirmation으로 identity를 다룬다.")
