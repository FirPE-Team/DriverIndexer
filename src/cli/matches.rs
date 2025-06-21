use crate::cli::cli::{cli, ALL_DEVICE, DRIVER_NAME, DRIVE_CLASS, DRIVE_PATH, EJECTDRIVERCD, EXPORT_PATH, EXTRACT_PATH, INDEX_PATH, MATCH_DEVICE, PASSWORD, PROGRAM_PATH, RENAME_DRIVER, SYSTEM_DRIVE};
use crate::command;
use crate::i18n::getLocaleText;
use crate::utils::console::{writeConsole, ConsoleType};
use crate::utils::setupAPI;
use crate::utils::util::{ejectDrive, getFileList, isDriverCD};
use crate::LOG_PATH;
use clap::ArgMatches;
use fluent_templates::fluent_bundle::FluentValue;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::path::{Path, PathBuf};

pub fn matches(matches: ArgMatches) -> Result<(), Box<dyn Error>> {
    if isDebug() {
        let arg: HashMap<String, FluentValue> =
            hash_map!("path".to_string() => LOG_PATH.to_str().unwrap().into());
        writeConsole(
            ConsoleType::Info,
            &getLocaleText("opened-debug", Some(&arg)),
        );
    }

    // 创建索引
    if let Some(matches) = matches.subcommand_matches("create-index") {
        let driverPath = PathBuf::from(matches.get_one::<String>(DRIVE_PATH).unwrap());
        let password = matches.get_one(PASSWORD).map(String::as_str);

        let indexPath = if matches.contains_id(INDEX_PATH) {
            PathBuf::from(matches.get_one::<String>(INDEX_PATH).unwrap())
        } else {
            // 没有指定索引文件，使用默认索引文件名: 驱动包名.index
            let indexName = format!("{}.index", driverPath.file_stem().unwrap().to_str().unwrap());
            driverPath.parent().unwrap().join(indexName)
        };

        writeConsole(ConsoleType::Info, &getLocaleText("processing", None));
        return match command::create_index::createIndex(&driverPath, password, &indexPath) {
            Ok(_) => { Ok(()) }
            Err(e) => {
                writeConsole(ConsoleType::Err, &e.to_string());
                Err(e)
            }
        };
    }

    // 加载驱动
    if let Some(matches) = matches.subcommand_matches("load-driver") {
        let drivePath = PathBuf::from(matches.get_one::<String>(DRIVE_PATH).unwrap());
        let password = matches.get_one::<String>(PASSWORD).map(String::as_str);
        let extractPath = matches.get_one::<String>(EXTRACT_PATH).map(String::as_str);

        // 弹出免驱设备虚拟光驱
        if matches.contains_id(EJECTDRIVERCD) {
            for letter in b'C'..=b'Z' {
                let drive = format!("{}:", letter as char);
                let path = Path::new(&drive);
                if path.exists() && isDriverCD(path) {
                    let args: HashMap<String, FluentValue> = hash_map!("drive".to_string() => drive.clone().into());
                    writeConsole(ConsoleType::Info, &getLocaleText("ejecting-driver-cd", Some(&args)));

                    let _ = ejectDrive(path);
                }
            }
        }

        // 处理通配符
        let driveName = drivePath.file_name().unwrap().to_str().unwrap();
        if driveName.contains('*') || driveName.contains('?') {
            let driveList = getFileList(&PathBuf::from(&drivePath.parent().unwrap()), driveName).unwrap();
            if driveList.is_empty() {
                writeConsole(ConsoleType::Err, "No driver package was found in this directory");
                return Err(String::from("No driver package was found in this directory").into());
            }

            // 创建索引列表（无索引则使用None）
            let mut indexList: Vec<Option<PathBuf>> = Vec::new();
            if matches.contains_id(INDEX_PATH) {
                let inedxPath = PathBuf::from(matches.get_one::<String>(INDEX_PATH).unwrap());
                let indexName = inedxPath.file_name().unwrap().to_str().unwrap();
                if indexName.contains('*') || indexName.contains('?') {
                    for item in getFileList(&PathBuf::from(&inedxPath.parent().unwrap()), indexName)
                        .unwrap()
                    {
                        indexList.push(Some(item));
                    }
                } else {
                    indexList.push(Some(PathBuf::from(matches.get_one::<String>(INDEX_PATH).unwrap())));
                }
            } else {
                indexList.append(
                    &mut driveList
                        .iter()
                        .map(|_item| None)
                        .collect::<Vec<Option<PathBuf>>>(),
                );
            }

            let mut indexIter = indexList.iter();

            // 遍历驱动包
            for drivePathItem in driveList.iter() {
                let index = indexIter.next().unwrap().clone();
                let class: Option<String> = matches.get_one::<String>(DRIVE_CLASS).cloned();

                let args: HashMap<String, FluentValue> = hash_map!("path".to_string() => drivePathItem.to_str().unwrap().into());
                writeConsole(ConsoleType::Info, &getLocaleText("load-driver-package", Some(&args)));

                command::load_driver::loadDriver(drivePathItem, password, index, matches.contains_id(ALL_DEVICE), class, extractPath)?;
            }
        } else {
            // 无通配符
            let index = match matches.contains_id(INDEX_PATH) {
                true => Some(PathBuf::from(matches.get_one::<String>(INDEX_PATH).unwrap())),
                false => None,
            };
            let class = matches.get_one::<String>(DRIVE_CLASS).map(|class: &String| class.to_string());

            let args: HashMap<String, FluentValue> = hash_map!("path".to_string() => drivePath.to_str().unwrap().into());
            writeConsole(ConsoleType::Info, &getLocaleText("load-driver-package", Some(&args)));

            command::load_driver::loadDriver(&drivePath, password, index, matches.contains_id(ALL_DEVICE), class, extractPath)?;
        }
    }

    // 加载离线驱动
    if let Some(matches) = matches.subcommand_matches("load-offline-driver") {
        let systemDrive = matches.get_one::<String>(SYSTEM_DRIVE).map(Path::new);
        let class = matches.get_one::<String>(DRIVE_CLASS).cloned();

        return match command::load_offline_driver::load_offline_driver(systemDrive, matches.contains_id(ALL_DEVICE), class) {
            Ok(_) => {
                Ok(())
            }
            Err(e) => {
                writeConsole(ConsoleType::Err, &e.to_string());
                Err(e)
            }
        };
    }

    // 导入驱动
    if let Some(matches) = matches.subcommand_matches("import-driver") {
        let systemDrive = PathBuf::from(matches.get_one::<String>(SYSTEM_DRIVE).unwrap());
        let drivePath = PathBuf::from(matches.get_one::<String>(DRIVE_PATH).unwrap());
        let password = matches.get_one::<&str>(PASSWORD).cloned();

        // 处理通配符
        let driveName = drivePath.file_name().unwrap().to_str().unwrap();
        if driveName.contains('*') || driveName.contains('?') {
            let driveList = getFileList(&PathBuf::from(&drivePath.parent().unwrap()), driveName).unwrap();
            if driveList.is_empty() {
                writeConsole(ConsoleType::Err, "No driver package was found in this directory");
                return Err("No driver package was found in this directory".into());
            }
            for item in driveList {
                let args: HashMap<String, FluentValue> = hash_map!("path".to_string() => item.to_str().unwrap().into());
                writeConsole(ConsoleType::Info, &getLocaleText("load-driver-package", Some(&args)));

                match command::import_driver::import_driver(&systemDrive, &item, password, matches.contains_id(MATCH_DEVICE)) {
                    Ok(_) => {}
                    Err(e) => {
                        writeConsole(ConsoleType::Err, &e.to_string());
                    }
                };
            }
        } else {
            // 无通配符
            let args: HashMap<String, FluentValue> = hash_map!("path".to_string() => drivePath.to_str().unwrap().into());
            writeConsole(ConsoleType::Info, &getLocaleText("load-driver-package", Some(&args)));

            return match command::import_driver::import_driver(&systemDrive, &drivePath, password, matches.contains_id(MATCH_DEVICE)) {
                Ok(_) => {
                    Ok(())
                }
                Err(e) => {
                    writeConsole(ConsoleType::Err, &e.to_string());
                    Err(e)
                }
            };
        }
    }

    // 导出驱动
    if let Some(matches) = matches.subcommand_matches("export-driver") {
        let systemDrive = PathBuf::from(matches.get_one::<String>(SYSTEM_DRIVE).unwrap());
        let exportPath = PathBuf::from(matches.get_one::<String>(EXPORT_PATH).unwrap());
        let name = matches.get_one(DRIVER_NAME).cloned();
        let class = matches.get_one(DRIVE_CLASS).cloned();

        return match command::export_driver::export_driver(&systemDrive, &exportPath, name, class) {
            Ok(_) => {
                writeConsole(ConsoleType::Success, &getLocaleText("driver-export-success", None));
                Ok(())
            }
            Err(_e) => {
                writeConsole(ConsoleType::Err, &getLocaleText("driver-export-failed", None));
                Err(getLocaleText("driver-export-failed", None).into())
            }
        };
    }

    // 删除驱动
    if let Some(matches) = matches.subcommand_matches("remove-driver") {
        let systemDrive = PathBuf::from(matches.get_one::<String>(SYSTEM_DRIVE).unwrap());
        let driveName = matches.get_one(DRIVER_NAME).cloned();
        let class = matches.get_one(DRIVE_CLASS).cloned();

        return match command::remove_driver::remove_driver(&systemDrive, driveName, class) {
            Ok(_) => {
                writeConsole(ConsoleType::Success, &getLocaleText("driver-remove-success", None));
                Ok(())
            }
            Err(e) => {
                writeConsole(ConsoleType::Err, &e.to_string());
                Err(e)
            }
        };
    }

    // 整理驱动
    if let Some(matches) = matches.subcommand_matches("classify-driver") {
        let inputPath = PathBuf::from(matches.get_one::<String>(DRIVE_PATH).unwrap());
        let exportPath = PathBuf::from(matches.get_one::<String>(EXPORT_PATH).unwrap());

        return match command::classify_driver::classify_driver(&inputPath, &exportPath, matches.contains_id(RENAME_DRIVER)) {
            Ok(_) => {
                writeConsole(ConsoleType::Success, &getLocaleText("Drivers-finishing-complete", None));
                Ok(())
            }
            Err(e) => {
                writeConsole(ConsoleType::Err, &getLocaleText("Drivers-finishing-failed", None));
                Err(e)
            }
        };
    }

    // 创建驱动包程序
    if let Some(matches) = matches.subcommand_matches("create-driver") {
        let inputPath = PathBuf::from(matches.get_one::<String>(DRIVE_PATH).unwrap());
        let outputPath = PathBuf::from(matches.get_one::<String>(PROGRAM_PATH).unwrap());

        writeConsole(ConsoleType::Info, &getLocaleText("processing", None));
        return match command::create_driver::createDriver(&inputPath, &outputPath) {
            Ok(_) => {
                writeConsole(ConsoleType::Success, &getLocaleText("Driver-finishing-create", None));
                Ok(())
            }
            Err(e) => {
                writeConsole(ConsoleType::Err, &e.to_string());
                Err(e)
            }
        };
    }

    // 扫描硬件设备更改
    if let Some(_matches) = matches.subcommand_matches("scan-devices") {
        unsafe {
            match setupAPI::rescan() {
                true => {
                    writeConsole(ConsoleType::Success, &getLocaleText("scan-devices-success", None));
                }
                false => {
                    writeConsole(ConsoleType::Err, &getLocaleText("scan-devices-failed", None));
                }
            };
        }
    }


    Ok(())
}

/// 是否为调试模式
pub fn isDebug() -> bool {
    // 调试环境
    if env::var("CARGO_PKG_NAME").is_ok() {
        return false;
    }
    if env::args().skip(1).count() == 0 {
        return false;
    }
    cli().contains_id("debug")
}
