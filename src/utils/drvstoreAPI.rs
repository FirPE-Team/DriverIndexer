// [drvstore API 索引](https://github.com/strontic/strontic.github.io/blob/1e4b8bca9cc9bc152a4e82107a90fa7b2556fcb4/xcyclopedia/library/drvstore.dll-090C64B4CEBBB4527C64D8D8E7C637E9.md)
// [drvstore API 参数](https://github.com/WOA-Project/DriverUpdater/blob/0508f7ab731010361931fb9f704fd95caae53924/DriverUpdater/NativeMethods.cs)
// [drvstore API 示例](https://github.com/WOA-Project/DriverUpdater/blob/2a5b56bd16de18799a54b9d9a56676ac68f259ef/DriverUpdater/Program.cs)

use libloading::Library;
use std::mem::MaybeUninit;
use std::{error::Error, ffi::OsStr, os::windows::ffi::OsStrExt, path::Path, ptr};
use windows::core::GUID;
use windows::Win32::Foundation::{GetLastError, FILETIME, MAX_PATH};

type PCWSTR = *const u16;
type PWSTR = *mut u16;
type PDWORD = *mut u32;
type HANDLE = usize;

/// DriverStoreOpenW
type DSOF_OpenStoreW = unsafe extern "system" fn(
    PCWSTR,    // OfflineSystemRoot (e.g "C:\\Windows")
    PCWSTR,    // OfflineSystemDrive (e.g "C:")
    u32,       // Flags (0)
    usize,     // Reserved（0）
) -> HANDLE;   // DriverStore handle

/// DriverStoreClose
type DSOF_CloseStoreW = unsafe extern "system" fn(
    HANDLE     // DriverStore handle
) -> bool;

/// DriverPackageOpenW
type DSOF_OpenPackageW = unsafe extern "system" fn(
    PCWSTR,   // InfPath
    u16,      // ProcessorArchitecture (0=x86, 9=AMD64, 12=ARM64)
    PCWSTR,   // LocaleName (e.g "en-US" or NULL)
    u32,      // Flags
    usize,    // resolveContext (0)
) -> HANDLE;

/// DriverPackageClose
type DSOF_ClosePackageW = unsafe extern "system" fn(
    HANDLE     // DriverStore handle
) -> bool;

/// DriverStoreCopyW
type DSOF_CopyW = unsafe extern "system" fn(
    HANDLE,  // DriverStore handle
    PCWSTR,  // DriverPackageInfPath
    u16,     // ProcessorArchitecture (0=x86, 9=AMD64, 12=ARM64)
    PCWSTR,  // LocaleName (e.g "en-US" or NULL)
    u32,     // Flags
    PCWSTR,  // DestinationPath
) -> u32;    // NTSTATUS

///DriverStoreFindW
type DSOF_FindW = unsafe extern "system" fn(
    HANDLE,   // DriverStore handle
    PCWSTR,   // DriverPackageFilename
    u16,      // ProcessorArchitecture (0=x86, 9=AMD64, 12=ARM64)
    PCWSTR,   // LocaleName (e.g "en-US" or NULL)
    u32,      // Flags
    PWSTR,    // OutInfPath
    u32,      // OutInfPathSizes
    *mut DRIVERSTORE_DRIVERPACKAGE_INFOW,
) -> u32;     // NTSTATUS

/// DriverPackageGetVersionInfoW
type DSOF_GetVerInfoW = unsafe extern "system" fn(
    handle: HANDLE,                          // DriverStore handle
    info: *mut DRIVER_PACKAGE_VERSION_INFO,  // DRIVER_PACKAGE_VERSION_INFO
) -> bool;                                   // NTSTATUS

/// DriverStoreImportW
type DSOF_ImportW = unsafe extern "system" fn(
    HANDLE,   // DriverStore handle
    PCWSTR,   // DriverPackageFileName
    u16,      // ProcessorArchitecture (0=x86, 9=AMD64, 12=ARM64)
    PCWSTR,   // LocaleName (e.g "en-US" or NULL)
    u32,      // Flags
    PWSTR,    // DestInfPath (out buffer)
    i32,      // DestInfPathSize (just capacity)
) -> u32;     // NTSTATUS

