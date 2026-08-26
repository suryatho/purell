# Combinator Examples
@stdlib.pl

# S combinator: S x y z = x z (y z)
S id id one

# K combinator (constant): K x y = x
K one two

# I combinator (identity): I x = x
I one

# Function composition: comp f g x = f(g(x))
comp succ succ zero

# Flip arguments: flip f x y = f y x
flip const one two

# Self application
app id one
