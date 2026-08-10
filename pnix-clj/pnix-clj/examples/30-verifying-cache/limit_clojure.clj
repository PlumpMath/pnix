;;; plain Clojure의 한계 — memoize는 인자 기준 캐시이고,
;;; fresh evaluation/purity/content-address verification receipt를 기본으로 주지 않는다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/30-verifying-cache/limit_clojure.clj

(ns verifying-cache-limit)

(def calls
  (atom 0))

(def plain-eval
  (memoize
   (fn [source]
     (swap! calls inc)
     (eval (read-string source)))))

(def source-a
  "(+ 1 2)")

(def source-b
  "( + 1 2)")

(let [a (plain-eval source-a)
      b (plain-eval source-b)]
  (println "source-a:" source-a)
  (println "source-b:" source-b)
  (println "value a:" a)
  (println "value b:" b)
  (println "memoized calls:" @calls)
  (println "same semantic value?:" (= a b))
  (println "content-address key:" nil)
  (println "fresh verification receipt:" nil)
  (println "purity verdict:" nil)
  (assert (= 3 a b))
  ;; memoize key is the source string argument, so these two strings are distinct cache entries.
  (assert (= 2 @calls)))

(println)
(println "결론: plain memoize는 표현 문자열 기준 재사용은 가능하지만 pnix식 verified cache reuse receipt를 기본으로 주지 않는다.")