/// DriverStoreReflectW
type DSOF_ReflectW = unsafe extern "system" fn(HANDLE, u32) -> u32;

/// DriverStoreUnreflectW
type DSOF_UnreflectW = unsafe extern "system" fn(
    HANDLE,   // DriverStore handle
    PCWSTR,   // DriverStoreFileName (.inf)
    u32,      // Flags
    PCWSTR,   // FilterDeviceIds ("*" or null)
) -> u32;     // NTSTATUS

/// DriverStoreUnpublishW
type DSOF_UnpublishW = unsafe extern "system" fn(
    HANDLE,        // DriverStore handle
    PCWSTR,        // DriverStoreFileName (.inf)
    u32,           // Flags
    PWSTR,         // PublishedFileName (out buffer)
    i32,           // buffer size (count of WCHARs)
    *mut bool,     // isPublishedFileNameChanged
) -> u32;          // NTSTATUS

/// DriverStoreUnreflectCriticalW
type DSOF_UnreflectCriticalW = unsafe extern "system" fn(
    HANDLE,  // DriverStore handle
    PCWSTR,  // InfName
    u32,     // Flags
    PCWSTR,  // FilterDeviceIds ("*" or null)
) -> u32;    // NTSTATUS

/// DriverStoreDeleteW
type DSOF_DeleteW = unsafe extern "system" fn(
    HANDLE,   // DriverStore handle
    PCWSTR,   // Full INF path (in FileRepository)
    u32,      // Flags
) -> u32;     // NTSTATUS

/// DriverStoreOfflineAddDriverPackageW
type DSOF_OfflineAddW = unsafe extern "system" fn(
    PCWSTR,    // InfPath
    u32,       // Flags
    usize,     // Reserved（0）
    u16,       // ProcessorArchitecture (0=x86, 9=AMD64, 12=ARM64)
    PCWSTR,    // LocaleName (e.g "en-US" or NULL)
    PWSTR,     // DestInfPath buffer
    *mut i32,  // cchDestInfPath (in/out)
    PCWSTR,    // TargetSystemRoot
    PCWSTR,    // TargetSystemDrive
) -> u32;      // NTSTATUS

/// DriverStoreOfflineDeleteDriverPackageW
type DSOF_OfflineDeleteW = unsafe extern "system" fn(
    PCWSTR,    // InfPath
    u32,       // Flags
    u32,       // Reserved（0）
    PCWSTR,    // TargetSystemRoot
    PCWSTR,    // TargetSystemDrive
) -> u32;      // NTSTATUS

#[repr(C)]
#[derive(Debug, Clone)]
/// DriverStoreImportW 结构体
struct DRIVERSTORE_DRIVERPACKAGE_INFOW {
    /// 目标驱动的处理器架构
    pub ProcessorArchitecture: u16,
    /// 区域语言代码，表示此驱动包的语言版本
    pub LocaleName: [u16; 85],
    /// 驱动包在系统中注册的INF 名称
    pub PublishedInfName: [u16; MAX_PATH as usize],
    /// 标志位
    pub Flags: u32,
}

/// DRIVERSTORE_DRIVERPACKAGE_INFOW 封装结构体
#[derive(Debug, Clone)]
pub struct DriverPackageInfo {
    /// 目标驱动的处理器架构
    pub processor_architecture: u16,
    /// 区域语言代码，表示此驱动包的语言版本
    pub locale_name: String,
    /// 驱动包在系统中注册的INF 名称
    pub published_inf_name: String,
    /// 标志位
    pub flags: u32,
}

