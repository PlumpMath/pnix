;;; pnix-clj의 방식: 실행은 gate verdict와 witness를 가진다.
;;;
;;; 실행:
;;;   cd pnix-clj
;;;   clojure -M examples/05-witness-and-gate/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.safe-eval :as safe]
            [pnix-clj.interop :as interop]))

(let [ok (safe/safe-eval "1 + 2 + 3")]
  (println "safe pure eval:" (:status ok) (:value ok))
  (assert (= :ok (:status ok)))
  (assert (= 6 (:value ok))))

(let [impure (safe/safe-eval "builtins.getEnv \"HOME\"" {:pure-only? true})]
  (println "safe impure verdict:" (:status impure) (:limit-exceeded impure) (:reason impure))
  (assert (= :held (:status impure)))
  (assert (= :impure (:limit-exceeded impure))))

(let [bounded (safe/safe-eval "let f = x: f x; in f 1" {:fuel 50000})]
  (println "safe bounded verdict:" (:status bounded) (:reason bounded))
  (assert (= :held (:status bounded))))

(let [denied (interop/host-eval-form :denied-host-eval
                                      '(+ 20 22)
                                      interop/default-capabilities)]
  (println "denied host eval:" (:status denied) (:reason denied))
  (println "denied witness:" (get-in denied [:witness :witness-hash]))
  (assert (= :held (:status denied)))
  (assert (string? (get-in denied [:witness :witness-hash]))))

(let [allowed (interop/host-eval-form :allowed-host-eval
                                       '(+ 20 22)
                                       interop/host-eval-capabilities)]
  (println "allowed host eval:" (:status allowed) (:value allowed))
  (println "allowed witness:" (get-in allowed [:witness :witness-hash]))
  (assert (= :ok (:status allowed)))
  (assert (= 42 (:value allowed)))
  (assert (string? (get-in allowed [:witness :witness-hash]))))

(println)
(println "결론: pnix-clj는 실행을 값만이 아니라 gate verdict와 witness evidence로 다룬다.")
