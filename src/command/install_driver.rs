use crate::command::check_if_bundled;
use crate::driver_index::{DriverArch, DriverIndex, HardwareEntry, InfInfo};
use crate::hardware::{enumerate_hardware, update_driver_for_plug_and_play_devices, HardwareInfo};
use crate::utils::console::{write_console, ConsoleType};
use crate::utils::setupapi::SetupAPI;
use crate::utils::sevenzip::SevenZip;
use crate::utils::utils::{compare_version, find_offline_system, get_file_list, get_native_arch};
use crate::{DEBUG, TEMP_PATH};
use anyhow::{anyhow, Context, Result};
use rust_i18n::t;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::time::UNIX_EPOCH;
use threadpool::ThreadPool;
use windows::Win32::Foundation::ERROR_NO_MORE_ITEMS;
use windows::Win32::System::SystemInformation::{
    PROCESSOR_ARCHITECTURE_AMD64, PROCESSOR_ARCHITECTURE_ARM, PROCESSOR_ARCHITECTURE_ARM64,
    PROCESSOR_ARCHITECTURE_IA64, PROCESSOR_ARCHITECTURE_INTEL,
};
use windows_version::OsVersion;

pub struct DriverInstaller {
    zip: SevenZip,
}

impl DriverInstaller {
    pub fn new() -> Self {
        Self {
            zip: SevenZip::new().expect("Create SevenZip instance failed"),
        }
    }

