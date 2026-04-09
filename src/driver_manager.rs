use crate::driver_index::InfInfo;
use crate::hardware::enumerate_hardware;
use crate::utils::console::{write_console, ConsoleType};
use crate::utils::drvstore::DriverStore;
use crate::utils::setupapi::SetupAPI;
use crate::utils::sevenzip::SevenZip;
use crate::utils::utils::{
    copy_dir, filetime_to_string, get_current_system_root, get_file_list, get_file_version,
    get_offline_system_arch, is_offline_system,
};
use crate::{command, DEBUG, TEMP_PATH};
use anyhow::Result;
use anyhow::{anyhow, Context};
use rust_i18n::t;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::{env, fs};

pub struct DriverManger {
    driver_store: DriverStore,
}

impl DriverManger {
    pub fn new(system_drive: &Path) -> Result<Self> {
        // 判断是否为离线系统
        let drvstore_path = if is_offline_system(system_drive)
            .with_context(|| "check offline system failed")?
        {
            if DEBUG.load(Ordering::Relaxed) {
                write_console(
                    ConsoleType::Debug,
                    &format!("Offline system drive: {}", system_drive.display()),
                );
            }

            // 获取离线系统版本
            let (offline_major, offline_minor, offline_build, offline_revision) = get_file_version(
                &system_drive
                    .join("Windows")
                    .join("System32")
                    .join("cmd.exe"),
            )
            .ok_or_else(|| anyhow!("get offline sys version failed"))?;
            if DEBUG.load(Ordering::Relaxed) {
                write_console(
                    ConsoleType::Debug,
                    &format!(
                        "Offline system version: {:?}",
                        format!(
                            "{}.{}.{}.{}",
                            offline_major, offline_minor, offline_build, offline_revision
                        )
                    ),
                );
            }

            // 获取当前系统版本
            let (current_major, current_minor, current_build, current_revision) = get_file_version(
                &get_current_system_root()
                    .with_context(|| "get current system root failed")?
                    .join("System32")
                    .join("cmd.exe"),
            )
            .ok_or_else(|| anyhow!("get current sys version failed"))?;
            if DEBUG.load(Ordering::Relaxed) {
                write_console(
                    ConsoleType::Debug,
                    &format!(
                        "Current system version: {:?}",
                        format!(
                            "{}.{}.{}.{}",
                            current_major, current_minor, current_build, current_revision
                        )
                    ),
                );
            }

            // 如果当前是 Win7 (6.1) 或更老，但离线系统是 Win8+ (>= 6.2)则不支持
            if (current_major < 6 || (current_major == 6 && current_minor < 2))
                && (offline_major > 6 || (offline_major == 6 && offline_minor >= 2))
            {
                return Err(anyhow!(t!("not-support-call")));
            }

            // 获取离线系统架构
            let offline_arch = get_offline_system_arch(system_drive)
                .with_context(|| "get offline system arch failed")?;
            if DEBUG.load(Ordering::Relaxed) {
                write_console(
                    ConsoleType::Debug,
                    &format!("Offline system arch: {:?}", offline_arch),
                );
            }

            // 判断系统版本 (以 Win8 / NT 6.2 为分水岭)
            if offline_major < 6 || (offline_major == 6 && offline_minor < 2) {
                // 离线系统是 Win7 (及以下)，为了保证数据库写入兼容性，优先用离线系统的 drvstore.dll
                match (env::consts::ARCH, offline_arch) {
                    ("x86", 0x014c) | ("x86_64", 0x8664) | ("aarch64", 0xAA64) => system_drive
                        .join("Windows")
                        .join("System32")
                        .join("drvstore.dll"),
                    ("x86", 0x8664) | ("aarch64", 0x014c) => system_drive
                        .join("Windows")
                        .join("SysWOW64")
                        .join("drvstore.dll"),
                    (_, _) => return Err(anyhow!(t!("not-support-call"))),
                }
            } else {
                // 离线系统是 Win8, Win10, Win11
                PathBuf::from("drvstore.dll")
            }
        } else {
            // 在线系统
            PathBuf::from("drvstore.dll")
        };

        if DEBUG.load(Ordering::Relaxed) {
            write_console(
                ConsoleType::Debug,
                &format!("Drvstore path: {}", drvstore_path.display()),
            );
        }

        Ok(DriverManger {
            driver_store: DriverStore::new(Some(&drvstore_path))?,
        })
    }

