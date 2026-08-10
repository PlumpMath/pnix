;;; plain Clojure의 한계 — 일부 입력에 특화된 '잔여 프로그램'을 뽑을 수 없다.
;;;
;;; partial/클로저는 런타임 값일 뿐, 특화된 소스/AST를 주지 않는다:
;;;   1) 잔여 프로그램의 소스/구조를 볼 수 없다,
;;;   2) 특화가 원본과 같은 의미인지 증거가 없다,
;;;   3) 잔여를 컴파일 결과(bytecode)로 투영한다는 개념 자체가 없다.
;;;
;;; 실행:  cd pnix-clj && clojure -M examples/03-specialization-futamura/limit_clojure.clj

(ns limit-clojure)

;; partial 은 "특화된 클로저"를 주지만, 잔여 '소스'는 없다.
(def add (fn [x a] (+ x a)))
(def specialized (partial add 40))   ; a=... 가 아니라 x=40 을 고정한 클로저
(println "partial 결과값:" (specialized 2))       ; => 42
(println "잔여 소스는?" (pr-str specialized))      ; => #object[...] — 소스가 아니다

;; 잔여 프로그램의 '구조'를 볼 수 없다: 그냥 함수 객체.
(println "특화된 프로그램의 AST/소스를 표준으로 꺼낼 수 없다 (함수 객체뿐).")

;; 특화가 의미를 보존하는지(원본 == 잔여) 자동 증거도 없다.
(println "특화 == 원본 임을 증명하는 표준 수단이 없다 (수동 테스트뿐).")

(println "\n결론: plain Clojure는 '부분평가로 잔여 소스를 만들고 의미보존을 증명'하지 못한다.")
