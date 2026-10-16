//! Maps a file to one of the eight user-facing categories (PRD §11 Tahap 1).

use std::path::Path;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Video,
    Photo,
    Document,
    Application,
    System,
    Temporary,
    Development,
    Other,
}

impl Category {
    pub const ALL: [Category; 8] = [
        Self::Video,
        Self::Photo,
        Self::Document,
        Self::Application,
        Self::System,
        Self::Temporary,
        Self::Development,
        Self::Other,
    ];

    /// Decide from the directory context first, then the extension.
    pub fn of(path: &Path) -> Self {
        if let Some(c) = from_dir_context(path) {
            return c;
        }
        from_extension(path)
    }
}

const TEMP_DIRS: &[&str] = &[
    ".cache",
    "Caches",
    "cache",
    "tmp",
    "Temp",
    "temp",
    ".tmp",
    "Thumbnails",
    "thumbnails",
    "CrashReporter",
    "DiagnosticReports",
    "logs",
    "Logs",
];
const DEV_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    "target",
    ".gradle",
    ".cargo",
    ".npm",
    ".pnpm-store",
    ".yarn",
    ".m2",
    ".nuget",
    "DerivedData",
    "CoreSimulator",
    ".venv",
    "venv",
    "__pycache__",
    ".next",
    ".nuxt",
    ".turbo",
    ".parcel-cache",
    "dist",
    "build",
    "out",
    ".docker",
    "go",
    ".rustup",
    ".pub-cache",
    ".android",
    "Pods",
    ".idea",
    ".vscode",
];
const SYSTEM_SEGMENTS: &[&str] = &[
    "System", "usr", "bin", "sbin", "etc", "var", "private", "Windows", "boot", "lib", "lib64",
    "opt", "proc", "dev",
];
const APP_SEGMENTS: &[&str] = &[
    "Applications",
    "Program Files",
    "Program Files (x86)",
    "Microsoft",
    "snap",
    "flatpak",
];

fn from_dir_context(path: &Path) -> Option<Category> {
    let comps: Vec<&str> = path
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();
    // Skip the file name itself: only directories carry context.
    let dirs = comps.get(..comps.len().saturating_sub(1)).unwrap_or(&[]);

    // Deeper, more specific context wins over broad roots, so walk from the leaf.
    for seg in dirs.iter().rev() {
        if seg.ends_with(".app") {
            return Some(Category::Application);
        }
        if TEMP_DIRS.contains(seg) {
            return Some(Category::Temporary);
        }
        if DEV_DIRS.contains(seg) {
            return Some(Category::Development);
        }
    }
    // Root-level OS and application folders.
    let first = dirs
        .iter()
        .find(|s| !s.is_empty() && !s.ends_with(['/', '\\']) && !s.ends_with(':'))?;
    if APP_SEGMENTS.contains(first) {
        return Some(Category::Application);
    }
    if SYSTEM_SEGMENTS.contains(first) {
        return Some(Category::System);
    }
    if *first == "Library" {
        // /Library (not ~/Library) is system-owned.
        return Some(Category::System);
    }
    None
}

fn from_extension(path: &Path) -> Category {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return Category::Other;
    };
    let ext = ext.to_ascii_lowercase();
    match ext.as_str() {
        "mp4" | "mkv" | "mov" | "avi" | "webm" | "m4v" | "wmv" | "flv" | "mpg" | "mpeg" | "3gp"
        | "m2ts" => Category::Video,
        "jpg" | "jpeg" | "png" | "heic" | "heif" | "gif" | "webp" | "bmp" | "tiff" | "tif"
        | "raw" | "cr2" | "nef" | "arw" | "dng" | "svg" | "psd" | "ai" => Category::Photo,
        "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "txt" | "md" | "rtf" | "odt"
        | "ods" | "odp" | "csv" | "pages" | "numbers" | "key" | "epub" => Category::Document,
        "app" | "exe" | "msi" | "dmg" | "pkg" | "apk" | "appimage" | "deb" | "rpm" | "ipa"
        | "iso" => Category::Application,
        "log" | "tmp" | "temp" | "bak" | "crash" | "dmp" | "part" | "crdownload" => {
            Category::Temporary
        }
        "js" | "ts" | "tsx" | "jsx" | "rs" | "py" | "go" | "java" | "kt" | "swift" | "c"
        | "cpp" | "h" | "cs" | "rb" | "php" | "dart" | "o" | "a" | "so" | "dylib" | "dll"
        | "rlib" | "class" | "jar" | "wasm" => Category::Development,
        _ => Category::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cat(p: &str) -> Category {
        Category::of(Path::new(p))
    }

    #[test]
    fn extension_mapping() {
        assert_eq!(cat("/home/a/Movies/x.MP4"), Category::Video);
        assert_eq!(cat("/home/a/Pictures/x.heic"), Category::Photo);
        assert_eq!(cat("/home/a/Documents/x.pdf"), Category::Document);
        assert_eq!(cat("/home/a/Downloads/setup.dmg"), Category::Application);
        assert_eq!(cat("/home/a/Downloads/noext"), Category::Other);
    }

    #[test]
    fn directory_context_beats_extension() {
        assert_eq!(
            cat("/home/a/proj/node_modules/x/readme.pdf"),
            Category::Development
        );
        assert_eq!(
            cat("/home/a/Library/Caches/app/movie.mp4"),
            Category::Temporary
        );
        assert_eq!(
            cat("/Applications/Safari.app/Contents/x.png"),
            Category::Application
        );
        assert_eq!(cat("/System/Library/x.txt"), Category::System);
        assert_eq!(cat("/usr/lib/x.so"), Category::System);
    }

    #[cfg(windows)]
    #[test]
    fn windows_roots() {
        assert_eq!(cat("C:\\Windows\\System32\\x.dll"), Category::System);
        assert_eq!(cat("C:\\Program Files\\App\\x.exe"), Category::Application);
    }

    #[test]
    fn user_library_is_not_system() {
        assert_eq!(cat("/Users/a/Library/Mail/x.emlx"), Category::Other);
        assert_eq!(cat("/Users/a/Library/Caches/x"), Category::Temporary);
    }

    #[test]
    fn deepest_context_wins() {
        // A cache folder inside a dev tree is temporary, not development.
        assert_eq!(
            cat("/home/a/proj/node_modules/.cache/x"),
            Category::Temporary
        );
    }
}
