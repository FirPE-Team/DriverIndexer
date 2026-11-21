//! Safe Windows interface to bit7z. The caller supplies a matching-architecture
//! 7z.dll. C++ sources are vendored; no helper executable is used.
#![cfg(windows)]

use std::ffi::{CStr, OsStr, OsString, c_char, c_void};
use std::marker::PhantomData;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Component, Path, PathBuf};
use std::ptr::NonNull;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct Error {
    pub code: i32,
    pub message: String,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (code {})", self.message, self.code)
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self {
            code: error.raw_os_error().unwrap_or(-1),
            message: error.to_string(),
        }
    }
}
type Result<T> = std::result::Result<T, Error>;
fn invalid(message: impl Into<String>) -> Error {
    Error {
        code: -1,
        message: message.into(),
    }
}

#[repr(C)]
struct NativeError {
    code: i32,
    message: [c_char; 2048],
}
#[repr(C)]
struct NativeEntry {
    path: *const u16,
    path_length: usize,
    size: u64,
    is_directory: u32,
    is_symlink: u32,
}
unsafe extern "C" {
    fn b7_compress(
        dll: *const u16,
        input: *const u16,
        output: *const u16,
        level: u32,
        solid_size: u64,
        threads: u32,
        error: *mut NativeError,
    ) -> i32;
    fn b7_library_check(dll: *const u16, error: *mut NativeError) -> i32;
    fn b7_open(
        dll: *const u16,
        path: *const u16,
        password: *const u16,
        region: u32,
        offset: u64,
        length: u64,
        output: *mut *mut c_void,
        error: *mut NativeError,
    ) -> i32;
    fn b7_close(context: *mut c_void);
    fn b7_count(context: *mut c_void, count: *mut u32, error: *mut NativeError) -> i32;
    fn b7_entry(
        context: *mut c_void,
        index: u32,
        entry: *mut NativeEntry,
        error: *mut NativeError,
    ) -> i32;
    fn b7_extract(
        context: *mut c_void,
        indices: *const u32,
        count: u32,
        destination: *const u16,
        retain: u32,
        skip: u32,
        error: *mut NativeError,
    ) -> i32;
}

fn call(function: impl FnOnce(*mut NativeError) -> i32) -> Result<()> {
    let mut error = NativeError {
        code: 0,
        message: [0; 2048],
    };
    if function(&mut error) == 0 {
        return Ok(());
    }
    // SAFETY: every native failure writes a terminated string into this fixed buffer.
    let message = unsafe { CStr::from_ptr(error.message.as_ptr()) }
        .to_string_lossy()
        .into_owned();
    Err(Error {
        code: error.code,
        message,
    })
}
fn wide(value: &OsStr) -> Result<Vec<u16>> {
    let mut value: Vec<_> = value.encode_wide().collect();
    if value.contains(&0) {
        return Err(invalid("Strings cannot contain NUL"));
    }
    value.push(0);
    Ok(value)
}

/// A cheap, cloneable library configuration; native handles are local to each
/// opened archive. It is safe to clone this into independent worker threads.
#[derive(Debug, Clone)]
pub struct Bit7z {
    dll: PathBuf,
}

/// Optional view into a larger file, expressed in bytes.
#[derive(Debug, Clone, Copy)]
pub struct FileRange {
    pub offset: u64,
    pub length: u64,
}

