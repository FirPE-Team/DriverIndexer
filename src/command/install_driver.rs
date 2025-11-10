use crate::driver_index::{DriverArch, DriverIndex, InfInfo};
use crate::utils::console::{write_console, ConsoleType};
use crate::utils::devcon::{Devcon, HardwareInfo};
use crate::utils::newdev;
use crate::utils::setupapi::SetupAPI;
use crate::utils::sevenzip::SevenZip;
use crate::utils::utils::{compare_version, find_offline_system, get_file_list, get_native_arch};
use crate::{DEBUG, TEMP_PATH};
use anyhow::{anyhow, Context, Result};
use rust_i18n::t;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::mpsc::channel;
use std::sync::Arc;
use threadpool::ThreadPool;
use windows::Win32::System::SystemInformation::{
    PROCESSOR_ARCHITECTURE_AMD64, PROCESSOR_ARCHITECTURE_ARM, PROCESSOR_ARCHITECTURE_ARM64,
    PROCESSOR_ARCHITECTURE_IA64, PROCESSOR_ARCHITECTURE_INTEL,
};

pub struct DriverInstaller {
    zip: SevenZip,
    devcon: Devcon,
}

impl DriverInstaller {
    pub fn new() -> Self {
        Self {
            zip: SevenZip::new().expect("Create SevenZip instance failed"),
            devcon: Devcon::new().expect("Create Devcon instance failed"),
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
        match_all: bool,
        class: Option<&str>,
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
            self.find_config(driver_pack_path, password, &extract_path)
        };

        // 解析索引文件
        let config = match config_path {
            Some(config_path) => {
                if let Ok(config) = DriverIndex::from_json(&config_path) {
                    // 索引文件解析成功，校验索引文件是否与驱动包匹配
                    let driver_size = driver_pack_path
                        .metadata()
                        .with_context(|| "get driver pack size failed")?
                        .len();
                    if driver_size != config.size {
                        // 驱动包大小与索引文件大小不一致，即时建立索引文件
                        write_console(ConsoleType::Warning, &t!("driver-not-match-config"));
                        write_console(ConsoleType::Info, &t!("create-index-info"));
                        self.build_config(driver_pack_path, password, &extract_path)?
                    } else {
                        // 校验通过，加载索引文件
                        write_console(
                            ConsoleType::Info,
                            &format!("{}: {}", t!("load-index"), config_path.display()),
                        );
                        config
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

            // 获取真实硬件信息
            if DEBUG.load(Ordering::Relaxed) {
                write_console(ConsoleType::Debug, "Get hardware info");
            }
            let mut hwid_list = self
                .devcon
                .get_hardware_device_info(None)
                .with_context(|| "get real info failed")?;
            if hwid_list.is_empty() {
                // 获取硬件信息失败
                write_console(ConsoleType::Error, &t!("no-device"));
                return Err(anyhow!(t!("no-device")));
            }

            // 判断是否需要获取有问题的硬件信息
            if !match_all {
                hwid_list = self
                    .devcon
                    .get_problem_device_info(&hwid_list)
                    .with_context(|| "get problem info failed")?;
                if hwid_list.is_empty() {
                    // 没有需要安装驱动的设备
                    write_console(ConsoleType::Error, &t!("no-found-driver-currently"));
                    return Err(anyhow!(t!("no-found-driver-currently")));
                }
            }

            // 过滤前一次安装的硬件信息
            let hwid_list: Vec<HardwareInfo> = hwid_list
                .clone()
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
            let match_hardware_and_driver = match_driver_info(&hwid_list, &config.drivers, class);
            if scan_count == 0 && match_hardware_and_driver.is_empty() {
                write_console(ConsoleType::Error, &t!("no-found-driver-currently"));
                break;
            }

            // 创建线程池（池大小以可用 CPU 核心数为准）
            let pool = ThreadPool::new(num_cpus::get());
            let (tx, rx) = channel();

            // 循环匹配信息
            for (hardware, infInfo) in match_hardware_and_driver {
                // 当前状态：一个设备中有一个或多个驱动

                // 调试模式下输出匹配信息
                if DEBUG.load(Ordering::Relaxed) {
                    write_console(
                        ConsoleType::Debug,
                        &format!(
                            "Match info:\n            Name: {}\n            HWID: {}\n            Driver:\n            {}",
                            hardware.name,
                            hardware.hardware_id.join(","),
                            infInfo
                                .iter()
                                .map(|item| item.path.clone())
                                .collect::<Vec<String>>()
                                .join("\n            ")
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
                let hardware = hardware.clone();
                let inf_info = infInfo.clone();
                let tx = tx.clone();
                let zip = self.zip.clone();

                // 为每个需要安装驱动的设备分配一个线程
                pool.execute(move || {
                    // 遍历匹配的驱动
                    for inf_info_item in inf_info.iter() {
                        // 判断驱动包是否需要解压
                        let inf_path = if driver_pack_path.is_file() {
                            // 获取解压路径（相对于解压所有INF文件的路径）
                            let extract_path = Path::new(inf_info_item.path.as_str())
                                .parent()
                                .expect("get extract path failed");

                            if !zip
                                .extract_files_from_path(
                                    &driver_pack_path,
                                    password.as_deref(),
                                    &extract_path.to_string_lossy(),
                                    &drivers_path,
                                )
                                .unwrap_or(false)
                            {
                                // 解压失败
                                if Some(inf_info_item) != inf_info.last() {
                                    // 继续解压下一驱动
                                    if DEBUG.load(Ordering::Relaxed) {
                                        write_console(
                                            ConsoleType::Debug,
                                            &format!("Extract failed: {}", extract_path.display()),
                                        );
                                    };
                                    continue;
                                } else {
                                    // 最后一个驱动，返回失败
                                    tx.send((hardware, Some(inf_info_item.clone()), false))
                                        .expect("send result failed");
                                    if DEBUG.load(Ordering::Relaxed) {
                                        write_console(
                                            ConsoleType::Debug,
                                            "All driver extract failed",
                                        );
                                    }
                                    return;
                                }
                            };

                            // 仅解压驱动文件，返回成功
                            if only_extract {
                                tx.send((hardware, Some(inf_info_item.clone()), true))
                                    .expect("send result failed");
                                return;
                            }

                            // 获取INF路径
                            let inf_path = drivers_path.join(&inf_info_item.path);
                            if !inf_path.exists() {
                                // 驱动文件不存在
                                if DEBUG.load(Ordering::Relaxed) {
                                    write_console(
                                        ConsoleType::Debug,
                                        &format!("Driver file not found: {}", inf_path.display()),
                                    );
                                };
                                if Some(inf_info_item) != inf_info.last() {
                                    continue;
                                } else {
                                    // 最后一个驱动，返回失败
                                    tx.send((hardware, Some(inf_info_item.clone()), false))
                                        .expect("send result failed");
                                    return;
                                }
                            }

                            inf_path
                        } else {
                            // 驱动文件指定路径
                            drivers_path.join(&inf_info_item.path)
                        };

                        // 加载驱动
                        if let Some(hwid) = hardware.hardware_id.first() {
                            if DEBUG.load(Ordering::Relaxed) {
                                write_console(
                                    ConsoleType::Debug,
                                    &format!("Install driver: {}", inf_path.display()),
                                );
                            }
                            match newdev::update_driver_for_plug_and_play_devices(
                                hwid, &inf_path, force,
                            ) {
                                Ok(()) => {
                                    // 安装驱动成功
                                    tx.send((hardware, Some(inf_info_item.clone()), true))
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
                                    if Some(inf_info_item) != inf_info.last() {
                                        continue;
                                    } else {
                                        // 最后一个驱动，返回失败
                                        tx.send((hardware, Some(inf_info_item.clone()), false))
                                            .expect("send result failed");
                                        return;
                                    }
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
                            tx.send((hardware, Some(inf_info_item.clone()), false))
                                .expect("send result failed");
                            return;
                        }
                    }

                    // 没有找到合适的驱动
                    tx.send((hardware, None, false))
                        .expect("send result failed");
                });
            }

            // 等待所有线程执行完成
            drop(tx); // 关闭发送端

            // 在主线程中进行消息格式化和输出
            for (hardware, inf_info_item_opt, success) in rx.iter() {
                if let Some(inf_info_item) = inf_info_item_opt {
                    if success {
                        write_console(
                            ConsoleType::Success,
                            &t!(
                                "install-success",
                                class = inf_info_item.class.clone(),
                                name = hardware.name.clone(),
                                id = hardware
                                    .hardware_id
                                    .first()
                                    .unwrap_or(&"".to_string())
                                    .clone(),
                                driver = Path::new(inf_info_item.path.as_str())
                                    .file_name()
                                    .unwrap_or("".as_ref())
                                    .to_string_lossy()
                                    .to_string(),
                                version = inf_info_item.version.clone(),
                                date = inf_info_item.date
                            ),
                        );
                    } else {
                        write_console(
                            ConsoleType::Error,
                            &t!(
                                "install-failed",
                                name = hardware.name.clone(),
                                id = hardware
                                    .hardware_id
                                    .first()
                                    .unwrap_or(&"".to_string())
                                    .clone()
                            ),
                        );
                    }
                } else {
                    // 没有找到合适的驱动或初始化失败
                    write_console(ConsoleType::Error, &t!("driver-install-failed"));
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
    ///
    /// # 返回值
    /// - `Ok(())`: 成功加载驱动
    /// - `Err(...)`: 加载驱动失败
    pub fn load_offline_driver(
        &self,
        system_drive: Option<&Path>,
        match_all: bool,
        class: Option<&str>,
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
                    "load-offline-driver",
                    path = driver_path.to_string_lossy().to_string()
                ),
            );
            return self.install_driver(&driver_path, None, None, false, None, None, false);
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
                    "loading-offline-driver",
                    path = system_drive.to_string_lossy().to_string()
                ),
            );
            self.install_driver(&system_drive, None, None, match_all, class, None, false)?;
        }
        Ok(())
    }

    /// 加载当前可执行文件中的驱动程序
    ///
    /// # 返回值
    /// - `Ok(())`: 成功加载驱动
    /// - `Err(...)`: 加载驱动失败
    pub fn load_self_driver_program(&self) -> Result<()> {
        let current_exe =
            env::current_exe().with_context(|| "Get Current Executable Path Failed")?;

        let index = self.find_config(&current_exe, None, &TEMP_PATH);
        self.install_driver(
            &current_exe,
            None,
            index.as_deref(),
            false,
            None,
            None,
            false,
        )
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
    fn find_config(
        &self,
        driver_path: &Path,
        password: Option<&str>,
        extract_path: &Path,
    ) -> Option<PathBuf> {
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

        // 检测压缩包内索引文件
        if driver_path.is_file() {
            // 解压索引文件到临时目录
            if let Ok(true) =
                self.zip
                    .extract_files_from_path(driver_path, password, "*.index", extract_path)
            {
                // 目前假设只有一个 index 文件，直接 glob 查找
                if let Some(found) =
                    glob::glob(&format!("{}/**/*.index", extract_path.to_string_lossy()))
                        .expect("glob index file failed")
                        .filter_map(Result::ok)
                        .next()
                {
                    return Some(found);
                }
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
            // 解压所有 INF 文件
            if !self.zip.extract_files_from_path(
                driver_pack_path,
                password,
                "*.inf",
                extract_path,
            )? {
                return Err(anyhow!(t!("driver-unzip-failed")));
            }
            extract_path
        } else {
            driver_pack_path
        };

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

        // 遍历INF文件
        for inf_file in inf_list.into_iter() {
            let tx = tx.clone();
            let base_path = Arc::clone(&base_path);

            pool.execute(move || {
                // 解析INF文件，解析失败的INF将自动跳过
                if let Ok(inf_info) = InfInfo::parse_from_inf(&base_path, &inf_file) {
                    // 发送到主线程
                    tx.send(inf_info).expect("Send inf_info failed");
                }
            });
        }

        drop(tx);
        Ok(DriverIndex::new(
            driver_pack_path
                .metadata()
                .with_context(|| "Get driver pack path metadata failed")?
                .len(),
            rx.into_iter().collect(),
        ))
    }
}

/// 获取匹配驱动的信息
///
/// # 参数
/// - `idInfo` - 硬件ID列表
/// - `infInfoList` - INF驱动信息列表
/// - `driveClass` - 驱动类别
///
/// # 规则
/// 1. 专用驱动优先级大于公版
/// 2. 高版本优先级大于低版本
///
/// # 参考
/// - [Windows 如何对驱动程序包进行排名](https://learn.microsoft.com/zh-cn/windows-hardware/drivers/install/how-windows-ranks-driver-packages/)
/// - [Windows驱动匹配详解](https://www.cnblogs.com/glacierh/p/5738232.html)
///
/// # 返回值
/// - `Vec<(HwID, Vec<InfInfo>)>` - 匹配驱动信息列表
pub fn match_driver_info(
    hardware_info: &[HardwareInfo],
    inf_info_list: &[InfInfo],
    class: Option<&str>,
) -> Vec<(HardwareInfo, Vec<InfInfo>)> {
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

    // 过滤INF信息列表
    let inf_info_list: Vec<InfInfo> = inf_info_list
        .iter()
        // 过滤不支持当前系统架构的INF
        .filter(|inf_info| inf_info.arch.contains(&current_arch))
        // 过滤指定驱动类别
        .filter(|inf_info| class.is_none_or(|class| class.eq_ignore_ascii_case(&inf_info.class)))
        .cloned()
        .collect();

    // 匹配驱动信息结果
    let mut macth_info: Vec<(HardwareInfo, Vec<InfInfo>)> = Vec::new();

    // 遍历每个设备匹配INF文件中的硬件ID和兼容ID
    for device_info in hardware_info.iter() {
        let mut macth_inf_list: Vec<InfInfo> = Vec::new();

        // 对比设备硬件ID与INF文件中的硬件ID
        for hwid in device_info.hardware_id.iter() {
            let mut macth_list_item: Vec<InfInfo> = inf_info_list
                .iter()
                .filter(|inf_info| {
                    inf_info
                        .hwid
                        .iter()
                        .any(|inf_id| hwid.eq_ignore_ascii_case(inf_id))
                })
                .cloned()
                .collect();

            // 排序：高版本优先级大于低版本
            macth_list_item.sort_by(|b, a| {
                compare_version(&a.version, &b.version).unwrap_or(std::cmp::Ordering::Less)
            });

            macth_inf_list.append(&mut macth_list_item);
        }

        // 对比设备硬件ID与INF文件中的兼容ID
        for hwid in device_info.hardware_id.iter() {
            let mut macth_list_item: Vec<InfInfo> = inf_info_list
                .iter()
                // 过滤已匹配的INF文件
                .filter(|inf_info| !macth_inf_list.contains(inf_info))
                .filter(|inf_info| {
                    inf_info
                        .cid
                        .iter()
                        .any(|inf_id| hwid.eq_ignore_ascii_case(inf_id))
                })
                .cloned()
                .collect();

            // 排序：高版本优先级大于低版本
            macth_list_item.sort_by(|b, a| {
                compare_version(&a.version, &b.version).unwrap_or(std::cmp::Ordering::Less)
            });

            macth_inf_list.append(&mut macth_list_item);
        }

        // 对比设备兼容ID与INF文件中的硬件ID
        for cid in device_info.compatible_id.iter() {
            let mut macth_list_item: Vec<InfInfo> = inf_info_list
                .iter()
                // 过滤已匹配的INF文件
                .filter(|inf_info| !macth_inf_list.contains(inf_info))
                // 匹配INF文件中的兼容ID
                .filter(|inf_info| {
                    inf_info
                        .hwid
                        .iter()
                        .any(|inf_id| cid.eq_ignore_ascii_case(inf_id))
                })
                .cloned()
                .collect();

            // 排序：高版本优先级大于低版本
            macth_list_item.sort_by(|b, a| {
                compare_version(&a.version, &b.version).unwrap_or(std::cmp::Ordering::Less)
            });

            macth_inf_list.append(&mut macth_list_item);
        }

        // 对比设备兼容ID与INF文件中的兼容ID
        for cid in device_info.compatible_id.iter() {
            let mut macth_list_item: Vec<InfInfo> = inf_info_list
                .iter()
                // 过滤已匹配的INF文件
                .filter(|inf_info| !macth_inf_list.contains(inf_info))
                // 匹配INF文件中的兼容ID
                .filter(|inf_info| {
                    inf_info
                        .cid
                        .iter()
                        .any(|inf_id| cid.eq_ignore_ascii_case(inf_id))
                })
                .cloned()
                .collect();

            // 排序：高版本优先级大于低版本
            macth_list_item.sort_by(|b, a| {
                compare_version(&a.version, &b.version).unwrap_or(std::cmp::Ordering::Less)
            });

            macth_inf_list.append(&mut macth_list_item);
        }

        // 没有匹配到该设备的驱动信息，匹配下一个设备
        if macth_inf_list.is_empty() {
            continue;
        }

        macth_info.push((device_info.clone(), macth_inf_list));
    }
    macth_info
}
