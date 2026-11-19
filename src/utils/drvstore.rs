//! Private drvstore ABI recovered from the supplied CBSHost and verified on Windows.
//! BOOL is a 32-bit integer; package/store close functions return void.
use libloading::Library;
use std::{
    ffi::{OsStr, c_void},
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
    ptr,
};
use windows::{
    Win32::Foundation::{FILETIME, GetLastError},
    core::GUID,
};

pub type Handle = usize;
type Wide = *const u16;
type OpenStore = unsafe extern "system" fn(Wide, Wide, u32, usize) -> Handle;
type Close = unsafe extern "system" fn(Handle);
type OpenPackage = unsafe extern "system" fn(Wide, u16, Wide, u32, usize) -> Handle;
type Find =
    unsafe extern "system" fn(Handle, Wide, u16, Wide, u32, *mut u16, u32, *mut c_void) -> i32;
type Version = unsafe extern "system" fn(Handle, *mut c_void) -> i32;
type Import = unsafe extern "system" fn(Handle, Wide, u16, Wide, u32, *mut u16, u32) -> u32;
type Delete = unsafe extern "system" fn(Handle, Wide, u32) -> u32;
type Critical = unsafe extern "system" fn(Handle, Wide, u32, Wide) -> u32;
type EnumCallback = unsafe extern "system" fn(Handle, *const c_void, *mut c_void) -> i32;
type EnumDrivers = unsafe extern "system" fn(Handle, u32, EnumCallback, *mut c_void) -> i32;

#[derive(Debug)]
pub enum DrvstoreError {
    Win32Error(u32),
    LibraryError(libloading::Error),
    Message(String),
}
impl std::fmt::Display for DrvstoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Win32Error(n) => write!(f, "Windows error {n} (0x{n:08X})"),
            Self::LibraryError(e) => write!(f, "{e}"),
            Self::Message(e) => write!(f, "{e}"),
        }
    }
}
impl std::error::Error for DrvstoreError {}
impl From<libloading::Error> for DrvstoreError {
    fn from(e: libloading::Error) -> Self {
        Self::LibraryError(e)
    }
}
impl DrvstoreError {
    pub fn code(&self) -> Option<u32> {
        if let Self::Win32Error(n) = self {
            Some(*n)
        } else {
            None
        }
    }
}
fn last_error() -> DrvstoreError {
    DrvstoreError::Win32Error(unsafe { GetLastError().0 })
}
fn wide(s: &OsStr) -> Vec<u16> {
    s.encode_wide().chain(Some(0)).collect()
}
fn string(buf: &[u16]) -> String {
    String::from_utf16_lossy(&buf[..buf.iter().position(|n| *n == 0).unwrap_or(buf.len())])
}

#[derive(Debug, Clone)]
pub struct DriverPackageVersionInfo {
    pub architecture: u16,
    pub provider_name: String,
    pub driver_date: FILETIME,
    pub driver_version: String,
    pub class_guid: GUID,
    pub class_name: String,
    pub catalog_file: String,
}

