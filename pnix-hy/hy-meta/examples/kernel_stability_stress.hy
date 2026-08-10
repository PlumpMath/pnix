"Kernel generated-output stability stress module."

(import contextlib [nullcontext])

(defmacro twice [x]
  `(+ ~x ~x))

(setv events [])
(setv try-value
      (try
        (raise (ValueError "stress"))
        (except [ValueError]
          (twice 21))))
(setv with-value
      (with [x (nullcontext 40)]
        (setv inner 2)
        (+ x inner)))
(setv comp-value
      (sum (lfor x [1 2 3]
              :do (.append events x)
              :if (> x 1)
              x)))
(setv let-value
      (let [a 10
            b (do
                (.append events 4)
                32)]
        (+ a b)))
(setv match-value
      (match [10 32]
        [a b] :if (do
                    (.append events 5)
                    (= (+ a b) 42))
          (+ a b)
        _ 0))
(setv call-value (+ #* [1 2 3] #* [4]))
(setv unpacked-dict
      (dfor pair [["a" 10] ["b" 20]]
        #** {(get pair 0) (get pair 1)}))

(+ try-value
   with-value
   comp-value
   let-value
   match-value
   call-value
   (len events)
   (get unpacked-dict "a")
   (get unpacked-dict "b"))
