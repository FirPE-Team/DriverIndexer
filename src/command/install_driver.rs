use crate::DEBUG;
use crate::command::check_if_bundled;
use crate::driver_index::{CatalogSignatureCache, DriverArch, DriverIndex, HardwareEntry, InfInfo};
use crate::driver_match::{DriverLookup, MatchContext, match_drivers};
use crate::hardware::{HardwareInfo, enumerate_hardware, update_driver_for_plug_and_play_devices};
use crate::temp_workspace::TempWorkspace;
use crate::utils::console::{ConsoleType, write_console};
use crate::utils::setupapi::SetupAPI;
use crate::utils::sevenzip::SevenZip;
use crate::utils::utils::{find_offline_system, get_file_list, get_native_arch};
use anyhow::{Context, Result, anyhow};
use rust_i18n::t;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::Component;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Instant, UNIX_EPOCH};
use threadpool::ThreadPool;
use windows::Win32::Foundation::ERROR_NO_MORE_ITEMS;
use windows::Win32::System::SystemInformation::{
    PROCESSOR_ARCHITECTURE_AMD64, PROCESSOR_ARCHITECTURE_ARM, PROCESSOR_ARCHITECTURE_ARM64,
    PROCESSOR_ARCHITECTURE_IA64, PROCESSOR_ARCHITECTURE_INTEL,
};
use windows_version::OsVersion;

pub struct DriverInstaller {
    zip: SevenZip,
    workspace: TempWorkspace,
}

/// Options controlling an online or offline driver installation.
#[derive(Debug, Clone, Default)]
pub struct InstallOptions {
    pub driver_pack_path: PathBuf,
    pub password: Option<String>,
    pub config: Option<PathBuf>,
    pub skip_verify: bool,
    pub missing_only: bool,
    pub class: Option<Vec<String>>,
    pub exclude_class: Option<Vec<String>>,
    pub user_extract_path: Option<PathBuf>,
    pub force: bool,
}