    /// 列举驱动程序
    ///
    /// # 参数
    ///
    /// - `system_drive`: 系统盘路径
    /// - `include_system`: 是否包含系统驱动程序，默认值为 `true`
    /// - `system_only`: 是否仅包含系统驱动程序，默认值为 `false`
    /// - `class`: 驱动程序类名，可选，用于筛选特定类别的驱动程序
    /// - `exclude_class`: 排除的驱动程序类名，可选，用于筛选出不包含指定类别的驱动程序
    /// - `provider`: 驱动程序供应商名，可选，用于筛选出指定供应商的驱动程序
    ///
    /// # 返回值
    ///
    /// * `Ok(())` - 成功列举驱动程序
    /// * `Err(anyhow::Error)` - 列举驱动程序失败
    pub fn list_driver(
        &self,
        system_drive: &Path,
        include_system: bool,
        system_only: bool,
        class: Option<&[String]>,
        exclude_class: Option<&[String]>,
        provider: Option<&[String]>,
    ) -> Result<()> {
        let system_root = system_drive.join("Windows");

        // 打开驱动数据库
        let store_handle = self
            .driver_store
            .open_store(&system_root, system_drive)
            .with_context(|| "Open driver store failed")?;

        // 获取系统架构
        let arch = match get_offline_system_arch(system_drive)
            .with_context(|| "get offline system arch failed")?
        {
            // x86
            0x014c => 0,
            // x64
            0x8664 => 9,
            // ARM64
            0xAA64 => 12,
            _ => {
                return Err(anyhow!(t!("offline-Arch-Err")));
            }
        };

        let mut result = String::new();

        let driver_list = if system_only {
            // 仅系统驱动。获取所有，然后过滤掉 oem*.inf
            get_file_list(&system_root.join("INF"), "*.inf")
                .with_context(|| "Get driver list failed")?
                .into_iter()
                .filter(|path| {
                    !path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.to_lowercase().starts_with("oem"))
                        .unwrap_or(false)
                })
                .collect()
        } else if include_system {
            // 包含系统和第三方。获取所有 *.inf
            get_file_list(&system_root.join("INF"), "*.inf")
                .with_context(|| "Get driver list failed")?
        } else {
            // 默认模式，仅第三方。获取 oem*.inf
            get_file_list(&system_root.join("INF"), "oem*.inf")
                .with_context(|| "Get driver list failed")?
        };
        if driver_list.is_empty() {
            return Err(anyhow!(t!("no-driver-found")));
        }

