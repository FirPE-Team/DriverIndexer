// 禁用变量命名警告
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
// 禁用未使用代码警告
#![allow(dead_code)]

extern crate dotenvy_macro;
#[macro_use]
extern crate lazy_static;

rust_i18n::i18n!("locales");

mod cli;
mod command;
mod driver_index;
mod driver_manager;
mod hardware;
mod tests;
mod utils;

use crate::cli::Cli;
use crate::cli::Command;
use crate::command::{check_if_bundled, DriverInstaller};
use crate::driver_index::DriverIndex;
use crate::driver_manager::DriverManger;
use crate::utils::console::{write_console, ConsoleType};
use crate::utils::setupapi::SetupAPI;
use crate::utils::utils::{
    decrypt_password, encrypt_password, get_file_list, get_temp_name, launched_from_explorer,
};
use anyhow::{anyhow, Context};
use clap::Parser;
use dotenvy_macro::dotenv;
use remove_dir_all::remove_dir_all;
use rust_embed::Embed;
use rust_i18n::{set_locale, t};
use std::env::temp_dir;
use std::fs::create_dir_all;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::thread::sleep;
use std::time::Duration;
use std::{env, process};
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

/// 驱动包密码加密密钥
pub static SECRET_KEY: &str = dotenv!("SECRET_KEY");

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
    if let Some(footer) = check_if_bundled() {
        // 创建临时目录
        if !TEMP_PATH.exists() && create_dir_all(&*TEMP_PATH).is_err() {
            write_console(ConsoleType::Error, &t!("temp-create-failed"));
            process::exit(exitcode::IOERR);
        }

        let current_exe = env::current_exe().expect("Get Current Executable Path Failed");
        let driver_loader = DriverInstaller::new();
        let result = match driver_loader.install_driver(
            &current_exe,
            footer.get_password(),
            None,
            true,
            false,
            None,
            None,
            None,
            false,
        ) {
            Ok(_) => Ok(()),
            Err(e) => {
                write_console(ConsoleType::Error, &e.to_string());
                Err(e)
            }
        };
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
    if launched_from_explorer() && env::args().len() == 1 {
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
            if e.to_string() == t!("no-found-driver-currently")
                || e.to_string() == t!("not-found-virtual-drive")
            {
                exitcode::UNAVAILABLE
            } else if e.to_string() == t!("create-index-failed")
                || e.to_string() == t!("no-driver-package")
                || e.to_string() == t!("driver-unzip-failed")
            {
                exitcode::DATAERR
            } else if e.to_string() == t!("index-save-failed") {
                exitcode::CANTCREAT
            } else if e.to_string() == t!("offline-Arch-Err") {
                exitcode::OSERR
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
            compress,
        } => {
            // 创建临时目录
            if !TEMP_PATH.exists() && create_dir_all(&*TEMP_PATH).is_err() {
                write_console(ConsoleType::Error, &t!("temp-create-failed"));
                process::exit(exitcode::IOERR);
            }
            if DEBUG.load(Ordering::Relaxed) {
                write_console(
                    ConsoleType::Debug,
                    &format!("Temp path: {}", TEMP_PATH.display(),),
                );
            };

            // 解密密码
            let mut password = password.clone();
            if let Some(crypt_text) = &password {
                match try_decrypt_password(crypt_text) {
                    Ok(p) => {
                        password = Some(p);
                    }
                    Err(e) => {
                        write_console(
                            ConsoleType::Error,
                            &format!("{}: {}", &t!("decrypt-password-failed"), e),
                        );
                        process::exit(exitcode::DATAERR);
                    }
                }
            }

            // 处理通配符
            if let Some(driver_name) = driver_path.file_name() {
                let driver_name = driver_name.to_string_lossy().to_string();
                if driver_name.contains('*') || driver_name.contains('?') {
                    let driver_list =
                        get_file_list(&PathBuf::from(&driver_path.parent().unwrap()), &driver_name)
                            .with_context(|| "Get driver package list failed")?;
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
                        let index_path_buf = PathBuf::from(index_path);
                        let index_name = index_path_buf.file_name().unwrap().to_str().unwrap();
                        if index_name.contains('*') || index_name.contains('?') {
                            for item in get_file_list(
                                &PathBuf::from(&index_path_buf.parent().unwrap()),
                                index_name,
                            )
                            .with_context(|| "Get index file list failed")?
                            {
                                index_list.push(Some(item));
                            }
                        } else {
                            index_list.push(Some(PathBuf::from(index_path)));
                        }
                    } else {
                        // 为每个驱动包生成默认索引路径
                        for driver_item in &driver_list {
                            let config_name = format!(
                                "{}.index",
                                driver_item
                                    .file_stem()
                                    .unwrap_or("driver".as_ref())
                                    .to_string_lossy()
                            );
                            let config_path = driver_item
                                .parent()
                                .unwrap_or(driver_item)
                                .join(config_name);
                            index_list.push(Some(config_path));
                        }
                    }

                    let mut index_iter = index_list.iter();

                    // 遍历驱动包
                    for driver_path_item in driver_list.iter() {
                        let index = index_iter.next().unwrap().clone();

                        write_console(
                            ConsoleType::Info,
                            &format!(
                                "{}: {}",
                                &t!("create-index-info"),
                                driver_path_item.to_string_lossy()
                            ),
                        );

                        let config_path = if let Some(index) = index {
                            index
                        } else {
                            let config_name = format!(
                                "{}.index",
                                driver_path_item
                                    .file_stem()
                                    .unwrap_or("driver".as_ref())
                                    .to_string_lossy()
                            );
                            driver_path_item
                                .parent()
                                .unwrap_or(driver_path_item)
                                .join(config_name)
                        };

                        match command::create_index(
                            driver_path_item,
                            password.as_deref(),
                            &config_path,
                            *compress,
                        ) {
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
                            }
                            Err(e) => {
                                write_console(ConsoleType::Error, &e.to_string());
                            }
                        }
                    }
                    return Ok(());
                }
            }

            // 无通配符
            let config_path = if let Some(index_path) = index_path {
                index_path
            } else {
                // 没有指定索引文件，使用默认索引文件名(驱动包名.index)
                let config_name = format!(
                    "{}.index",
                    driver_path
                        .file_stem()
                        .unwrap_or("driver".as_ref())
                        .to_string_lossy()
                );
                &driver_path
                    .parent()
                    .unwrap_or(driver_path)
                    .join(config_name)
            };

            write_console(ConsoleType::Info, &t!("create-index-info"));
            match command::create_index(driver_path, password.as_deref(), config_path, *compress) {
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
                DriverIndex::from_path(index_path).with_context(|| "Parse driver index failed")?;
            println!("{}", driver_index.get_driver_index_info());
            Ok(())
        }

        // 加载驱动程序
        Command::Install {
            driver_path,
            index_path,
            skip_verify,
            password,
            missing_only,
            class,
            exclude_class,
            extract_to: extract_path,
            force,
        } => {
            // 创建临时目录
            if !TEMP_PATH.exists() && create_dir_all(&*TEMP_PATH).is_err() {
                write_console(ConsoleType::Error, &t!("temp-create-failed"));
                process::exit(exitcode::IOERR);
            }
            if DEBUG.load(Ordering::Relaxed) {
                write_console(
                    ConsoleType::Debug,
                    &format!("Temp path: {}", TEMP_PATH.display(),),
                );
            };

            // 解密密码
            let mut password = password.clone();
            if let Some(crypt_text) = &password {
                match try_decrypt_password(crypt_text) {
                    Ok(p) => {
                        password = Some(p);
                    }
                    Err(e) => {
                        write_console(
                            ConsoleType::Error,
                            &format!("{}: {}", &t!("decrypt-password-failed"), e),
                        );
                        process::exit(exitcode::DATAERR);
                    }
                }
            }

            let driver_loader = DriverInstaller::new();

            // 处理通配符
            if let Some(driver_name) = driver_path.file_name() {
                let driver_name = driver_name.to_string_lossy().to_string();
                if driver_name.contains('*') || driver_name.contains('?') {
                    let driver_list =
                        get_file_list(&PathBuf::from(&driver_path.parent().unwrap()), &driver_name)
                            .with_context(|| "Get driver package list failed")?;
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
                        let index_name = index_path.file_name().unwrap().to_str().unwrap();
                        if index_name.contains('*') || index_name.contains('?') {
                            for item in get_file_list(
                                &PathBuf::from(&index_path.parent().unwrap()),
                                index_name,
                            )
                            .with_context(|| "Get driver package list failed")?
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

                        match driver_loader.install_driver(
                            drive_path_item,
                            password.as_deref(),
                            index.as_deref(),
                            *skip_verify,
                            *missing_only,
                            class.as_deref(),
                            exclude_class.as_deref(),
                            extract_path.as_deref(),
                            *force,
                        ) {
                            Ok(_) => {}
                            Err(e) => {
                                write_console(ConsoleType::Error, &e.to_string());
                            }
                        }
                    }
                    return Ok(());
                }
            }

            // 无通配符
            write_console(
                ConsoleType::Info,
                &format!(
                    "{}: {}",
                    &t!("driver-install-info"),
                    driver_path.to_string_lossy()
                ),
            );

            match driver_loader.install_driver(
                driver_path,
                password.as_deref(),
                index_path.as_deref(),
                *skip_verify,
                *missing_only,
                class.as_deref(),
                exclude_class.as_deref(),
                extract_path.as_deref(),
                *force,
            ) {
                Ok(_) => Ok(()),
                Err(e) => {
                    write_console(ConsoleType::Error, &e.to_string());
                    Err(e)
                }
            }
        }

        // 加载离线驱动程序
        Command::InstallOffline {
            system_drive,
            missing_only,
            class,
            exclude_class,
        } => {
            // 创建临时目录
            if !TEMP_PATH.exists() && create_dir_all(&*TEMP_PATH).is_err() {
                write_console(ConsoleType::Error, &t!("temp-create-failed"));
                process::exit(exitcode::IOERR);
            }
            if DEBUG.load(Ordering::Relaxed) {
                write_console(
                    ConsoleType::Debug,
                    &format!("Temp path: {}", TEMP_PATH.display(),),
                );
            };

            let driver_loader = DriverInstaller::new();
            match driver_loader.load_offline_driver(
                system_drive.as_deref(),
                *missing_only,
                class.as_deref(),
                exclude_class.as_deref(),
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
            if DEBUG.load(Ordering::Relaxed) {
                write_console(
                    ConsoleType::Debug,
                    &format!("Temp path: {}", TEMP_PATH.display(),),
                );
            };

            // 解密密码
            let mut password = password.clone();
            if let Some(crypt_text) = &password {
                match try_decrypt_password(crypt_text) {
                    Ok(p) => {
                        password = Some(p);
                    }
                    Err(e) => {
                        write_console(
                            ConsoleType::Error,
                            &format!("{}: {}", &t!("decrypt-password-failed"), e),
                        );
                        process::exit(exitcode::DATAERR);
                    }
                }
            }

            let driver_manger = DriverManger::new(system_drive)?;

            // 处理通配符
            if let Some(driver_name) = drive_path.file_name() {
                let driver_name = driver_name.to_string_lossy().to_string();
                if driver_name.contains('*') || driver_name.contains('?') {
                    let drive_list =
                        get_file_list(&PathBuf::from(&drive_path.parent().unwrap()), &driver_name)
                            .with_context(|| "Get driver package list failed")?;
                    if drive_list.is_empty() {
                        write_console(
                            ConsoleType::Error,
                            "No driver package was found in this directory",
                        );
                        return Err(anyhow!("No driver package was found in this directory"));
                    }

                    // 遍历驱动包列表
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
                    return Ok(());
                }
            }

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
            Ok(())
        }

        // 导出驱动程序
        Command::Export {
            system_drive,
            export_path,
            inf,
            class,
            exclude_class,
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
                exclude_class.as_deref(),
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
            exclude_class,
            provider,
        } => {
            let driver_manger = DriverManger::new(system_drive)?;

            match driver_manger.list_driver(
                system_drive,
                class.as_deref(),
                exclude_class.as_deref(),
                provider.as_deref(),
            ) {
                Ok(_) => Ok(()),
                Err(e) => {
                    // write_console(ConsoleType::Error, &e.to_string());
                    let mut level_message = e.to_string();
                    if let Some(cause) = e.source() {
                        level_message = format!("{} ({})", level_message, cause);
                    }
                    write_console(ConsoleType::Error, &level_message);
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
            password,
        } => {
            // 创建临时目录
            if !TEMP_PATH.exists() && create_dir_all(&*TEMP_PATH).is_err() {
                write_console(ConsoleType::Error, &t!("temp-create-failed"));
                process::exit(exitcode::IOERR);
            }
            if DEBUG.load(Ordering::Relaxed) {
                write_console(
                    ConsoleType::Debug,
                    &format!("Temp path: {}", TEMP_PATH.display(),),
                );
            };

            write_console(ConsoleType::Info, &t!("driver-pack-info"));
            match command::pack_driver_program(drive_path, program_path, password.as_deref()) {
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

        // 加密密码子命令
        Command::Encrypt { text } => {
            let encrypted = encrypt_password(text);
            println!("enc:{}", encrypted);
            Ok(())
        }
    }
}

/// 尝试解密密码
///
/// # 参数
/// - `input`：待解密的密码字符串，支持 "enc:" 或 "raw:" 前缀。
///
/// # 返回值
/// - `Ok(String)`：成功解密后的明文密码。
/// - `Err(anyhow::Error)`：解密失败，包含错误信息。
///
/// # 解析规则
/// 1. 如果输入以 `raw:` 开头，则强制视为明文（去除前缀后使用），用于处理密码本身以 "enc:" 开头的情况。
/// 2. 如果输入以 `enc:` 开头，则尝试使用内置密钥进行 AES 解密。
/// 3. 其他情况，视为普通明文密码。
fn try_decrypt_password(input: &str) -> anyhow::Result<String> {
    const ENC_PREFIX: &str = "enc:";
    const RAW_PREFIX: &str = "raw:";

    // 优先处理转义前缀
    if let Some(raw) = input.strip_prefix(RAW_PREFIX) {
        return Ok(raw.to_string());
    }

    // 不是加密格式，直接返回明文
    if !input.starts_with(ENC_PREFIX) {
        return Ok(input.to_string());
    }

    let cipher_text = &input[ENC_PREFIX.len()..];
    decrypt_password(cipher_text)
}
