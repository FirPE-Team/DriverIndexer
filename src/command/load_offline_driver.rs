use crate::command::load_driver::loadDriver;
use crate::i18n::getLocaleText;
use crate::utils::console::{writeConsole, ConsoleType};
use crate::utils::util::findOfflineSystemDrive;
use fluent_templates::fluent_bundle::FluentValue;
use std::collections::HashMap;
use std::error::Error;
use std::path::Path;

/// 加载离线系统中的驱动
/// 参数
/// 1. 系统盘（可选，None则全盘搜索[排除当前系统盘]）
/// 2. 是否匹配全部设备（默认匹配未安装驱动的设备）
/// 3. 驱动类别
pub fn load_offline_driver(systemDrive: Option<&Path>, isAllDevice: bool, driveClass: Option<String>) -> Result<(), Box<dyn Error>> {
    if let Some(systemDrive) = systemDrive {
        let driverPath = systemDrive.join("Windows").join("System32").join("DriverStore").join("FileRepository");
        if !driverPath.exists() {
            writeConsole(ConsoleType::Err, &getLocaleText("path-not-exist", None));
            return Err(getLocaleText("path-not-exist", None).into());
        }
        let args: HashMap<String, FluentValue> = hash_map!("path".to_string() => systemDrive.to_str().unwrap().into());
        writeConsole(ConsoleType::Info, &getLocaleText("load-offline-driver", Some(&args)));
        return loadDriver(&*driverPath, None, None, false, None, None);
    }

    // 未指定系统盘，全盘搜索离线系统驱动
    let offlineSystemDriveList = findOfflineSystemDrive();

    // 未找到离线系统
    if offlineSystemDriveList.len() == 0 {
        writeConsole(ConsoleType::Err, &getLocaleText("not-found-offline-system", None));
        return Err(getLocaleText("not-found-offline-system", None).into());
    }

    // 遍历离线系统加载驱动
    for systemDrive in findOfflineSystemDrive() {
        let args: HashMap<String, FluentValue> = hash_map!("path".to_string() => systemDrive.to_str().unwrap().into());
        writeConsole(ConsoleType::Info, &getLocaleText("load-offline-driver", Some(&args)));
        loadDriver(&*systemDrive, None, None, isAllDevice, driveClass.clone(), None)?;
    }
    Ok(())
}
