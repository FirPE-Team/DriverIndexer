// [drvstore API 索引](https://github.com/strontic/strontic.github.io/blob/1e4b8bca9cc9bc152a4e82107a90fa7b2556fcb4/xcyclopedia/library/drvstore.dll-090C64B4CEBBB4527C64D8D8E7C637E9.md)
// [drvstore API 参数](https://github.com/WOA-Project/DriverUpdater/blob/0508f7ab731010361931fb9f704fd95caae53924/DriverUpdater/NativeMethods.cs)
// [drvstore API 示例](https://github.com/WOA-Project/DriverUpdater/blob/2a5b56bd16de18799a54b9d9a56676ac68f259ef/DriverUpdater/Program.cs)

use libloading::Library;
use std::{
    error::Error,
    ffi::OsStr,
    os::windows::ffi::OsStrExt,
    path::Path,
};

type PCWSTR = *const u16;
type PWSTR = *mut u16;
type HANDLE = usize;

/// DriverPackageOpenW
type DSOF_OpenPackageW = unsafe extern "system" fn(
    PCWSTR,   // infPath
    u16,      // architecture (如 9 = AMD64)
    PCWSTR,   // locale (en-US)
    u32,      // flags
    usize,    // resolveContext = 0
) -> HANDLE;

/// DriverPackageClose
type DSOF_ClosePackageW = unsafe extern "system" fn(HANDLE) -> bool;

/// DriverStoreOpenW
type DSOF_OpenStoreW = unsafe extern "system" fn(
    PCWSTR,    // OfflineSystemRoot (如 "C:\\Windows")
    PCWSTR,    // OfflineSystemDrive (如 "C:")
    u32,       // Flags（保留 0）
    usize,     // Reserved（保留 NULL）
) -> HANDLE;

/// DriverStoreImportW
type DSOF_ImportW = unsafe extern "system" fn(
    HANDLE,   // hDriverStore
    PCWSTR,   // DriverPackageFileName
    u16,      // ProcessorArchitecture
    PCWSTR,   // LocaleName
    u32,      // Flags
    PWSTR,    // DestInfPath (out buffer)
    i32,      // DestInfPathSize (just capacity)
) -> u32;

/// DriverStoreReflectW
type DSOF_ReflectW = unsafe extern "system" fn(HANDLE, u32) -> u32;

/// DriverStoreUnreflectW
type DSOF_UnreflectW = unsafe extern "system" fn(
    HANDLE,   // hDriverStore
    PCWSTR,   // DriverStoreFileName (.inf)
    u32,      // Flags
    PCWSTR,   // FilterDeviceIds (可选，通常传 "*" 或 null)
) -> u32;     // NTSTATUS

/// DriverStoreUnpublishW
type DSOF_UnpublishW = unsafe extern "system" fn(
    HANDLE,        // hDriverStore
    PCWSTR,        // DriverStoreFileName (.inf)
    u32,           // Flags
    PWSTR,         // PublishedFileName (out buffer)
    i32,           // buffer size (count of WCHARs)
    *mut bool,     // isPublishedFileNameChanged
) -> u32;          // NTSTATUS

/// DriverStoreUnreflectCriticalW
type DSOF_UnreflectCriticalW = unsafe extern "system" fn(
    HANDLE,  // hDriverStore
    PCWSTR,  // InfName
    u32,     // Flags
    PCWSTR,  // FilterDeviceIds (可为 "*" 或 null)
) -> u32;

/// DriverStoreDeleteW
type DSOF_DeleteW = unsafe extern "system" fn(
    HANDLE,   // DriverStore handle
    PCWSTR,   // full INF path (in FileRepository)
    u32,      // flags
) -> u32;

/// DriverStoreOfflineAddDriverPackageW
type DSOF_OfflineAddW = unsafe extern "system" fn(
    PCWSTR,    // InfPath
    u32,       // Flags
    usize,     // Reserved（传 0）
    u16,       // ProcessorArchitecture（ushort）
    PCWSTR,    // LocaleName
    PWSTR,     // DestInfPath buffer
    *mut i32,  // cchDestInfPath (in/out)
    PCWSTR,    // TargetSystemRoot
    PCWSTR,    // TargetSystemDrive
) -> u32;

/// DriverStoreOfflineDeleteDriverPackageW
type DSOF_OfflineDeleteW = unsafe extern "system" fn(PCWSTR, u32, u32, PCWSTR, PCWSTR) -> u32;

