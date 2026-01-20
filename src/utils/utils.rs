use crate::{Asset, SECRET_KEY};
use anyhow::{anyhow, Context, Result};
use chrono::Local;
use crc32fast::Hasher;
use glob::MatchOptions;
use goblin::pe::PE;
use magic_crypt::{new_magic_crypt, MagicCryptTrait};
use std::cmp::Ordering;
use std::ffi::c_void;
use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::iter::repeat_with;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::{env, fs, io, ptr};
use walkdir::WalkDir;
use windows::core::{BOOL, GUID, HSTRING, PWSTR};
use windows::Win32::Foundation::{FILETIME, HANDLE, HWND, MAX_PATH, SYSTEMTIME};
use windows::Win32::Security::WinTrust::{
    WinVerifyTrust, WINTRUST_ACTION_GENERIC_VERIFY_V2, WINTRUST_DATA,
    WINTRUST_DATA_0, WINTRUST_DATA_PROVIDER_FLAGS, WINTRUST_FILE_INFO, WTD_CHOICE_FILE,
    WTD_REVOKE_NONE, WTD_STATEACTION_CLOSE, WTD_STATEACTION_VERIFY, WTD_UICONTEXT_EXECUTE,
    WTD_UI_NONE,
};
use windows::Win32::Storage::FileSystem::{
    GetDiskFreeSpaceExW, GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW,
    VS_FIXEDFILEINFO,
};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Ioctl::{
    PropertyStandardQuery, StorageDeviceProperty, IOCTL_STORAGE_QUERY_PROPERTY,
    STORAGE_DEVICE_DESCRIPTOR, STORAGE_PROPERTY_QUERY,
};
use windows::Win32::System::SystemInformation::{
    GetNativeSystemInfo, GetWindowsDirectoryW, PROCESSOR_ARCHITECTURE, SYSTEM_INFO,
};
use windows::Win32::System::Threading::{GetCurrentProcess, GetCurrentProcessId, IsWow64Process};
use windows::Win32::System::Time::FileTimeToSystemTime;
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
        Storage::FileSystem::GetDriveTypeW,
        Storage::FileSystem::FILE_ATTRIBUTE_NORMAL,
        Storage::FileSystem::{
            CreateFileW, FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_SHARE_READ, FILE_SHARE_WRITE,
            OPEN_EXISTING,
        },
        System::Ioctl::IOCTL_STORAGE_EJECT_MEDIA,
        System::IO::DeviceIoControl,
    },
};

/// 写到文件
///
/// # 参数
/// - `file_path`: 静态文件名
/// - `out_path`: 输出路径
///
/// # 返回值
/// - `Ok(())`: 写入成功
/// - `Err(...)`：失败则返回错误
pub fn write_embed_file(file_path: &str, out_path: &Path) -> Result<()> {
    let file =
        Asset::get(file_path).with_context(|| format!("Embedded file not found: {}", file_path))?;
    File::create(out_path)
        .with_context(|| format!("Failed to create file: {}", out_path.display()))?
        .write_all(&file.data)
        .with_context(|| format!("Failed to write file: {}", out_path.display()))?;
    Ok(())
}

/// 写日志
///
/// # 参数
/// - `log_path`: 日志路径
/// - `content` 日志内容
///
/// # 返回值
/// - `Ok(())`: 写入成功
/// - `Err(...)`：失败则返回错误
pub fn write_log(log_path: &Path, content: &str) -> Result<()> {
    let file = OpenOptions::new()
        .create(true) // 如果不存在则创建
        .append(true) // 追加模式
        .open(log_path)
        .with_context(|| format!("Open log file failed: {}", log_path.display()))?;
    let datetime = Local::now().format("%Y-%m-%d %T").to_string();

    let mut writer = BufWriter::new(file);
    writeln!(writer, "{} {}", datetime, content)
        .with_context(|| format!("Write log file failed: {}", log_path.display()))?;
    Ok(())
}