pub struct DriverStore {
    lib: Library,
    open: OpenStore,
    close: Close,
    open_package: OpenPackage,
    close_package_fn: Close,
    find: Find,
    version: Version,
    sized_version: bool,
}
impl DriverStore {
    pub fn new(path: Option<&Path>) -> Result<Self, DrvstoreError> {
        let path = path.map(Path::to_path_buf).unwrap_or_else(|| {
            crate::utils::utils::get_current_system_root()
                .unwrap_or_else(|_| PathBuf::from("C:/Windows"))
                .join("System32/drvstore.dll")
        });
        // Select the ABI of the library actually loaded, not the image being serviced.
        let v = crate::utils::utils::get_file_version(&path).unwrap_or((6, 2, 0, 0));
        let lib = unsafe { Library::new(path) }?;
        unsafe {
            Ok(Self {
                open: *lib.get(b"DriverStoreOpenW")?,
                close: *lib.get(b"DriverStoreClose")?,
                open_package: *lib.get(b"DriverPackageOpenW")?,
                close_package_fn: *lib.get(b"DriverPackageClose")?,
                find: *lib.get(b"DriverStoreFindW")?,
                version: *lib.get(b"DriverPackageGetVersionInfoW")?,
                sized_version: (v.0, v.1) >= (6, 2),
                lib,
            })
        }
    }
    fn symbol<T: Copy>(&self, name: &[u8]) -> Result<T, DrvstoreError> {
        Ok(unsafe { *self.lib.get::<T>(name)? })
    }
    pub fn session<'a>(
        &'a self,
        system_root: &Path,
        offline: bool,
    ) -> Result<StoreSession<'a>, DrvstoreError> {
        let root = wide(system_root.as_os_str());
        // DriverStoreOpenW owns loading/unloading its offline hives. The second
        // parameter is a unique registry namespace, never a drive or hive handle.
        let ns = wide(OsStr::new(&format!(
            "DriverIndexer-{}-{:016x}",
            std::process::id(),
            fastrand::u64(..)
        )));
        let handle = unsafe {
            (self.open)(
                if offline { root.as_ptr() } else { ptr::null() },
                if offline { ns.as_ptr() } else { ptr::null() },
                0,
                0,
            )
        };
        if handle == 0 {
            return Err(last_error());
        }
        Ok(StoreSession { api: self, handle })
    }
    pub fn open_driver(&self, inf: &Path, arch: u16) -> Result<Handle, DrvstoreError> {
        let path = wide(inf.as_os_str());
        let h = unsafe { (self.open_package)(path.as_ptr(), arch, ptr::null(), 0, 0) };
        if h == 0 { Err(last_error()) } else { Ok(h) }
    }
    pub fn package(&self, inf: &Path, arch: u16) -> Result<Package<'_>, DrvstoreError> {
        Ok(Package {
            api: self,
            handle: self.open_driver(inf, arch)?,
        })
    }
    pub fn close_package(&self, handle: Handle) -> Result<(), DrvstoreError> {
        unsafe { (self.close_package_fn)(handle) };
        Ok(())
    }
    pub fn get_version_info(
        &self,
        handle: Handle,
    ) -> Result<DriverPackageVersionInfo, DrvstoreError> {
        // 8-byte aligned scratch storage; size is an ABI field, NOT package bytes.
        let mut words = [0u64; 0x6f0 / 8];
        if self.sized_version {
            words[0] = 0x6f0;
        }
        if unsafe { (self.version)(handle, words.as_mut_ptr().cast()) } == 0 {
            return Err(last_error());
        }
        let bytes = unsafe { std::slice::from_raw_parts(words.as_ptr().cast::<u8>(), 0x6f0) };
        Ok(decode_version(bytes, self.sized_version))
    }
}
fn decode_version(b: &[u8], sized: bool) -> DriverPackageVersionInfo {
    let u16_at = |i| u16::from_le_bytes([b[i], b[i + 1]]);
    let text =
        |i: usize, len: usize| string(&(i..i + len * 2).step_by(2).map(u16_at).collect::<Vec<_>>());
    let version = u64::from_le_bytes(b[0x2c0..0x2c8].try_into().unwrap());
    let date = if sized { 0x2b8 } else { 0x2b4 };
    let mut guid = [0; 16];
    guid.copy_from_slice(&b[0x2c8..0x2d8]);
    DriverPackageVersionInfo {
        architecture: u16_at(if sized { 4 } else { 0 }),
        provider_name: text(if sized { 0xb0 } else { 0xac }, 260),
        driver_date: FILETIME {
            dwLowDateTime: u32::from_le_bytes(b[date..date + 4].try_into().unwrap()),
            dwHighDateTime: u32::from_le_bytes(b[date + 4..date + 8].try_into().unwrap()),
        },
        driver_version: format!(
            "{}.{}.{}.{}",
            version >> 48,
            (version >> 32) & 65535,
            (version >> 16) & 65535,
            version & 65535
        ),
        class_guid: GUID::from_u128(u128::from_be_bytes([
            guid[3], guid[2], guid[1], guid[0], guid[5], guid[4], guid[7], guid[6], guid[8],
            guid[9], guid[10], guid[11], guid[12], guid[13], guid[14], guid[15],
        ])),
        class_name: text(0x2d8, 260),
        catalog_file: text(0x4e4, 260),
    }
}

