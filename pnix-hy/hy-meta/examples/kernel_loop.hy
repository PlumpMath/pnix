(setv n 5)
(setv acc 1)

(while (> n 1)
  (setv acc (* acc n))
  (setv n (- n 1)))

acc
