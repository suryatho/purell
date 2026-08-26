# Lambda Calculus Standard Library
# Common lambda calculus definitions and combinators

# Identity function
:id \x.x

# Constant function (returns first argument, ignores second)
:const \x.\y.x

# Flip/Exchange function (swaps arguments)
:flip \f.\x.\y.f y x

# Composition function (f after g)
:comp \f.\g.\x.f (g x)

# Application function (applies f to x)
:app \f.\x.f x

# Self-application (Y combinator fixed point)
:Y \f.(\x.f (x x)) (\x.f (x x))

# SKI Combinators
# S combinator (stronger composition)
:S \x.\y.\z.x z (y z)

# K combinator (constant, same as const)
:K \x.\y.x

# I combinator (identity)
:I \x.x

# Church Booleans
:true \x.\y.x
:false \x.\y.y

# Boolean operators
:and \p.\q.p q p
:or \p.\q.p p q
:not \p.p false true

# Church Numerals
# 0 = \f.\x.x (apply f zero times)
:zero \f.\x.x

# 1 = \f.\x.f x (apply f once)
:one \f.\x.f x

# 2 = \f.\x.f (f x) (apply f twice)
:two \f.\x.f (f x)

# 3 = \f.\x.f (f (f x))
:three \f.\x.f (f (f x))

# Successor function (increment)
:succ \n.\f.\x.f (n f x)

# Addition
:add \m.\n.\f.\x.m f (n f x)

# Multiplication
:mul \m.\n.\f.m (n f)

# Church pairs/tuples
:pair \x.\y.\f.f x y
:lhs \p.p true
:rhs \p.p false

# List operations
# Empty list
:nil \f.f true

# Is list empty
:isEmpty \l.l (\x.\y.false)
