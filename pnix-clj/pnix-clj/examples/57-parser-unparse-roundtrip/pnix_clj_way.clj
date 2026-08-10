;;; pnix-clj의 방식 - parse -> unparse -> reparse가 위치 metadata를 제외하고 같은 AST가 된다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/57-parser-unparse-roundtrip/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.parser :as parser]
            [pnix-clj.unparse :as unparse]))

(def sources
  ["let x = 40; in x + 2"
   "{ a = 1; b = [ 2 3 ]; }.b"
   "if 1 < 2 then \"yes\" else \"no\""])

(doseq [source sources]
  (let [ast (:ast (parser/parse-source source))
        rendered (unparse/unparse ast)
        ast2 (:ast (parser/parse-source rendered))]
    (println "source:" source)
    (println "rendered:" rendered)
    (assert (= (unparse/strip-positions ast)
               (unparse/strip-positions ast2)))))

(println)
(println "결론: pnix-clj unparse는 residual/rendered source를 parser structural roundtrip으로 고정한다.")
(shutdown-agents)

