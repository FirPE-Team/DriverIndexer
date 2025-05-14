// [drvstore API 索引](https://github.com/strontic/strontic.github.io/blob/1e4b8bca9cc9bc152a4e82107a90fa7b2556fcb4/xcyclopedia/library/drvstore.dll-090C64B4CEBBB4527C64D8D8E7C637E9.md)
// [drvstore API 参数](https://github.com/WOA-Project/DriverUpdater/blob/0508f7ab731010361931fb9f704fd95caae53924/DriverUpdater/NativeMethods.cs)
// [drvstore API 示例](https://github.com/WOA-Project/DriverUpdater/blob/2a5b56bd16de18799a54b9d9a56676ac68f259ef/DriverUpdater/Program.cs)

use libloading::{Library};
use std::{
    error::Error,
    ffi::OsStr,
    os::windows::ffi::OsStrExt,
    path::Path,
};

type PCWSTR = *const u16;
type PWSTR  = *mut u16;
type HANDLE = usize;

/// 返回值类型：与 C# 的 `uint` 对应
type NTSTATUS = u32;
const STATUS_SUCCESS: NTSTATUS = 0;

/// DriverStoreOpenW
type DSOF_OpenW = unsafe extern "system" fn(
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
    i32       // DestInfPathSize (just capacity)
) -> NTSTATUS;

/// DriverStoreReflectW
type DSOF_ReflectW = unsafe extern "system" fn(HANDLE, u32) -> u32;

/// DriverStoreDeleteW
type DSOF_DeleteW = unsafe extern "system" fn(PCWSTR, u32) -> u32;

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
) -> NTSTATUS;

/// DriverStoreOfflineDeleteDriverPackageW
type DSOF_OfflineDeleteW = unsafe extern "system" fn(
    PCWSTR, u32, u32, PCWSTR, PCWSTR
) -> u32;

/// DriverStoreClose
type DSOF_CloseW = unsafe extern "system" fn(HANDLE) -> bool;

/// 将 &OsStr 转成以 NUL 结尾的 UTF-16 Vec<u16>
fn to_wide(s: &OsStr) -> Vec<u16> {
    s.encode_wide().chain(Some(0)).collect()
}

pub struct DriverStore {
    _lib:    Library,
    open:    DSOF_OpenW,
    import: DSOF_ImportW,
    reflect: DSOF_ReflectW,
    delete: DSOF_DeleteW,
    offline_add: DSOF_OfflineAddW,
    offline_delete: DSOF_OfflineDeleteW,
    close:   DSOF_CloseW,
}

impl DriverStore {
    /// 加载 drvstore.dll 并解析所需函数
    pub unsafe fn new() -> Result<Self, Box<dyn Error>> {
        let lib = Library::new("drvstore.dll")?;
        Ok(Self {
            open: *lib.get(b"DriverStoreOpenW")?,
            import: *lib.get(b"DriverStoreImportW")?,
            reflect: *lib.get(b"DriverStoreReflectW")?,
            delete: *lib.get(b"DriverStoreDeleteW")?,
            offline_add: *lib.get(b"DriverStoreOfflineAddDriverPackageW")?,
            offline_delete: *lib.get(b"DriverStoreOfflineDeleteDriverPackageW")?,
            close: *lib.get(b"DriverStoreClose")?,
            _lib: lib
        })
    }

    /// 打开离线仓库，返回句柄
    pub unsafe fn open_store(
        &self,
        system_root: &Path,   // e.g. "D:\\Mount\\Windows"
        system_drive: &Path,  // e.g. "D:\\Mount"
    ) -> Result<HANDLE, Box<dyn Error>> {
        let root = to_wide(system_root.as_os_str());
        let drv  = to_wide(system_drive.as_os_str());
        let handle = (self.open)(root.as_ptr(), drv.as_ptr(), 0, 0, );
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
    pub unsafe fn import_driver_to_store(
        &self,
        handle: HANDLE,
        inf_path: &Path,
        arch: u32,
        flags: u32,
    ) -> Result<String, Box<dyn Error>> {
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
    pub unsafe fn offline_add_driver(
        &self,
        inf_path: &Path,
        system_root: &Path,
        system_drive: &Path,
        flags: u32,
        arch: u32,
    ) -> Result<String, Box<dyn Error>> {
        let inf_w = to_wide(inf_path.as_os_str());
        let lang  = to_wide(OsStr::new("en-US"));
        let mut buf = vec![0u16; 260];
        let mut buf_len: i32 = buf.len() as i32;
        let root = to_wide(system_root.as_os_str());
        let drv  = to_wide(system_drive.as_os_str());

        let status = (self.offline_add)(
            inf_w.as_ptr(),
            flags,
            0,                         // Reserved = NULL
            arch as u16,              // ProcessorArchitecture
            lang.as_ptr(),
            buf.as_mut_ptr(),
            &mut buf_len as *mut i32,
            root.as_ptr(),
            drv.as_ptr()
        );
        if status != STATUS_SUCCESS {
            return Err(format!("OfflineAddDriverPackageW Error: 0x{:08X}", status).into());
        }

        // 从 buf 提取返回的 INF 文件名或相对路径
        let returned = String::from_utf16_lossy(&buf[..buf_len as usize]);
        Ok(returned)
    }

    /// 删除当前系统中已导入的驱动包（基于 INF 名）
    /// - `inf_name`: 例如 `"netrtwlanu.inf"`，不带路径
    /// - `flags`: 删除标志（通常为 0）
    pub unsafe fn delete_driver(&self, inf_name: &str, flags: u32) -> Result<(), Box<dyn Error>> {
        let name_w = to_wide(OsStr::new(inf_name));
        let status = (self.delete)(name_w.as_ptr(), flags);
        if status == 0 {
            Ok(())
        } else {
            Err(format!("DriverStoreDeleteW Error: 0x{:08X}", status).into())
        }
    }

    /// 离线删除驱动包（通过完整 INF 路径）
    /// - `inf_path`: 离线系统中的 `.inf` 路径（如 `C:\\Drivers\\netrtwlanu.inf`）
    /// - `system_root`: 离线 Windows 路径（如 `D:\\Mount\\Windows`）
    /// - `system_drive`: 离线系统盘根目录（如 `D:\\Mount`）
    pub unsafe fn offline_delete_driver(
        &self,
        inf_path: &Path,
        system_root: &Path,
        system_drive: &Path,
        flags: u32,
    ) -> Result<(), Box<dyn Error>> {
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

    /// 关闭仓库句柄
    pub unsafe fn close_store(&self, handle: HANDLE) -> Result<(), Box<dyn Error>> {
        if !(self.close)(handle) {
            Err("DriverStoreClose Error".into())
        } else {
            Ok(())
        }
    }
}
