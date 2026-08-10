;;; pnix-clj의 방식 - whitelisted Clojure expression form을 pnix source로 합성하고,
;;; tower collapse로 의미보존을 확인한다. 비대상 host form은 held로 남긴다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/10-reverse-synthesis/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.synthesize :as synth]
            [pnix-clj.tower :as tower]))

(def form
  '(+ 1 (* 2 3)))

(let [projected (synth/form->pnix form)
      climbed (tower/run-tower (:source projected))
      held (synth/form->pnix '(.length "abc"))]
  (println "form:" form)
  (println "projected:" (:source projected))
  (println "tower collapse:"
           (get-in climbed [:collapse :status])
           "value=" (get-in climbed [:collapse :value]))
  (println "host interop held:" (select-keys held [:status :reason :offending-form]))

  (assert (= :ok (:status projected)))
  (assert (string? (:source projected)))
  (assert (= :collapsed (get-in climbed [:collapse :status])))
  (assert (= 7 (get-in climbed [:collapse :value])))
  (assert (= :held (:status held)))
  (assert (= :non-projectable-form (:reason held))))

(println)
(println "결론: pnix-clj synthesize는 허용된 form만 pnix로 투영하고, 나머지는 정직하게 held로 둔다.")
