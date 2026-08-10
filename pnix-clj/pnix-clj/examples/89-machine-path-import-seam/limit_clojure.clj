;;; plain Clojure의 한계 - path/import를 문자열과 map lookup으로 흉내 내면 의미 경계가 흐려진다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/89-machine-path-import-seam/limit_clojure.clj

(ns machine-path-import-seam-limit)

(def modules
  {"./m" "1 + 2"})

(defn fake-import
  [path]
  ;; 그냥 map에서 문자열을 꺼낸다. path 값인지 import resolver인지 구분이 없다.
  (get modules path))

(println "plain path is just a string:" "./m")
(println "fake imported source:" (fake-import "./m"))
(println "path value receipt?:" false)
(println "unwired import held reason?:" false)

(assert (= "1 + 2" (fake-import "./m")))

(println)
(println "결론: plain Clojure는 path/import를 문자열 규칙으로 흉내 낼 수 있지만, runtime resolver seam과 held reason을 남기지 않는다.")