/// 返回当前进程的父进程 PID
///
/// # 参数
/// - `pid`: 子进程 PID
///
/// # 返回值
/// - `Ok(pid)`: 父进程 PID
/// - `Err(...)`：失败则返回错误
fn get_parent_pid(pid: u32) -> windows::core::Result<u32> {
    unsafe {
        // 全进程快照
        let h = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)?;
        if h.is_invalid() {
            return Err(windows::core::Error::from_thread());
        }
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };

        // 枚举第一个
        Process32FirstW(h, &mut entry)?;
        loop {
            if entry.th32ProcessID == pid {
                let _ = CloseHandle(h);
                return Ok(entry.th32ParentProcessID);
            }
            if Process32NextW(h, &mut entry).is_err() {
                break;
            }
        }
        let _ = CloseHandle(h);
        Err(windows::core::Error::from_thread())
    }
}

/// 给定 PID，返回进程名（不含路径），如 "explorer.exe"
///
/// # 参数
/// - `pid`: 进程 PID
///
/// # 返回值
/// - `Ok(name)`: 进程名
/// - `Err(...)`：失败则返回错误
fn get_process_name(pid: u32) -> windows::core::Result<String> {
    unsafe {
        let h = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)?;
        if h.is_invalid() {
            return Err(windows::core::Error::from_thread());
        }
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };

        Process32FirstW(h, &mut entry)?;
        loop {
            if entry.th32ProcessID == pid {
                // 找到第一个 NUL 终止符
                let len = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(MAX_PATH as usize);
                let name = OsString::from_wide(&entry.szExeFile[..len])
                    .into_string()
                    .map_err(|_| windows::core::Error::from_thread())?;
                let _ = CloseHandle(h);
                return Ok(name);
            }
            if Process32NextW(h, &mut entry).is_err() {
                break;
            }
        }
        let _ = CloseHandle(h);
        Err(windows::core::Error::from_thread())
    }
}

/// 检查父进程名是否为 explorer.exe
///
/// # 返回值
/// - `true`: 是
/// - `false`: 否
pub fn launched_from_explorer() -> bool {
    let self_pid = unsafe { GetCurrentProcessId() };
    if let Ok(ppid) = get_parent_pid(self_pid)
        && let Ok(name) = get_process_name(ppid)
    {
        return name.eq_ignore_ascii_case("explorer.exe");
    }
    false
}

/// 加密密码
///
/// # 参数
/// - `password`: 密码
///
/// # 返回值
/// - `Ok(String)`: 加密后的密码
/// - `Err(...)`：失败则返回错误
pub fn encrypt_password(password: &str) -> String {
    let mc = new_magic_crypt!(SECRET_KEY, 128);
    // 加密密码并移除尾部的 '='
    mc.encrypt_str_to_base64(password)
        .trim_end_matches('=')
        .to_string()
}

/// 解密密码
///
/// # 参数
/// - `encrypted_password`: 加密后的密码
///
/// # 返回值
/// - `Ok(String)`: 解密后的密码
/// - `Err(...)`：失败则返回错误
pub fn decrypt_password(encrypted_password: &str) -> Result<String> {
    // 复制一份可变的密文
    let mut cipher_text = encrypted_password.to_string();
    let len = cipher_text.len();
    let remainder = len % 4;

    // 只有当 remainder 为 2 或 3 时，才需要补齐 '='
    if remainder == 2 {
        cipher_text.push_str("==");
    } else if remainder == 3 {
        cipher_text.push('=');
    }

    let mc = new_magic_crypt!(SECRET_KEY, 128);
    Ok(mc.decrypt_base64_to_string(cipher_text)?)
}

/// 遍历目录及子目录下的所有指定文件
///
/// # 参数
/// - `path`: 目录路径
/// - `file_type`: 文件通配符（如 *.inf）
///
/// # 返回值
/// - `Ok(Vec<PathBuf>)`: 文件列表
/// - `Err(...)`：失败则返回错误
pub fn get_file_list(path: &Path, file_type: &str) -> Result<Vec<PathBuf>> {
    let pattern = path.join("**").join(file_type);
    let search = glob::glob_with(
        &pattern.to_string_lossy(),
        MatchOptions {
            case_sensitive: false,
            require_literal_separator: false,
            require_literal_leading_dot: false,
        },
    )
    .with_context(|| format!("Failed to glob path: {}", path.display()))?;

    Ok(search
        // 过滤掉错误项
        .filter_map(Result::ok)
        // 只保留文件
        .filter(|p| p.is_file())
        .collect())
}

