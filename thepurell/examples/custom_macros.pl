# Custom Macros Example
# Defining your own reusable lambda expressions

# Define identity
:id \x.x

# Define constant function
:K \x.\y.x

# Define a twice function
:twice \f.\x.f (f x)

# Use the macros
id (\a.a)

K (\x.x) (\y.y)

# Apply a function twice
twice (\n.\f.\x.f (n f x)) (\f.\x.x)

# Define and use church numeral 5
:five \f.\x.f (f (f (f (f x))))

# Add custom macros that reference each other
:double \n.\f.\x.n f (n f x)

double five
