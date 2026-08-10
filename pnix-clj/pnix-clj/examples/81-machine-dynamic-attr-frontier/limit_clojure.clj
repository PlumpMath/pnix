;;; plain Clojure의 한계 - dynamic key를 느슨하게 흉내 내면 collision semantics가 흐려진다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/81-machine-dynamic-attr-frontier/limit_clojure.clj

(ns machine-dynamic-attr-frontier-limit)

(defn loose-attrset
  [pairs]
  ;; dynamic key를 모두 string으로 보고 reduce/assoc 한다. 충돌은 조용히 overwrite된다.
  (reduce (fn [m [k v]] (assoc m (str k) v)) {} pairs))

(def ok-ish
  (loose-attrset [["x" 1]]))

(def overwritten
  (loose-attrset [["a" 1] ["a" 2]]))

(println "loose dynamic key value:" (get ok-ish "x"))
(println "loose collision overwrite:" (get overwritten "a"))
(println "duplicate held reason?:" false)

(assert (= 1 (get ok-ish "x")))
(assert (= 2 (get overwritten "a")))

(println)
(println "결론: plain Clojure 흉내는 dynamic key를 실행해버리거나 덮어써서, D20/M7e의 duplicate/non-string reason을 남기지 않는다.")
