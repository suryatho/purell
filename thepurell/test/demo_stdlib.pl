# Demonstrating stdlib include and useful combinators
@stdmath.pl

# Test Church numeral successor
# succ applied to zero should give one
succ zero

# Test Church numeral arithmetic
# This creates (add one two) which should reduce to three when applied
add one two

# Test pair/tuple creation and access
# Create a pair with true and false
pair true false

# Extract first element from pair (should be true)
lhs (pair true false)

# Extract second element (should be false)
rhs (pair true false)

