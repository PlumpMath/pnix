;;; plain Clojure의 한계 - text search는 term shape/free-vars/event search를 구분하지 못한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/17-structural-search/limit_clojure.clj

(ns structural-search-limit)

(def corpus ["1 + 2" "3 + 4" "x: x + y"])

(println "text hits for '+':" (filter #(clojure.string/includes? % "+") corpus))
(println "missing: anonymous skeleton, free-vars, structural distance, event index")

(assert (= 3 (count (filter #(clojure.string/includes? % "+") corpus))))

(println)
(println "결론: 문자열 검색은 비슷한 구조 후보와 의미상 같은 term을 구분하지 못한다.")
