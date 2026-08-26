# Closures that outlive the frame that built them.
#
# A global capture stack cannot express any of these: each `add` below carries
# its own captured `x`, and they are alive at the same time.

:makeAdder \x.\y.+ x y

makeAdder 10 5

# Two adders built from different captures, applied in the opposite order.
(\add3.\add100.+ (add100 1) (add3 1)) (makeAdder 3) (makeAdder 100)

# A closure captured inside another closure, three levels deep.
(\a.(\b.(\c.+ a (+ b c)) 3) 2) 1

# Partial application reused twice.
(\f.+ (f 1) (f 2)) (makeAdder 1000)
