use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const COMMENT_CHAR: char = '#';
const MACRO_CHAR: char = ':';
const INCLUDE_CHAR: char = '@';

/// Preprocessor state for handling macros and includes
pub struct Preprocessor {
    macros: HashMap<String, String>,
    include_paths: Vec<PathBuf>,
}

/// An expression with both original and expanded forms
#[derive(Debug, Clone)]
pub struct ProcessedExpr {
    pub original: String,
}

impl Preprocessor {
    /// Create a new preprocessor
    pub fn new() -> Self {
        Preprocessor {
            macros: HashMap::new(),
            include_paths: Vec::new(),
        }
    }

    /// Create a preprocessor with default std library paths
    pub fn with_stdlib() -> Self {
        let mut p = Preprocessor {
            macros: HashMap::new(),
            include_paths: Vec::new(),
        };
        p.add_default_paths();
        p
    }

    /// Add default library paths (std folder and fallbacks)
    fn add_default_paths(&mut self) {
        if let Ok(cwd) = std::env::current_dir() {
            // Add std subfolder
            self.include_paths.push(cwd.join("std"));

            // Add current directory
            self.include_paths.push(cwd.clone());

            // Try parent directories
            let mut parent = cwd.as_path();
            for _ in 0..3 {
                if let Some(p) = parent.parent() {
                    self.include_paths.push(p.join("std"));
                    self.include_paths.push(p.to_path_buf());
                    parent = p;
                }
            }
        }

        // Add executable directory
        if let Ok(exe_path) = std::env::current_exe()
            && let Some(exe_dir) = exe_path.parent()
        {
            self.include_paths.push(exe_dir.join("std"));
            self.include_paths.push(exe_dir.to_path_buf());
        }
    }

    /// Parse a macro definition line like ":name body"
    fn parse_macro(&mut self, line: &str) -> Result<(), String> {
        let line = line.trim();
        if !line.starts_with(MACRO_CHAR) {
            return Err("Macro definition must start with ':'".to_string());
        }

        let content = &line[1..];
        let parts: Vec<&str> = content.splitn(2, ' ').collect();

        if parts.len() < 2 {
            return Err("Macro definition must have format: :name body".to_string());
        }

        let name = parts[0].trim().to_string();
        let body = parts[1].trim().to_string();

        if name.is_empty() || body.is_empty() {
            return Err("Macro name and body cannot be empty".to_string());
        }

        // Store raw body for lazy macro expansion during evaluation
        self.macros.insert(name.clone(), body);
        Ok(())
    }

    /// Get the expansion of a specific macro
    pub fn get_macro_expansion(&self, name: &str) -> Option<String> {
        self.macros.get(name).cloned()
    }

    /// Split expressions with a base directory for resolving includes
    pub fn split_expressions_with_base(
        &mut self,
        contents: &str,
        base_dir: &Path,
    ) -> Result<Vec<ProcessedExpr>, String> {
        self.split_expressions_internal(contents, base_dir, &mut vec![])
    }

    /// Internal recursive implementation with cycle detection
    fn split_expressions_internal(
        &mut self,
        contents: &str,
        base_dir: &Path,
        seen_files: &mut Vec<String>,
    ) -> Result<Vec<ProcessedExpr>, String> {
        let mut exprs = Vec::new();
        let mut current_expr = String::new();

        for line in contents.lines() {
            let trimmed = line.trim();

            // Skip empty lines
            if trimmed.is_empty() {
                if !current_expr.trim().is_empty() {
                    exprs.push(ProcessedExpr {
                        original: current_expr.clone(),
                    });
                    current_expr.clear();
                }
                continue;
            }

            // Skip comment lines
            if trimmed.starts_with(COMMENT_CHAR) {
                continue;
            }

            // Handle includes (@filename.pl)
            if trimmed.starts_with(INCLUDE_CHAR) {
                if !current_expr.trim().is_empty() {
                    exprs.push(ProcessedExpr {
                        original: current_expr.clone(),
                    });
                    current_expr.clear();
                }
                let include_file = trimmed[1..].trim();
                if !include_file.ends_with(".pl") {
                    return Err(format!(
                        "Include file must have .pl extension: {}",
                        include_file
                    ));
                }

                // Try to resolve include path
                // 1. First try relative to base directory
                let mut include_path = base_dir.join(include_file);

                // 2. If not found, try include search paths
                if !include_path.exists() {
                    let mut found = false;
                    for search_path in &self.include_paths {
                        let candidate = search_path.join(include_file);
                        if candidate.exists() {
                            include_path = candidate;
                            found = true;
                            break;
                        }
                    }
                    if !found && !include_path.exists() {
                        return Err(format!("Cannot find include file: {}", include_file));
                    }
                }

                let include_path_str = include_path.to_string_lossy().to_string();

                if seen_files.contains(&include_path_str) {
                    return Err(format!("Circular include detected: {}", include_file));
                }

                let included_content = fs::read_to_string(&include_path)
                    .map_err(|e| format!("Cannot read include file {}: {}", include_file, e))?;

                // Get the directory of the included file for nested includes
                let include_base_dir = include_path.parent().unwrap_or(Path::new("."));

                seen_files.push(include_path_str);
                let included_exprs = self.split_expressions_internal(
                    &included_content,
                    include_base_dir,
                    seen_files,
                )?;
                exprs.extend(included_exprs);
                seen_files.pop();
                continue;
            }

            // Handle macro definitions
            if trimmed.starts_with(MACRO_CHAR) {
                self.parse_macro(trimmed)?;
                continue;
            }

            // Accumulate expression lines
            if !current_expr.trim().is_empty() {
                current_expr.push(' ');
            }
            current_expr.push_str(trimmed);
        }

        // Add remaining expression
        if !current_expr.trim().is_empty() {
            exprs.push(ProcessedExpr {
                original: current_expr,
            });
        }

        Ok(exprs)
    }
}

impl Default for Preprocessor {
    fn default() -> Self {
        Self::new()
    }
}