impl From<DRIVERSTORE_DRIVERPACKAGE_INFOW> for DriverPackageInfo {
    /// 将原始 DRIVERSTORE_DRIVERPACKAGE_INFOW 转为 DriverPackageInfo
    fn from(raw: DRIVERSTORE_DRIVERPACKAGE_INFOW) -> Self {
        Self {
            processor_architecture: raw.ProcessorArchitecture,
            locale_name: utf16_array_to_string(&raw.LocaleName),
            published_inf_name: utf16_array_to_string(&raw.PublishedInfName),
            flags: raw.Flags,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
struct DRIVER_PACKAGE_VERSION_INFO {
    pub size: u32,                                 // Size of the structure
    pub architecture: u16,                         // ProcessorArchitecture (enum)
    pub locale_name: [u16; 85],                    // LOCALE_NAME_MAX_LENGTH
    pub provider_name: [u16; MAX_PATH as usize],   // MAX_PATH
    pub driver_date: FILETIME,                     // Driver date
    pub driver_version: u64,                       // Packed version
    pub class_guid: GUID,                          // GUID of device class
    pub class_name: [u16; MAX_PATH as usize],      // MAX_PATH
    pub class_version: u32,                        // Class version
    pub catalog_file: [u16; MAX_PATH as usize],    // MAX_PATH
    pub flags: u32,                                // Flags
}

/// DRIVER_PACKAGE_VERSION_INFO 封装结构体
#[derive(Debug, Clone)]
pub struct DriverPackageVersionInfo {
    pub size: u32,
    pub architecture: u16,
    pub locale_name: String,
    pub provider_name: String,
    pub driver_date: FILETIME,
    pub driver_version: String,
    pub class_guid: GUID,
    pub class_name: String,
    pub class_version: u32,
    pub catalog_file: String,
    pub flags: u32,
}

impl From<DRIVER_PACKAGE_VERSION_INFO> for DriverPackageVersionInfo {
    fn from(raw: DRIVER_PACKAGE_VERSION_INFO) -> Self {
        // 解包 driver_version
        let major = (raw.driver_version >> 48) & 0xFFFF;
        let minor = (raw.driver_version >> 32) & 0xFFFF;
        let build = (raw.driver_version >> 16) & 0xFFFF;
        let revision = raw.driver_version & 0xFFFF;
        let driver_version = format!("{}.{}.{}.{}", major, minor, build, revision);

        DriverPackageVersionInfo {
            size: raw.size,
            architecture: raw.architecture,
            locale_name: utf16_array_to_string(&raw.locale_name),
            provider_name: utf16_array_to_string(&raw.provider_name),
            driver_date: raw.driver_date,
            driver_version,
            class_guid: raw.class_guid,
            class_name: utf16_array_to_string(&raw.class_name),
            class_version: raw.class_version,
            catalog_file: utf16_array_to_string(&raw.catalog_file),
            flags: raw.flags,
        }
    }
}

/// 将 &OsStr 转成以 NUL 结尾的 UTF-16 Vec<u16>
fn to_wide(s: &OsStr) -> Vec<u16> {
    s.encode_wide().chain(Some(0)).collect()
}

fn utf16_array_to_string(buf: &[u16]) -> String {
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len])
}

pub struct DriverStore {
    _lib: Library,
    openPackage: DSOF_OpenPackageW,
    closePackage: DSOF_ClosePackageW,
    openStore: DSOF_OpenStoreW,
    copy: Option<DSOF_CopyW>,
    find: DSOF_FindW,
    getVerinfo: DSOF_GetVerInfoW,
    import: DSOF_ImportW,
    reflect: DSOF_ReflectW,
    unreflect: DSOF_UnreflectW,
    unpublish: DSOF_UnpublishW,
    unreflect_critical: DSOF_UnreflectCriticalW,
    delete: DSOF_DeleteW,
    offline_add: DSOF_OfflineAddW,
    offline_delete: DSOF_OfflineDeleteW,
    closeStore: DSOF_CloseStoreW,
}

impl DriverStore {
    /// 加载 drvstore.dll 并解析所需函数
    pub unsafe fn new() -> Result<Self, Box<dyn Error>> {
        let lib = Library::new("drvstore.dll")?;
        Ok(Self {
            openPackage: *lib.get(b"DriverPackageOpenW")?,
            closePackage: *lib.get(b"DriverPackageClose")?,
            openStore: *lib.get(b"DriverStoreOpenW")?,
            copy: *lib.get(b"DriverStoreCopyW")?,
            find: *lib.get(b"DriverStoreFindW")?,
            getVerinfo: *lib.get(b"DriverPackageGetVersionInfoW")?,
            import: *lib.get(b"DriverStoreImportW")?,
            reflect: *lib.get(b"DriverStoreReflectW")?,
            unreflect: *lib.get(b"DriverStoreUnreflectW")?,
            unpublish: *lib.get(b"DriverStoreUnpublishW")?,
            unreflect_critical: *lib.get(b"DriverStoreUnreflectCriticalW")?,
            delete: *lib.get(b"DriverStoreDeleteW")?,
            offline_add: *lib.get(b"DriverStoreOfflineAddDriverPackageW")?,
            offline_delete: *lib.get(b"DriverStoreOfflineDeleteDriverPackageW")?,
            closeStore: *lib.get(b"DriverStoreClose")?,
            _lib: lib,
        })
    }

