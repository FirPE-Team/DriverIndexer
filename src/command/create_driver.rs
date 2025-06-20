use crate::command::create_index::createIndex;
use crate::i18n::getLocaleText;
use crate::{command, sevenZip, TEMP_PATH};
use std::error::Error;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::{env, fs};

/// 缓冲区大小（512KB）
pub const BUFFER_SIZE: usize = 1024 * 512;

/// # 创建驱动包程序
/// 注意：自身程序需要进行加壳处理，否则7z无法处理压缩包程序
pub fn createDriver(driverPath: &Path, outPath: &Path) -> Result<(), Box<dyn Error>> {
    let zip = sevenZip::new().unwrap();
    let mut driverPath = driverPath.to_path_buf();

    if driverPath.is_file() && !zip.isDriverPackage(&driverPath).unwrap_or(false) {
        return Err(String::from(&getLocaleText("no-driver-package", None)).into());
    }
    if driverPath.is_dir() {
        // 创建驱动索引
        let indexPath = driverPath.join(format!("{}.index", driverPath.file_stem().unwrap().to_str().unwrap()));
        createIndex(&driverPath, None, &indexPath);

        // 打包驱动
        let tempArchivePath = TEMP_PATH.join(format!("{}.7z", driverPath.file_stem().unwrap().to_str().unwrap()));
        if zip.createArchivePage(&driverPath, &tempArchivePath).unwrap_or(false) && !tempArchivePath.exists() {
            return Err(String::from(&getLocaleText("Pack-Driver-failed", None)).into());
        }
        driverPath = tempArchivePath;
    }

    // 写入主程序
    fs::copy(env::current_exe().unwrap(), outPath)?;

    // 以追加模式打开目标文件
    let mut outputFile = OpenOptions::new().append(true).open(outPath)?;
    let mut sourceFile = fs::File::open(&driverPath)?;

    // 缓冲区
    let mut buffer = [0u8; BUFFER_SIZE];
    // 循环读取并写入资源文件
    loop {
        let nbytes = sourceFile.read(&mut buffer)?;
        outputFile.write_all(&buffer[..nbytes])?;
        if nbytes < buffer.len() { break; }
    }

    Ok(())
}

/// 检测到当前程序内嵌驱动包时则自动加载
pub fn selfDriver() -> Result<bool, Box<dyn Error>> {
    let zip = sevenZip::new()?;
    if zip.isDriverPackage(&env::current_exe().unwrap())? == false {
        return Ok(false);
    }

    let mut index: Option<PathBuf> = None;
    // 尝试解压索引文件
    if zip.extractFiles(&env::current_exe().unwrap(), None, "*.index", &TEMP_PATH).unwrap_or(false) {
        let indexList: Vec<PathBuf> = fs::read_dir(&*TEMP_PATH).unwrap().filter_map(|item| item.ok())
            .filter(|item| item.path().extension().unwrap().to_str().unwrap().to_lowercase() == "index")
            .map(|item| item.path()).collect();
        if !indexList.is_empty() {
            index = Option::from(indexList[0].clone());
        }
    };
    command::load_driver::loadDriver(&env::current_exe().unwrap(), None, index, false, None, None)?;
    Ok(true)
}