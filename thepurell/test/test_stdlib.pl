# Test file using the standard library
@stdlib.pl

# Test identity
id (\x.x)

# Test constants with Church numerals
const true false

# Test Church numeral successor
succ zero

# Test addition with Church numerals
add one two

# Test boolean operations
and true false

# Test composition
comp id id