#[derive(Debug, Clone)]
pub struct InstallAttempt {
    pub candidate: String,
    pub outcome: String,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct InstallSummary {
    pub succeeded: usize,
    pub failed: usize,
    pub skipped: usize,
    pub attempts: usize,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct DeviceKey {
    instance: String,
    primary_hwid: String,
}

#[derive(Default)]
struct ExtractionCache {
    entries: Mutex<HashMap<ExtractionKey, ExtractionState>>,
}

type ExtractionState = Arc<OnceLock<Result<(), String>>>;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct ExtractionKey {
    archive: String,
    relative_dir: String,
    destination: String,
    password_context: String,
}

impl ExtractionCache {
    fn extract_once(
        &self,
        zip: &SevenZip,
        archive: &Path,
        password: Option<&str>,
        relative_dir: &Path,
        destination: &Path,
    ) -> Result<bool> {
        std::fs::create_dir_all(destination)
            .with_context(|| format!("create extraction root {}", destination.display()))?;
        let target_dir = contained_path(destination, relative_dir)?;
        let key = ExtractionKey {
            archive: cache_path(archive)?,
            relative_dir: normalized_relative_path(relative_dir)?,
            destination: cache_path(destination)?,
            password_context: password_context(password),
        };
        let (entry, cache_hit) = {
            let mut entries = self
                .entries
                .lock()
                .map_err(|_| anyhow!("driver extraction cache lock poisoned"))?;
            let cache_hit = entries.contains_key(&key);
            let entry = entries
                .entry(key)
                .or_insert_with(|| Arc::new(OnceLock::new()))
                .clone();
            (entry, cache_hit)
        };
        entry
            .get_or_init(|| {
                std::fs::create_dir_all(&target_dir).map_err(|error| error.to_string())?;
                ensure_contained(destination, &target_dir).map_err(|error| error.to_string())?;
                zip.extract_files_from_path(
                    archive,
                    password,
                    &relative_dir.to_string_lossy(),
                    destination,
                )
                .map_err(|error| error.to_string())?;
                ensure_contained(destination, &target_dir).map_err(|error| error.to_string())
            })
            .clone()
            .map_err(anyhow::Error::msg)
            .map(|()| cache_hit)
    }
}

impl DriverInstaller {
    pub fn new() -> Result<Self> {
        let workspace = TempWorkspace::create("install")?;
        Ok(Self {
            zip: SevenZip::new_in(workspace.path())
                .with_context(|| "Create SevenZip instance failed")?,
            workspace,
        })
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
    pub fn install_driver(&self, options: &InstallOptions) -> Result<()> {
        let driver_pack_path = &options.driver_pack_path;
        let password = options.password.as_deref();
        let config = options.config.as_deref();
        let skip_verify = options.skip_verify;
        let missing_only = options.missing_only;
        let class = options.class.as_deref();
        let exclude_class = options.exclude_class.as_deref();
        let user_extract_path = options.user_extract_path.as_deref();
        let force = options.force;
        // 当前临时驱动解压路径
        let extract_path = if driver_pack_path.is_dir() {
            driver_pack_path.to_path_buf()
        } else {
            self.workspace.path().join(
                driver_pack_path
                    .file_stem()
                    .ok_or_else(|| anyhow!("driver package has no file name"))?,
            )
        };

        // 索引文件路径
        let explicit_config = config.is_some();
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
                        if !skip_verify && let Err(error) = config.check_config(driver_pack_path) {
                            if explicit_config {
                                return Err(error).with_context(|| {
                                    format!(
                                        "driver index {} does not match the driver source; rebuild it",
                                        config_path.display()
                                    )
                                });
                            }
                            // 驱动包与索引文件不匹配，即时建立索引文件
                            write_console(ConsoleType::Warning, &t!("driver-not-match-config"));
                            write_console(ConsoleType::Info, &t!("create-index-info"));
                            self.build_config(driver_pack_path, password, &extract_path)?
                        } else {
                            // 校验通过或跳过校验，加载索引文件
                            if skip_verify {
                                debug_event(
                                    "index.verify",
                                    &[("skipped", "true".into()), ("reason", "cli".into())],
                                );
                            }
                            write_console(
                                ConsoleType::Info,
                                &format!("{}: {}", t!("load-index"), config_path.display()),
                            );
                            config
                        }
                    }
                } else if explicit_config {
                    return Err(anyhow!(
                        "driver index {} is invalid or obsolete; rebuild it with the index command",
                        config_path.display()
                    ));
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

        let driver_lookup = DriverLookup::new(&config.drivers);
        let match_context = current_match_context();
        let extraction_cache = Arc::new(ExtractionCache::default());
        let mut processed_instances = HashSet::new();

        // 3次匹配，避免部分驱动安装不全
        let mut summary = InstallSummary::default();
        for scan_count in 0..3 {
            let scan_started = Instant::now();
            if DEBUG.load(Ordering::Relaxed) {
                write_console(
                    ConsoleType::Debug,
                    &format!("Start scan {}", scan_count + 1),
                );
            }

            // 扫描以发现新的硬件
            let rescan_succeeded = SetupAPI::rescan();
            if !rescan_succeeded {
                write_console(
                    ConsoleType::Warning,
                    &format!("device rescan failed on pass {}", scan_count + 1),
                );
            }

            // 获取硬件信息
            if DEBUG.load(Ordering::Relaxed) {
                write_console(ConsoleType::Debug, "Get hardware info");
            }
            let discovered_hardware = enumerate_hardware(None, missing_only)
                .with_context(|| "Get hardware info failed")?;
            if DEBUG.load(Ordering::Relaxed) {
                write_console(
                    ConsoleType::Debug,
                    &format!("Found {} devices", discovered_hardware.len()),
                );
            }
            if discovered_hardware.is_empty() {
                // 没有需要安装驱动的设备
                return Err(anyhow!(t!("no-found-driver-currently")));
            }

            let discovered_count = discovered_hardware.len();
            let mut already_processed = 0;
            let hwid_list: Vec<HardwareInfo> = discovered_hardware
                .into_iter()
                .filter(|item| {
                    let instance = normalize_device_id(&item.device_instance_path);
                    if processed_instances.insert(instance) {
                        true
                    } else {
                        already_processed += 1;
                        false
                    }
                })
                .collect();

            // 硬件信息为空，当前没有需要安装驱动的设备
            if hwid_list.is_empty() {
                if DEBUG.load(Ordering::Relaxed) {
                    write_console(
                        ConsoleType::Debug,
                        &format!(
                            "scan {}: discovered={}, new=0, already_processed={}",
                            scan_count + 1,
                            discovered_count,
                            already_processed
                        ),
                    );
                }
                break;
            }

            // 匹配硬件设备和驱动信息
            if DEBUG.load(Ordering::Relaxed) {
                write_console(ConsoleType::Debug, "Match hardware info");
            }
            let mut match_hardware_and_driver =
                driver_lookup.match_devices(&hwid_list, &match_context, class, exclude_class);

            // 由于存在多个设备匹配到同一个硬件ID的情况（但设备实例不同），而 UpdateDriverForPlugAndPlayDevices 需要提供硬件id而不是设备实例
            // 故需要去重（保留第一个出现的 HWID，删除后续相同的 HWID项目）
            let mut seen_devices = HashSet::new();
            match_hardware_and_driver.retain(|(device, _)| {
                device.hardware_id.first().is_some_and(|primary_hwid| {
                    seen_devices.insert(DeviceKey {
                        instance: normalize_device_id(&device.device_instance_path),
                        primary_hwid: normalize_device_id(primary_hwid),
                    })
                })
            });

            if DEBUG.load(Ordering::Relaxed) {
                write_console(
                    ConsoleType::Debug,
                    &format!(
                        "scan {}: discovered={}, new={}, already_processed={}, matched={}",
                        scan_count + 1,
                        discovered_count,
                        hwid_list.len(),
                        already_processed,
                        match_hardware_and_driver.len()
                    ),
                );
            }

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
            let worker_count = num_cpus::get().min(match_hardware_and_driver.len()).max(1);
            let pool = ThreadPool::new(worker_count);
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
                                .map(|candidate| {
                                    format!("{} [{:?}]", candidate.inf.path, candidate.rank)
                                })
                                .collect::<Vec<String>>()
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
                    .map(|candidate| (candidate.inf.clone(), candidate.entry.clone()))
                    .collect();
                let tx = tx.clone();
                let zip = self.zip.clone();
                let extraction_cache = Arc::clone(&extraction_cache);
                let mut attempts = Vec::new();

                // 为每个需要安装驱动的设备分配一个线程
                pool.execute(move || {
                    // 遍历匹配的驱动
                    for (index, (inf_info_item, entry)) in match_info.iter().enumerate() {
                        attempts.push(InstallAttempt {
                            candidate: inf_info_item.path.clone(),
                            outcome: "attempted".into(),
                        });
                        debug_event(
                            "install.attempt",
                            &[
                                ("device_instance", hardware.device_instance_path.clone()),
                                ("candidate_path", inf_info_item.path.clone()),
                                ("attempt", (index + 1).to_string()),
                                ("outcome", "started".into()),
                            ],
                        );
                        // 判断驱动包是否需要解压
                        let inf_path = if driver_pack_path.is_file() {
                            // 获取解压路径（相对于解压所有INF文件的路径）
                            let relative_inf = match safe_relative_path(&inf_info_item.path) {
                                Ok(path) => path,
                                Err(error) => {
                                    if index == match_info.len() - 1 {
                                        let _ = tx.send((hardware, Err(error), attempts));
                                        return;
                                    }
                                    continue;
                                }
                            };
                            let extract_path =
                                relative_inf.parent().unwrap_or_else(|| Path::new(""));

                            let cache_hit = match extraction_cache.extract_once(
                                &zip,
                                &driver_pack_path,
                                password.as_deref(),
                                extract_path,
                                &drivers_path,
                            ) {
                                Ok(cache_hit) => cache_hit,
                                Err(e) => {
                                    // 解压失败
                                    if index == match_info.len() - 1 {
                                        // 最后一个驱动，返回失败
                                        let _ = tx.send((
                                            hardware,
                                            Err(anyhow!("{}: {}", t!("driver-unzip-failed"), e)),
                                            attempts,
                                        ));
                                        return;
                                    }
                                    // 继续解压下一驱动
                                    if DEBUG.load(Ordering::Relaxed) {
                                        debug_event(
                                            "install.extract",
                                            &[
                                                ("archive", archive_label(&driver_pack_path)),
                                                ("candidate_path", inf_info_item.path.clone()),
                                                ("cache_hit", "true".into()),
                                                ("attempt", (index + 1).to_string()),
                                                ("outcome", format!("failed: {e}")),
                                            ],
                                        );
                                    }
                                    continue;
                                }
                            };
                            debug_event(
                                "install.extract",
                                &[
                                    ("archive", archive_label(&driver_pack_path)),
                                    ("candidate_path", inf_info_item.path.clone()),
                                    ("cache_hit", cache_hit.to_string()),
                                    ("attempt", (index + 1).to_string()),
                                ],
                            );

                            // 仅解压驱动文件，返回成功
                            if only_extract {
                                let _ = tx.send((
                                    hardware,
                                    Ok((inf_info_item.clone(), entry.clone())),
                                    attempts,
                                ));
                                return;
                            }

                            // 获取INF路径
                            let inf_path = match contained_path(&drivers_path, &relative_inf) {
                                Ok(path) => path,
                                Err(error) => {
                                    if index == match_info.len() - 1 {
                                        let _ = tx.send((hardware, Err(error), attempts));
                                        return;
                                    }
                                    continue;
                                }
                            };
                            if !inf_path.is_file() {
                                // INF文件不存在
                                if DEBUG.load(Ordering::Relaxed) {
                                    write_console(
                                        ConsoleType::Debug,
                                        &format!("Driver file not found: {}", inf_path.display()),
                                    );
                                };
                                if index == match_info.len() - 1 {
                                    // 最后一个驱动，返回失败
                                    let _ = tx.send((
                                        hardware,
                                        Err(anyhow!(
                                            "Driver file not found: {}",
                                            inf_path.display()
                                        )),
                                        attempts,
                                    ));
                                    return;
                                }
                                continue;
                            }

                            inf_path
                        } else {
                            // 驱动文件指定路径
                            let relative_inf = match safe_relative_path(&inf_info_item.path) {
                                Ok(path) => path,
                                Err(error) => {
                                    if index == match_info.len() - 1 {
                                        let _ = tx.send((hardware, Err(error), attempts));
                                        return;
                                    }
                                    continue;
                                }
                            };
                            let inf_path = match contained_path(&drivers_path, &relative_inf) {
                                Ok(path) => path,
                                Err(error) => {
                                    if index == match_info.len() - 1 {
                                        let _ = tx.send((hardware, Err(error), attempts));
                                        return;
                                    }
                                    continue;
                                }
                            };
                            if !inf_path.is_file() {
                                if index == match_info.len() - 1 {
                                    let _ = tx.send((
                                        hardware,
                                        Err(anyhow!(
                                            "Driver file not found: {}",
                                            inf_path.display()
                                        )),
                                        attempts,
                                    ));
                                    return;
                                }
                                continue;
                            }
                            inf_path
                        };

                        // 安装驱动
                        if let Some(hwid) = hardware.hardware_id.first() {
                            if DEBUG.load(Ordering::Relaxed) {
                                debug_event(
                                    "install.setupapi",
                                    &[
                                        ("device_instance", hardware.device_instance_path.clone()),
                                        ("candidate_path", inf_info_item.path.clone()),
                                        ("attempt", (index + 1).to_string()),
                                    ],
                                );
                            }
                            match update_driver_for_plug_and_play_devices(hwid, &inf_path, force) {
                                Ok(()) => {
                                    // 安装驱动成功
                                    debug_event(
                                        "install.attempt",
                                        &[
                                            (
                                                "device_instance",
                                                hardware.device_instance_path.clone(),
                                            ),
                                            ("candidate_path", inf_info_item.path.clone()),
                                            ("attempt", (index + 1).to_string()),
                                            ("outcome", "success".into()),
                                        ],
                                    );
                                    let _ = tx.send((
                                        hardware,
                                        Ok((inf_info_item.clone(), entry.clone())),
                                        attempts,
                                    ));
                                    return;
                                }
                                Err(e) => {
                                    // 安装驱动失败，继续加载下一驱动
                                    debug_event(
                                        "install.attempt",
                                        &[
                                            (
                                                "device_instance",
                                                hardware.device_instance_path.clone(),
                                            ),
                                            ("candidate_path", inf_info_item.path.clone()),
                                            ("attempt", (index + 1).to_string()),
                                            ("outcome", "failed".into()),
                                            ("win32_error", e.to_string()),
                                        ],
                                    );
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
                                            let _ = tx.send((
                                                hardware,
                                                Err(anyhow!("driver is not a better match")),
                                                attempts,
                                            ));
                                            return;
                                        }
                                        let _ = tx.send((hardware, Err(e.into()), attempts));
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
                            let _ = tx.send((
                                hardware,
                                Err(anyhow!("No hardware ID found for: {}", inf_info_item.path)),
                                attempts,
                            ));
                            return;
                        }
                    }

                    // 没有找到合适的驱动
                    let _ = tx.send((hardware, Err(anyhow!("No driver found")), attempts));
                });
            }

            // 等待所有线程执行完成
            drop(tx); // 关闭发送端

            // 在主线程中进行消息格式化和输出
            let mut install_results: Vec<_> = rx.iter().collect();
            install_results.sort_by(|(left, _, _), (right, _, _)| {
                left.device_instance_path
                    .to_ascii_lowercase()
                    .cmp(&right.device_instance_path.to_ascii_lowercase())
            });
            for (hardware, result, attempts) in install_results {
                summary.attempts += attempts.len();
                match result {
                    Ok((inf_info_item, entry)) => {
                        summary.succeeded += 1;
                        write_console(
                            ConsoleType::Success,
                            &t!(
                                "install-success",
                                class = inf_info_item.class,
                                name = hardware.name,
                                desc = entry.desc,
                                id = hardware
                                    .hardware_id
                                    .first()
                                    .map(String::as_str)
                                    .unwrap_or(""),
                                driver = inf_info_item.path,
                                version = inf_info_item.version,
                                date = inf_info_item.date
                            ),
                        );
                    }
                    Err(e) => {
                        if e.to_string().contains("not a better match") {
                            summary.skipped += 1;
                        } else {
                            summary.failed += 1;
                        }
                        write_console(
                            ConsoleType::Error,
                            &t!(
                                "install-failed",
                                name = hardware.name,
                                id = hardware
                                    .hardware_id
                                    .first()
                                    .map(String::as_str)
                                    .unwrap_or(""),
                                info = e
                            ),
                        );
                    }
                }
            }
            if DEBUG.load(Ordering::Relaxed) {
                write_console(
                    ConsoleType::Debug,
                    &format!(
                        "install summary: succeeded={}, failed={}, skipped={}, attempts={}",
                        summary.succeeded, summary.failed, summary.skipped, summary.attempts
                    ),
                );
            }
            debug_event(
                "install.scan",
                &[
                    (
                        "duration_ms",
                        scan_started.elapsed().as_millis().to_string(),
                    ),
                    ("attempt", (scan_count + 1).to_string()),
                    ("succeeded", summary.succeeded.to_string()),
                    ("failed", summary.failed.to_string()),
                    ("skipped", summary.skipped.to_string()),
                ],
            );
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
            return self.install_driver(&InstallOptions {
                driver_pack_path: driver_path,
                skip_verify: true,
                missing_only,
                class: class.map(|items| items.to_vec()),
                exclude_class: exclude_class.map(|items| items.to_vec()),
                ..Default::default()
            });
        }

        // 未指定系统盘，全盘搜索离线系统驱动
        let offline_system_drive_list = find_offline_system();

        // 未找到离线系统
        if offline_system_drive_list.is_empty() {
            return Err(anyhow!(t!("not-found-offline-system")));
        }

        // 遍历离线系统加载驱动
        for system_drive in offline_system_drive_list {
            let driver_path = system_drive
                .join("Windows")
                .join("System32")
                .join("DriverStore")
                .join("FileRepository");
            if !driver_path.exists() {
                continue;
            }
            write_console(
                ConsoleType::Info,
                &t!(
                    "install-offline-driver",
                    path = system_drive.to_string_lossy().to_string()
                ),
            );
            self.install_driver(&InstallOptions {
                driver_pack_path: driver_path,
                skip_verify: true,
                missing_only,
                class: class.map(|items| items.to_vec()),
                exclude_class: exclude_class.map(|items| items.to_vec()),
                ..Default::default()
            })?;
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
        let started = Instant::now();
        let drivers_path = if driver_pack_path.is_file() {
            // 解压全部 INF 文件
            if let Err(_e) = self.zip.extract_files_from_paths(
                driver_pack_path,
                password,
                &["*.inf", "*.cat"],
                extract_path,
            ) {
                return Err(anyhow!(t!("driver-unzip-failed")));
            }
            extract_path
        } else {
            driver_pack_path
        };

        // 列出INF文件
        let mut inf_list = get_file_list(drivers_path, "*.inf")?;
        if inf_list.is_empty() {
            return Err(anyhow!(t!("no-driver-package")));
        }
        inf_list.sort_by_key(|path| path.to_string_lossy().to_ascii_lowercase());
        debug_event(
            "index.extract",
            &[
                ("duration_ms", started.elapsed().as_millis().to_string()),
                ("archive", archive_label(driver_pack_path)),
                ("inf_count", inf_list.len().to_string()),
                (
                    "cat_count",
                    get_file_list(drivers_path, "*.cat")
                        .map_or(0, |files| files.len())
                        .to_string(),
                ),
            ],
        );

        // 创建线程池（池大小以可用 CPU 核心数为准）
        let pool = ThreadPool::new(num_cpus::get());

        // 通道，用于收集每个线程的 InfInfo
        let (tx, rx) = channel();
        let base_path = Arc::new(drivers_path.to_path_buf());
        let workspace_prefix = self.workspace.path().to_string_lossy().into_owned();

        let success_count = Arc::new(AtomicI32::new(0));
        let error_count = Arc::new(AtomicI32::new(0));
        let catalog_cache = Arc::new(CatalogSignatureCache::new());

        // 遍历INF文件
        for inf_file in inf_list.into_iter() {
            let tx = tx.clone();
            let base_path = Arc::clone(&base_path);
            let success_count = Arc::clone(&success_count);
            let error_count = Arc::clone(&error_count);
            let catalog_cache = Arc::clone(&catalog_cache);
            let workspace_prefix = workspace_prefix.clone();

            pool.execute(move || {
                // 解析INF文件，解析失败的INF将自动跳过
                match InfInfo::parse_inf_with_catalog_cache(
                    &base_path,
                    &inf_file,
                    Some(&catalog_cache),
                ) {
                    Ok(inf_info) => {
                        // 增加成功计数
                        success_count.fetch_add(1, Ordering::Relaxed);
                        // 发送到主线程
                        let _ = tx.send(inf_info);
                    }
                    Err(e) => {
                        write_console(
                            ConsoleType::Warning,
                            &format!(
                                "{}: {} ({})",
                                t!("inf-parse-error"),
                                inf_file
                                    .to_string_lossy()
                                    .trim_start_matches(&workspace_prefix),
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

        debug_event(
            "index.parse",
            &[
                ("duration_ms", started.elapsed().as_millis().to_string()),
                ("archive", archive_label(driver_pack_path)),
                ("inf_count", inf_info_list.len().to_string()),
                ("cat_count", "unknown".into()),
            ],
        );

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

        let mut index = DriverIndex::new(
            driver_pack_path
                .metadata()
                .with_context(|| "Get driver pack path metadata failed")?
                .len(),
            timestamp,
            None,
            crate::driver_index::source_fingerprint(driver_pack_path)?,
            inf_info_list,
        );
        index.source_fingerprint.source_type = if driver_pack_path.is_file() {
            crate::driver_index::SourceType::File
        } else {
            crate::driver_index::SourceType::Directory
        };
        Ok(index)
    }
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
    let context = current_match_context();
    match_drivers(
        hardware_info_list,
        inf_info_list,
        &context,
        class_filter,
        class_exclude,
    )
    .into_iter()
    .map(|(device, candidates)| {
        (
            device,
            candidates
                .into_iter()
                .map(|candidate| (candidate.inf, candidate.entry))
                .collect(),
        )
    })
    .collect()
}

fn current_match_context() -> MatchContext {
    let current_arch = match get_native_arch() {
        PROCESSOR_ARCHITECTURE_INTEL => DriverArch::NTx86,
        PROCESSOR_ARCHITECTURE_AMD64 => DriverArch::NTamd64,
        PROCESSOR_ARCHITECTURE_ARM64 => DriverArch::NTarm64,
        PROCESSOR_ARCHITECTURE_IA64 => DriverArch::NTia64,
        PROCESSOR_ARCHITECTURE_ARM => DriverArch::NTarm,
        _ => DriverArch::Nt,
    };
    let version = OsVersion::current();
    MatchContext {
        arch: current_arch,
        os_version: format!("{}.{}.{}", version.major, version.minor, version.build),
    }
}

fn normalize_device_id(value: &str) -> String {
    value.trim().to_ascii_uppercase()
}

fn archive_label(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "<unnamed>".into())
}

fn debug_event(stage: &str, fields: &[(&str, String)]) {
    if !DEBUG.load(Ordering::Relaxed) {
        return;
    }
    let mut event = serde_json::Map::new();
    event.insert("stage".into(), serde_json::Value::String(stage.into()));
    for (key, value) in fields {
        event.insert((*key).into(), serde_json::Value::String(value.clone()));
    }
    if let Ok(serialized) = serde_json::to_string(&event) {
        write_console(ConsoleType::Debug, &serialized);
    }
}

fn safe_relative_path(path: &str) -> Result<PathBuf> {
    if path.is_empty() || path.contains('\0') {
        return Err(anyhow!("unsafe driver path in index: {path}"));
    }
    let normalized = path.replace('\\', "/");
    let candidate = Path::new(&normalized);
    if candidate.components().any(|component| {
        matches!(
            component,
            Component::Prefix(_) | Component::RootDir | Component::ParentDir
        )
    }) {
        return Err(anyhow!("unsafe driver path in index: {path}"));
    }
    Ok(candidate.to_path_buf())
}

fn normalized_relative_path(path: &Path) -> Result<String> {
    let text = path.to_string_lossy().replace('\\', "/");
    if text.is_empty() {
        return Ok(String::new());
    }
    let normalized = safe_relative_path(&text)?;
    Ok(normalized.to_string_lossy().replace('\\', "/"))
}

fn cache_path(path: &Path) -> Result<String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    Ok(absolute
        .to_string_lossy()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_ascii_lowercase())
}

fn password_context(password: Option<&str>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(password.unwrap_or_default().as_bytes());
    let digest = hasher.finalize();
    let hex = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("sha256:{hex}")
}

fn contained_path(root: &Path, relative: &Path) -> Result<PathBuf> {
    if relative.as_os_str().is_empty() {
        ensure_contained(root, root)?;
        return Ok(root.to_path_buf());
    }
    let relative = safe_relative_path(&relative.to_string_lossy())?;
    let candidate = root.join(relative);
    ensure_contained(root, &candidate)?;
    Ok(candidate)
}

fn ensure_contained(root: &Path, candidate: &Path) -> Result<()> {
    let root = cache_path(root)?;
    let candidate = cache_path(candidate)?;
    if candidate == root || candidate.strip_prefix(&format!("{root}/")).is_some() {
        Ok(())
    } else {
        Err(anyhow!("path escapes extraction root: {}", candidate))
    }
}
