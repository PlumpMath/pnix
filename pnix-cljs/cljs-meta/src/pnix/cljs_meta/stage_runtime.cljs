(ns pnix.cljs-meta.stage-runtime
  (:require [pnix.cljs-meta.fixed-point :as fixed-point]))

(set! (.-exports js/module)
      #js {:compile fixed-point/compile-promise
           :compileCompiler fixed-point/compile-compiler-promise
           :evaluate fixed-point/evaluate-promise})

(defn -main [& _])
(set! *main-cli-fn* -main)
