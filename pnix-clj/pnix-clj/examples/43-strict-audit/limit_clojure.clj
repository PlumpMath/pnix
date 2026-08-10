;;; plain Clojure의 한계 - truthiness는 strict Nix boolean typing audit가 아니다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/43-strict-audit/limit_clojure.clj

(ns strict-audit-limit)

(def result
  (if 0 :then :else))

(println "Clojure if with 0:" result)
(println "strict Nix boolean frontier?:" false)

(assert (= :then result))

(println)
(println "결론: plain Clojure truthiness는 stricter Nix typing에서 막힐 frontier를 알려주지 않는다.")

