;;; plain Clojure의 한계 - eval/compile 성공은 classfile artifact hash나 common-mode risk receipt가 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/34-classfile-trust-receipt/limit_clojure.clj

(ns classfile-trust-receipt-limit)

(let [value (eval '(+ 40 2))]
  (println "value:" value)
  (println "missing: generated classfile hashes, bytecode verifier status, dependency pins, trust boundary")
  (assert (= 42 value)))

(println)
(println "결론: plain eval은 JVM artifact identity와 N-version common-mode risk를 receipt로 남기지 않는다.")