    /// 打开驱动库，返回句柄
    /// 参数
    /// - `system_root` - 系统的 Windows 路径（如 `C:\Windows`、`D:\Mount\Windows`）
    /// - `system_drive` - 系统的根目录路径（如 `C:`、`D:\Mount`）
    /// 返回
    /// - `Ok(HANDLE)`: 驱动库句柄
    /// - `Err(...)`：失败则返回 FALSE，则包含 Win32 错误码或说明
    pub unsafe fn open_store(&self, system_root: &Path, system_drive: &Path) -> Result<HANDLE, Box<dyn Error>> {
        let root = to_wide(system_root.as_os_str());
        let drv = to_wide(system_drive.as_os_str());
        let handle = (self.openStore)(root.as_ptr(), drv.as_ptr(), 0, 0);
        if handle != 0 {
            Ok(handle)
        } else {
            Err("DriverStoreOpenW Error".into())
        }
    }

    /// 关闭驱动库
    /// 参数
    ///  - `handle`: 通过调用 `open_store` 返回的句柄
    ///
    /// 返回
    /// - `Ok(())`: 返回成功
    /// - `Err(...)`：失败则返回包含 Win32 错误码的说明
    pub unsafe fn close_store(&self, handle: HANDLE) -> Result<(), Box<dyn Error>> {
        if !(self.closeStore)(handle) {
            Err(format!("DriverStoreClose Error: {:?}", GetLastError()).into())
        } else {
            Ok(())
        }
    }

    /// 打开驱动包（从 INF 文件获取句柄）
    /// 参数
    /// - `inf_path`: INF文件路径
    /// - `arch`:     架构值 (0 = x86, 9 = AMD64, 12 = ARM64)
    ///
    /// 返回
    /// - `Ok(HANDLE)`: 驱动包句柄
    /// - `Err(...)`：失败则返回 FALSE，则包含 Win32 错误码或说明
    pub unsafe fn open_driver(&self, inf_path: &Path, arch: u16) -> Result<HANDLE, Box<dyn Error>> {
        let inf_path = to_wide(inf_path.as_os_str());

        let handle = (self.openPackage)(
            inf_path.as_ptr(),
            arch,
            ptr::null(),
            0,  // flags
            0,  // resolveContext
        );

        if handle != 0 {
            Ok(handle)
        } else {
            Err(format!("DriverPackageOpenW Error: {:?}", GetLastError()).into())
        }
    }