/// 7z compression settings. A solid block size of zero disables solid mode;
/// zero threads delegates the thread count to 7-Zip.
#[derive(Debug, Clone, Copy)]
pub struct CompressionOptions {
    pub level: u32,
    pub solid_block_size: u64,
    pub threads: u32,
}
impl Default for CompressionOptions {
    fn default() -> Self {
        Self {
            level: 9,
            solid_block_size: 8 * 1024 * 1024,
            threads: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveEntry {
    pub index: u32,
    pub path: PathBuf,
    pub size: u64,
    pub is_directory: bool,
    pub is_symlink: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct ExtractOptions {
    pub retain_directories: bool,
    pub skip_existing: bool,
}
impl Default for ExtractOptions {
    fn default() -> Self {
        Self {
            retain_directories: true,
            skip_existing: true,
        }
    }
}

/// Owns the native reader, input stream, and DLL. Deliberately !Send and !Sync;
/// open an independent reader inside each worker instead of sharing one.
pub struct Archive {
    native: NonNull<c_void>,
    _thread_local: PhantomData<Rc<()>>,
}
impl Drop for Archive {
    fn drop(&mut self) {
        // SAFETY: this is the unique owner of a context created by b7_open.
        unsafe { b7_close(self.native.as_ptr()) }
    }
}
impl Bit7z {
    /// Recursively compress the directory's contents without a parent directory
    /// prefix. Existing output archives are rejected rather than silently updated.
    pub fn compress_directory(
        &self,
        input: impl AsRef<Path>,
        output: impl AsRef<Path>,
        options: CompressionOptions,
    ) -> Result<()> {
        if !matches!(options.level, 0 | 1 | 3 | 5 | 7 | 9) {
            return Err(invalid("Compression level must be 0, 1, 3, 5, 7, or 9"));
        }
        let input = input.as_ref().canonicalize()?;
        if !input.is_dir() {
            return Err(invalid("Compression input must be a directory"));
        }
        // Make the final filename absolute even though it does not exist yet.
        let output = std::path::absolute(output.as_ref())?;
        let output_parent = output
            .parent()
            .ok_or_else(|| invalid("Archive output has no parent"))?
            .canonicalize()?;
        let output = output_parent.join(
            output
                .file_name()
                .ok_or_else(|| invalid("Archive output has no file name"))?,
        );
        if output.starts_with(&input) {
            return Err(invalid(
                "Archive output must be outside the input directory",
            ));
        }
        let dll = wide(self.dll.as_os_str())?;
        let input = wide(input.as_os_str())?;
        let output = wide(output.as_os_str())?;
        // SAFETY: native code only borrows these buffers and catches exceptions.
        call(|error| unsafe {
            b7_compress(
                dll.as_ptr(),
                input.as_ptr(),
                output.as_ptr(),
                options.level,
                options.solid_block_size,
                options.threads,
                error,
            )
        })
    }
    pub fn new(dll: impl AsRef<Path>) -> Result<Self> {
        // An absolute path avoids the Windows DLL search path for the primary DLL.
        let dll = dll.as_ref().canonicalize()?;
        let encoded = wide(dll.as_os_str())?;
        // SAFETY: encoded is NUL terminated and lives through this synchronous call.
        call(|error| unsafe { b7_library_check(encoded.as_ptr(), error) })?;
        Ok(Self { dll })
    }
    pub fn open(&self, path: impl AsRef<Path>, password: Option<&str>) -> Result<Archive> {
        self.open_source(path.as_ref(), password, None)
    }
    pub fn open_range(
        &self,
        path: impl AsRef<Path>,
        range: FileRange,
        password: Option<&str>,
    ) -> Result<Archive> {
        self.open_source(path.as_ref(), password, Some(range))
    }
    fn open_source(
        &self,
        path: &Path,
        password: Option<&str>,
        range: Option<FileRange>,
    ) -> Result<Archive> {
        let dll = wide(self.dll.as_os_str())?;
        let path = wide(path.as_os_str())?;
        let password = wide(OsStr::new(password.unwrap_or("")))?;
        let mut native = std::ptr::null_mut();
        let region = range.is_some() as u32;
        let range = range.unwrap_or(FileRange {
            offset: 0,
            length: 0,
        });
        // SAFETY: all buffers and the output slot live through this call; C++ owns
        // a successfully returned context and releases partial state on failure.
        call(|error| unsafe {
            b7_open(
                dll.as_ptr(),
                path.as_ptr(),
                password.as_ptr(),
                region,
                range.offset,
                range.length,
                &mut native,
                error,
            )
        })?;
        Ok(Archive {
            native: NonNull::new(native).ok_or_else(|| invalid("Native reader was null"))?,
            _thread_local: PhantomData,
        })
    }
}
impl Archive {
    pub fn entries(&mut self) -> Result<Vec<ArchiveEntry>> {
        let mut count = 0;
        // SAFETY: self owns a live context and count is a valid output slot.
        call(|error| unsafe { b7_count(self.native.as_ptr(), &mut count, error) })?;
        let mut entries = Vec::new();
        for index in 0..count {
            let mut entry = NativeEntry {
                path: std::ptr::null(),
                path_length: 0,
                size: 0,
                is_directory: 0,
                is_symlink: 0,
            };
            // SAFETY: index is in range and the returned path is copied before
            // the next native call can invalidate its context-owned buffer.
            call(|error| unsafe { b7_entry(self.native.as_ptr(), index, &mut entry, error) })?;
            let path = unsafe { std::slice::from_raw_parts(entry.path, entry.path_length) };
            entries.push(ArchiveEntry {
                index,
                path: OsString::from_wide(path).into(),
                size: entry.size,
                is_directory: entry.is_directory != 0,
                is_symlink: entry.is_symlink != 0,
            });
        }
        Ok(entries)
    }
    /// Extract exactly these indices. An empty slice extracts nothing.
    /// Selected paths must stay inside destination; symbolic links are rejected.
    /// As with ordinary filesystem writes, callers must prevent concurrent
    /// replacement of destination directories by another process.
    pub fn extract(
        &mut self,
        indices: &[u32],
        destination: impl AsRef<Path>,
        options: ExtractOptions,
    ) -> Result<()> {
        if indices.is_empty() {
            return Ok(());
        }
        let mut indices = indices.to_vec();
        indices.sort_unstable();
        indices.dedup();
        let entries = self.entries()?;
        let mut relative_paths = Vec::new();
        for &index in &indices {
            let entry = entries
                .get(index as usize)
                .ok_or_else(|| invalid("Archive item index out of range"))?;
            if entry.is_symlink {
                return Err(invalid("Extracting symbolic links is not supported"));
            }
            let relative = safe_relative_path(&entry.path)?;
            let relative = if options.retain_directories {
                relative
            } else {
                relative
                    .file_name()
                    .ok_or_else(|| invalid("Empty archive path"))?
                    .into()
            };
            relative_paths.push(relative);
        }
        std::fs::create_dir_all(destination.as_ref())?;
        let root = destination.as_ref().canonicalize()?;
        for relative in relative_paths {
            check_destination(&root, &relative)?;
        }
        let output = wide(root.as_os_str())?;
        // SAFETY: the sorted, unique indices are validated above; C++ only borrows
        // the buffers for the duration of extraction and catches every exception.
        call(|error| unsafe {
            b7_extract(
                self.native.as_ptr(),
                indices.as_ptr(),
                indices.len() as u32,
                output.as_ptr(),
                options.retain_directories as u32,
                options.skip_existing as u32,
                error,
            )
        })
    }
}

fn safe_relative_path(path: &Path) -> Result<PathBuf> {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(name) if !name.encode_wide().any(|c| c == b':' as u16) => {
                result.push(name)
            }
            Component::CurDir => (),
            _ => return Err(invalid(format!("Unsafe archive path: {}", path.display()))),
        }
    }
    if result.as_os_str().is_empty() {
        return Err(invalid("Empty archive path"));
    }
    Ok(result)
}
fn check_destination(root: &Path, relative: &Path) -> Result<()> {
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component);
        match std::fs::symlink_metadata(&current) {
            Ok(_) => {
                if !current.canonicalize()?.starts_with(root) {
                    return Err(invalid("Extraction path escapes destination"));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}
