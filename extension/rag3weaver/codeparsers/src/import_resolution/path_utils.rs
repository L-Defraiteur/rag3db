use std::path::Path;

/// Check if a path/import is local (relative or absolute file path)
///
/// # Examples
/// - `is_local_path("./utils")` → true (relative)
/// - `is_local_path("../lib")` → true (relative parent)
/// - `is_local_path("/home/user/lib")` → true (absolute Unix)
/// - `is_local_path("lodash")` → false (package)
/// - `is_local_path("@scope/package")` → false (scoped package)

pub fn is_local_path(p: &str) -> bool {
    p.starts_with('.') || Path::new(p).is_absolute()
}

/// Check if a path looks like a relative path (starts with . or ..)

pub fn is_relative_path(p: &str) -> bool {
    p.starts_with("./") || p.starts_with("../") || p == "." || p == ".."
}

/// Normalize path separators to Unix style (forward slashes)

pub fn to_unix_path(p: &str) -> String {
    p.replace('\\', "/")
}
