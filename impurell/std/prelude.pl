# impurell prelude
#
# Adapted from thepurell's stdlib.pl for a strict (call-by-value) compiler.
# The important difference: Y diverges under CBV, so recursion goes through Z.

# --- Core combinators -------------------------------------------------------

:id    \x.x
:const \x.\y.x
:flip  \f.\x.\y.f y x
:comp  \f.\g.\x.f (g x)
:app   \f.\x.f x

:S \x.\y.\z.x z (y z)
:K \x.\y.x
:I \x.x

# Call-by-value fixpoint combinator. The extra \v. eta-expansion is what stops
# the self-application from being evaluated before it is needed:
#
#   Y = \f.(\x.f (x x)) (\x.f (x x))       -- diverges under CBV
#   Z = \f.(\x.f (\v.x x v)) (\x.f (\v.x x v))
#
# Use it as:  Z (\rec.\n. ... rec ... )
:Z \f.(\x.f (\v.x x v)) (\x.f (\v.x x v))

# --- Booleans ---------------------------------------------------------------
#
# `true` and `false` are runtime primitives, and the comparison operators
# (< > <= >= = /=) return them, so Church booleans and native numbers mix.
#
# CBV caveat: `cond a b` evaluates BOTH branches before choosing. When a branch
# must not run (a recursive call, a division), wrap both in \_. and apply the
# result, which is what `if` below does.

:not \p.p false true
:and \p.\q.p q p
:or  \p.\q.p p q
:xor \p.\q.p (not q) q

# Lazy conditional: if c (\_.then) (\_.else)
:if \c.\t.\e.c t e 0

:zero? \n.= n 0

# --- Pairs ------------------------------------------------------------------

:pair \x.\y.\f.f x y
:fst  \p.p true
:snd  \p.p false

# --- Numeric helpers --------------------------------------------------------

:succ \n.+ n 1
:pred \n.- n 1
:neg  \n.- 0 n
:abs  \n.if (< n 0) (\_.- 0 n) (\_.n)
:max  \a.\b.if (< a b) (\_.b) (\_.a)
:min  \a.\b.if (< a b) (\_.a) (\_.b)

# Apply f to x, n times.
:iter \n.\f.\x.Z (\rec.\k.\acc.if (= k 0) (\_.acc) (\_.rec (- k 1) (f acc))) n x
