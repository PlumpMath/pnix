;;; pnix-clj의 방식 - version/math helper를 Nix-style runtime surface로 제공한다.
;;;
;;; 실행: cd pnix-clj && clojure -M examples/65-version-math-helpers/pnix_clj_way.clj

(ns pnix-clj-way
  (:require [pnix-clj.math :as math]
            [pnix-clj.version :as version]))

(let [pre-vs-final (version/compare-versions "1.0pre" "1.0")
      numeric-order (version/compare-versions "10" "2")
      drv (version/parse-drv-name "hello-1.2.3")
      int-div (math/div 5 2)
      float-div (math/div 5.0 2)]
  (println "current-system:" version/current-system)
  (println "nix-version:" version/nix-version)
  (println "compare pre/final:" pre-vs-final)
  (println "compare numeric:" numeric-order)
  (println "drv:" drv)
  (println "div:" int-div float-div)

  (assert (neg? pre-vs-final))
  (assert (pos? numeric-order))
  (assert (= {"name" "hello" "version" "1.2.3"} drv))
  (assert (= 2 int-div))
  (assert (= 2.5 float-div)))

(println)
(println "결론: pnix-clj version/math helper는 evaluator builtins가 기대하는 작은 Nix-style surface를 분리한다.")
(shutdown-agents)

