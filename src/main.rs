// 禁用变量命名警告
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
// 禁用未使用代码警告
#![allow(dead_code)]

#[macro_use]
mod macros;
mod cli;
mod i18n;
mod command;
mod utils;

mod bindings {
    ::windows::include_bindings!();
}

#[cfg(test)]
mod tests;

#[macro_use]
extern crate clap;

use std::{env};
use std::env::temp_dir;
use std::fs::create_dir_all;
use std::path::PathBuf;
use crate::utils::console::{writeConsole, ConsoleType};
use crate::i18n::getLocaleText;
use crate::utils::sevenZIP::sevenZip;
use crate::utils::util::getTmpName;
use remove_dir_all::remove_dir_all;
use rust_embed::Embed;

// 设置静态资源

// x64平台
#[cfg(target_arch = "x86_64")]
#[derive(Embed)]
#[folder = "./assets-x64"]
pub struct Asset;

// x86平台
#[cfg(target_arch = "x86")]
#[derive(Embed)]
#[folder = "./assets-x86"]
pub struct Asset;

// ARM平台
#[cfg(target_arch = "aarch64")]
#[derive(Embed)]
#[folder = "./assets-ARM64"]
pub struct Asset;

// 设置静态变量
#[macro_use]
extern crate lazy_static;
lazy_static! {
    pub static ref TEMP_PATH: PathBuf = temp_dir().join(getTmpName(".tmp", "", 6));
    pub static ref LOG_PATH: PathBuf = env::current_dir().unwrap().join(PathBuf::from(env::current_exe().unwrap().file_stem().unwrap()).with_extension("log"));
}

fn main() {
    // 创建临时目录
    if !TEMP_PATH.exists() && create_dir_all(&*TEMP_PATH).is_err(){
        writeConsole(ConsoleType::Err, &getLocaleText("temp-create-failed", None));
        return;
    }

    // 检测到当前程序内嵌驱动包时则自动加载
    if command::create_driver::selfDriver() {
        remove_dir_all(&*TEMP_PATH).ok();
        return;
    };

    // 处理CLI
    let matches = cli::cli::cli();
    cli::matches::matches(matches);

    // 清除临时目录
    if TEMP_PATH.exists() && remove_dir_all(&*TEMP_PATH).is_err() {
        writeConsole(ConsoleType::Err, &getLocaleText("temp-remove-failed", None));
    }
}
