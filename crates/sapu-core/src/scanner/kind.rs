//! Finer-grained "kind" of a file for the browser (Finder-like "Kind"
//! column), decided from the extension only. Categories stay the eight
//! user-facing buckets; kinds are what a person sees next to a file name.

use std::path::Path;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum FileKind {
    Video,
    Audio,
    Image,
    Pdf,
    Document,
    Spreadsheet,
    Presentation,
    Archive,
    DiskImage,
    Installer,
    Application,
    Code,
    Log,
    Database,
    Font,
    Library,
    VirtualDisk,
    Backup,
    Folder,
    Other,
}

impl FileKind {
    /// Kind from the file's extension (case-insensitive); `Other` when unknown.
    pub fn of(path: &Path) -> Self {
        if path.extension().is_none()
            && let Some(name) = path.file_name().and_then(|n| n.to_str())
            && name.ends_with(".app")
        {
            return Self::Application;
        }
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            return Self::Other;
        };
        let ext = ext.to_ascii_lowercase();
        match ext.as_str() {
            "mp4" | "mov" | "mkv" | "avi" | "m4v" | "webm" | "wmv" | "flv" | "mpg" | "mpeg"
            | "m2ts" => Self::Video,
            "mp3" | "aac" | "m4a" | "flac" | "wav" | "ogg" | "opus" | "aiff" | "wma" => Self::Audio,
            "jpg" | "jpeg" | "png" | "gif" | "heic" | "heif" | "webp" | "tiff" | "tif" | "bmp"
            | "raw" | "cr2" | "nef" | "arw" | "dng" | "psd" | "svg" | "ai" | "sketch" => {
                Self::Image
            }
            "pdf" => Self::Pdf,
            "doc" | "docx" | "odt" | "rtf" | "txt" | "md" | "pages" | "epub" | "tex" => {
                Self::Document
            }
            "xls" | "xlsx" | "ods" | "csv" | "numbers" | "tsv" => Self::Spreadsheet,
            "ppt" | "pptx" | "odp" | "key" => Self::Presentation,
            "zip" | "rar" | "7z" | "tar" | "gz" | "tgz" | "bz2" | "xz" | "zst" | "lz4" => {
                Self::Archive
            }
            "dmg" | "iso" | "img" | "cdr" | "sparseimage" | "sparsebundle" => Self::DiskImage,
            "pkg" | "mpkg" | "exe" | "msi" | "msix" | "deb" | "rpm" | "appimage" | "apk"
            | "aab" | "ipa" | "xpi" => Self::Installer,
            "app" | "jar" => Self::Application,
            "rs" | "js" | "ts" | "tsx" | "jsx" | "py" | "go" | "java" | "kt" | "swift" | "c"
            | "cc" | "cpp" | "h" | "hpp" | "cs" | "rb" | "php" | "sh" | "json" | "toml"
            | "yaml" | "yml" | "xml" | "html" | "css" | "scss" | "vue" | "dart" | "lua" | "sql" => {
                Self::Code
            }
            "log" | "crash" | "diag" | "ips" | "trace" => Self::Log,
            "db" | "sqlite" | "sqlite3" | "realm" | "mdb" | "accdb" | "ldb" => Self::Database,
            "ttf" | "otf" | "woff" | "woff2" | "ttc" => Self::Font,
            "dylib" | "so" | "dll" | "a" | "lib" | "framework" | "o" | "rlib" | "wasm" => {
                Self::Library
            }
            "vmdk" | "vdi" | "vhd" | "vhdx" | "qcow2" | "utm" | "pvm" | "ova" | "ovf" => {
                Self::VirtualDisk
            }
            "bak" | "backup" | "old" | "orig" | "tmp" | "part" | "crdownload" | "download" => {
                Self::Backup
            }
            _ => Self::Other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_common_extensions() {
        assert_eq!(FileKind::of(Path::new("a/b.MOV")), FileKind::Video);
        assert_eq!(FileKind::of(Path::new("Claude.dmg")), FileKind::DiskImage);
        assert_eq!(FileKind::of(Path::new("worker.log")), FileKind::Log);
        assert_eq!(FileKind::of(Path::new("x.aab")), FileKind::Installer);
        assert_eq!(
            FileKind::of(Path::new("libtensorflow.2.dylib")),
            FileKind::Library
        );
        assert_eq!(
            FileKind::of(Path::new("Electron.app")),
            FileKind::Application
        );
        assert_eq!(FileKind::of(Path::new("README")), FileKind::Other);
        assert_eq!(FileKind::of(Path::new("photo.unknownext")), FileKind::Other);
    }
}