    /// 加载驱动包。支持驱动包路径、驱动路径
    ///
    /// # 参数
    /// - `driverPackPath` - 驱动包路径
    /// - `password` - 驱动包密码
    /// - `indexPath` - 索引文件路径
    /// - `isAllDevice` - 是否为精确匹配
    /// - `driveClass` - 驱动类别
    /// - `extractPath` - 释放路径
    ///
    /// # 返回值
    /// - `Result<()>` - 加载驱动结果
    pub fn install_driver(
        &self,
        driver_pack_path: &Path,
        password: Option<&str>,
        config: Option<&Path>,
        skip_verify: bool,
        missing_only: bool,
        class: Option<&[String]>,
        exclude_class: Option<&[String]>,
        user_extract_path: Option<&Path>,
        force: bool,
    ) -> Result<()> {
        // 当前临时驱动解压路径
        let extract_path = if driver_pack_path.is_dir() {
            driver_pack_path.to_path_buf()
        } else {
            TEMP_PATH.join(driver_pack_path.file_stem().unwrap())
        };

        // 索引文件路径
        let config_path = if let Some(config) = config {
            Some(config.to_path_buf())
        } else {
            self.find_config(driver_pack_path)
        };

        // 解析索引文件
        let config = match config_path {
            Some(config_path) => {
                if let Ok(config) = DriverIndex::from_path(&config_path) {
                    // 校验是否为自解压驱动包
                    if check_if_bundled().is_some() {
                        config
                    } else {
                        // 索引文件解析成功，如果不跳过校验且校验失败，则重新构建索引文件校
                        if !skip_verify && config.check_config(driver_pack_path).is_err() {
                            // 驱动包与索引文件不匹配，即时建立索引文件
                            write_console(ConsoleType::Warning, &t!("driver-not-match-config"));
                            write_console(ConsoleType::Info, &t!("create-index-info"));
                            self.build_config(driver_pack_path, password, &extract_path)?
                        } else {
                            // 校验通过或跳过校验，加载索引文件
                            write_console(
                                ConsoleType::Info,
                                &format!("{}: {}", t!("load-index"), config_path.display()),
                            );
                            config
                        }
                    }
                } else {
                    // 索引文件解析失败，即时建立索引文件
                    write_console(ConsoleType::Warning, &t!("config-parse-failed"));
                    write_console(ConsoleType::Info, &t!("create-index-info"));
                    self.build_config(driver_pack_path, password, &extract_path)?
                }
            }
            None => {
                // 即时建立索引文件
                write_console(ConsoleType::Info, &t!("create-index-info"));
                self.build_config(driver_pack_path, password, &extract_path)?
            }
        };

        let mut total_list: Vec<HardwareInfo> = Vec::new();

        // 3次匹配，避免部分驱动安装不全
        for scan_count in 0..3 {
            if DEBUG.load(Ordering::Relaxed) {
                write_console(
                    ConsoleType::Debug,
                    &format!("Start scan {}", scan_count + 1),
                );
            }

            // 扫描以发现新的硬件
            SetupAPI::rescan();

            // 获取硬件信息
            if DEBUG.load(Ordering::Relaxed) {
                write_console(ConsoleType::Debug, "Get hardware info");
            }
            let hwid_list = enumerate_hardware(None, missing_only)
                .with_context(|| "Get hardware info failed")?;
            if DEBUG.load(Ordering::Relaxed) {
                write_console(
                    ConsoleType::Debug,
                    &format!("Found {} devices", hwid_list.len()),
                );
            }
            if hwid_list.is_empty() {
                // 没有需要安装驱动的设备
                return Err(anyhow!(t!("no-found-driver-currently")));
            }

            // 过滤前一次安装的硬件信息
            let hwid_list: Vec<HardwareInfo> = hwid_list
                .into_iter()
                .filter(|item| !total_list.contains(item))
                .collect();

            // 硬件信息为空，当前没有需要安装驱动的设备
            if hwid_list.is_empty() {
                break;
            }

            // 合并当前扫描的硬件信息
            total_list.extend(hwid_list.iter().cloned());

            // 匹配硬件设备和驱动信息
            if DEBUG.load(Ordering::Relaxed) {
                write_console(ConsoleType::Debug, "Match hardware info");
            }
            let mut match_hardware_and_driver =
                match_driver_info(&hwid_list, &config.drivers, class, exclude_class);

            // 由于存在多个设备匹配到同一个硬件ID的情况（但设备实例不同），而 UpdateDriverForPlugAndPlayDevices 需要提供硬件id而不是设备实例
            // 故需要去重（保留第一个出现的 HWID，删除后续相同的 HWID项目）
            let mut seen_hwids = HashSet::new();
            match_hardware_and_driver.retain(|(device, _)| {
                if let Some(primary_hwid) = device.hardware_id.first() {
                    seen_hwids.insert(primary_hwid.clone())
                } else {
                    false
                }
            });

            if match_hardware_and_driver.is_empty() {
                if scan_count == 0 {
                    return Err(anyhow!(t!("no-found-driver-currently")));
                }
                continue;
            }
            write_console(
                ConsoleType::Info,
                &t!("found-devices", total = match_hardware_and_driver.len()),
            );

            // 创建线程池（池大小以可用 CPU 核心数为准）
            let pool = ThreadPool::new(num_cpus::get());
            let (tx, rx) = channel();

            // 循环匹配信息
            for (hardware, driver_info) in match_hardware_and_driver {
                // 当前状态：一个设备中有一个或多个驱动

                // 调试模式下输出匹配信息
                if DEBUG.load(Ordering::Relaxed) {
                    write_console(
                        ConsoleType::Debug,
                        &format!(
                            "Match info:\n            Name: {}\n            Instance:{}\n            HWID: {}\n            Driver:\n            {}",
                            hardware.name,
                            hardware.device_instance_path,
                            hardware.hardware_id.join(","),
                            driver_info
                                .iter()
                                .map(|(inf_info, _entry)| inf_info.path.as_str())
                                .collect::<Vec<&str>>()
                                .join("\n            "),
                        ),
                    );
                }

                let driver_pack_path = driver_pack_path.to_path_buf();
                let password = password.map(|password| password.to_string());
                let only_extract = user_extract_path.is_some();
                let drivers_path = match user_extract_path {
                    None => extract_path.clone(),
                    Some(path) => PathBuf::from(path),
                };
                // 克隆硬件信息和驱动匹配信息用于线程
                let hardware = hardware.clone();
                let match_info: Vec<(InfInfo, HardwareEntry)> = driver_info
                    .iter()
                    .map(|(inf, entry)| ((*inf).clone(), (*entry).clone()))
                    .collect();
                let tx = tx.clone();
                let zip = self.zip.clone();

                // 为每个需要安装驱动的设备分配一个线程
                pool.execute(move || {
                    // 遍历匹配的驱动
                    for (index, (inf_info_item, entry)) in match_info.iter().enumerate() {
                        // 判断驱动包是否需要解压
                        let inf_path = if driver_pack_path.is_file() {
                            // 获取解压路径（相对于解压所有INF文件的路径）
                            let extract_path = Path::new(inf_info_item.path.as_str())
                                .parent()
                                .expect("get extract path failed");

                            if let Err(e) = zip.extract_files_from_path(
                                &driver_pack_path,
                                password.as_deref(),
                                &extract_path.to_string_lossy(),
                                &drivers_path,
                            ) {
                                // 解压失败
                                if index == match_info.len() - 1 {
                                    // 最后一个驱动，返回失败
                                    tx.send((
                                        hardware,
                                        Err(anyhow!("{}: {}", t!("driver-unzip-failed"), e)),
                                    ))
                                    .expect("send result failed");
                                    return;
                                }
                                // 继续解压下一驱动
                                if DEBUG.load(Ordering::Relaxed) {
                                    write_console(
                                        ConsoleType::Debug,
                                        &format!("Extract failed: {}", extract_path.display()),
                                    );
                                };
                                continue;
                            };

                            // 仅解压驱动文件，返回成功
                            if only_extract {
                                tx.send((hardware, Ok((inf_info_item.clone(), entry.clone()))))
                                    .expect("send result failed");
                                return;
                            }

                            // 获取INF路径
                            let inf_path = drivers_path.join(&inf_info_item.path);
                            if !inf_path.exists() {
                                // INF文件不存在
                                if DEBUG.load(Ordering::Relaxed) {
                                    write_console(
                                        ConsoleType::Debug,
                                        &format!("Driver file not found: {}", inf_path.display()),
                                    );
                                };
                                if index == match_info.len() - 1 {
                                    // 最后一个驱动，返回失败
                                    tx.send((
                                        hardware,
                                        Err(anyhow!(
                                            "Driver file not found: {}",
                                            inf_path.display()
                                        )),
                                    ))
                                    .expect("send result failed");
                                    return;
                                }
                                continue;
                            }

                            inf_path
                        } else {
                            // 驱动文件指定路径
                            drivers_path.join(&inf_info_item.path)
                        };

                        // 安装驱动
                        if let Some(hwid) = hardware.hardware_id.first() {
                            if DEBUG.load(Ordering::Relaxed) {
                                write_console(
                                    ConsoleType::Debug,
                                    &format!("Install driver: {}", inf_path.display()),
                                );
                            }
                            match update_driver_for_plug_and_play_devices(hwid, &inf_path, force) {
                                Ok(()) => {
                                    // 安装驱动成功
                                    tx.send((hardware, Ok((inf_info_item.clone(), entry.clone()))))
                                        .expect("send result failed");
                                    return;
                                }
                                Err(e) => {
                                    // 安装驱动失败，继续加载下一驱动
                                    if DEBUG.load(Ordering::Relaxed) {
                                        write_console(
                                            ConsoleType::Debug,
                                            &format!(
                                                "Install driver failed: {}({})",
                                                inf_info_item.path, e
                                            ),
                                        );
                                    };
                                    if index == match_info.len() - 1 {
                                        // 最后一个驱动也返回失败
                                        if e == ERROR_NO_MORE_ITEMS.into() {
                                            // 函数找到了 HardwareId 值的匹配项，但指定的驱动程序不是比当前驱动程序更好的匹配项
                                            // 忽略当前设备
                                            write_console(
                                                ConsoleType::Info,
                                                &t!(
                                                    "install-skipped",
                                                    name = hardware.name.clone()
                                                ),
                                            );
                                            return;
                                        }
                                        tx.send((hardware, Err(e.into())))
                                            .expect("send result failed");
                                        return;
                                    }
                                    continue;
                                }
                            }
                        } else {
                            if DEBUG.load(Ordering::Relaxed) {
                                write_console(
                                    ConsoleType::Debug,
                                    &format!("No hardware ID found for: {}", inf_info_item.path),
                                );
                            }
                            // 没有硬件ID，返回失败
                            tx.send((
                                hardware,
                                Err(anyhow!("No hardware ID found for: {}", inf_info_item.path)),
                            ))
                            .expect("send result failed");
                            return;
                        }
                    }

                    // 没有找到合适的驱动
                    tx.send((hardware, Err(anyhow!("No driver found"))))
                        .expect("send result failed");
                });
            }

            // 等待所有线程执行完成
            drop(tx); // 关闭发送端

            // 在主线程中进行消息格式化和输出
            for (hardware, result) in rx.iter() {
                match result {
                    Ok((inf_info_item, entry)) => {
                        write_console(
                            ConsoleType::Success,
                            &t!(
                                "install-success",
                                class = inf_info_item.class,
                                name = hardware.name,
                                desc = entry.desc,
                                id = hardware.hardware_id.first().unwrap_or(&"".to_string()),
                                driver = inf_info_item.path,
                                version = inf_info_item.version,
                                date = inf_info_item.date
                            ),
                        );
                    }
                    Err(e) => {
                        write_console(
                            ConsoleType::Error,
                            &t!(
                                "install-failed",
                                name = hardware.name,
                                id = hardware.hardware_id.first().unwrap_or(&"".to_string()),
                                info = e
                            ),
                        );
                    }
                }
            }
        }
        Ok(())
    }

