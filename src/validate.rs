use regex::Regex;
use std::fmt;

pub fn validate_name(name: &str) -> Result<(), InvalidName> {
    let rgx = Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap();
    if !rgx.is_match(name) {
        Err(InvalidName { name: name.into() })
    } else {
        Ok(())
    }
}

#[derive(Debug)]
pub struct InvalidName {
    pub name: String,
}

impl fmt::Display for InvalidName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&format!("\"{}\". {}", self.name, self))
    }
}

pub fn validate_path(path: &str) -> Result<(), InvalidPath> {
    let rgx = Regex::new(r"^[a-zA-Z0-9/*._-]+$").unwrap();
    if !rgx.is_match(path) {
        Err(InvalidPath { path: path.into() })
    } else {
        Ok(())
    }
}

#[derive(Debug)]
pub struct InvalidPath {
    path: String,
}

impl fmt::Display for InvalidPath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&format!("\"{}\". {}", self.path, self.to_string()))
    }
}