    /// 关闭驱动包句柄
    /// 参数
    ///  - `handle`:  来自 open_store 的句柄
    ///
    /// 返回
    /// - `Ok(())`:   返回成功
    /// - `Err(...)`：失败则返回包含 Win32 错误码的说明
    pub unsafe fn close_package(&self, handle: HANDLE) -> Result<(), Box<dyn Error>> {
        if !(self.closePackage)(handle) {
            Err(format!("DriverPackageClose Error: {:?}", GetLastError()).into())
        } else {
            Ok(())
        }
    }

    /// 复制驱动
    ///
    /// 注意：此 API 不支持 Win7
    ///
    /// 参数
    /// - `handle`:      来自 open_store 的句柄
    /// - `inf_full`:    INF 全路径，例如
    ///                  "C:\\Windows\\System32\\DriverStore\\FileRepository\\xxx\\xxx.inf"
    /// - `arch`:        架构值 (0=x86, 9=AMD64, 12=ARM64)
    /// - `destination`: 目标文件夹（必须存在）
    ///
    /// 返回
    /// - `Ok(())`:      返回成功
    /// - `Err(...)`：   失败则返回包含 Win32 错误码的说明
    pub unsafe fn copy_driver(&self, handle: HANDLE, inf_full: &Path, arch: u16, destination: &Path) -> Result<(), Box<dyn Error>> {
        let copy_fn = self.copy.ok_or_else(|| "DriverStoreCopyW is not supported on this system".to_string())?;

        let inf_wide = to_wide(inf_full.as_os_str());
        let dest_wide = to_wide(destination.as_os_str());

        // flags = 0 表示常规复制
        let result = copy_fn(
            handle,
            inf_wide.as_ptr(),
            arch,
            ptr::null(),      // locale = NULL
            0,                // flags = None
            dest_wide.as_ptr(),
        );
        if result != 0 {
            Err(format!("DriverStoreCopyW Error: 0x{:08X}", result).into())
        } else {
            Ok(())
        }
    }

    /// 通过 %SystemRoot%\INF 中的 INF 文件，查找 FileRepository 中实际 INF 所在路径，
    ///
    /// 参数：
    ///
    /// - `handle`：     来自 open_store 的句柄
    /// - `inf_path`：   要查找的 INF 路径
    /// - `arch`：       架构值 (0 = x86, 9 = AMD64, 12 = ARM64)
    ///
    /// 返回：
    /// - `Some((found_path, DriverPackageInfo))`
    ///    - `found_path` 是在 FileRepository 下的完整 INF 路径（Rust String）
    ///    - `DriverPackageInfo` 是 `DRIVERSTORE_DRIVERPACKAGE_INFOW` 的结构体
    /// - `None`：未找到对应的 INF 驱动库路径
    pub unsafe fn find_driver_package(&self, handle: HANDLE, inf_path: &Path, arch: u16) -> Option<(String, DriverPackageInfo)> {
        // 宽字符化输入
        let inf_wide = to_wide(inf_path.as_os_str());

        // 准备输出缓冲
        let mut out_buf: Vec<u16> = vec![0; MAX_PATH as usize];
        let buf_size: u32 = MAX_PATH;
        let mut info = MaybeUninit::<DRIVERSTORE_DRIVERPACKAGE_INFOW>::zeroed();

        // 调用 DriverStoreFindW
        let ok = (self.find)(
            handle,
            inf_wide.as_ptr(),
            arch,
            ptr::null(),
            0,
            out_buf.as_mut_ptr(),
            buf_size,
            info.as_mut_ptr(),
        );

        if ok == 0 {
            return None;
        }

        // 从 out_buf 提取 Rust String
        let len = out_buf.iter().position(|&c| c == 0).unwrap_or(out_buf.len());
        let found_path = String::from_utf16_lossy(&out_buf[..len]);

        Some((found_path, DriverPackageInfo::from(info.assume_init())))
    }

