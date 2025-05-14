use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use crate::cli::cli::{cli, ALL_DEVICE, DRIVE_CLASS, DRIVE_PATH, INDEX_PATH, EXTRACT_DRIVER, PROGRAM_PATH, PASSWORD, SYSTEM_DRIVE, MATCH_DEVICE};
use crate::i18n::getLocaleText;
use crate::command;
use crate::utils::console::{writeConsole, ConsoleType};
use crate::utils::util::{getFileList};
use crate::LOG_PATH;
use clap::ArgMatches;
use fluent_templates::fluent_bundle::FluentValue;

pub fn matches(matches: ArgMatches<'_>) {
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
        let driverPath = PathBuf::from(matches.value_of(DRIVE_PATH).unwrap());
        let password = matches.value_of(PASSWORD);

        let indexPath = if matches.is_present(INDEX_PATH) {
            PathBuf::from(matches.value_of(INDEX_PATH).unwrap())
        } else {
            // 没有指定索引文件，使用默认索引文件名: 驱动包名.index
            let indexName = format!(
                "{}.index",
                driverPath.file_stem().unwrap().to_str().unwrap()
            );
            driverPath.parent().unwrap().join(indexName)
        };

        command::create_index::createIndex(&driverPath, password, &indexPath);
    }

    // 加载驱动
    if let Some(matches) = matches.subcommand_matches("load-driver") {
        let drivePath = PathBuf::from(matches.value_of(DRIVE_PATH).unwrap());
        let password = matches.value_of(PASSWORD);
        let extractPath = matches.value_of(EXTRACT_DRIVER);

        // 处理通配符
        let driveName = drivePath.file_name().unwrap().to_str().unwrap();
        if driveName.contains('*') || driveName.contains('?') {
            let driveList = getFileList(&PathBuf::from(&drivePath.parent().unwrap()), driveName).unwrap();
            if driveList.is_empty() {
                writeConsole(ConsoleType::Err, "No driver package was found in this directory");
                return;
            }

            // 创建索引列表（无索引则使用None）
            let mut indexList: Vec<Option<PathBuf>> = Vec::new();
            if matches.is_present(INDEX_PATH) {
                let inedxPath = PathBuf::from(matches.value_of(INDEX_PATH).unwrap());
                let indexName = inedxPath.file_name().unwrap().to_str().unwrap();
                if indexName.contains('*') || indexName.contains('?') {
                    for item in getFileList(&PathBuf::from(&inedxPath.parent().unwrap()), indexName)
                        .unwrap()
                    {
                        indexList.push(Some(item));
                    }
                } else {
                    indexList.push(Some(PathBuf::from(matches.value_of(INDEX_PATH).unwrap())));
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
                let class = matches.value_of(DRIVE_CLASS).map(|class| class.to_string());
                command::load_driver::loadDriver(
                    drivePathItem,
                    password,
                    index,
                    matches.is_present(ALL_DEVICE),
                    class,
                    extractPath,
                );
            }
        } else {
            // 无通配符
            let index = match matches.is_present(INDEX_PATH) {
                true => Some(PathBuf::from(matches.value_of(INDEX_PATH).unwrap())),
                false => None,
            };
            let class = matches.value_of(DRIVE_CLASS).map(|class| class.to_string());
            command::load_driver::loadDriver(
                &drivePath,
                password,
                index,
                matches.is_present(ALL_DEVICE),
                class,
                extractPath,
            );
        }
    }

    // 导入驱动
    if let Some(matches) = matches.subcommand_matches("import-driver") {
        let systemDrive = PathBuf::from(matches.value_of(SYSTEM_DRIVE).unwrap());
        let drivePath = PathBuf::from(matches.value_of(DRIVE_PATH).unwrap());
        let password = matches.value_of(PASSWORD);
        
        // 处理通配符
        let driveName = drivePath.file_name().unwrap().to_str().unwrap();
        if driveName.contains('*') || driveName.contains('?') {
            let driveList = getFileList(&PathBuf::from(&drivePath.parent().unwrap()), driveName).unwrap();
            if driveList.is_empty() {
                writeConsole(ConsoleType::Err, "No driver package was found in this directory");
                return;
            }
            for item in driveList {
                let args: HashMap<String, FluentValue> = hash_map!("path".to_string() => item.to_str().unwrap().into());
                writeConsole(ConsoleType::Info, &getLocaleText("load-driver-package", Some(&args)));

                command::import_driver::import_driver(&systemDrive, &item, password, matches.is_present(MATCH_DEVICE)).ok();
            }
        } else {
            // 无通配符
            let args: HashMap<String, FluentValue> = hash_map!("path".to_string() => drivePath.to_str().unwrap().into());
            writeConsole(ConsoleType::Info, &getLocaleText("load-driver-package", Some(&args)));

            command::import_driver::import_driver(&systemDrive, &drivePath, password, matches.is_present(MATCH_DEVICE)).ok();
        }
    }

    // 整理驱动
    if let Some(matches) = matches.subcommand_matches("classify-driver") {
        let inputPath = PathBuf::from(matches.value_of(DRIVE_PATH).unwrap());

        let _result = command::classify_driver::classify_driver(&inputPath);
        writeConsole(
            ConsoleType::Success,
            &getLocaleText("Drivers-finishing-complete", None),
        );
    }

    // 创建驱动包程序
    if let Some(matches) = matches.subcommand_matches("create-driver") {
        let inputPath = PathBuf::from(matches.value_of(DRIVE_PATH).unwrap());
        let outputPath = PathBuf::from(matches.value_of(PROGRAM_PATH).unwrap());

        command::create_driver::createDriver(&inputPath, &outputPath).ok();
    }
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
    cli().is_present("debug")
}
