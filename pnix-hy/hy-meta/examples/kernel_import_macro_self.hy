(defmacro import-self-bar [expr]
  `(+ ~expr 10))

(defmacro import-self-foo [expr]
  `(do
     (require kernel_import_macro_self [import-self-bar])
     (import-self-bar ~expr)))

(setv IMPORT_SELF_VALUE (import-self-foo 32))