    /// 获取INF驱动信息
    ///
    /// 参数：
    /// - `handle`: 来自 open_driver 的句柄
    ///
    /// 返回
    /// - `Ok(DRIVER_PACKAGE_VERSION_INFO)`:      返回成功
    /// - `Err(...)`：   失败则返回包含 Win32 错误码的说明
    pub unsafe fn get_version_info(&self, handle: HANDLE) -> Result<DriverPackageVersionInfo, Box<dyn Error>> {
        let mut info = DRIVER_PACKAGE_VERSION_INFO {
            size: size_of::<DRIVER_PACKAGE_VERSION_INFO>() as u32,
            architecture: 0,
            locale_name: [0; 85],
            provider_name: [0; MAX_PATH as usize],
            driver_date: FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 },
            driver_version: 0,
            class_guid: GUID::zeroed(),
            class_name: [0; MAX_PATH as usize],
            class_version: 0,
            catalog_file: [0; MAX_PATH as usize],
            flags: 0,
        };

        if !(self.getVerinfo)(handle, &mut info as *mut _) {
            return Err(format!("DriverPackageGetVersionInfoW Error: {:?}", GetLastError()).into());
        }

        Ok(DriverPackageVersionInfo::from(info))
    }

    /// 导入 INF 驱动文件到指定 DriverStore 句柄
    ///
    /// 参数：
    /// - `handle`:     来自 open_store 的句柄
    /// - `inf_path`:   .inf 文件完整路径
    /// - `arch`:       架构值（0 = x86, 9 = AMD64, 12 = ARM64）
    /// - `flags`:      通常为 0
    ///
    /// 返回：
    /// - `Ok(import_path)`
    ///    导入后的目标路径，如 `%SystemRoot%\System32\DriverStore\FileRepository\xxxx.inf_amd64_...`
    /// - `Err(...)`：如果调用失败或返回 FALSE，则包含 Win32 错误码或说明
    pub unsafe fn import_driver_to_store(&self, handle: HANDLE, inf_path: &Path, arch: u32, flags: u32) -> Result<String, Box<dyn Error>> {
        let inf_w = to_wide(inf_path.as_os_str());

        // 输出缓冲区 & 容量
        let mut buf = vec![0u16; MAX_PATH as usize];
        let buf_size = buf.len() as i32;

        let status = (self.import)(
            handle,
            inf_w.as_ptr(),
            arch as u16,
            ptr::null(),
            flags,
            buf.as_mut_ptr(),
            buf_size,
        );

        if status != 0 {
            return Err(format!("DriverStoreImportW Error: 0x{:08X}", status).into());
        }

        // 从 buf 提取返回的 INF 文件名或相对路径
        let ret = String::from_utf16_lossy(buf.split(|&c| c == 0).next().unwrap_or(&[]));
        Ok(ret)
    }

    /// 删除当前系统中已导入的驱动包
    ///
    /// 参数
    /// - `handle`:    来自 open_store 的句柄
    /// - `inf_path`:  在驱动库中的 inf 文件完整路径
    /// - `flags`:     删除标志（通常为 0）
    ///
    /// 返回
    /// - `Ok(())`: 返回成功
    /// - `Err(...)`：如果调用失败则返回包含 Win32 错误码的说明
    pub unsafe fn delete_driver(&self, handle: HANDLE, inf_path: &Path, flags: u32) -> Result<(), Box<dyn Error>> {
        let inf_w = to_wide(inf_path.as_os_str());
        let status = (self.delete)(handle, inf_w.as_ptr(), flags);
        if status == 0 {
            Ok(())
        } else {
            Err(format!("DriverStoreDeleteW Error: 0x{:08X}", status).into())
        }
    }

    /// 安装驱动（Reflect）到匹配设备
    ///
    /// 参数
    /// - `handle`: 来自 open_store 的句柄
    /// - `flags`
    ///    - 0x0000: 默认：尝试绑定所有匹配设备
    ///    - 0x0001: INSTALL_FLAGS_REFLECT_CRITICAL
    ///    - 0x0002: INSTALL_FLAGS_REFLECT_FORCE
    ///
    /// 返回
    /// - `Ok(())`: 返回成功
    /// - `Err(...)`：如果调用失败则返回包含 Win32 错误码的说明
    pub unsafe fn reflect_driver(&self, handle: HANDLE, flags: u32) -> Result<(), Box<dyn Error>> {
        let status = (self.reflect)(handle, flags);
        if status == 0 {
            Ok(())
        } else {
            Err(format!("DriverStoreReflectW Error: 0x{:08X}", status).into())
        }
    }

