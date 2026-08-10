;;; plain Clojure의 한계 - pr-str/read-string은 pnix parser/unparser roundtrip이 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/57-parser-unparse-roundtrip/limit_clojure.clj

(ns parser-unparse-roundtrip-limit)

(def clj-data
  '(let [x 40] (+ x 2)))

(def rendered
  (pr-str clj-data))

(def reread
  (read-string rendered))

(println "Clojure rendered:" rendered)
(println "reread equal?:" (= clj-data reread))
(println "pnix precedence/AST metadata checked?:" false)

(assert (= clj-data reread))

(println)
(println "결론: Clojure data roundtrip은 pnix source parser/unparser 구조 동치가 아니다.")