        // 遍历驱动数据库中的每个驱动程序
        for item in driver_list {
            if let Some((path, _info_opt)) =
                self.driver_store
                    .find_driver_package(store_handle, &item, arch)
            {
                let inf_path = PathBuf::from(&path);

                // 打开驱动
                let driver_handle = match self.driver_store.open_driver(&inf_path, arch) {
                    Ok(handle) => handle,
                    Err(e) => {
                        write_console(
                            ConsoleType::Error,
                            &format!("Open driver failed: {}({})", inf_path.display(), e),
                        );
                        continue;
                    }
                };

                // 获取驱动基本信息
                let driver_info = match self.driver_store.get_version_info(driver_handle) {
                    Ok(info) => info,
                    Err(e) => {
                        write_console(
                            ConsoleType::Error,
                            &format!("Get driver info failed: {}({})", inf_path.display(), e),
                        );
                        continue;
                    }
                };

                // 关闭驱动
                match self.driver_store.close_package(driver_handle) {
                    Ok(_) => {}
                    Err(e) => {
                        write_console(
                            ConsoleType::Error,
                            &format!("Close driver failed: {}({})", inf_path.display(), e),
                        );
                        continue;
                    }
                }

                // 指定驱动类
                if let Some(class) = class
                    && !class
                        .iter()
                        .any(|c| driver_info.class_name.eq_ignore_ascii_case(c))
                {
                    continue;
                }

                // 排除指定驱动类
                if let Some(exclude_class) = exclude_class
                    && exclude_class
                        .iter()
                        .any(|c| driver_info.class_name.eq_ignore_ascii_case(c))
                {
                    continue;
                }

                // 指定驱动厂商
                if let Some(provider) = provider
                    && !provider
                        .iter()
                        .any(|p| driver_info.provider_name.eq_ignore_ascii_case(p))
                {
                    continue;
                }

                let label_w = 18;
                let total_w = label_w + inf_path.display().to_string().len();
                result.push_str("Driver Info:\n");
                result.push_str(&format!("{:-^total_w$}\n", "-", total_w = total_w));
                result.push_str(&format!(
                    "{:<label_w$}{}\n",
                    "Inf Path:",
                    inf_path.display(),
                    label_w = label_w
                ));

                result.push_str(&format!(
                    "{:<label_w$}{}\n",
                    "OEM Name:",
                    item.file_name().unwrap().to_string_lossy(),
                    label_w = label_w
                ));
                result.push_str(&format!(
                    "{:<label_w$}{}\n",
                    "Class Name:",
                    driver_info.class_name,
                    label_w = label_w
                ));
                result.push_str(&format!(
                    "{:<label_w$}{}\n",
                    "Class Desc:",
                    SetupAPI::get_class_description_from_guid(&driver_info.class_guid)
                        .unwrap_or("".to_string()),
                    label_w = label_w
                ));
                result.push_str(&format!(
                    "{:<label_w$}{{{:?}}}\n",
                    "Class GUID:",
                    driver_info.class_guid,
                    label_w = label_w
                ));

                result.push_str(&format!(
                    "{:<label_w$}{}\n",
                    "Provider:",
                    driver_info.provider_name,
                    label_w = label_w
                ));
                result.push_str(&format!(
                    "{:<label_w$}{}\n",
                    "Version:",
                    driver_info.driver_version,
                    label_w = label_w
                ));
                result.push_str(&format!(
                    "{:<label_w$}{}\n",
                    "Date:",
                    filetime_to_string(&driver_info.driver_date)
                        .unwrap_or_else(|e| format!("{:?}", e)),
                    label_w = label_w
                ));
                result.push_str(&format!(
                    "{:<label_w$}{}\n",
                    "Size:",
                    driver_info.size,
                    label_w = label_w
                ));
                result.push('\n');

                println!("{}", result);
            }
        }
        self.driver_store
            .close_store(store_handle)
            .with_context(|| "Close driver store failed")?;

