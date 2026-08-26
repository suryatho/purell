# Standard Math Library
# Church numerals and arithmetic operations
@stdlib.pl

# Church Numerals
# 0 = \f.\x.x (apply f zero times)
:zero \f.\x.x

# 1 = \f.\x.f x (apply f once)
:one \f.\x.f x

# 2 = \f.\x.f (f x) (apply f twice)
:two \f.\x.f (f x)

# 3 = \f.\x.f (f (f x))
:three \f.\x.f (f (f x))

# 4 = \f.\x.f (f (f (f x)))
:four \f.\x.f (f (f (f x)))

# 5 = \f.\x.f (f (f (f (f x))))
:five \f.\x.f (f (f (f (f x))))

# Successor function (increment)
:succ \n.\f.\x.f (n f x)

# Addition
:add \m.\n.\f.\x.m f (n f x)

# Multiplication
:mul \m.\n.\f.m (n f)

# Power/Exponentiation
:pow \m.\n.n m

# Predecessor (decrement) - more complex
:pred \n.\f.\x.n (\g.\h.h (g f)) (\u.x) (\u.u)

# Subtraction (using predecessor)
:sub \m.\n.n pred m

# Boolean comparison predicates
# is zero
:isZero \n.n (\x.false) true

# Less than or equal
:lte \m.\n.isZero (sub m n)

# Greater than or equal
:gte \m.\n.isZero (sub n m)

# Equal
:eq \m.\n.and (lte m n) (gte m n)
