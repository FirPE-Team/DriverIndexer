// 禁用变量命名警告
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
// 禁用未使用代码警告
#![allow(dead_code)]

#[macro_use]
extern crate lazy_static;
rust_i18n::i18n!("locales");

mod cli;
mod command;
mod driver_index;
mod driver_manger;
mod hardware;
mod tests;
mod utils;

use crate::cli::Cli;
use crate::cli::Command;
use crate::command::{check_if_bundled, DriverInstaller};
use crate::driver_index::DriverIndex;
use crate::driver_manger::DriverManger;
use crate::utils::console::{write_console, ConsoleType};
use crate::utils::setupapi::SetupAPI;
use crate::utils::utils::{get_file_list, get_temp_name, launched_from_explorer};
use anyhow::{anyhow, Context};
use clap::Parser;
use remove_dir_all::remove_dir_all;
use rust_embed::Embed;
use rust_i18n::{set_locale, t};
use std::env::temp_dir;
use std::fs::create_dir_all;
use std::path::PathBuf;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::thread::sleep;
use std::time::Duration;
use sys_locale::get_locale;

// 设置静态资源: x64平台
#[cfg(target_arch = "x86_64")]
#[derive(Embed)]
#[folder = "./assets-x64"]
pub struct Asset;

// 设置静态资源: x86平台
#[cfg(target_arch = "x86")]
#[derive(Embed)]
#[folder = "./assets-x86"]
pub struct Asset;

// 设置静态资源: ARM64平台
#[cfg(target_arch = "aarch64")]
#[derive(Embed)]
#[folder = "./assets-ARM64"]
pub struct Asset;

static DEBUG: AtomicBool = AtomicBool::new(false);
static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();

lazy_static! {
    pub static ref TEMP_PATH: PathBuf = temp_dir().join(get_temp_name(".tmp", "", 6));
}

fn main() {
    // 设置国际化
    let system_locale = get_locale().unwrap_or("en".into());
    match system_locale.as_str() {
        "zh-CN" => set_locale("zh-CN"),
        "zh-TW" => set_locale("zh-TW"),
        "ja-JP" => set_locale("ja-JP"),
        "ko-KR" => set_locale("ko-KR"),
        _ => set_locale("en"),
    };

    // 检测到当前程序内嵌驱动包时则自动加载驱动
    if check_if_bundled().is_some() {
        // 创建临时目录
        if !TEMP_PATH.exists() && create_dir_all(&*TEMP_PATH).is_err() {
            write_console(ConsoleType::Error, &t!("temp-create-failed"));
            process::exit(exitcode::IOERR);
        }

        let driver_loader = DriverInstaller::new();
        let result = driver_loader.load_self_driver_program();
        if TEMP_PATH.exists()
            && let Err(e) = remove_dir_all(&*TEMP_PATH)
        {
            write_console(
                ConsoleType::Warning,
                &format!("{}: {}", t!("temp-remove-failed"), e),
            );
        }
        process::exit(if result.is_ok() { 0 } else { 1 });
    }

    // 判断是否从资源管理器启动
    if launched_from_explorer() {
        println!("{}", t!("cmdline_tool_tips"));
        sleep(Duration::from_secs(5));
        process::exit(exitcode::OK);
    }

    // 处理CLI
    let cli = Cli::parse();
    // 设置调试模式
    if cli.debug {
        DEBUG.store(true, Ordering::Relaxed);
    }
    // 设置日志文件路径
    if let Some(log_path) = &cli.log_path {
        LOG_PATH.set(PathBuf::from(log_path)).ok();
    }
    // 设置语言
    if let Some(language) = &cli.language {
        set_locale(match language {
            cli::Language::En => "en",
            cli::Language::ZhCn => "zh-CN",
            cli::Language::ZhTw => "zh-TW",
            cli::Language::JaJp => "ja-JP",
            cli::Language::KoKr => "ko-KR",
        });
    }

    // 处理子命令
    let result = handle_subcommand(&cli);

    // 清除临时目录
    if TEMP_PATH.exists()
        && let Err(e) = remove_dir_all(&*TEMP_PATH)
    {
        write_console(
            ConsoleType::Warning,
            &format!("{}: {}", t!("temp-remove-failed"), e),
        );
    }

    // 退出程序
    process::exit(match result {
        Ok(_) => exitcode::OK,
        Err(e) => {
            if e.to_string() == t!("no-found-driver-currently") {
                exitcode::CONFIG
            } else {
                1
            }
        }
    });
}

