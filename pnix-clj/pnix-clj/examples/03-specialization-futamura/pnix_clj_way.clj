;;; pnix-clj의 방식 — specialize = Futamura 1차 사영 (잔여 소스 + JVM bytecode 투영).
;;;
;;; specialize 는 statics(정적으로 아는 입력)를 접어 잔여 pnix 소스를 만든다. 건전성 우선:
;;; 부분 fold로 의미가 어긋나면 접지 않고 gap으로 남긴다. specialize-to-host 는 잔여를
;;; lowering->clj-meta 로 bytecode 컴파일해 동적값에 적용, 원본과 값 일치를 증명한다.
;;;
;;; 실행:  cd pnix-clj && clojure -M examples/03-specialization-futamura/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.specialize :as sp]))

;; 1) statics를 접어 '잔여 소스'를 만든다 (fully-static면 상수로 접힘).
(let [r (sp/specialize "let x = 40; in x + a" {"a" 2})]
  (println "잔여 소스:" (pr-str (:residual-source r))
           "| fully-static?:" (:fully-static? r))
  (assert (and (= :ok (:status r)) (= "42" (:residual-source r)))))

;; 2) 동적 부분이 남으면 gap으로 기록하고, 동적 구조를 잔여에 유지한다.
(let [r (sp/specialize "let x = 40; in x + a" {})]  ; a를 모름 -> 동적
  (println "동적 잔여:" (pr-str (:residual-source r))
           "| gaps:" (count (:gaps r)))
  (assert (= :ok (:status r))))

;; 3) Futamura 투영: 잔여를 JVM bytecode로 컴파일해 동적값에 적용, 원본과 값 일치.
(let [r (sp/specialize-to-host "let x = 40; in x + a" {} {"a" 2})]
  (println "bytecode 투영:" (:status r)
           "| invoked value:" (:value (:invoked r))
           "| bytecode 결정성:" (:bytecode-determinism r))
  (println "  wrapper-source:" (pr-str (:wrapper-source r)))
  (assert (and (= :ok (:status r))
               (= 42 (:value (:invoked r)))
               (= :ok (:bytecode-determinism r)))))

;; 4) 건전성: non-bool static if 는 접지 않고 gap으로 (의미 왜곡 금지).
(let [r (sp/specialize "if x then 1 else 2" {"x" 5})]  ; x는 bool이 아님
  (println "건전성(non-bool if):" (:status r) "| gaps:" (count (:gaps r)))
  (assert (pos? (count (:gaps r)))))

(println "\n결론: 인터프리터+알려진 입력 = 특화된 잔여 프로그램(+bytecode), 의미보존과 함께.")
