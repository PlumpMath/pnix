(ns pnix.clr-meta.compiler-kernel.v1)

(def compile-source
  (fn* compile-source [left right]
    (if left
      right
      0)))
