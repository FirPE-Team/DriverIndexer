use crate::command::create_index::InfInfo;
use crate::i18n::getLocaleText;
use crate::utils::console::{writeConsole, ConsoleType};
use crate::utils::devcon::{Devcon, HwID};
use crate::utils::sevenZIP::sevenZip;
use crate::utils::util::{compareVersion, getFileList};
use crate::utils::{newdevAPI, setupAPI};
use crate::TEMP_PATH;
use fluent_templates::fluent_bundle::FluentValue;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use threadpool::ThreadPool;

/// 加载驱动包。支持驱动包路径、驱动路径
/// # 参数
/// 1. 驱动包路径
/// 2. 驱动包密码
/// 3. 索引Option
/// 4. 是否为精确匹配
/// 5. 驱动类别
/// 6. 释放路径
pub fn loadDriver(
    driverPackPath: &Path,
    password: Option<&str>,
    indexPath: Option<PathBuf>,
    isAllDevice: bool,
    driveClass: Option<String>,
    extractPath: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    let zip = sevenZip::new()?;
    let devcon = Devcon::new()?;

    // 当前临时驱动解压路径
    let driversPath = if driverPackPath.is_dir() {
        driverPackPath.to_path_buf()
    } else {
        TEMP_PATH.join(driverPackPath.file_stem().unwrap())
    };

    let infInfoList = {
        // 指定索引文件
        if let Some(idx) = indexPath {
            // 根据相对/绝对路径定位
            let index = if idx.is_relative() {
                let relativeIndex = driverPackPath.parent().unwrap().join(&idx);
                if relativeIndex.exists() { relativeIndex } else { idx.to_path_buf() }
            } else {
                idx.to_path_buf()
            };

            let args: HashMap<String, FluentValue> = hash_map!("path".to_string() => index.to_str().unwrap().into());
            writeConsole(ConsoleType::Info, &getLocaleText("load-index", Some(&args)));
            InfInfo::parsingIndex(&index).unwrap_or_else(|_| {
                writeConsole(ConsoleType::Warning, &getLocaleText("index-parsing-failed", None));
                Vec::new()
            })
        } else {
            Vec::new()
        }
    };

    // 自动检测同目录下的索引文件
    let infInfoList = if !infInfoList.is_empty() {
        infInfoList
    } else {
        let same_index = driverPackPath
            .parent()
            .unwrap()
            .join(format!("{}.index", driverPackPath.file_stem().unwrap().to_string_lossy()));
        if same_index.exists() {
            let args: HashMap<String, FluentValue> = hash_map!("path".to_string() => same_index.to_str().unwrap().into());
            writeConsole(ConsoleType::Info, &getLocaleText("load-index", Some(&args)));
            InfInfo::parsingIndex(&same_index).unwrap_or_else(|_| {
                writeConsole(ConsoleType::Warning, &getLocaleText("index-parsing-failed", None));
                Vec::new()
            })
        } else {
            Vec::new()
        }
    };


    // 自动检测压缩包内索引文件
    let infInfoList = if infInfoList.is_empty() && driverPackPath.is_file() {
        // 解压所有索引文件到临时目录
        if zip.extractFilesFromPath(driverPackPath, password, "*.index", &driversPath)? == true {
            // 目前假设只有一个 index 文件，直接 glob 查找
            if let Some(found) = glob::glob(&format!("{}/**/*.index", driversPath.display()))?
                .filter_map(Result::ok)
                .next() {
                let args: HashMap<String, FluentValue> = hash_map!("path".to_string() => found.file_name().unwrap().to_str().unwrap().into());
                writeConsole(ConsoleType::Info, &getLocaleText("load-index", Some(&args)));
                InfInfo::parsingIndex(&found).unwrap_or_else(|_| {
                    writeConsole(ConsoleType::Warning, &getLocaleText("index-parsing-failed", None));
                    Vec::new()
                })
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    // 即时建立索引
    let infInfoList = if !infInfoList.is_empty() {
        infInfoList
    } else {
        let driversPath = if driverPackPath.is_file() {
            // 解压所有 INF 文件
            if !zip.extractFilesFromPath(driverPackPath, password, "*.inf", &driversPath)? {
                writeConsole(ConsoleType::Err, &getLocaleText("driver-unzip-failed", None));
                return Err(getLocaleText("driver-unzip-failed", None).into());
            }
            &driversPath
        } else {
            driverPackPath
        };

        // 列出INF文件
        let infList = getFileList(&driversPath, "*.inf")?;
        if infList.is_empty() {
            writeConsole(ConsoleType::Err, &getLocaleText("no-driver-package", None));
            return Err(getLocaleText("no-driver-package", None).into());
        }

        // 多线程解析INF文件
        InfInfo::parsingInfFileList(&driversPath, &infList)
    };

    let mut totalList: Vec<HwID> = Vec::new();

    // 3次匹配，避免部分驱动安装不全
    for scanCount in 0..3 {
        // 扫描以发现新的硬件
        // devcon.rescan().unwrap();
        unsafe { setupAPI::rescan(); }

        // 获取真实硬件信息
        let mut hwIDList = devcon.getRealIdInfo(None).unwrap();
        if hwIDList.is_empty() {
            writeConsole(ConsoleType::Err, &getLocaleText("no-device", None));
            return Err(getLocaleText("no-device", None).into());
        }

        // 判断是否需要获取有问题的硬件信息
        if !isAllDevice {
            hwIDList = devcon.getProblemIdInfo(hwIDList).unwrap();
            if hwIDList.is_empty() {
                // 没有需要安装驱动的设备
                writeConsole(ConsoleType::Err, &getLocaleText("no-found-driver-currently", None));
                return Err(getLocaleText("no-found-driver-currently", None).into());
            }
        }

        // 过滤前一次安装的硬件信息
        let hwIDList: Vec<HwID> = hwIDList
            .clone()
            .into_iter()
            .filter(|item| !totalList.contains(item))
            .collect();
        // 硬件信息为空，当前没有需要安装驱动的设备
        if hwIDList.is_empty() {
            break;
        }
        for item in hwIDList.iter() {
            totalList.push(item.clone())
        }

        // 获取匹配硬件id
        let matchHardwareAndDriver = getMatchInfo(&hwIDList, &infInfoList, driveClass.clone());
        if scanCount == 0 && matchHardwareAndDriver.is_empty() {
            writeConsole(ConsoleType::Err, &getLocaleText("no-found-driver-currently", None));
            break;
        }

        // 任务列表
        let pool = ThreadPool::new(num_cpus::get());
        let (tx, rx) = channel();

        // 循环匹配信息
        for (hardware, infInfo) in matchHardwareAndDriver {
            // 当前状态：一个设备中有一个或多个驱动
            let driverPackPath = driverPackPath.to_path_buf();
            let password = password.map(|password| password.to_string());
            let onlyExtract = extractPath.is_some();

            let driversPath = match extractPath {
                None => driversPath.clone(),
                Some(path) => PathBuf::from(path)
            };
            let hardware = hardware.clone();
            let infInfo = infInfo.clone();

            // 为每个设备分配一个线程
            let tx = tx.clone();

            pool.execute(move || {
                let result = installDriver(
                    &driverPackPath,
                    password.clone(),
                    &driversPath,
                    &hardware,
                    &infInfo,
                    onlyExtract,
                );
                tx.send((hardware, result)).expect("send result");
            });
        }

        // 等待所有线程执行完成
        drop(tx);  // 关闭发送端
        for (_hardware, result) in rx.iter() {
            match result {
                Ok(msg) => writeConsole(ConsoleType::Success, &msg),
                Err(err) => writeConsole(ConsoleType::Err, &err),
            }
        }
    }
    Ok(())
}


/// 安装驱动包
/// # 参数
/// 1. 驱动包路径
/// 2. 解压密码
/// 3. 驱动解压路径
/// 4. 硬件信息
/// 5. INF信息列表
/// 6. 是否仅解压
fn installDriver(
    driverPackPath: &Path,
    password: Option<String>,
    driversPath: &Path,
    hardware: &HwID,
    infInfo: &[InfInfo],
    onlyExtract: bool,
) -> Result<String, String> {
    lazy_static! {
        pub static ref ZIP: sevenZip = sevenZip::new().unwrap();
    }

    // 遍历匹配的驱动
    unsafe {
        for infInfoItem in infInfo.iter() {
            let arg: HashMap<String, FluentValue> = hash_map!(
                "class".to_string() => infInfoItem.Class.clone().into(),
                "deviceName".to_string() => hardware.Name.clone().into(),
                "deviceID".to_string() => hardware.HardwareIDs.first().unwrap_or(&"".to_string()).clone().into(),
                "driver".to_string() => infInfoItem.Inf.clone().into(),
                "version".to_string() => infInfoItem.Version.clone().into(),
            );

            // 获取解压路径（相对于解压所有INF文件的路径）
            let extractPath = &infInfoItem.Path;
            let password = password.as_deref();

            // 解压匹配的驱动
            if driverPackPath.is_file() && !ZIP.extractFilesFromPath(driverPackPath, password, extractPath.as_str(), driversPath).unwrap() {
                if Some(infInfoItem) != infInfo.last() {
                    continue;
                } else {
                    return Err(getLocaleText("install-message", Some(&arg)));
                }
            }

            // 仅解压驱动
            if onlyExtract {
                return Ok(getLocaleText("install-message", Some(&arg)));
            }

            // 获取INF路径
            let driveInfPath = driversPath.join(extractPath).join(&infInfoItem.Inf);

            // 加载驱动
            let result: bool = infInfoItem
                .DriverList
                .iter()
                .any(|hwId| newdevAPI::updateDriverForPlugAndPlayDevices(&driveInfPath, hwId));
            // 如果当前驱动加载失败则加载下一驱动
            if !result {
                if Some(infInfoItem) != infInfo.last() {
                    continue;
                } else {
                    return Err(getLocaleText("install-message", Some(&arg)));
                }
            }

            return Ok(getLocaleText("install-message", Some(&arg)));
        }
        Err(getLocaleText("driver-install-failed", None))
    }
}

/// 获取匹配驱动的信息
/// # 参数
/// 1. 硬件ID列表
/// 2. INF驱动信息列表
/// # 规则
/// 1. 专用驱动优先级大于公版
/// 2. 高版本优先级大于低版本
pub fn getMatchInfo<T1>(idInfo: &[HwID], infInfoList: &[InfInfo], driveClass: T1) -> Vec<(HwID, Vec<InfInfo>)>
where
    T1: Into<Option<String>> + Clone,
{
    // 提示：
    // 循环次数少的放在外层，减少内层变量的操作次数
    // 一个设备信息 对应 多个匹配驱动信息

    // 当前系统架构
    let currentArch = match env::consts::ARCH {
        "x86" => "NTx86",
        "x86_64" => "NTamd64",
        "arm" => "NTarm",
        _ => "",
    };

    // 闭包函数-匹配
    let matchFn = |haID: &String| {
        let mut macthList: Vec<InfInfo> = Vec::new();
        // 遍历INF信息列表
        for infInfoItem in infInfoList.iter() {
            // 如果INF不适用当前系统则进行匹配下一个INF
            if !infInfoItem.Arch.contains(&currentArch.to_string()) {
                continue;
            }
            // 如果指定了驱动类别且类别不匹配则匹配下一个INF
            if let Some(class) = driveClass.clone().into() {
                if class.to_lowercase() != infInfoItem.Class.to_lowercase() {
                    continue;
                }
            }

            let mut matchInfInfo = InfInfo {
                DriverList: vec![],
                ..infInfoItem.clone()
            };
            // 遍历INF中的硬件id
            let mut driverList: Vec<String> = infInfoItem
                .DriverList
                .clone()
                .into_iter()
                .filter(|infID| haID.to_lowercase() == infID.to_lowercase())
                .collect();
            matchInfInfo.DriverList.append(&mut driverList);
            if !matchInfInfo.DriverList.is_empty() {
                macthList.push(matchInfInfo.clone());
            }
        }
        // 排序：高版本优先级大于低版本
        macthList.sort_by(|b, a| compareVersion(&a.Version, &b.Version).unwrap_or(Ordering::Less));
        macthList
    };

    // 匹配驱动信息
    let mut macthInfo: Vec<(HwID, Vec<InfInfo>)> = Vec::new();

    // 遍历有问题的硬件id信息
    for idInfo in idInfo.iter() {
        // 创建匹配信息列表
        let mut macthList: Vec<InfInfo> = Vec::new();

        // 优先对比硬件id
        for haID in idInfo.HardwareIDs.iter() {
            macthList.append(&mut matchFn(haID));
        }

        // 对比兼容id
        for haID in idInfo.CompatibleIDs.iter() {
            macthList.append(&mut matchFn(haID));
        }

        // 没有匹配到该设备的驱动信息，则匹配下一个设备
        if macthList.is_empty() {
            continue;
        }

        macthInfo.push((idInfo.clone(), macthList));
    }
    macthInfo
}
