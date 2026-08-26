# Define macros
:id \x.x
:const \x.\y.x

# Use macros
id id

# Apply const to itself
const (\x.x) (\y.y)
