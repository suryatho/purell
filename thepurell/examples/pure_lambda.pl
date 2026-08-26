# Identity function
\x.x

# Apply identity to itself
(\x.x) (\x.x)

# Constant function (K combinator)
\x.\y.x

# Apply constant
(\x.\y.x) (\a.a) (\b.b)

# Self-application
(\x.x x) (\y.y)

# Church numeral 0
\f.\x.x

# Church numeral 1
\f.\x.f x

# Church numeral 2
\f.\x.f (f x)

# Church boolean true
\x.\y.x

# Church boolean false
\x.\y.y

# Apply true to two values
(\x.\y.x) (\a.a) (\b.b)
