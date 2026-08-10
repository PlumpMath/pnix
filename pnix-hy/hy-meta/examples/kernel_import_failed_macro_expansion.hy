(defmacro expansion-before-failure []
  '41)

(defmacro expansion-boom []
  (/ 1 0))

(expansion-boom)
