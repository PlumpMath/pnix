;;; pnix-clj의 방식: source -> AST -> lowering -> clj-meta host proof lane.
;;;
;;; 실행:
;;;   cd pnix-clj
;;;   clojure -M examples/06-ast-lowering-roundtrip/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.parser :as parser]
            [pnix-clj.hash :as hash]
            [pnix-clj.lowering :as lowering]
            [pnix-clj.clj-meta :as clj-meta]
            [pnix-clj.core :as pnix]))

(def source "1 + 2 * 3")

(let [parsed (parser/parse-source source)
      ast (:ast parsed)
      ast-hash (hash/data-hash ast)
      lowered (lowering/lower-ast ast)
      host-result (clj-meta/eval-lowered (:form lowered))
      direct (pnix/eval-source source)]
  (println "source:" source)
  (println "parse status:" (:status parsed))
  (println "ast op:" (:op ast))
  (println "ast hash:" ast-hash)
  (println "lowering status:" (:status lowered))
  (println "lowered form hash:" (:form-hash lowered))
  (println "direct eval:" (:status direct) (:value direct))
  (println "clj-meta eval:" (:status host-result) (:value host-result))
  (println "compile receipt status:" (get-in host-result [:compile-receipt :determinism :status]))

  (assert (= :ok (:status parsed)))
  (assert (string? ast-hash))
  (assert (= :ok (:status lowered)))
  (assert (string? (:form-hash lowered)))
  (assert (= :ok (:status direct)))
  (assert (= :ok (:status host-result)))
  (assert (= (:value direct) (:value host-result)))
  (assert (= 7 (:value host-result))))

(println)
(println "결론: pnix-clj는 값뿐 아니라 AST identity, lowering identity, clj-meta host proof result까지 함께 남긴다.")
