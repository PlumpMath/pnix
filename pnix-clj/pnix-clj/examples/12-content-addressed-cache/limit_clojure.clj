;;; plain Clojure의 한계 — memoize는 '표현' 기반이라 같은 뜻도 포맷이 다르면 미스.
;;;
;;; clojure.core/memoize 는:
;;;   1) 인자(문자열)를 그대로 키로 쓴다 -> "1 + 2" 와 "1 +  2" 를 다른 것으로 본다,
;;;   2) 같은 프로그램의 포맷 변형마다 별도 엔트리 (중복 계산/메모리 낭비),
;;;   3) 캐시 무효화/버전(정확성 유지)을 직접 챙겨야 한다.
;;;
;;; 실행:  cd pnix-clj && clojure -M examples/12-content-addressed-cache/limit_clojure.clj

(ns limit-clojure)

(def calls (atom 0))
(def evaluate
  (memoize (fn [src]
             (swap! calls inc)
             ;; (실제 평가 대신) 계산 횟수만 센다
             (count src))))

(evaluate "1 + 2")     ; miss -> 계산
(evaluate "1 + 2")     ; hit  (같은 문자열)
(evaluate "1 +  2")    ; MISS! 뜻은 같지만 공백이 달라 다른 키
(println "memoize 계산 횟수:" @calls
         "  <- '1 + 2' 와 '1 +  2' 는 같은 뜻인데도 2번 계산됨")
(assert (= 2 @calls))

(println "표현 기반 키라 alpha/포맷 변형에 취약하고, 정본 내용주소가 아니다.")
(println "\n결론: plain memoize는 '같은 뜻이면 한 칸'을 보장하지 못한다.")
