# Church Pairs (Tuples) Examples
@stdlib.pl

# Create a pair of Church numerals
pair one two

# Extract left element (returns one)
lhs (pair one two)

# Extract right element (returns two)
rhs (pair one two)

# Pair of booleans
pair true false

# Extract from boolean pair
lhs (pair true false)

rhs (pair true false)

# Nested pairs
pair (pair one two) (pair three zero)
