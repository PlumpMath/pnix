;;; plain Clojure의 한계: 실행 결과와 예외는 있지만 gate/witness가 없다.
;;;
;;; 실행:
;;;   cd pnix-clj
;;;   clojure -M examples/05-witness-and-gate/limit_clojure.clj

(ns limit-clojure)

(let [value (eval '(+ 1 2 3))]
  (println "plain eval:" value)
  (assert (= 6 value)))

(try
  (eval '(/ 1 0))
  (catch Throwable t
    (println "plain exception:" (.getSimpleName (class t)))
    (assert t)))

(let [dangerous '(System/getenv "HOME")]
  (println "plain form exists, but no policy verdict:" dangerous)
  (assert (seq? dangerous)))

(println)
(println "결론: plain Clojure는 값/예외를 주지만 accepted/held/rejected gate와 witness hash를 기본으로 주지 않는다.")
