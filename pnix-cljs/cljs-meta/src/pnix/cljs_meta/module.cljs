(ns pnix.cljs-meta.module
  (:require [pnix.cljs-meta.core :as core]
            [pnix.cljs-meta.fixed-point :as fixed-point]))

(set! (.-exports js/module)
      #js {:compile core/compile-promise
           :compileCompiler fixed-point/compile-compiler-promise
           :evaluate core/evaluate-promise})

(defn -main [& _])
(set! *main-cli-fn* -main)
