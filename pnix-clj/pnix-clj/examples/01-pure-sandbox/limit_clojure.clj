;;; plain Clojure의 한계 — eval은 '신뢰 가능한 샌드박스'가 될 수 없다.
;;;
;;; Clojure의 eval/load-string은:
;;;   1) 부작용(파일/네트워크/전역 변경)을 막지 못한다,
;;;   2) 무한 루프/과도한 자원 사용을 막지 못한다,
;;;   3) 실행 전에 "이 코드가 순수한가?"를 정적으로 알 수 없다.
;;; 아래는 (안전한 범위에서) 그 한계를 '증명'한다 — 실제 파괴는 하지 않는다.
;;;
;;; 실행:  cd pnix-clj && clojure -M examples/01-pure-sandbox/limit_clojure.clj

(ns limit-clojure)

;; 1) 부작용을 막을 수 없다: eval된 코드가 바깥 상태를 조용히 바꾼다.
(def side-effects (atom []))
(eval `(swap! side-effects conj "나는 바깥을 건드렸다"))
(println "부작용 발생:" @side-effects)  ; => ["나는 바깥을 건드렸다"]

;; 실제 위험(주석으로만): 아래는 임의 파일 삭제/명령 실행이 '문법적으로' 가능하다.
;;   (eval '(clojure.java.shell/sh "rm" "-rf" (System/getProperty "user.home")))
(println "주의: (eval '(sh \"rm\" \"-rf\" ...)) 도 문법상 허용된다 (여기선 실행 안 함)")

;; 2) 자원(무한 루프/스텝)을 막을 수 없다: eval에는 스텝/시간 한계가 없다.
;;   (eval '(loop [] (recur)))   ; <- 영원히 멈추지 않는다 (실행하지 않음)
(println "주의: eval에는 fuel/step 한계가 없다 -> 무한 루프를 막지 못한다")

;; 3) 실행 '전에' 순수성을 정적으로 판정할 표준 방법이 없다.
(def untrusted "(slurp \"/etc/passwd\")")
(println "이 코드가 순수한지 실행 전에 표준 방법으로 알 수 없다:" (pr-str untrusted))

(println "\n결론: 신뢰할 수 없는 입력을 Clojure eval로 돌리는 것은 안전하지 않다.")
