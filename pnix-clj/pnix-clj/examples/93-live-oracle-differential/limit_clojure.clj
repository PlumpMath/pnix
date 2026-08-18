;;; 한계 - plain Clojure로 "우리 evaluator가 실제 Nix와 같은 값을 낸다"를
;;; 확인하려면 nix-instantiate를 손으로 셸아웃하고, 없으면 그냥 통과시키고,
;;; 결과 JSON을 손으로 파싱해야 한다 - 구조화된 매치/불일치 report가 없다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/93-live-oracle-differential/limit_clojure.clj

(ns live-oracle-limit)

(defn pretend-checked-against-nix?
  [source]
  ;; 실제로 nix-instantiate를 부르지도, 없을 때를 구조화해서 건너뛰지도 않는다.
  (boolean (seq source)))

(println "pretend-checked-against-nix?:" (pretend-checked-against-nix? "1 + 2"))
(println "nix-instantiate 없을 때 skip 사유가 구조화돼 있나?:" false)
(println "여러 소스에 대한 matched/mismatched 집계 report가 있나?:" false)
(println)
(println "결론: plain Clojure는 실제 Nix와의 차분 비교를 자동화·구조화하지 않는다.")
