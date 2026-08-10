(defreader leakreader "reader leak")

(setv READER_VALUE #leakreader)

(raise (RuntimeError "kernel import reader broken"))
