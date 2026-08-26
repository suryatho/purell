import sys

with open("/Users/surya/Developer/Rust/purell/thepurell/src/repl.rs", "r") as f:
    lines = f.readlines()

# Find line 45 "return Ok(());" in load_stdlib_macros and replace the whole function
output = []
i = 0
while i < len(lines):
    if "fn load_stdlib_macros" in lines[i]:
        # Found it, replace until next function
        output.append(lines[i])  # Add the fn line
        i += 1
        # Add new implementation
        new_impl = '''        for path_str in &["std/stdlib.pl", "stdlib.pl"] {
            if let Ok(contents) = std::fs::read_to_string(path_str) {
                let base_dir = Path::new(path_str).parent().unwrap_or(Path::new("."));
                let _ = preprocessor.split_expressions_with_base(&contents, base_dir);
                for stdmath in &["std/stdmath.pl", "stdmath.pl"] {
                    if let Ok(c) = std::fs::read_to_string(stdmath) {
                        let d = Path::new(stdmath).parent().unwrap_or(Path::new("."));
                        let _ = preprocessor.split_expressions_with_base(&c, d);
                    }
                }
                return Ok(());
            }
        }
        Err("stdlib.pl not found".to_string())
    }
'''
        output.append(new_impl)
        # Skip old implementation
        while i < len(lines) and "}" not in lines[i]:
            i += 1
        if i < len(lines):
            i += 1  # Skip the closing brace
    else:
        output.append(lines[i])
        i += 1

with open("/Users/surya/Developer/Rust/purell/thepurell/src/repl.rs", "w") as f:
    f.writelines(output)

print("Updated!")
