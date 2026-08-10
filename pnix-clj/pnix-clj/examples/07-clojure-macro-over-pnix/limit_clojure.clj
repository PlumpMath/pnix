;;; plain Clojure의 한계 — macroexpand/eval은 가능하지만,
;;; expanded form의 pnix projection/tower/witness를 기본으로 만들지 않는다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/07-clojure-macro-over-pnix/limit_clojure.clj

(ns clojure-macro-over-pnix-limit)

(defmacro add-two
  [x]
  `(+ ~x 2))

(defmacro choose
  [condition then-expr else-expr]
  `(if ~condition ~then-expr ~else-expr))

(let [form '(add-two 40)
      expanded (macroexpand form)
      value (eval expanded)]
  (println "macro form:" form)
  (println "expanded:" expanded)
  (println "eval value:" value)
  (println "pnix source:" nil)
  (println "tower collapse:" nil)
  (println "tower witness:" nil)
  (assert (= '(clojure.core/+ 40 2) expanded))
  (assert (= 42 value)))

(let [form '(choose true (+ 20 22) 0)
      expanded (macroexpand form)
      value (eval expanded)]
  (println)
  (println "macro form:" form)
  (println "expanded:" expanded)
  (println "eval value:" value)
  (println "pnix source:" nil)
  (println "tower collapse:" nil)
  (println "tower witness:" nil)
  ;; 내부 form의 symbol qualification은 Clojure macroexpand 세부사항이라 고정하지 않는다.
  ;; 여기서는 macroexpand/eval 값은 나오지만 pnix projection/tower/witness는 없다는 점만 본다.
  (assert (= 'if (first expanded)))
  (assert (= 42 value)))

(println)
(println "결론: plain Clojure macroexpand는 값은 만들지만 pnix projection/tower/witness를 기본으로 주지 않는다.")
