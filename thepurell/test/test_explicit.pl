# Without macros to see what's happening

# Identity applied
(\x.x) (\f.\x.f x)

# Const applied twice  
((\x.\y.x) (\f.\x.f x)) (\f.\x.f (f x))

# True applied twice
((\x.\y.x) (\f.\x.f x)) (\f.\x.f (f x))