    /// 加载离线系统中的驱动
    ///
    /// # 参数
    /// - `system_drive`: 系统盘（可选，None则全盘搜索[排除当前系统盘]）
    /// - `match_all`: 是否匹配全部设备（默认匹配未安装驱动的设备）
    /// - `drive_class`: 驱动类别（可选，None则加载所有驱动）
    /// - `exclude_class`: 排除的驱动类别（可选，None则不排除）
    ///
    /// # 返回值
    /// - `Ok(())`: 成功加载驱动
    /// - `Err(...)`: 加载驱动失败
    pub fn load_offline_driver(
        &self,
        system_drive: Option<&Path>,
        missing_only: bool,
        class: Option<&[String]>,
        exclude_class: Option<&[String]>,
    ) -> Result<()> {
        if let Some(system_drive) = system_drive {
            let driver_path = system_drive
                .join("Windows")
                .join("System32")
                .join("DriverStore")
                .join("FileRepository");
            if !driver_path.exists() {
                return Err(anyhow!("path-not-exist"));
            }
            write_console(
                ConsoleType::Info,
                &t!(
                    "install-offline-driver",
                    path = driver_path.to_string_lossy().to_string()
                ),
            );
            return self.install_driver(
                &driver_path,
                None,
                None,
                true,
                missing_only,
                class,
                exclude_class,
                None,
                false,
            );
        }

        // 未指定系统盘，全盘搜索离线系统驱动
        let offline_system_drive_list = find_offline_system();

        // 未找到离线系统
        if offline_system_drive_list.is_empty() {
            return Err(anyhow!(t!("not-found-offline-system")));
        }

        // 遍历离线系统加载驱动
        for system_drive in offline_system_drive_list {
            write_console(
                ConsoleType::Info,
                &t!(
                    "install-offline-driver",
                    path = system_drive.to_string_lossy().to_string()
                ),
            );
            self.install_driver(
                &system_drive,
                None,
                None,
                true,
                missing_only,
                class,
                exclude_class,
                None,
                false,
            )?;
        }
        Ok(())
    }