/// 复制目录及子目录下的所有文件
///
/// # 参数
/// - `src`: 源路径
/// - `dst`: 目标路径
///
/// # 返回值
/// - `Ok(())`: 成功
/// - `Err(...)`：失败则返回错误
pub fn copy_dir(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in WalkDir::new(src) {
        let entry = entry?;
        let rel = entry.path().strip_prefix(src).unwrap();
        let target = dst.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// 移动目录及子目录下的所有文件
///
/// # 参数
/// - `src`: 源路径
/// - `dst`: 目标路径
///
/// # 返回值
/// - `Ok(())`: 成功
/// - `Err(...)`：失败则返回错误
pub fn move_dir(src: &Path, dst: &Path) -> io::Result<()> {
    // 如果在同一文件系统，rename 最快
    if fs::rename(src, dst).is_ok() {
        return Ok(());
    }
    // 否则，退回到复制 + 删除
    copy_dir(src, dst)?;
    fs::remove_dir_all(src)?;
    Ok(())
}

/// 将 FILETIME 转换为字符串格式（yyyy-MM-dd）
///
/// # 参数
/// - `filetime`: 文件时间
///
/// # 返回值
/// - `Ok(String)`: 成功
/// - `Err(...)`：失败则返回错误
pub fn filetime_to_string(filetime: &FILETIME) -> Result<String, windows::core::Error> {
    let mut system_time: SYSTEMTIME = unsafe { std::mem::zeroed() };

    unsafe {
        match FileTimeToSystemTime(filetime, &mut system_time) {
            Ok(_) => Ok(format!(
                "{:04}-{:02}-{:02}",
                system_time.wYear, system_time.wMonth, system_time.wDay,
            )),
            Err(e) => Err(e),
        }
    }
}

/// 比较版本号大小
///
/// # 参数
/// - `version1`: 版本号1
/// - `version2`: 版本号2
///
/// # 返回值
/// - `Ok(Ordering)`
/// - `Err(...)`：失败则返回错误
pub fn compare_version(version1: &str, version2: &str) -> Ordering {
    let ver1: Vec<u64> = version1
        .split('.')
        .filter_map(|s| s.parse::<u64>().ok())
        .collect();
    let ver2: Vec<u64> = version2
        .split('.')
        .filter_map(|s| s.parse::<u64>().ok())
        .collect();

    // 逐位比较
    for (n1, n2) in ver1.iter().zip(ver2.iter()) {
        match n1.cmp(n2) {
            Ordering::Equal => continue,
            ord => return ord,
        }
    }

    // 如果前缀都一样，长的版本号更大 (e.g. 10.0.1 > 10.0)
    ver1.len().cmp(&ver2.len())
}

/// 生成临时文件名
///
/// # 参数
/// - `prefix`: 前缀
/// - `suffix`: 后缀
/// - `rand_len`: 长度
///
/// # 返回值
/// - `OsString` : 临时文件名
pub fn get_temp_name(prefix: &str, suffix: &str, rand_len: usize) -> OsString {
    let capacity = prefix
        .len()
        .saturating_add(suffix.len())
        .saturating_add(rand_len);
    let mut buf = OsString::with_capacity(capacity);
    buf.push(prefix);
    let mut char_buf = [0u8; 4];
    for c in repeat_with(fastrand::alphanumeric).take(rand_len) {
        buf.push(c.encode_utf8(&mut char_buf));
    }
    buf.push(suffix);
    buf
}

/// 提取指定字符串中全部%%形式的变量名
pub fn extract_vars(s: &str) -> Vec<String> {
    s.split('%')
        .enumerate()
        .filter_map(|(i, part)| {
            // 只保留奇数索引部分（两个%之间的内容）
            if i % 2 == 1 && !part.is_empty() {
                // 过滤合法字符（字母、数字、下划线）
                part.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_')
                    .then(|| part.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// 是否为离线系统
///
/// # 参数
/// - `system_path`: 系统盘路径
///
/// # 返回值
/// - `Ok(bool)`: 是返回 `true`，否返回 `false`
/// - `Err(...)`：失败则返回错误
pub fn is_offline_system(system_path: &Path) -> Result<bool> {
    // 拼接 Windows 子目录
    let system_path = PathBuf::from(system_path).join("Windows");

    // 判断系统目录是否存在
    if !system_path.exists() {
        return Ok(false);
    }

    // 获取当前系统的 SystemRoot，如 C:\Windows
    let mut buffer = [0u16; 260];
    let len = unsafe { GetWindowsDirectoryW(Some(&mut buffer)) };
    if len == 0 {
        return Err(anyhow!("Get system path failed"));
    }
    let current_system = PathBuf::from(String::from_utf16_lossy(&buffer[..len as usize]));

    let input_path = system_path
        .canonicalize()
        .with_context(|| "canonicalize system path failed")?;
    let current_path = current_system
        .canonicalize()
        .with_context(|| "canonicalize current system path failed")?;

    Ok(input_path != current_path)
}

/// 获取当前系统的根目录（如 C:\Windows）
///
/// # 返回值
/// - `Ok(PathBuf)`: 当前系统的根目录路径
/// - `Err(...)`：失败则返回错误
pub fn get_current_system_root() -> Result<PathBuf> {
    let mut buffer = [0u16; 260];
    let len = unsafe { GetWindowsDirectoryW(Some(&mut buffer)) };
    if len == 0 {
        return Err(anyhow!("Get system path failed"));
    }
    Ok(PathBuf::from(String::from_utf16_lossy(
        &buffer[..len as usize],
    )))
}

/// 判断是否运行在64位系统中
///
/// # 返回值
/// - `Ok(bool)`: 运行在64位系统中
///   - `true`: 运行在64位系统中
///   - `false`: 运行在32位系统中
/// - `Err(...)`：获取系统信息失败
///
/// # 说明
/// - 此函数通过查询当前进程是否为WOW64进程来判断是否运行在64位系统中。
/// - 仅当本进程为 32-bit 编译时才需要判断（64-bit 编译的进程上 IsWow64Process 返回 false）
pub fn is_running_under_wow64() -> Result<bool, windows::core::Error> {
    // 仅当本进程为 32-bit 编译时才需要判断（64-bit 编译的进程上 IsWow64Process 返回 false）
    unsafe {
        let mut wow64: BOOL = BOOL(0);
        IsWow64Process(GetCurrentProcess(), &mut wow64)?;
        Ok(wow64.as_bool())
    }
}

/// 获取当前系统的处理器架构。
///
/// 此函数通过调用 Windows API `GetNativeSystemInfo` 来检索有关当前系统体系结构的信息。
/// 它返回 `wProcessorArchitecture` 字段的值，该值标识处理器架构。
///
/// # 返回值
/// - `u16`: 代表系统处理器架构的数值。常见的值包括：
///   - `0` (PROCESSOR_ARCHITECTURE_INTEL): Intel 或兼容的 x86 架构。
///   - `9` (PROCESSOR_ARCHITECTURE_AMD64): x64 (AMD64) 架构。
///   - `12` (PROCESSOR_ARCHITECTURE_ARM64): ARM64 架构。
///   - 其他值表示其他或未知的架构类型。
pub fn get_native_arch() -> PROCESSOR_ARCHITECTURE {
    let mut sys_info = SYSTEM_INFO::default();
    unsafe {
        GetNativeSystemInfo(&mut sys_info);
        sys_info.Anonymous.Anonymous.wProcessorArchitecture
    }
}

/// 获取离线系统架构
///
/// # 参数
/// - `system_path`: 系统目录
///
/// # 返回值
/// - `Ok(u16)`: PE 文件 Machine 字段
///   - `0x014c` → x86
///   - `0x8664` → x64
///   - `0xAA64` → ARM64
/// - `Err(...)`：读取或解析失败
pub fn get_offline_system_arch(system_path: &Path) -> Result<u16> {
    let krnl_path = system_path
        .join("Windows")
        .join("System32")
        .join("ntoskrnl.exe");
    let bytes = fs::read(&krnl_path).with_context(|| format!("read {:?}", krnl_path))?;
    let pe = PE::parse(&bytes).with_context(|| format!("parse {:?}", krnl_path))?;

    let machine = pe.header.coff_header.machine;
    Ok(machine)
}

/// 查找离线系统盘（跳过当前系统盘）
///
/// # 返回值
/// - `Vec<PathBuf>`: 系统盘列表
pub fn find_offline_system() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    // 获取当前系统盘符
    let current_system_drive = env::var("SystemDrive");

    for letter in b'C'..=b'Z' {
        let drive = format!("{}:\\", letter as char);
        // 跳过当前系统盘
        if let Ok(current_system_drive) = &current_system_drive {
            if drive.eq_ignore_ascii_case(&format!("{}\\", current_system_drive)) {
                continue;
            }
        }
        let path = PathBuf::from(format!("{}\\", drive));
        if path.exists()
            && path
                .join("Windows")
                .join("System32")
                .join("ntoskrnl.exe")
                .exists()
        {
            candidates.push(path.to_path_buf());
        }
    }
    candidates
}

/// 弹出可移动设备（U盘、CDROM设备等）
///
/// # 参数
/// - `drive_path`: 盘符，如 "D:"
///
/// # 返回值
/// - `Ok(())`: 成功弹出设备
/// - `Err(...)`：弹出失败则返回错误
pub fn eject_drive(drive_path: &Path) -> Result<()> {
    // 盘符必须形如 "D:"，我们需要构造设备路径 "\\.\D:"
    let drive_letter = drive_path
        .to_str()
        .ok_or(anyhow!("Invalid drive path"))?
        .chars()
        .take(2)
        .collect::<String>();
    let device_path = format!(r"\\.\{}", drive_letter);

    // Windows API 要求宽字符串，转成 Vec<u16>，并以0结尾
    let device_path_w = HSTRING::from(&device_path);

    // 以读写方式打开设备
    let handle = unsafe {
        CreateFileW(
            PCWSTR(device_path_w.as_ptr()),
            FILE_GENERIC_READ.0 | FILE_GENERIC_WRITE.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None, // lpsecurityattributes: Option<*const SECURITY_ATTRIBUTES>
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None, // htemplatefile: Option<HANDLE>
        )
        .with_context(|| format!("Failed to open device handle for {}", device_path))?
    };

    if handle == INVALID_HANDLE_VALUE {
        return Err(anyhow!("Failed to open device handle"));
    }

    // 调用 DeviceIoControl 发送弹出命令
    let mut bytes_returned: u32 = 0;
    let result = unsafe {
        DeviceIoControl(
            handle,
            IOCTL_STORAGE_EJECT_MEDIA,
            None, // lpinbuffer: Option<*const c_void>
            0,
            None, // lpoutbuffer: Option<*mut c_void>
            0,
            Some(&mut bytes_returned), // lpbytesreturned: Option<*mut u32>
            None,                      // lpoverlapped: Option<*mut OVERLAPPED>
        )
    };

    unsafe {
        CloseHandle(handle).ok();
    }

    result.map_err(|e| e.into())
}

/// 获取设备类型
///
/// # 参数
/// - `drive_path`: 盘符
///
/// # 返回值
/// - `0`: 无法确定驱动器类型。
/// - `1`: 根路径无效;例如，在指定路径上没有装载卷。
/// - `2`: 驱动器具有可移动媒体;例如，软盘驱动器、拇指驱动器或闪存卡读卡器。
/// - `3`: 驱动器具有固定媒体;例如，硬盘驱动器或闪存驱动器。
/// - `4`: 驱动器是远程（网络）驱动器。
/// - `5`: 驱动器是 CD-ROM 驱动器。
/// - `6`: 驱动器是 RAM 磁盘。
pub fn get_drive_type(drive_path: &Path) -> u32 {
    // 传入格式需要类似 "E:\" 的路径，确保最后有反斜杠
    let mut drive_str = drive_path.as_os_str().encode_wide().collect::<Vec<u16>>();
    if !drive_str.ends_with(&[b'\\' as u16]) {
        drive_str.push(b'\\' as u16);
    }
    drive_str.push(0); // 结尾null

    unsafe { GetDriveTypeW(PCWSTR(drive_str.as_ptr())) }
}

/// 获取指定盘符设备的 BusType（返回值为 u8），失败返回 None
///
/// # 参数
/// - `drive_path`: 盘符
///
/// # 返回值
/// - `Some(u32)`: 成功获取 BusType，返回值为 u32 类型
/// - `None`: 获取失败，返回 None
pub fn get_drive_bus(drive_path: &Path) -> Option<u32> {
    // 盘符必须形如 "D:"，我们需要构造设备路径 "\\.\D:"
    let drive_letter = drive_path
        .to_string_lossy()
        .chars()
        .take(2)
        .collect::<String>();
    let device_path = format!(r"\\.\{}", drive_letter);

    // Windows API 要求宽字符串，转成 Vec<u16>，并以0结尾
    let device_path_w = HSTRING::from(&device_path);

    // 以读写方式打开设备
    let handle = unsafe {
        CreateFileW(
            PCWSTR(device_path_w.as_ptr()),
            FILE_GENERIC_READ.0 | FILE_GENERIC_WRITE.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None, // lpsecurityattributes: Option<*const SECURITY_ATTRIBUTES>
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None, // htemplatefile: Option<HANDLE>
        )
        .unwrap()
    };

    if handle == INVALID_HANDLE_VALUE {
        return None;
    }

    let query = STORAGE_PROPERTY_QUERY {
        PropertyId: StorageDeviceProperty,
        QueryType: PropertyStandardQuery,
        ..Default::default()
    };

    let mut buffer = vec![0u8; 512];
    let mut returned = 0u32;

    let ok = unsafe {
        DeviceIoControl(
            handle,
            IOCTL_STORAGE_QUERY_PROPERTY,
            Some(&query as *const _ as _),
            size_of_val(&query) as _,
            Some(buffer.as_mut_ptr() as _),
            buffer.len() as _,
            Some(&mut returned),
            None,
        )
    }
    .is_ok();

    unsafe {
        CloseHandle(handle).ok();
    }

    if !ok {
        return None;
    }

    let desc = unsafe { &*(buffer.as_ptr() as *const STORAGE_DEVICE_DESCRIPTOR) };
    Some(desc.BusType.0 as u32)
}

/// 获取指定盘符设备的总空间
///
/// # 参数
/// - `drive_path`: 盘符
///
/// # 返回值
/// - `Some(u64)`: 返回值单位为字节（返回值 ÷ 1024 ÷ 1024即为MB）
/// - `None`: 获取出错
pub fn get_drive_space(drive_path: &Path) -> Option<u64> {
    // 转换 &Path 为 null 结尾的宽字符 Vec<u16>
    let wide_path = HSTRING::from(drive_path);

    let mut free_bytes_available = 0u64;
    let mut total_number_of_bytes = 0u64;
    let mut total_number_of_free_bytes = 0u64;

    let result = unsafe {
        GetDiskFreeSpaceExW(
            PCWSTR(wide_path.as_ptr()),
            Some(&mut free_bytes_available),
            Some(&mut total_number_of_bytes),
            Some(&mut total_number_of_free_bytes),
        )
    };

    match result {
        Ok(_) => Some(total_number_of_bytes),
        Err(_) => None,
    }
}

/// 将文件大小格式化为可读字节单位（MiB/KiB）
///
/// # 参数
/// - `bytes`: 字节数
///
/// # 返回值
/// - `String` : 可读的字节单位
pub fn format_bytes(bytes: u64) -> String {
    let kb = 1024f64;
    let b = bytes as f64;
    if b >= kb.powi(3) {
        format!("{:.1} GB", b / kb.powi(3))
    } else if b >= kb.powi(2) {
        format!("{:.1} MB", b / kb.powi(2))
    } else if b >= kb {
        format!("{:.1} KB", b / kb)
    } else {
        format!("{} B", bytes)
    }
}

/// 检查 .cat 签名文件是否有效
///
/// # 参数
/// - `file_path`: .cat 签名文件路径
///
/// # 返回值
/// - `bool` : 是否有效
pub fn check_catalog_signature(file_path: &Path) -> bool {
    // 路径预处理
    if !file_path.exists() {
        return false;
    }
    // 获取绝对路径 (处理 .. 或相对路径)
    let abs_path = match file_path.canonicalize() {
        Ok(p) => p,
        Err(_) => return false,
    };
    let path_hstring = HSTRING::from(abs_path.as_path());

    // 构造 WINTRUST_FILE_INFO
    let mut file_info = WINTRUST_FILE_INFO {
        cbStruct: std::mem::size_of::<WINTRUST_FILE_INFO>() as u32,
        pcwszFilePath: PCWSTR(path_hstring.as_ptr()),
        // 注意：hFile 必须是 INVALID_HANDLE_VALUE (表示通过路径验证)
        hFile: INVALID_HANDLE_VALUE,
        pgKnownSubject: ptr::null_mut(),
    };

    // 构造 WINTRUST_DATA
    let mut trust_data = WINTRUST_DATA {
        cbStruct: std::mem::size_of::<WINTRUST_DATA>() as u32,
        pPolicyCallbackData: ptr::null_mut(),
        pSIPClientData: ptr::null_mut(),
        // UI 设置：不弹窗
        dwUIChoice: WTD_UI_NONE,
        // 吊销检查：为了速度暂时禁用 (如需严谨可改为 WTD_REVOKE_WHOLECHAIN)
        fdwRevocationChecks: WTD_REVOKE_NONE,
        // 验证目标：文件
        dwUnionChoice: WTD_CHOICE_FILE,
        // 绑定 file_info
        Anonymous: WINTRUST_DATA_0 {
            pFile: &mut file_info,
        },
        // 动作：验证 (Verify)
        dwStateAction: WTD_STATEACTION_VERIFY,
        hWVTStateData: HANDLE(ptr::null_mut()),
        pwszURLReference: PWSTR(ptr::null_mut()),
        // 默认 Provider Flags
        dwProvFlags: WINTRUST_DATA_PROVIDER_FLAGS(0),
        // 上下文：执行文件/驱动
        dwUIContext: WTD_UICONTEXT_EXECUTE,
        pSignatureSettings: ptr::null_mut(),
    };

    // 获取 Action GUID (Generic Verify V2)
    let mut action_guid: GUID = WINTRUST_ACTION_GENERIC_VERIFY_V2;

    // 第一次调用：执行验证
    let status = unsafe {
        WinVerifyTrust(
            HWND(ptr::null_mut()),
            &mut action_guid,
            &mut trust_data as *mut _ as *mut c_void,
        )
    };

    // 资源清理 (Close)
    // 无论第一次成功与否，只要指定了 StateAction，就必须调用 Close 释放 hWVTStateData 分配的内存
    trust_data.dwStateAction = WTD_STATEACTION_CLOSE;

    unsafe {
        WinVerifyTrust(
            HWND(ptr::null_mut()),
            &mut action_guid,
            &mut trust_data as *mut _ as *mut c_void,
        )
    };

    // 0 表示 ERROR_SUCCESS (签名有效且被信任)
    status == 0
}

/// 判断 CAT 文件是否包含 WHQL 签名
/// 前提：建议先调用 check_catalog_signature 确保签名本身是有效的
///
/// # 参数
/// - `file_path`: .cat 签名文件路径
///
/// # 返回值
/// - `bool` : 是否包含 WHQL 签名
pub fn is_whql_signature(file_path: &Path) -> bool {
    // 读取文件内容
    let bytes = match fs::read(file_path) {
        Ok(b) => b,
        Err(_) => return false,
    };
    // 简单暴力搜索

    let pattern_ascii = b"Microsoft Windows";
    if bytes
        .windows(pattern_ascii.len())
        .any(|window| window == pattern_ascii)
    {
        return true;
    }

    // "Microsoft Windows Hardware Compatibility" 的 UTF-16LE 字节序列
    let pattern_utf16: &[u8] = &[
        0x4D, 0x00, 0x69, 0x00, 0x63, 0x00, 0x72, 0x00, 0x6F, 0x00, 0x73, 0x00, 0x6F, 0x00, 0x66,
        0x00, 0x74, 0x00, 0x20, 0x00, 0x57, 0x00, 0x69, 0x00, 0x6E, 0x00, 0x64, 0x00, 0x6F, 0x00,
        0x77, 0x00, 0x73, 0x00, 0x20, 0x00, 0x48, 0x00, 0x61, 0x00, 0x72, 0x00, 0x64, 0x00, 0x77,
        0x00, 0x61, 0x00, 0x72, 0x00, 0x65, 0x00,
    ];
    if bytes
        .windows(pattern_utf16.len())
        .any(|window| window == pattern_utf16)
    {
        return true;
    }

    false
}

/// 获取文件的版本号。
///
/// 此函数通过 Windows API 查询文件的版本信息，提取其中的数字版本号。
///
/// # 参数
/// - `path`: 文件的路径。
///
/// # 返回值
/// - `Ok(Some((major, minor, build, revision)))`: 如果成功获取到版本号，返回元组 (主版本, 次版本, 构建版本, 修订版本)。
/// - `Ok(None)`: 如果文件没有版本信息，或者无法获取。
/// - `Err(error)`: 如果在读取或解析过程中发生错误。
pub fn get_file_version(path: &Path) -> Option<(u16, u16, u16, u16)> {
    unsafe {
        // 将路径转换为 UTF-16
        let path_hstring: HSTRING = path.as_os_str().into();
        let path_pcwstr = PCWSTR(path_hstring.as_ptr());

        // 获取版本信息块的大小
        let mut _handle: u32 = 0;
        let size = GetFileVersionInfoSizeW(path_pcwstr, Some(&mut _handle));
        if size == 0 {
            return None;
        }

        // 分配缓冲区并获取数据
        let mut buffer = vec![0u8; size as usize];
        if GetFileVersionInfoW(path_pcwstr, Some(0), size, buffer.as_mut_ptr() as *mut _).is_err() {
            return None;
        }

        // 查询固定文件信息 (Fixed File Info)
        let mut sub_block: *mut std::ffi::c_void = std::ptr::null_mut();
        let mut len: u32 = 0;

        // "\\" 表示根块，即 VS_FIXEDFILEINFO
        let root_block: HSTRING = "\\".into();
        if !VerQueryValueW(
            buffer.as_ptr() as *const _,
            PCWSTR(root_block.as_ptr()),
            &mut sub_block,
            &mut len,
        )
        .as_bool()
        {
            return None;
        }

        if len == 0 || sub_block.is_null() {
            return None;
        }

        // 解析结构体
        let info = &*(sub_block as *const VS_FIXEDFILEINFO);

        // dwFileVersionMS: 高 16 位是 Major，低 16 位是 Minor
        // dwFileVersionLS: 高 16 位是 Build，低 16 位是 Revision
        let major = (info.dwFileVersionMS >> 16) as u16;
        let minor = (info.dwFileVersionMS & 0xFFFF) as u16;
        let build = (info.dwFileVersionLS >> 16) as u16;
        let revision = (info.dwFileVersionLS & 0xFFFF) as u16;

        Some((major, minor, build, revision))
    }
}

/// 获取文件的 CRC32 校验值。
///
/// 此函数通过读取文件内容，计算其 CRC32 校验值。
///
/// # 参数
/// - `path`: 文件的路径。
///
/// # 返回值
/// - `Ok(crc32)`: 如果成功计算到 CRC32 值，返回该值。
/// - `Err(error)`: 如果在读取或计算过程中发生错误。
pub fn get_file_crc32(path: &Path) -> std::io::Result<u32> {
    let file = fs::File::open(path)?;
    // 64KB 缓冲区
    let mut reader = BufReader::with_capacity(64 * 1024, file);
    let mut hasher = Hasher::new();
    let mut buffer = [0u8; 64 * 1024];

    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hasher.finalize())
}
