use std::path::Path;

pub struct WorkspacePath;

impl WorkspacePath {
    /// Generates a stable, filesystem-safe ID for a source folder.
    pub fn folder_id(path: &Path) -> String {
        let name = Self::sanitize_name(path);
        let hash = Self::hash_path(path);
        format!("{}_{:x}", if name.is_empty() { "folder" } else { &name }, hash)
    }

    fn sanitize_name(path: &Path) -> String {
        path.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("folder")
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>()
    }

    fn hash_path(path: &Path) -> u64 {
        use xxhash_rust::xxh3::xxh3_64;
        let normalized = Self::normalize(path);
        xxh3_64(normalized.as_bytes())
    }

    /// Normalizes a path for consistent database storage and comparison.
    /// Handles Windows UNC paths (strips \\?\) and ensures forward slashes.
    pub fn normalize(path: &Path) -> String {
        let simplified = dunce::simplified(path);
        simplified.to_string_lossy().to_lowercase().replace("\\", "/")
    }

    /// Checks if a file is a supported image format.
    pub fn is_image(path: &Path) -> bool {
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_lowercase();
        matches!(
            ext.as_str(),
            "jpg" | "jpeg" | "png" | "webp" | "gif" | "bmp" | "tiff"
        )
    }
}