    /// 尝试解除一个驱动包（INF）在 DriverStore 中的"反射"状态
    ///
    /// 参数
    /// - `handle`:            来自 open_store 的句柄
    /// - `inf_name`:          INF 文件名称
    /// - `flags`:             标志位
    /// - `filter_device_ids`
    ///
    /// 返回
    /// - `Ok(())`: 返回成功
    /// - `Err(...)`：如果调用失败则返回包含 Win32 错误码的说明
    pub unsafe fn unreflect_driver(&self, handle: HANDLE, inf_name: &str, flags: u32, filter_device_ids: Option<&str>) -> Result<(), Box<dyn Error>> {
        let inf_w = to_wide(OsStr::new(inf_name));
        let filter_wide = match filter_device_ids {
            Some(s) => to_wide(OsStr::new(s)),
            None => vec![0], // 传 null wide 字符串
        };

        let status = (self.unreflect)(
            handle,
            inf_w.as_ptr(),
            flags,
            if filter_device_ids.is_some() {
                filter_wide.as_ptr()
            } else {
                ptr::null()
            },
        );

        // 检查返回状态
        match status {
            0 => Ok(()),
            0x000000A1 => Err("Driver is blocked or in use".into()),
            _ => Err(format!("DriverStoreUnreflectW Error: 0x{:08X}", status).into())
        }
    }

