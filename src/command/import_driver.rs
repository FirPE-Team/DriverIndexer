use std::error::Error;
use std::path::Path;
use crate::command::create_index::InfInfo;
use crate::command::load_driver::getMatchInfo;
use crate::i18n::getLocaleText;
use crate::TEMP_PATH;
use crate::utils::console::{writeConsole, ConsoleType};
use crate::utils::devcon::Devcon;
use crate::utils::drvstoreAPI::DriverStore;
use crate::utils::sevenZIP::sevenZip;
use crate::utils::util::{getFileList, getArchCode, isOfflineSystem};

pub fn import_driver(systemDrive: &Path, driverPath: &Path, password: Option<&str>, matchDevice: bool,) -> Result<(), Box<dyn Error>> {
    let mut real_driver_path = driverPath.to_path_buf();
    let zip = sevenZip::new()?;

    // 判断是否为驱动包
    if driverPath.is_file() {
        if !zip.isDriverPackage(driverPath).unwrap_or(false) {
            writeConsole(ConsoleType::Err, &getLocaleText("no-driver-package", None));
            return Err(String::from(&getLocaleText("no-driver-package", None)).into());
        }

        let driversPath = TEMP_PATH.join(driverPath.file_stem().unwrap());
        if matchDevice {
            // 解压全部INF文件
            if !zip.extractFilesFromPath(driverPath, password, "*.inf", &driversPath)? {
                writeConsole(ConsoleType::Err, &getLocaleText("driver-unzip-failed", None));
                return Err(getLocaleText("driver-unzip-failed", None).into());
            }
        } else {
            // 解压全部驱动文件
            if !zip.extractFilesFromPath(driverPath, password, "*", &driversPath)? {
                writeConsole(
                    ConsoleType::Err,
                    &getLocaleText("driver-unzip-failed", None),
                );
                return Err(String::from(&getLocaleText("driver-unzip-failed", None)).into());
            };
        }
        real_driver_path = driversPath;
    }

    // 遍历INF文件列表
    let mut infList = getFileList(&real_driver_path, "*.inf")?;

    // 匹配当前设备驱动
    if matchDevice {
        // 获取真实硬件信息
        let devcon = Devcon::new()?;
        let hwIDList = devcon.getRealIdInfo(None)?;
        if hwIDList.is_empty() {
            writeConsole(ConsoleType::Err, &getLocaleText("no-device", None));
            return Err(getLocaleText("no-device", None).into());
        }

        // 解析INF文件
        let mut infInfoList: Vec<InfInfo> = Vec::new();
        for infPath in &infList {
            if let Ok(currentInfo) = InfInfo::parsingInfFile(&real_driver_path, infPath) {
                if !currentInfo.DriverList.is_empty() { infInfoList.push(currentInfo); }
            }
        }

        // 匹配驱动
        let matchHardwareAndDriver = getMatchInfo(&hwIDList, &infInfoList, None);
        if matchHardwareAndDriver.is_empty() {
            writeConsole(ConsoleType::Err, &getLocaleText("no-found-driver-currently", None));
            return Err(String::from(&getLocaleText("no-found-driver-currently", None)).into());
        }

        infList.clear();
        for (_hardware, infInfo) in matchHardwareAndDriver.iter() {
            // 仅匹配第一个最佳的驱动
            if let Some(InfInfo) = infInfo.first(){
                if !zip.extractFilesFromPath(driverPath, password, &InfInfo.Path, &real_driver_path)? {
                    writeConsole(ConsoleType::Err, &getLocaleText("driver-unzip-failed", None));
                    continue;
                }
                infList.push(real_driver_path.join(InfInfo.Path.clone()).join(InfInfo.Inf.clone()));
            }
        }
    }

    // 获取系统架构
    let archCode = match getArchCode(systemDrive) {
        Ok(code @ (0x014c | 0x8664 | 0xAA64)) => code,
        Err(_) | _ => {
            writeConsole(ConsoleType::Err, &getLocaleText("offline-Arch-Err", None));
            return Err(getLocaleText("getOfflineArchErr", None).into());
        },
    };
    let arch = match archCode {
        // x86
        0x014c => 0,
        // x64
        0x8664 => 9,
        // ARM64
        0xAA64 => 12,
        _ => {unreachable!()}
    };

    let mut success_count = 0;
    let mut fail_count = 0;
    let systemRoot = systemDrive.join("Windows");

    if !isOfflineSystem(systemDrive)? {
        // 在线导入驱动
        unsafe {
            let driverStore = DriverStore::new()?;
            // 打开驱动库
            let handle = driverStore.open_store(&*systemRoot, systemDrive)?;

            for infPath in infList {
                match driverStore.import_driver_to_store(handle, &infPath, arch, 0) {
                    Ok(result) => {
                        let infName = Path::new(&result).file_name().and_then(|os_str| os_str.to_str()).unwrap_or(&result);
                        let arg = hash_map!("inf".to_string() => infName.into());
                        writeConsole(ConsoleType::Success, &getLocaleText("driver-import-success", Some(&arg)));
                        success_count += 1;
                    }
                    Err(_) => {
                        let arg = hash_map!("inf".to_string() => infPath.display().to_string().into());
                        writeConsole(ConsoleType::Err, &getLocaleText("driver-import-failed", Some(&arg)));
                        fail_count += 1;
                    }
                };
            }

            // 关闭驱动库
            driverStore.close_store(handle).ok();
        }
    } else {
        // 离线导入驱动
        unsafe {
            let driverStore = DriverStore::new()?;

            for infPath in infList {
                match driverStore.offline_add_driver(&infPath, &systemRoot, systemDrive, 0, arch) {
                    Ok(result) => {
                        let infName = Path::new(&result).file_name().and_then(|os_str| os_str.to_str()).unwrap_or(&result);
                        let arg = hash_map!("inf".to_string() => infName.into());
                        writeConsole(ConsoleType::Success, &getLocaleText("driver-import-success", Some(&arg)));
                        success_count += 1;
                    }
                    Err(_) => {
                        let arg = hash_map!("inf".to_string() => infPath.display().to_string().into());
                        writeConsole(ConsoleType::Err, &getLocaleText("driver-import-failed", Some(&arg)));
                        fail_count += 1;
                    }
                };
            }
        }
    }

    let arg = hash_map!(
        "success".to_string() => success_count.into(),
        "fail".to_string() => fail_count.into(),
        "total".to_string() => (success_count + fail_count).into()
    );
    writeConsole(ConsoleType::Info, &getLocaleText("driver-import-summary", Some(&arg)));
    Ok(())
}
