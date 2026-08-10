;;; pnix-clj의 방식 - Nix pattern lambda를 서비스 옵션 contract처럼 사용한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/86-service-option-contract/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.core :as pnix]))

(def strict-ok
  "({ port, tls ? false }: if tls then port + 443 else port) { port = 8080; }")

(def typo
  "({ port, tls ? false }: port) { prt = 8080; }")

(def forward-compatible
  "({ port, ... }: port) { port = 8080; debug = true; }")

(let [ok (pnix/eval-source strict-ok)
      bad (pnix/eval-source typo)
      compat (pnix/eval-source forward-compatible)]
  (println "strict ok:" (select-keys ok [:status :value :reason]))
  (println "typo held:" (select-keys bad [:status :value :reason]))
  (println "ellipsis ok:" (select-keys compat [:status :value :reason]))

  (assert (= :ok (:status ok)))
  (assert (= 8080 (:value ok)))
  (assert (= :held (:status bad)))
  (assert (= :missing-lambda-pattern-arg (:reason bad)))
  (assert (= :ok (:status compat)))
  (assert (= 8080 (:value compat))))

(println)
(println "결론: pnix-clj pattern lambda는 옵션 contract로 쓰기 좋다. 엄격 모드와 forward-compatible ellipsis를 코드에서 선택할 수 있다.")
(shutdown-agents)
