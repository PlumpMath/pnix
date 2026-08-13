(ns demo.lib
  "Sample library namespace for the multi-file CLR project template.")

(defn add
  "Admitted host-language Clojure on the bootstrap substrate (not pnix-clr guest)."
  [a b]
  (+ a b))