    /// 解除系统对驱动的依赖
    ///
    /// 参数
    /// - `handle`:        来自 open_store 的句柄
    /// - `inf_name`:      INF 文件名称
    /// - `flags`:         标志位
    ///
    /// 返回
    /// - `Ok(String)`: 返回成功
    /// - `Err(...)`：如果调用失败则返回包含 Win32 错误码的说明
    pub unsafe fn unpublish_driver(&self, handle: HANDLE, inf_name: &str, flags: u32) -> Result<String, Box<dyn Error>> {
        let inf_w = to_wide(OsStr::new(inf_name));
        let mut buffer: Vec<u16> = vec![0; MAX_PATH as usize];
        let mut changed: bool = false;

        let status = (self.unpublish)(handle, inf_w.as_ptr(), flags, buffer.as_mut_ptr(), buffer.len() as i32, &mut changed as *mut bool);
        if status != 0 {
            return Err(format!("DriverStoreUnpublishW Error: 0x{:08X}", status).into());
        }

        let result = String::from_utf16_lossy(&buffer[..buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len())]);
        Ok(result)
    }

    /// 解除系统对**关键驱动**的依赖
    ///
    /// 参数
    /// - `handle`:     来自 open_store 的句柄
    /// - `inf_name`:   INF 文件名称
    /// - `flags`:      标志位
    /// - `filter_device_ids`
    ///
    /// 返回
    /// - `Ok(())`: 返回成功
    /// - `Err(...)`：如果调用失败则返回包含 Win32 错误码的说明
    pub unsafe fn unreflect_critical_driver(&self, handle: HANDLE, inf_name: &str, flags: u32, filter_device_ids: Option<&str>) -> Result<(), Box<dyn Error>> {
        let inf_w = to_wide(OsStr::new(inf_name));
        let filter_wide = match filter_device_ids {
            Some(s) => to_wide(OsStr::new(s)),
            None => vec![0], // 传 null wide 字符串
        };

        let status = (self.unreflect_critical)(
            handle,
            inf_w.as_ptr(),
            flags,
            if filter_device_ids.is_some() {
                filter_wide.as_ptr()
            } else {
                ptr::null()
            },
        );

        if status == 0 {
            Ok(())
        } else {
            Err(format!("DriverStoreUnreflectCriticalW Error: 0x{:08X}", status).into())
        }
    }

    /// 导入驱动包到离线系统的驱动仓库
    ///
    /// 参数
    ///
    /// - `inf_path`：驱动包的 INF 文件路径
    /// - `system_root`：离线系统的 `Windows` 目录路径（如 `D:\Mount\Windows`）
    /// - `system_drive`：离线系统所在根目录（如 `D:\Mount`）
    /// - `flags`：导入行为控制位。详见 [`DriverStoreOfflineAddDriverPackageFlags`]：
    ///     - `0x01`: SkipInstall（仅添加驱动，不安装）
    ///     - `0x02`: Inbox（标记为系统内置）
    ///     - `0x04`: F6 加载模式
    ///     - `0x08`: 跳过外部文件检查
    ///     - `0x10`: 不复制到临时目录
    ///     - `0x20`: 使用硬链接
    ///     - `0x40`: 仅反射已有驱动包
    ///     - `0x80`: 替换已有包
    ///     - `0x100`: 强制导入
    ///     - `0x200`: 标记为基础版本驱动
    /// - `arch`：目标系统架构。常用值：
    ///     - `0`: INTEL（x86）
    ///     - `9`: AMD64（x64）
    ///     - `12`: ARM64
    ///
    /// 返回
    /// - `Ok(String)`：成功时返回仓库中的实际 INF 路径
    /// - `Err(...)`：导入失败，包含错误码描述
    pub unsafe fn offline_add_driver(&self, inf_path: &Path, system_root: &Path, system_drive: &Path, flags: u32, arch: u32) -> Result<String, Box<dyn Error>> {
        let inf_w = to_wide(inf_path.as_os_str());
        let mut buf = vec![0u16; MAX_PATH as usize];
        let mut buf_len: i32 = buf.len() as i32;
        let root = to_wide(system_root.as_os_str());
        let drv = to_wide(system_drive.as_os_str());

        let status = (self.offline_add)(
            inf_w.as_ptr(),
            flags,
            0,
            arch as u16,
            ptr::null(),
            buf.as_mut_ptr(),
            &mut buf_len as *mut i32,
            root.as_ptr(),
            drv.as_ptr(),
        );
        if status != 0 {
            return Err(format!("OfflineAddDriverPackageW Error: 0x{:08X}", status).into());
        }

        // 从 buf 提取返回的 INF 文件名或相对路径
        let returned = String::from_utf16_lossy(&buf[..buf_len as usize]);
        Ok(returned)
    }

    /// 离线删除驱动包（通过完整 INF 路径）
    ///
    /// 参数
    /// - `inf_path`: 离线系统中的 `.inf` 路径（如 `D:\\Drivers\\netrtwlanu.inf`）
    /// - `system_root`: 离线 Windows 路径（如 `D:\\Mount\\Windows`）
    /// - `system_drive`: 离线系统盘根目录（如 `D:\\Mount`）
    ///
    /// 返回
    /// - `Ok(())`: 返回成功
    /// - `Err(...)`：如果调用失败则返回包含 Win32 错误码的说明
    pub unsafe fn offline_delete_driver(&self, inf_path: &Path, system_root: &Path, system_drive: &Path, flags: u32) -> Result<(), Box<dyn Error>> {
        let inf_w = to_wide(inf_path.as_os_str());
        let root_w = to_wide(system_root.as_os_str());
        let drive_w = to_wide(system_drive.as_os_str());

        let status = (self.offline_delete)(
            inf_w.as_ptr(),
            flags,
            0, // Reserved
            root_w.as_ptr(),
            drive_w.as_ptr(),
        );
        if status == 0 {
            Ok(())
        } else {
            Err(format!("DriverStoreOfflineDeleteDriverPackageW Error: 0x{:08X}", status).into())
        }
    }
}