        Ok(())
    }

    /// 导入驱动
    ///
    /// # 参数
    ///
    /// - `systemDrive`: 系统盘路径
    /// - `driverPath`: 驱动路径
    /// - `password`: 驱动密码
    /// - `match_device`: 是否匹配当前设备驱动
    ///
    /// # 返回值
    /// - `Ok(())`: 导入驱动成功
    /// - `Err(e)`: 导入驱动失败
    pub fn import_driver(
        &self,
        system_drive: &Path,
        driver_path: &Path,
        password: Option<&str>,
        match_device: bool,
    ) -> Result<(u32, u32, u32)> {
        let mut real_driver_path = driver_path.to_path_buf();
        let zip = SevenZip::new().with_context(|| "Initialize 7zip failed")?;

        // 判断是否为驱动包
        if driver_path.is_file() {
            if zip.is_driver_package(driver_path, password).is_err() {
                return Err(anyhow!(t!("no-driver-package")));
            }

            let drivers_path = TEMP_PATH.join(driver_path.file_stem().unwrap());
            if match_device {
                // 解压全部INF文件
                if let Err(e) =
                    zip.extract_files_from_path(driver_path, password, "*.inf", &drivers_path)
                {
                    return Err(anyhow!("{}: {}", t!("driver-unzip-failed"), e));
                }
                // 解压全部CAT文件
                let _ = zip.extract_files_from_path(driver_path, password, "*.cat", &drivers_path);
            } else {
                // 解压全部驱动文件
                if let Err(e) =
                    zip.extract_files_from_path(driver_path, password, "*", &drivers_path)
                {
                    return Err(anyhow!("{}: {}", t!("driver-unzip-failed"), e));
                }
            }
            real_driver_path = drivers_path;
        }

        // 遍历INF文件列表
        let mut inf_list = get_file_list(&real_driver_path, "*.inf")
            .with_context(|| "get inf file list failed")?;

        // 匹配当前设备驱动
        if match_device {
            // 获取真实硬件信息
            let hwid_list =
                enumerate_hardware(None, false).with_context(|| "get real id info failed")?;
            if hwid_list.is_empty() {
                return Err(anyhow!(t!("no-device")));
            }

            // 解析INF文件
            let mut inf_info_list: Vec<InfInfo> = Vec::new();
            for inf_path in &inf_list {
                if let Ok(current_info) = InfInfo::parse_inf(&real_driver_path, inf_path)
                    && !current_info.hardware.is_empty()
                {
                    inf_info_list.push(current_info.clone());
                }
            }

            // 匹配驱动
            let match_hardware_and_driver =
                command::match_driver_info(&hwid_list, &inf_info_list, None, None);
            if match_hardware_and_driver.is_empty() {
                return Err(anyhow!(t!("no-found-driver-currently")));
            }

            inf_list.clear();
            for (_hardware, match_info) in match_hardware_and_driver.iter() {
                // 仅匹配第一个最佳的驱动
                if let Some((inf_info_item, _entry)) = match_info.first() {
                    if let Err(e) = zip.extract_files_from_path(
                        driver_path,
                        password,
                        &inf_info_item.path,
                        &real_driver_path,
                    ) {
                        write_console(
                            ConsoleType::Error,
                            &format!("{}: {}", t!("driver-unzip-failed"), e),
                        );
                        continue;
                    }
                    inf_list.push(real_driver_path.join(inf_info_item.path.clone()));
                }
            }
        }

        // 获取系统架构
        let arch = match get_offline_system_arch(system_drive)
            .with_context(|| "get offline system arch failed")?
        {
            // x86
            0x014c => 0,
            // x64
            0x8664 => 9,
            // ARM64
            0xAA64 => 12,
            _ => {
                return Err(anyhow!(t!("offline-Arch-Err")));
            }
        };

        // 计数器
        let mut success_count = 0;
        let mut fail_count = 0;
        let mut total_count = 0;

        let system_root = system_drive.join("Windows");
        if !is_offline_system(system_drive).with_context(|| "check offline system failed")? {
            // 在线导入驱动
            let driverStore =
                DriverStore::new(None).with_context(|| "create driver store failed")?;

            // 打开驱动库
            let handle = driverStore
                .open_store(&system_root, system_drive)
                .with_context(|| "open driver store failed")?;

            // 遍历驱动列表
            for inf_path in inf_list {
                total_count += 1;
                match driverStore.import_driver_to_store(handle, &inf_path, arch, 0) {
                    Ok(_result) => {
                        write_console(
                            ConsoleType::Success,
                            &format!(
                                "{}: {}",
                                &t!("driver-import-success"),
                                inf_path.to_string_lossy(),
                            ),
                        );
                        success_count += 1;
                    }
                    Err(e) => {
                        write_console(
                            ConsoleType::Error,
                            &format!(
                                "{}: {} ({})",
                                &t!("driver-import-failed"),
                                inf_path.to_string_lossy(),
                                e,
                            ),
                        );
                        fail_count += 1;
                    }
                };
            }

            // 关闭驱动库
            driverStore
                .close_store(handle)
                .with_context(|| "close driver store failed")?;
        } else {
            // 离线导入驱动
            for inf_path in inf_list {
                total_count += 1;
                match self.driver_store.offline_add_driver(
                    &inf_path,
                    &system_root,
                    system_drive,
                    0,
                    arch,
                ) {
                    Ok(_result) => {
                        write_console(
                            ConsoleType::Success,
                            &format!(
                                "{}: {}",
                                &t!("driver-import-success"),
                                inf_path.to_string_lossy(),
                            ),
                        );
                        success_count += 1;
                    }
                    Err(e) => {
                        write_console(
                            ConsoleType::Error,
                            &format!(
                                "{}: {} ({})",
                                &t!("driver-import-failed"),
                                inf_path.to_string_lossy(),
                                e,
                            ),
                        );
                        fail_count += 1;
                    }
                };
            }
        }
        Ok((success_count, fail_count, total_count))
    }

    /// 导出驱动
    ///
    /// # 参数
    /// - `system_drive`: 系统盘路径
    /// - `out_path`: 导出路径
    /// - `inf`: 驱动INF文件名称，可选，用于筛选特定的驱动INF文件
    /// - `class`: 驱动类，可选，用于筛选特定的驱动类
    /// - `exclude_class`: 排除的驱动程序类名，可选，用于筛选出不包含指定类别的驱动程序
    /// - `provider`: 驱动程序供应商名，可选，用于筛选出指定供应商的驱动程序
    ///
    /// # 返回值
    /// - `Ok(())`: 导出成功
    /// - `Err()`: 导出失败
    pub fn export_driver(
        &self,
        system_drive: &Path,
        out_path: &Path,
        include_system: bool,
        system_only: bool,
        inf: Option<&str>,
        class: Option<&[String]>,
        exclude_class: Option<&[String]>,
        provider: Option<&[String]>,
    ) -> Result<(u32, u32, u32)> {
        let system_root = system_drive.join("Windows");

        // 获取系统架构
        let arch = match get_offline_system_arch(system_drive)
            .with_context(|| "get offline system arch failed")?
        {
            // x86
            0x014c => 0,
            // x64
            0x8664 => 9,
            // ARM64
            0xAA64 => 12,
            _ => {
                return Err(anyhow!(t!("offline-Arch-Err")));
            }
        };

        let store_handle = self
            .driver_store
            .open_store(&system_root, system_drive)
            .with_context(|| "open driver store failed")?;

        // 计数器
        let mut success_count = 0;
        let mut fail_count = 0;
        let mut total_count = 0;

        let driver_list = if system_only {
            // 仅系统驱动。获取所有然后过滤掉 oem*.inf
            get_file_list(&system_root.join("INF"), "*.inf")
                .with_context(|| "Get driver list failed")?
                .into_iter()
                .filter(|path| {
                    !path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.to_lowercase().starts_with("oem"))
                        .unwrap_or(false)
                })
                .collect()
        } else if include_system {
            // 包含系统和第三方。获取所有 *.inf
            get_file_list(&system_root.join("INF"), "*.inf")
                .with_context(|| "Get driver list failed")?
        } else {
            // 默认模式，仅第三方。获取 oem*.inf
            get_file_list(&system_root.join("INF"), "oem*.inf")
                .with_context(|| "Get driver list failed")?
        };

        // 遍历驱动库
        for item in driver_list {
            if let Some((inf_path, _info_opt)) =
                self.driver_store
                    .find_driver_package(store_handle, &item, arch)
            {
                // 获取驱动基本信息
                let driver_handle = match self.driver_store.open_driver(&inf_path, arch) {
                    Ok(handle) => handle,
                    Err(e) => {
                        write_console(
                            ConsoleType::Warning,
                            &format!("Open driver failed {} : {}", inf_path.display(), e),
                        );
                        fail_count += 1;
                        continue;
                    }
                };

                let driver_info = match self.driver_store.get_version_info(driver_handle) {
                    Ok(info) => info,
                    Err(e) => {
                        write_console(
                            ConsoleType::Warning,
                            &format!("Get driver info failed {}: {}", inf_path.display(), e),
                        );
                        // 尝试关闭驱动包
                        let _ = self.driver_store.close_package(driver_handle);
                        fail_count += 1;
                        continue;
                    }
                };

                if let Err(e) = self.driver_store.close_package(driver_handle) {
                    write_console(
                        ConsoleType::Warning,
                        &format!("Close driver package failed {}: {}", inf_path.display(), e),
                    );
                }

                // 指定驱动名称
                if let Some(name) = inf
                    && !inf_path
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .eq_ignore_ascii_case(name)
                {
                    continue;
                }

                // 指定驱动类
                if let Some(class) = class
                    && !class
                        .iter()
                        .any(|c| driver_info.class_name.eq_ignore_ascii_case(c))
                {
                    continue;
                }

                // 排除指定驱动类
                if let Some(exclude_class) = exclude_class
                    && exclude_class
                        .iter()
                        .any(|c| driver_info.class_name.eq_ignore_ascii_case(c))
                {
                    continue;
                }

                // 指定驱动厂商
                if let Some(provider) = provider
                    && !provider
                        .iter()
                        .any(|p| driver_info.provider_name.eq_ignore_ascii_case(p))
                {
                    continue;
                }

                total_count += 1;

                // 获取驱动类描述
                let class_desc =
                    match SetupAPI::get_class_description_from_guid(&driver_info.class_guid) {
                        Ok(desc) => desc,
                        Err(e) => {
                            write_console(
                                ConsoleType::Error,
                                &format!(
                                    "Get class description failed {}: {}",
                                    inf_path.display(),
                                    e
                                ),
                            );
                            fail_count += 1;
                            continue;
                        }
                    };

                // 导出驱动
                let output_path = out_path
                    .join(class_desc.clone())
                    .join(inf_path.parent().unwrap().file_name().unwrap());

                if let Err(e) = fs::create_dir_all(&output_path) {
                    write_console(
                        ConsoleType::Error,
                        &format!("Create output path failed {}: {}", output_path.display(), e),
                    );
                    fail_count += 1;
                    continue;
                }

                match copy_dir(inf_path.parent().unwrap(), &output_path) {
                    Ok(_) => {
                        // 输出成功导出的驱动信息
                        let inf_name = inf_path.file_name().unwrap().to_string_lossy();
                        write_console(
                            ConsoleType::Success,
                            &t!(
                                "driver-export-success",
                                file = inf_name,
                                class = driver_info.class_name,
                                version = driver_info.driver_version.to_string(),
                                date = filetime_to_string(&driver_info.driver_date)
                                    .unwrap_or_else(|e| format!("{:?}", e)),
                            ),
                        );
                        success_count += 1;
                    }
                    Err(e) => {
                        write_console(
                            ConsoleType::Error,
                            &format!("Copy driver failed {}: {}", inf_path.display(), e),
                        );
                        fail_count += 1;
                    }
                }
            }
        }

        // 没有找到符合条件的驱动
        if total_count == 0 {
            return Err(anyhow!(t!("driver-export-not-found")));
        }

        Ok((success_count, fail_count, total_count))
    }

    /// 删除系统中的驱动
    /// # 参数
    /// - `systemDrive`: 系统盘
    /// - `inf`: 驱动INF文件名称（可选，None则删除所有驱动）
    /// - `class`: 驱动类别（可选，None则删除所有驱动）
    /// - `exclude_class`: 排除的驱动程序类名，可选，用于筛选出不包含指定类别的驱动程序
    /// - `provider`: 驱动程序供应商名，可选，用于筛选出指定供应商的驱动程序
    ///  - `all`: 是否删除所有驱动（可选，默认false）
    pub fn remove_driver(
        &self,
        system_drive: &Path,
        include_system: bool,
        system_only: bool,
        inf: Option<&str>,
        class: Option<&[String]>,
        provider: Option<&[String]>,
        all: bool,
    ) -> Result<(u32, u32, u32)> {
        let system_root = system_drive.join("Windows");

        // 获取系统架构
        let arch = match get_offline_system_arch(system_drive)
            .with_context(|| "get offline system arch failed")?
        {
            // x86
            0x014c => 0,
            // x64
            0x8664 => 9,
            // ARM64
            0xAA64 => 12,
            _ => {
                return Err(anyhow!(t!("offline-Arch-Err")));
            }
        };

        // 打开驱动库
        let store_handle = self
            .driver_store
            .open_store(&system_root, system_drive)
            .with_context(|| "Open driver store failed")?;

        // 计数器
        let mut success_count = 0;
        let mut fail_count = 0;
        let mut total_count = 0;

        let driver_list = if system_only {
            // 仅系统驱动。获取所有然后过滤掉 oem*.inf
            get_file_list(&system_root.join("INF"), "*.inf")
                .with_context(|| "Get driver list failed")?
                .into_iter()
                .filter(|path| {
                    !path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.to_lowercase().starts_with("oem"))
                        .unwrap_or(false)
                })
                .collect()
        } else if include_system {
            // 包含系统和第三方。获取所有 *.inf
            get_file_list(&system_root.join("INF"), "*.inf")
                .with_context(|| "Get driver list failed")?
        } else {
            // 默认模式，仅第三方。获取 oem*.inf
            get_file_list(&system_root.join("INF"), "oem*.inf")
                .with_context(|| "Get driver list failed")?
        };

        // 遍历所有驱动
        for item in driver_list {
            if let Some((inf_path, _info_opt)) =
                self.driver_store
                    .find_driver_package(store_handle, &item, arch)
            {
                // 获取驱动基本信息
                let driver_handle = self
                    .driver_store
                    .open_driver(&inf_path, arch)
                    .with_context(|| "Open driver failed")?;
                let driver_info = self
                    .driver_store
                    .get_version_info(driver_handle)
                    .with_context(|| "Get driver info failed")?;

                // 指定驱动名称
                if let Some(name) = inf
                    && !inf_path
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .eq_ignore_ascii_case(name)
                {
                    continue;
                }

                // 指定驱动类
                if let Some(class) = class
                    && !class
                        .iter()
                        .any(|c| driver_info.class_name.eq_ignore_ascii_case(c))
                {
                    continue;
                }

                // 指定驱动厂商
                if let Some(provider) = provider
                    && !provider
                        .iter()
                        .any(|p| driver_info.provider_name.eq_ignore_ascii_case(p))
                {
                    continue;
                }

                // 是否删除所有驱动
                if !all && inf.is_none() && class.is_none() && provider.is_none() {
                    continue;
                }

                total_count += 1;

                if is_offline_system(Path::new(system_drive))
                    .with_context(|| "Check offline system failed")?
                {
                    // 离线删除驱动
                    match self.driver_store.offline_delete_driver(
                        &inf_path,
                        &system_root,
                        system_drive,
                        0,
                    ) {
                        Ok(_) => {
                            success_count += 1;
                            write_console(
                                ConsoleType::Success,
                                &format!(
                                    "{}: {}",
                                    &t!("driver-remove-success"),
                                    inf_path.file_name().unwrap().to_string_lossy()
                                ),
                            );
                        }
                        Err(_) => {
                            fail_count += 1;
                            write_console(
                                ConsoleType::Error,
                                &format!("{}: {}", &t!("driver-remove-failed"), inf_path.display()),
                            );
                        }
                    }
                } else {
                    // 在线删除驱动
                    match self
                        .driver_store
                        .delete_driver(store_handle, &inf_path, 0)
                        .with_context(|| "Delete driver failed")
                    {
                        Ok(_) => {
                            success_count += 1;
                            write_console(
                                ConsoleType::Success,
                                &format!(
                                    "{}: {}",
                                    &t!("driver-remove-success"),
                                    inf_path.display()
                                ),
                            );
                        }
                        Err(_) => {
                            fail_count += 1;
                            write_console(
                                ConsoleType::Error,
                                &format!("{}: {}", &t!("driver-remove-failed"), inf_path.display()),
                            );
                        }
                    }
                }
            }
        }

        // 没有找到符合条件的驱动
        if total_count == 0 {
            return Err(anyhow!(t!("driver-remove-not-found")));
        }

        Ok((success_count, fail_count, total_count))
    }
}