/// 处理子命令
fn handle_subcommand(cli: &Cli) -> anyhow::Result<()> {
    match &cli.command {
        // 创建索引
        Command::Index {
            driver_path,
            index_path,
            password,
        } => {
            // 创建临时目录
            if !TEMP_PATH.exists() && create_dir_all(&*TEMP_PATH).is_err() {
                write_console(ConsoleType::Error, &t!("temp-create-failed"));
                process::exit(exitcode::IOERR);
            }

            let config_path = if let Some(index_path) = index_path {
                index_path
            } else {
                // 没有指定索引文件，使用默认索引文件名(驱动包名.index)
                let config_name = format!(
                    "{}.index",
                    driver_path.file_stem().unwrap().to_string_lossy()
                );
                &driver_path.parent().unwrap().join(config_name)
            };

            write_console(ConsoleType::Info, &t!("create-index-info"));
            match command::create_index(driver_path, password.as_deref(), config_path) {
                Ok((total, success_count, error_count, blank_count)) => {
                    // 打印统计信息
                    write_console(
                        ConsoleType::Info,
                        &t!(
                            "total-info",
                            total = total,
                            success = success_count,
                            error = error_count,
                            blank = blank_count,
                        ),
                    );
                    write_console(
                        ConsoleType::Success,
                        &t!(
                            "save-info",
                            path = config_path.to_string_lossy().to_string()
                        ),
                    );
                    Ok(())
                }
                Err(e) => {
                    write_console(ConsoleType::Error, &e.to_string());
                    Err(e)
                }
            }
        }

        // 索引信息
        Command::Info { index_path } => {
            let driver_index =
                DriverIndex::from_json(index_path).with_context(|| "Parse driver index failed")?;
            println!("{}", driver_index.get_driver_index_info());
            Ok(())
        }

        // 加载驱动程序
        Command::Install {
            driver_path: drive_path,
            index_path,
            password,
            missing_only,
            class,
            extract_to: extract_path,
            force,
        } => {
            // 创建临时目录
            if !TEMP_PATH.exists() && create_dir_all(&*TEMP_PATH).is_err() {
                write_console(ConsoleType::Error, &t!("temp-create-failed"));
                process::exit(exitcode::IOERR);
            }

            let driver_loader = DriverInstaller::new();

            // 处理通配符
            let drive_name = drive_path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string();
            if drive_name.contains('*') || drive_name.contains('?') {
                let driver_list =
                    get_file_list(&PathBuf::from(&drive_path.parent().unwrap()), &drive_name)
                        .unwrap();
                if driver_list.is_empty() {
                    write_console(
                        ConsoleType::Error,
                        "No driver package was found in this directory",
                    );
                    return Err(anyhow!("No driver package was found in this directory"));
                }

                // 创建索引列表（无索引则使用None）
                let mut index_list: Vec<Option<PathBuf>> = Vec::new();
                if let Some(index_path) = &index_path {
                    let inedx_path = PathBuf::from(index_path);
                    let index_name = inedx_path.file_name().unwrap().to_str().unwrap();
                    if index_name.contains('*') || index_name.contains('?') {
                        for item in
                            get_file_list(&PathBuf::from(&inedx_path.parent().unwrap()), index_name)
                                .unwrap()
                        {
                            index_list.push(Some(item));
                        }
                    } else {
                        index_list.push(Some(PathBuf::from(index_path)));
                    }
                } else {
                    index_list.append(
                        &mut driver_list
                            .iter()
                            .map(|_item| None)
                            .collect::<Vec<Option<PathBuf>>>(),
                    );
                }

                let mut index_iter = index_list.iter();

                // 遍历驱动包
                for drive_path_item in driver_list.iter() {
                    let index = index_iter.next().unwrap().clone();
                    let class = class.clone();

                    write_console(
                        ConsoleType::Info,
                        &format!(
                            "{}: {}",
                            &t!("driver-install-info"),
                            drive_path_item.to_string_lossy()
                        ),
                    );

                    driver_loader
                        .install_driver(
                            drive_path_item,
                            password.as_deref(),
                            index.as_deref(),
                            *missing_only,
                            class.as_deref(),
                            extract_path.as_deref(),
                            *force,
                        )
                        .ok();
                }
                Ok(())
            } else {
                // 无通配符
                write_console(
                    ConsoleType::Info,
                    &format!(
                        "{}: {}",
                        &t!("driver-install-info"),
                        drive_path.to_string_lossy()
                    ),
                );

                match driver_loader.install_driver(
                    drive_path,
                    password.as_deref(),
                    index_path.as_deref(),
                    *missing_only,
                    class.as_deref(),
                    extract_path.as_deref(),
                    *force,
                ) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(e),
                }
            }
        }

        // 加载离线驱动程序
        Command::InstallOffline {
            system_drive,
            missing_only,
            class,
        } => {
            // 创建临时目录
            if !TEMP_PATH.exists() && create_dir_all(&*TEMP_PATH).is_err() {
                write_console(ConsoleType::Error, &t!("temp-create-failed"));
                process::exit(exitcode::IOERR);
            }

            let driver_loader = DriverInstaller::new();
            match driver_loader.load_offline_driver(
                system_drive.as_deref(),
                *missing_only,
                class.as_deref(),
            ) {
                Ok(_) => Ok(()),
                Err(e) => {
                    write_console(ConsoleType::Error, &e.to_string());
                    Err(e)
                }
            }
        }

        // 导入驱动程序
        Command::Import {
            system_drive,
            driver_path: drive_path,
            password,
            match_all,
        } => {
            // 创建临时目录
            if !TEMP_PATH.exists() && create_dir_all(&*TEMP_PATH).is_err() {
                write_console(ConsoleType::Error, &t!("temp-create-failed"));
                process::exit(exitcode::IOERR);
            }

            // 处理通配符
            let drive_name = drive_path.file_name().unwrap().to_str().unwrap();
            let driver_manger = DriverManger::new(system_drive)?;

            if drive_name.contains('*') || drive_name.contains('?') {
                let drive_list =
                    get_file_list(&PathBuf::from(&drive_path.parent().unwrap()), drive_name)
                        .unwrap();
                if drive_list.is_empty() {
                    write_console(
                        ConsoleType::Error,
                        "No driver package was found in this directory",
                    );
                    return Err(anyhow!("No driver package was found in this directory"));
                }
                for item in drive_list {
                    write_console(
                        ConsoleType::Info,
                        &format!("{}: {}", &t!("driver-import-info"), item.to_string_lossy()),
                    );

                    match driver_manger.import_driver(
                        system_drive,
                        &item,
                        password.as_deref(),
                        *match_all,
                    ) {
                        Ok((success_count, fail_count, total_count)) => {
                            write_console(
                                ConsoleType::Info,
                                &t!(
                                    "driver-import-summary",
                                    success = success_count.to_string(),
                                    fail = fail_count.to_string(),
                                    total = total_count.to_string()
                                ),
                            );
                        }
                        Err(e) => {
                            write_console(ConsoleType::Error, &e.to_string());
                        }
                    };
                }
            } else {
                // 无通配符
                write_console(
                    ConsoleType::Info,
                    &format!(
                        "{}: {}",
                        &t!("driver-import-info"),
                        drive_path.to_string_lossy()
                    ),
                );

                match driver_manger.import_driver(
                    system_drive,
                    drive_path,
                    password.as_deref(),
                    *match_all,
                ) {
                    Ok((success_count, fail_count, total_count)) => {
                        write_console(
                            ConsoleType::Info,
                            &t!(
                                "driver-import-summary",
                                success = success_count.to_string(),
                                fail = fail_count.to_string(),
                                total = total_count.to_string()
                            ),
                        );
                    }
                    Err(e) => {
                        write_console(ConsoleType::Error, &e.to_string());
                        return Err(e);
                    }
                }
            }
            Ok(())
        }

        // 导出驱动程序
        Command::Export {
            system_drive,
            export_path,
            inf,
            class,
            provider,
        } => {
            // 创建临时目录
            if !TEMP_PATH.exists() && create_dir_all(&*TEMP_PATH).is_err() {
                write_console(ConsoleType::Error, &t!("temp-create-failed"));
                process::exit(exitcode::IOERR);
            }

            let driver_manger = DriverManger::new(system_drive)?;
            match driver_manger.export_driver(
                system_drive,
                export_path,
                inf.as_deref(),
                class.as_deref(),
                provider.as_deref(),
            ) {
                Ok((success_count, fail_count, total_count)) => {
                    write_console(
                        ConsoleType::Info,
                        &t!(
                            "driver-export-summary",
                            success = success_count.to_string(),
                            fail = fail_count.to_string(),
                            total = total_count.to_string()
                        ),
                    );
                    Ok(())
                }
                Err(e) => {
                    write_console(ConsoleType::Error, &e.to_string());
                    Err(e)
                }
            }
        }

        // 卸载驱动程序
        Command::Remove {
            system_drive,
            inf,
            class,
            provider,
            all,
        } => {
            // 创建临时目录
            if !TEMP_PATH.exists() && create_dir_all(&*TEMP_PATH).is_err() {
                write_console(ConsoleType::Error, &t!("temp-create-failed"));
                process::exit(exitcode::IOERR);
            }

            let driver_manger = DriverManger::new(system_drive)?;
            match driver_manger.remove_driver(
                system_drive,
                inf.as_deref(),
                class.as_deref(),
                provider.as_deref(),
                *all,
            ) {
                Ok((success_count, fail_count, total_count)) => {
                    // 输出导出统计信息
                    write_console(
                        ConsoleType::Info,
                        &t!(
                            "driver-remove-summary",
                            success = success_count.to_string(),
                            fail = fail_count.to_string(),
                            total = total_count.to_string()
                        ),
                    );
                    Ok(())
                }
                Err(e) => {
                    write_console(ConsoleType::Error, &e.to_string());
                    Err(e)
                }
            }
        }

        // 列举驱动程序
        Command::List {
            system_drive,
            class,
            provider,
        } => {
            let driver_manger = DriverManger::new(system_drive)?;

            match driver_manger.list_driver(system_drive, class.as_deref(), provider.as_deref()) {
                Ok(_) => Ok(()),
                Err(e) => {
                    write_console(ConsoleType::Error, &e.to_string());
                    Err(e)
                }
            }
        }

        // 分类驱动程序
        Command::Organize {
            drive_path,
            export_path,
            rename: rename_driver,
        } => match command::organize_driver(drive_path, export_path, *rename_driver) {
            Ok(_) => {
                write_console(ConsoleType::Success, &t!("organize-driver-success"));
                Ok(())
            }
            Err(e) => {
                write_console(ConsoleType::Error, &t!("organize-driver-failed"));
                Err(e)
            }
        },

        // 创建驱动程序
        Command::Pack {
            drive_path,
            program_path,
        } => {
            // 创建临时目录
            if !TEMP_PATH.exists() && create_dir_all(&*TEMP_PATH).is_err() {
                write_console(ConsoleType::Error, &t!("temp-create-failed"));
                process::exit(exitcode::IOERR);
            }

            match command::pack_driver_program(drive_path, program_path) {
                Ok(_) => {
                    write_console(ConsoleType::Success, &t!("pack-driver-success"));
                    Ok(())
                }
                Err(e) => {
                    write_console(ConsoleType::Error, &e.to_string());
                    Err(e)
                }
            }
        }

        // 扫描设备子命令
        Command::Scan => match SetupAPI::rescan() {
            true => {
                write_console(ConsoleType::Success, &t!("scan-devices-success"));
                Ok(())
            }
            false => {
                write_console(ConsoleType::Error, &t!("scan-devices-failed"));
                Err(anyhow!("scan-devices-failed"))
            }
        },

        // 卸载驱动CD
        Command::Eject => match command::eject_virtual_drive() {
            Ok(_) => Ok(()),
            Err(e) => {
                write_console(ConsoleType::Error, &e.to_string());
                Err(e)
            }
        },
    }
}