pub type EnumDriverPackageCallback = unsafe extern "system" fn(PCWSTR, usize) -> bool;

/// DriverStoreOfflineEnumDriverPackageW
type DSOF_OfflineEnumW = unsafe extern "system" fn(
    callback: EnumDriverPackageCallback,
    lparam: usize,
    target_system_root: PCWSTR,
) -> u32;

/// DriverStoreClose
type DSOF_CloseStoreW = unsafe extern "system" fn(HANDLE) -> bool;

/// 将 &OsStr 转成以 NUL 结尾的 UTF-16 Vec<u16>
fn to_wide(s: &OsStr) -> Vec<u16> {
    s.encode_wide().chain(Some(0)).collect()
}

pub struct DriverStore {
    _lib: Library,
    openPackage: DSOF_OpenPackageW,
    closePackage: DSOF_ClosePackageW,
    openStore: DSOF_OpenStoreW,
    import: DSOF_ImportW,
    reflect: DSOF_ReflectW,
    unreflect: DSOF_UnreflectW,
    unpublish: DSOF_UnpublishW,
    unreflect_critical: DSOF_UnreflectCriticalW,
    delete: DSOF_DeleteW,
    offline_add: DSOF_OfflineAddW,
    offline_delete: DSOF_OfflineDeleteW,
    offline_enum: DSOF_OfflineEnumW,
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
            import: *lib.get(b"DriverStoreImportW")?,
            reflect: *lib.get(b"DriverStoreReflectW")?,
            unreflect: *lib.get(b"DriverStoreUnreflectW")?,
            unpublish: *lib.get(b"DriverStoreUnpublishW")?,
            unreflect_critical: *lib.get(b"DriverStoreUnreflectCriticalW")?,
            delete: *lib.get(b"DriverStoreDeleteW")?,
            offline_add: *lib.get(b"DriverStoreOfflineAddDriverPackageW")?,
            offline_delete: *lib.get(b"DriverStoreOfflineDeleteDriverPackageW")?,
            offline_enum: *lib.get(b"DriverStoreOfflineEnumDriverPackageW")?,
            closeStore: *lib.get(b"DriverStoreClose")?,
            _lib: lib,
        })
    }

    /// 打开驱动包（从 OEMINF 文件获取句柄）
    /// 成功返回句柄（非零），失败返回 Err
    pub unsafe fn open_driver_package(&self, inf_path: &Path, arch: u16) -> Result<HANDLE, Box<dyn Error>> {
        let inf_path = to_wide(inf_path.as_os_str());
        let locale_w = to_wide(OsStr::new("en-US"));

        let handle = (self.openPackage)(
            inf_path.as_ptr(),
            arch,
            locale_w.as_ptr(),
            0,  // flags
            0,  // resolveContext
        );

        if handle != 0 {
            Ok(handle)
        } else {
            Err("DriverPackageOpenW Error".into())
        }
    }

    /// 关闭驱动包句柄
    pub unsafe fn close_package(&self, handle: HANDLE) -> Result<(), Box<dyn Error>> {
        if !(self.closePackage)(handle) {
            Err("DriverPackageClose Error".into())
        } else {
            Ok(())
        }
    }

    /// 打开离线仓库，返回句柄
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

    /// 导入 INF 驱动文件到指定 DriverStore 句柄
    /// - handle: DriverStoreOpenW 打开的句柄
    /// - inf_path: .inf 文件完整路径
    /// - arch: 处理器架构（0 = x86, 9 = AMD64, 12 = ARM64）
    /// - locale: 区域（一般 "en-US"）
    /// - flags: 通常为 0
    /// - 返回：导入后的目标路径，如 `%SystemRoot%\System32\DriverStore\FileRepository\xxxx.inf_amd64_...`
    pub unsafe fn import_driver_to_store(&self, handle: HANDLE, inf_path: &Path, arch: u32, flags: u32) -> Result<String, Box<dyn Error>> {
        let inf_w = to_wide(inf_path.as_os_str());
        let locale_w = to_wide(OsStr::new("en-US"));

        // 输出缓冲区 & 容量
        let mut buf = vec![0u16; 260];
        let buf_size = buf.len() as i32;

        let status = (self.import)(
            handle,
            inf_w.as_ptr(),
            arch as u16,
            locale_w.as_ptr(),
            flags,
            buf.as_mut_ptr(),
            buf_size,
        );

        if status != 0 {
            return Err(format!("DriverStoreImportW Error: 0x{:08X}", status).into());
        }

        // 从 buf 提取返回的 INF 文件名或相对路径
        let ret = String::from_utf16_lossy(
            &buf.split(|&c| c == 0).next().unwrap_or(&[])
        );
        Ok(ret)
    }

    /// # 安装驱动（Reflect）到匹配设备
    /// ## 参数
    /// 1. handle
    /// 2. flags
    ///    - 0x0000: 默认：尝试绑定所有匹配设备
    ///    - 0x0001: INSTALL_FLAGS_REFLECT_CRITICAL
    ///    - 0x0002: INSTALL_FLAGS_REFLECT_FORCE
    pub unsafe fn reflect_driver(&self, handle: HANDLE, flags: u32) -> Result<(), Box<dyn Error>> {
        let status = (self.reflect)(handle, flags);
        if status == 0 {
            Ok(())
        } else {
            Err(format!("DriverStoreReflectW Error: 0x{:08X}", status).into())
        }
    }

    /// # 离线添加驱动
    ///
    /// ## 参数
    ///
    /// 1. `system_root` - 离线系统的 Windows 路径（如 `D:\Mount\Windows`）
    /// 2. `system_drive` - 离线系统的根目录路径（如 `D:\Mount`）
    /// 3. `flags` - 导入选项（DriverStoreOfflineAddDriverPackageFlags）：
    ///     - `None = 0x00000000`：默认行为，复制并安装驱动包
    ///     - `SkipInstall = 0x00000001`：仅添加到仓库，不执行反射安装
    ///     - `Inbox = 0x00000002`：将驱动标记为系统内置驱动
    ///     - `F6 = 0x00000004`：模拟安装时通过 F6 加载驱动
    ///     - `SkipExternalFilePresenceCheck = 0x00000008`：跳过外部文件存在性检查
    ///     - `NoTempCopy = 0x00000010`：不使用临时目录复制
    ///     - `UseHardLinks = 0x00000020`：使用硬链接而不是复制文件
    ///     - `InstallOnly = 0x00000040`：仅安装（反射）已存在的驱动包
    ///     - `ReplacePackage = 0x00000080`：替换已存在的驱动包
    ///     - `Force = 0x00000100`：强制导入（忽略设备类别限制）
    ///     - `BaseVersion = 0x00000200`：标记为基础版本驱动
    ///
    /// 4. `arch` - 目标系统架构（ProcessorArchitecture）：
    ///     - `INTEL = 0`（x86）
    ///     - `MIPS = 1`
    ///     - `ALPHA = 2`
    ///     - `PPC = 3`
    ///     - `SHX = 4`
    ///     - `ARM = 5`（ARM32）
    ///     - `IA64 = 6`
    ///     - `ALPHA64 = 7`
    ///     - `MSIL = 8`
    ///     - `AMD64 = 9`（x64）
    ///     - `IA32_ON_WIN64 = 10`
    ///     - `NEUTRAL = 11`
    ///     - `ARM64 = 12`
    ///     - `UNKNOWN = 0xFFFF`
    ///
    /// ## 返回值
    ///
    /// - 成功返回导入驱动包在仓库中的 INF 路径（`Ok(String)`）
    /// - 失败返回错误信息（`Err`）
    pub unsafe fn offline_add_driver(&self, inf_path: &Path, system_root: &Path, system_drive: &Path, flags: u32, arch: u32) -> Result<String, Box<dyn Error>> {
        let inf_w = to_wide(inf_path.as_os_str());
        let lang = to_wide(OsStr::new("en-US"));
        let mut buf = vec![0u16; 260];
        let mut buf_len: i32 = buf.len() as i32;
        let root = to_wide(system_root.as_os_str());
        let drv = to_wide(system_drive.as_os_str());

        let status = (self.offline_add)(
            inf_w.as_ptr(),
            flags,
            0,                         // Reserved = NULL
            arch as u16,              // ProcessorArchitecture
            lang.as_ptr(),
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

    /// 删除当前系统中已导入的驱动包
    /// - `handle`: 驱动库句柄
    /// - `inf_path`: 在驱动库中的inf文件完整路径
    /// - `flags`: 删除标志（通常为 0）
    pub unsafe fn delete_driver(&self, handle: HANDLE, inf_path: &Path, flags: u32) -> Result<(), Box<dyn Error>> {
        let inf_w = to_wide(inf_path.as_os_str());
        let status = (self.delete)(handle, inf_w.as_ptr(), flags);
        if status == 0 {
            Ok(())
        } else {
            Err(format!("DriverStoreDeleteW Error: 0x{:08X}", status).into())
        }
    }

    /// 尝试解除一个驱动包（INF）在 DriverStore 中的"反射"状态
    pub unsafe fn unreflect_driver(&self, handle: HANDLE, inf_name: &str, flags: u32, filter_device_ids: Option<&str>) -> Result<(), Box<dyn Error>> {
        let inf_w = to_wide(OsStr::new(inf_name));
        let filter_wide = match filter_device_ids {
            Some(s) => to_wide(OsStr::new(s)),
            None => vec![0], // 传 null wide 字符串
        };

        // 确保 handle 有效
        if handle == 0 {
            return Err("Invalid handle".into());
        }

        // 确保 inf_name 不为空
        if inf_name.is_empty() {
            return Err("Invalid INF name".into());
        }

        let status = (self.unreflect)(
            handle,
            inf_w.as_ptr(),
            flags,
            if filter_device_ids.is_some() {
                filter_wide.as_ptr()
            } else {
                std::ptr::null()
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
    pub unsafe fn unpublish_driver(&self, handle: HANDLE, inf_name: &str, flags: u32) -> Result<String, Box<dyn Error>> {
        let inf_w = to_wide(OsStr::new(inf_name));
        let mut buffer: Vec<u16> = vec![0; 260];
        let mut changed: bool = false;

        let status = (self.unpublish)(handle, inf_w.as_ptr(), flags, buffer.as_mut_ptr(), buffer.len() as i32, &mut changed as *mut bool);
        if status != 0 {
            return Err(format!("DriverStoreUnpublishW Error: 0x{:08X}", status).into());
        }

        let result = String::from_utf16_lossy(&buffer[..buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len())]);
        Ok(result)
    }

    /// 解除系统对**关键驱动**的依赖
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
                std::ptr::null()
            },
        );

        if status == 0 {
            Ok(())
        } else {
            Err(format!("DriverStoreUnreflectCriticalW Error: 0x{:08X}", status).into())
        }
    }

    /// 离线删除驱动包（通过完整 INF 路径）
    /// - `inf_path`: 离线系统中的 `.inf` 路径（如 `C:\\Drivers\\netrtwlanu.inf`）
    /// - `system_root`: 离线 Windows 路径（如 `D:\\Mount\\Windows`）
    /// - `system_drive`: 离线系统盘根目录（如 `D:\\Mount`）
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

    /// 获取离线系统中所有驱动包 INF 文件名列表
    pub unsafe fn enum_offline_driver_packages(&self, offline_windows: &Path) -> Result<Vec<String>, Box<dyn Error>> {
        let win_w = to_wide(offline_windows.as_os_str());

        let mut result: Vec<String> = Vec::new();
        let result_ptr = &mut result as *mut _ as usize;

        let status = (self.offline_enum)(
            Self::collect_inf_callback,
            result_ptr,
            win_w.as_ptr(),
        );

        if status != 0 {
            return Err(format!("DriverStoreOfflineEnumDriverPackageW Error: 0x{:08X}", status).into());
        }

        Ok(result)
    }

    /// 回调函数：收集 INF 路径到 lparam 所指的 Vec<String>
    unsafe extern "system" fn collect_inf_callback(inf_path: PCWSTR, lparam: usize) -> bool {
        let vec_ptr = lparam as *mut Vec<String>;
        if let Some(vec) = vec_ptr.as_mut() {
            let len = (0..).take_while(|&i| *inf_path.add(i) != 0).count();
            let str_slice = std::slice::from_raw_parts(inf_path, len);
            let inf_str = String::from_utf16_lossy(str_slice);
            vec.push(inf_str);
        }
        true
    }

    /// 关闭仓库句柄
    pub unsafe fn close_store(&self, handle: HANDLE) -> Result<(), Box<dyn Error>> {
        if !(self.closeStore)(handle) {
            Err("DriverStoreClose Error".into())
        } else {
            Ok(())
        }
    }
}
