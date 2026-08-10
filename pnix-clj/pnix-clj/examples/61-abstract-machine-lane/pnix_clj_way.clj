;;; pnix-clj의 방식 - abstract machine lane이 evaluator와 같은 fragment 결과를 낸다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/61-abstract-machine-lane/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.core :as pnix]
            [pnix-clj.machine :as machine]))

(defn comparable
  [r]
  (if (= :ok (:status r))
    [:ok (:value r)]
    [(:status r) (:reason r)]))

(doseq [source ["let f = x: x + 1; in f 41"
                "builtins.length [ 1 2 3 ]"
                "{ a = 1; b = [ 2 3 ]; }.b"
                "assert false; 42"]]
  (let [evaluator (pnix/eval-source source)
        machine (machine/eval-source source)]
    (println "source:" source)
    (println "evaluator:" (comparable evaluator))
    (println "machine:" (comparable machine))
    (assert (= (comparable evaluator)
               (comparable machine)))))

(let [unsupported (machine/eval-source "let k = \"b\"; in { a.${k} = 1; }")]
  (println "unsupported dynamic path binding:"
           (select-keys unsupported [:status :reason]))
  (assert (= :held (:status unsupported)))
  (assert (= :machine-unsupported-op (:reason unsupported))))

(println)
(println "결론: pnix-clj machine lane은 지원 fragment에서 evaluator와 수렴하고, 밖에서는 추측하지 않고 held 한다.")
(shutdown-agents)
