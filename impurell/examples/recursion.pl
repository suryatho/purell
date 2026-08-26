@prelude.pl

# Recursion via the CBV fixpoint combinator. `if` keeps the recursive branch
# from being evaluated eagerly.

:fact Z (\rec.\n.if (<= n 1) (\_.1) (\_.* n (rec (- n 1))))

fact 20

:fib Z (\rec.\n.if (< n 2) (\_.n) (\_.+ (rec (- n 1)) (rec (- n 2))))

fib 25

# Accumulator-passing sum, tail-recursive: musttail means this runs in constant
# stack space, so ten million frames is fine.
:sumTo \n.Z (\rec.\k.\acc.if (= k 0) (\_.acc) (\_.rec (- k 1) (+ acc k))) n 0

sumTo 10000000

# gcd, mutual arithmetic on native numbers
:gcd Z (\rec.\a.\b.if (= b 0) (\_.a) (\_.rec b (% a b)))

gcd 462 1071

# Prelude helpers
iter 10 (\x.* x 2) 1

max 3 (min 99 7)
