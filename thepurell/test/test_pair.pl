@stdlib.pl

# Simple pair test
pair one two

# Apply lhs to get first element
lhs (pair one two)

# Direct application
(\p.p true) (pair one two)