pub struct StoreSession<'a> {
    api: &'a DriverStore,
    handle: Handle,
}
impl Drop for StoreSession<'_> {
    fn drop(&mut self) {
        unsafe { (self.api.close)(self.handle) };
    }
}
impl StoreSession<'_> {
    pub fn find(&self, inf: &Path, arch: u16) -> Result<Option<PathBuf>, DrvstoreError> {
        let path = wide(inf.as_os_str());
        let mut output = vec![0u16; 32768];
        let ok = unsafe {
            (self.api.find)(
                self.handle,
                path.as_ptr(),
                arch,
                ptr::null(),
                0,
                output.as_mut_ptr(),
                output.len() as u32,
                ptr::null_mut(),
            )
        };
        if ok != 0 {
            return Ok(Some(PathBuf::from(string(&output))));
        }
        let e = last_error();
        match e.code() {
            Some(2 | 3 | 1168) => Ok(None),
            _ => Err(e),
        }
    }
    pub fn import(&self, inf: &Path, arch: u16) -> Result<PathBuf, DrvstoreError> {
        let call: Import = self.api.symbol(b"DriverStoreImportW")?;
        let path = wide(inf.as_os_str());
        let mut output = vec![0u16; 32768];
        let code = unsafe {
            call(
                self.handle,
                path.as_ptr(),
                arch,
                ptr::null(),
                0,
                output.as_mut_ptr(),
                output.len() as u32,
            )
        };
        if code != 0 {
            return Err(DrvstoreError::Win32Error(code));
        }
        Ok(PathBuf::from(string(&output)))
    }
    pub fn delete(&self, inf: &Path) -> Result<(), DrvstoreError> {
        let call: Delete = self.api.symbol(b"DriverStoreDeleteW")?;
        let path = wide(inf.as_os_str());
        let code = unsafe { call(self.handle, path.as_ptr(), 0) };
        if code == 0 {
            Ok(())
        } else {
            Err(DrvstoreError::Win32Error(code))
        }
    }
    // These two entrypoints are auxiliary and optional in the reference.
    pub fn critical(&self, inf: &Path, remove: bool) -> Result<(), DrvstoreError> {
        let name: &[u8] = if remove {
            b"DriverStoreUnreflectCriticalW"
        } else {
            b"DriverStoreReflectCriticalW"
        };
        let Ok(call) = self.api.symbol::<Critical>(name) else {
            return Ok(());
        };
        let path = wide(inf.as_os_str());
        let code = unsafe { call(self.handle, path.as_ptr(), 0, ptr::null()) };
        if code == 0 {
            Ok(())
        } else {
            Err(DrvstoreError::Win32Error(code))
        }
    }
}
pub struct Package<'a> {
    api: &'a DriverStore,
    handle: Handle,
}
impl Drop for Package<'_> {
    fn drop(&mut self) {
        unsafe { (self.api.close_package_fn)(self.handle) };
    }
}
impl Package<'_> {
    pub fn raw(&self) -> Handle {
        self.handle
    }
    pub fn info(&self) -> Result<DriverPackageVersionInfo, DrvstoreError> {
        self.api.get_version_info(self.handle)
    }
    pub fn applicable(&self) -> Result<bool, DrvstoreError> {
        unsafe extern "system" fn found(_: Handle, _: *const c_void, context: *mut c_void) -> i32 {
            unsafe {
                *context.cast::<bool>() = true;
            }
            0
        }
        let call: EnumDrivers = self.api.symbol(b"DriverPackageEnumDriversW")?;
        let mut present = false;
        let ok = unsafe { call(self.handle, 0, found, (&mut present as *mut bool).cast()) };
        // Callback deliberately stops after one entry. A stop is not a failure.
        if present {
            Ok(true)
        } else if ok != 0 {
            Ok(false)
        } else {
            Err(last_error())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn owns_store_and_package_on_early_errors() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static STORES: AtomicUsize = AtomicUsize::new(0);
        static PACKAGES: AtomicUsize = AtomicUsize::new(0);
        unsafe extern "system" fn open(_: Wide, _: Wide, _: u32, _: usize) -> Handle {
            11
        }
        unsafe extern "system" fn close(_: Handle) {
            STORES.fetch_add(1, Ordering::SeqCst);
        }
        unsafe extern "system" fn package(_: Wide, _: u16, _: Wide, _: u32, _: usize) -> Handle {
            22
        }
        unsafe extern "system" fn close_package(_: Handle) {
            PACKAGES.fetch_add(1, Ordering::SeqCst);
        }
        unsafe extern "system" fn find(
            _: Handle,
            _: Wide,
            _: u16,
            _: Wide,
            _: u32,
            _: *mut u16,
            _: u32,
            _: *mut c_void,
        ) -> i32 {
            0
        }
        unsafe extern "system" fn version(_: Handle, _: *mut c_void) -> i32 {
            0
        }
        let api = DriverStore {
            lib: unsafe { Library::new("kernel32.dll") }.unwrap(),
            open,
            close,
            open_package: package,
            close_package_fn: close_package,
            find,
            version,
            sized_version: true,
        };
        let work = || -> Result<(), DrvstoreError> {
            let _store = api.session(Path::new("test/Windows"), true)?;
            let package = api.package(Path::new("test.inf"), 9)?;
            package.info()?;
            Ok(())
        };
        assert!(work().is_err());
        assert_eq!(STORES.load(Ordering::SeqCst), 1);
        assert_eq!(PACKAGES.load(Ordering::SeqCst), 1);
    }
    #[test]
    fn decodes_sized_and_legacy_layouts() {
        for sized in [false, true] {
            let mut b = [0u8; 0x6f0];
            let offset = if sized { 4 } else { 0 };
            b[offset..offset + 2].copy_from_slice(&9u16.to_le_bytes());
            let provider = if sized { 0xb0 } else { 0xac };
            for (i, x) in "Vendor".encode_utf16().enumerate() {
                b[provider + i * 2..provider + i * 2 + 2].copy_from_slice(&x.to_le_bytes());
            }
            b[0x2c0..0x2c8].copy_from_slice(&0x0001000200030004u64.to_le_bytes());
            let info = decode_version(&b, sized);
            assert_eq!(info.architecture, 9);
            assert_eq!(info.provider_name, "Vendor");
            assert_eq!(info.driver_version, "1.2.3.4");
        }
    }
}