    /// 查找配置文件
    ///
    /// # 参数
    /// - `driver_path`: 驱动路径
    /// - `password`: 驱动包密码（可选）
    /// - `extract_path`: 解压路径
    ///
    /// # 返回值
    /// - `Some(PathBuf)`: 找到的索引文件路径
    /// - `None`: 未找到索引文件
    fn find_config(&self, driver_path: &Path) -> Option<PathBuf> {
        // 检测同目录下的索引文件
        if let Some(parent) = driver_path.parent() {
            let same_config = parent.join(format!(
                "{}.index",
                driver_path.file_stem().unwrap().to_string_lossy()
            ));
            if same_config.exists() {
                return Some(same_config);
            }
        }

        None
    }

    /// 即时创建配置，跳过解析失败的INF文件
    ///
    /// # 参数
    /// - `driver_pack_path` - 驱动包路径
    /// - `password` - 驱动包密码（可选）
    /// - `extract_path` - 解压路径
    ///
    /// # 返回值
    /// - `Ok(Config)` - 成功创建配置
    /// - `Err(...)` - 创建配置失败
    fn build_config(
        &self,
        driver_pack_path: &Path,
        password: Option<&str>,
        extract_path: &Path,
    ) -> Result<DriverIndex> {
        let drivers_path = if driver_pack_path.is_file() {
            // 解压全部 INF 文件
            if let Err(_e) =
                self.zip
                    .extract_files_from_path(driver_pack_path, password, "*.inf", extract_path)
            {
                return Err(anyhow!(t!("driver-unzip-failed")));
            }
            extract_path
        } else {
            driver_pack_path
        };

        // 解压全部 CAT 文件
        let _ = self
            .zip
            .extract_files_from_path(driver_pack_path, password, "*.cat", extract_path);

        // 列出INF文件
        let inf_list = get_file_list(drivers_path, "*.inf")?;
        if inf_list.is_empty() {
            return Err(anyhow!(t!("no-driver-package")));
        }

        // 创建线程池（池大小以可用 CPU 核心数为准）
        let pool = ThreadPool::new(num_cpus::get());

        // 通道，用于收集每个线程的 InfInfo
        let (tx, rx) = channel();
        let base_path = Arc::new(drivers_path.to_path_buf());

        let success_count = Arc::new(AtomicI32::new(0));
        let error_count = Arc::new(AtomicI32::new(0));

        // 遍历INF文件
        for inf_file in inf_list.into_iter() {
            let tx = tx.clone();
            let base_path = Arc::clone(&base_path);
            let success_count = Arc::clone(&success_count);
            let error_count = Arc::clone(&error_count);

            pool.execute(move || {
                // 解析INF文件，解析失败的INF将自动跳过
                match InfInfo::parse_inf(&base_path, &inf_file) {
                    Ok(inf_info) => {
                        // 增加成功计数
                        success_count.fetch_add(1, Ordering::Relaxed);
                        // 发送到主线程
                        tx.send(inf_info).expect("Send inf_info failed");
                    }
                    Err(e) => {
                        write_console(
                            ConsoleType::Warning,
                            &format!(
                                "{}: {} ({})",
                                t!("inf-parse-error"),
                                inf_file
                                    .to_string_lossy()
                                    .trim_start_matches(&*TEMP_PATH.to_string_lossy()),
                                e
                            ),
                        );
                        // 增加错误计数
                        error_count.fetch_add(1, Ordering::Relaxed);
                    }
                }
            });
        }

        drop(tx);
        let inf_info_list = rx.into_iter().collect::<Vec<_>>();
        if DEBUG.load(Ordering::Relaxed) {
            write_console(
                ConsoleType::Debug,
                &format!(
                    "Build index: {}/{}",
                    success_count.load(Ordering::Relaxed),
                    inf_info_list.len()
                ),
            );
        }
        if inf_info_list.is_empty() {
            return Err(anyhow!(t!("create-index-failed")));
        }

        let timestamp = driver_pack_path
            .metadata()
            .with_context(|| format!("get drive path metadata {:?}", driver_pack_path))?
            .modified()
            .with_context(|| format!("get drive path modified {:?}", driver_pack_path))?
            .duration_since(UNIX_EPOCH)
            .with_context(|| {
                format!(
                    "get drive path duration since unix epoch {:?}",
                    driver_pack_path
                )
            })?
            .as_secs();

        Ok(DriverIndex::new(
            driver_pack_path
                .metadata()
                .with_context(|| "Get driver pack path metadata failed")?
                .len(),
            timestamp,
            None,
            inf_info_list,
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum MatchType {
    /// 最强：设备硬件ID - INF 硬件ID
    HardwareToHardware = 0x0,
    /// 强：设备兼容ID - INF 硬件ID
    CompatibleToHardware = 0x1,
    /// 弱：设备硬件ID - INF 兼容ID
    HardwareToCompatible = 0x2,
    /// 最弱：设备兼容ID - INF 兼容ID
    CompatibleToCompatible = 0x3,
}

/// 候选驱动结构体：包含排序所需的所有因子
#[derive(Debug)]
struct DriverCandidate<'a> {
    /// INF 驱动信息
    inf: &'a InfInfo,
    /// 硬件条目
    entry: &'a HardwareEntry,
    /// 排名 (0xSSGGTHHH)
    rank: u32,
    /// 类优先级：1 = Base (Media/Net等), 0 = Extension/SoftwareComponent
    /// 用于确保主驱动排在扩展驱动前面
    class_priority: u8,
}

/// 获取匹配驱动的信息
///
/// # 参数
/// - `idInfo` - 硬件ID列表
/// - `infInfoList` - INF驱动信息列表
/// - `driveClass` - 驱动类别
///
/// # 匹配规则
///
/// 1. 匹配当前系统架构
/// 2. 匹配当前操作系统版本
/// 3. 匹配当前设备的硬件ID
/// 4. 匹配当前设备的兼容ID
///
/// # 排序规则
/// 1. 签名状态（（微软签名 > 其他签名 > 未签名））
/// 2. 匹配分数（最强优先）
/// 3. 驱动日期（最新优先）
/// 4. 驱动版本（最新优先）
///
/// # 参考
/// - [Windows 如何对驱动程序包进行排名](https://learn.microsoft.com/zh-cn/windows-hardware/drivers/install/how-windows-ranks-driver-packages/)
/// - [Windows驱动匹配详解](https://www.cnblogs.com/glacierh/p/5738232.html)
///
/// # 返回值
/// - `Vec<(HwID, Vec<InfInfo>)>` - 匹配驱动信息列表
pub fn match_driver_info<'a>(
    hardware_info_list: &'a [HardwareInfo],
    inf_info_list: &'a [InfInfo],
    class_filter: Option<&[String]>,
    class_exclude: Option<&[String]>,
) -> Vec<(&'a HardwareInfo, Vec<(&'a InfInfo, &'a HardwareEntry)>)> {
    // 当前系统架构
    let current_arch = match get_native_arch() {
        // x86
        PROCESSOR_ARCHITECTURE_INTEL => DriverArch::NTx86,
        // x64
        PROCESSOR_ARCHITECTURE_AMD64 => DriverArch::NTamd64,
        // ARM64
        PROCESSOR_ARCHITECTURE_ARM64 => DriverArch::NTarm64,
        // IA64
        PROCESSOR_ARCHITECTURE_IA64 => DriverArch::NTia64,
        // ARM
        PROCESSOR_ARCHITECTURE_ARM => DriverArch::NTarm,
        // 其他架构
        _ => DriverArch::Nt,
    };

    // 获取当前操作系统版本信息
    let version = OsVersion::current();
    let current_os_version = format!("{}.{}.{}", version.major, version.minor, version.build);

    let mut results = Vec::new();

    // 2. 遍历每一个设备
    for device in hardware_info_list {
        let mut candidates: Vec<DriverCandidate> = Vec::new();

        // 3. 遍历每一个 INF
        for inf_info in inf_info_list {
            // [Filter] 类别筛选 (Class)
            if let Some(cls) = class_filter {
                if !cls.iter().any(|c| inf_info.class.eq_ignore_ascii_case(c)) {
                    continue;
                }
            }

            // [Exclude] 类别排除 (Exclude Class)
            if let Some(exclude_cls) = class_exclude {
                if exclude_cls
                    .iter()
                    .any(|c| inf_info.class.eq_ignore_ascii_case(c))
                {
                    continue;
                }
            }

            // 寻找该 INF 内部针对此设备的最佳条目
            // 一个 INF 可能有多个 Entry 匹配同一个硬件ID，找出 Rank 数值最小（最好）的作为该 INF 的代表。
            let mut best_candidate_in_inf: Option<DriverCandidate> = None;

            for entry in &inf_info.hardware {
                // [Filter] 架构筛选 (Arch)
                // 必须匹配当前系统架构，或者驱动是通用架构(如果业务允许)
                if entry.arch != current_arch && entry.arch != DriverArch::Nt {
                    continue;
                }

                // [Filter] 系统版本筛选 (OS Version)
                if !is_os_compatible(&entry.min_os_version, &current_os_version) {
                    continue;
                }

                // 计算匹配分量 (T 和 HHH)
                let (match_type, hhh) = match calculate_id_score(device, entry) {
                    Some(score) => score,
                    None => continue, // 没匹配上，跳过
                };

                // 组装 Rank (0xSSGGTHHH)
                // SS: 签名得分 (0x00 - 0xFF)
                let ss = inf_info.signature as u32;
                // GG: 功能得分 (0x00 - 0xFF)
                let gg = entry.feature_score as u32;
                // T: 匹配类型 (0x0 - 0x3)
                let t = match_type as u32;
                // HHH: ID列表索引 (0x000 - 0xFFF)
                let hhh_val = (hhh as u32).min(0xFFF);

                let current_rank = (ss << 24) | (gg << 16) | (t << 12) | hhh_val;

                // 更新本 INF 的最佳记录（注意：Rank 越小越好！）
                match best_candidate_in_inf {
                    None => {
                        best_candidate_in_inf = Some(DriverCandidate {
                            inf: inf_info,
                            entry,
                            class_priority: if is_extension_driver(inf_info) { 0 } else { 1 },
                            rank: current_rank,
                        });
                    }
                    Some(ref best) => {
                        if current_rank < best.rank {
                            best_candidate_in_inf = Some(DriverCandidate {
                                inf: inf_info,
                                entry,
                                class_priority: if is_extension_driver(inf_info) { 0 } else { 1 },
                                rank: current_rank,
                            });
                        }
                    }
                }
            }

            // 如果该 INF 中找到了匹配项，将其加入总候选池
            if let Some(candidate) = best_candidate_in_inf {
                candidates.push(candidate);
            }
        }

        // [Sort] 驱动排序逻辑 (Tie-Breaker)
        candidates.sort_by(|a, b| {
            // 1. [Class Priority] 确保 Base 驱动在 Extension 之前（防止扩展驱动被误装为主驱动）
            // b.cmp(a) 是降序 (Base(1) > Extension(0))
            b.class_priority
                .cmp(&a.class_priority)
                // 2. [Rank] 越小越好 (Ascending)
                // 0x00000000 (WHQL+完美匹配) 优于 0xFF...
                .then_with(|| a.rank.cmp(&b.rank))
                // 3. [Date] 越新越好 (Descending)
                .then_with(|| b.inf.date.cmp(&a.inf.date))
                // 4. [Version] 越高越好 (Descending)
                .then_with(|| b.inf.version.cmp(&a.inf.version))
        });

        // 提取排序后的 INF 引用
        let sorted_infs: Vec<(&'a InfInfo, &'a HardwareEntry)> =
            candidates.into_iter().map(|c| (c.inf, c.entry)).collect();

        if !sorted_infs.is_empty() {
            results.push((device, sorted_infs));
        }
    }

    results
}

/// 计算单个设备与单个 INF Entry 的 ID 匹配分数
///
/// # 参数
///  - `device`: 要匹配的设备信息
///  - `entry`: 要匹配的 INF Entry 信息
///
/// # 返回值
///  - `Some(MatchType, u16)`: 匹配类型和匹配索引
///  - `None`: 没有匹配项
fn calculate_id_score(device: &HardwareInfo, entry: &HardwareEntry) -> Option<(MatchType, u16)> {
    let device_hwids = &device.hardware_id;
    let device_cids = &device.compatible_id;
    let inf_hwids = &entry.hardware_id;
    let inf_cids = &entry.compatible_ids;

    // [Rank 0] Device HWID vs INF HWID
    if let Some(idx) = device_hwids
        .iter()
        .position(|id| id.eq_ignore_ascii_case(inf_hwids))
    {
        return Some((MatchType::HardwareToHardware, idx as u16));
    }

    // [Rank 1] Device CID vs INF HWID
    if let Some(idx) = device_cids
        .iter()
        .position(|id| id.eq_ignore_ascii_case(inf_hwids))
    {
        return Some((MatchType::CompatibleToHardware, idx as u16));
    }

    // [Rank 2] Device HWID vs INF CID
    for (idx, dev_id) in device_hwids.iter().enumerate() {
        if inf_cids.iter().any(|cid| cid.eq_ignore_ascii_case(dev_id)) {
            return Some((MatchType::HardwareToCompatible, idx as u16));
        }
    }

    // [Rank 3] Device CID vs INF CID
    for (idx, dev_id) in device_cids.iter().enumerate() {
        if inf_cids.iter().any(|cid| cid.eq_ignore_ascii_case(dev_id)) {
            return Some((MatchType::CompatibleToCompatible, idx as u16));
        }
    }

    None
}

/// 检查 INF 支持的版本是否满足当前系统要求
///
/// # 参数
///  - `inf_min_ver`: INF 中的 min_os_version (e.g. "10.0")
///  - `current_os_version`: 当前系统版本字符串 (e.g. "10.0.19041")
///
/// # 返回值
///  - `true`: 满足要求
///  - `false`: 不满足要求
fn is_os_compatible(inf_min_ver: &str, current_os_version: &str) -> bool {
    if inf_min_ver.is_empty() {
        // 通用驱动
        return true;
    }

    // 要求 INF 的最低版本 <= 当前系统版本
    match compare_version(inf_min_ver, current_os_version) {
        std::cmp::Ordering::Less | std::cmp::Ordering::Equal => true,
        std::cmp::Ordering::Greater => false,
    }
}

/// 检查 INF 是否为扩展驱动
///
/// # 参数
///  - `inf`: 要检查的 INF 驱动信息
///
/// # 返回值
///  - `true`: 是扩展驱动
///  - `false`: 不是扩展驱动
fn is_extension_driver(inf: &InfInfo) -> bool {
    inf.class.eq_ignore_ascii_case("Extension")
        || inf.class.eq_ignore_ascii_case("SoftwareComponent")
}
