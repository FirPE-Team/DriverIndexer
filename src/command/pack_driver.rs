use crate::command::create_index::create_index;
use crate::driver_index::DriverIndex;
use crate::utils::sevenzip::SevenZip;
use crate::TEMP_PATH;
use anyhow::{anyhow, Context, Result};
use bincode::error::{DecodeError, EncodeError};
use bincode::{config, Decode, Encode};
use rust_i18n::t;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::{env, fs};

/// 缓冲区大小（512KB）
pub const BUFFER_SIZE: usize = 1024 * 512;

/// 嵌入驱动文件头
#[derive(Serialize, Deserialize, Encode, Decode, Debug)]
pub struct EmbedDriverHead {
    /// 文件头魔数
    head: Vec<u8>,
    /// 嵌入驱动大小（单位：字节）
    size: u64,
    /// 驱动索引
    index: DriverIndex,
    /// 文件尾魔数
    end: Vec<u8>,
}

impl EmbedDriverHead {
    pub fn new(size: u64, index: DriverIndex) -> Self {
        EmbedDriverHead {
            head: [
                vec![0x89].as_slice(),
                b"EmbedDriver".to_vec().as_slice(),
                vec![0x0d, 0x0a, 0x1a, 0x0a].as_slice(),
            ]
            .concat(),
            size,
            index,
            // 资源文件尾(EDEND)
            end: [vec![0x45, 0x44, 0x45, 0x4E, 0x44].as_slice()].concat(),
        }
    }

    /// 获取文件头长度
    pub fn get_len(&self) -> usize {
        self.to_bytes().unwrap().len()
    }

    /// 获取文件头魔数（标识）
    pub fn get_head(&self) -> &Vec<u8> {
        &self.head
    }

    /// 转换为字节
    pub fn to_bytes(&self) -> Result<Vec<u8>, EncodeError> {
        let config = config::standard();
        bincode::encode_to_vec(self, config)
    }

    /// 将字节解析为当前数据
    pub fn from(data: &[u8]) -> Result<(Self, usize), DecodeError> {
        let config = config::standard();
        bincode::decode_from_slice(data, config)
    }
}

/// 创建驱动包程序
///
/// # 参数
/// - `driver_path` 驱动路径（可以是驱动目录或驱动包文件）
/// - `out_path` 输出路径（驱动包程序）
///
/// # 返回值
/// 如果函数成功，则返回 `true`；否则返回 `false`。
///
/// # 注意
/// 自身程序需要进行加壳处理，否则7z无法处理压缩包程序
pub fn pack_driver_program(driver_path: &Path, out_path: &Path) -> Result<()> {
    let zip = SevenZip::new().with_context(|| "Initialize 7z Failed")?;
    let mut driver_path = driver_path.to_path_buf();

    if driver_path.is_file() {
        // 检查驱动路径是否为驱动包文件
        if !zip.is_driver_package(&driver_path).unwrap_or(false) {
            return Err(anyhow!(t!("no-driver-package")));
        }
    } else if driver_path.is_dir() {
        // 创建驱动索引
        create_index(
            &driver_path,
            None,
            &driver_path.join(format!(
                "{}.index",
                driver_path.file_stem().unwrap().to_string_lossy()
            )),
        )?;

        // 打包驱动
        let temp_archive_path = TEMP_PATH.join(format!(
            "{}.7z",
            driver_path.file_stem().unwrap().to_string_lossy()
        ));
        if zip
            .create_archive(&driver_path, &temp_archive_path)
            .unwrap_or(false)
            && !temp_archive_path.exists()
        {
            // 打包失败
            return Err(anyhow!(t!("pack-driver-failed")));
        }
        driver_path = temp_archive_path;
    }

    // 写入主程序
    fs::copy(env::current_exe().unwrap(), out_path).with_context(|| {
        format!(
            "Copy current exe {:?} to {:?} failed",
            env::current_exe().unwrap(),
            out_path
        )
    })?;

    // 以追加模式打开目标文件
    let mut output_file = OpenOptions::new()
        .append(true)
        .open(out_path)
        .with_context(|| format!("Open output file {:?} failed", out_path))?;
    let mut source_file = File::open(&driver_path)
        .with_context(|| format!("Open driver file {:?} failed", driver_path))?;

    // 缓冲区
    let mut buffer = [0u8; BUFFER_SIZE];

    // 循环读取并写入资源文件
    loop {
        let nbytes = source_file
            .read(&mut buffer)
            .with_context(|| format!("Read driver file {:?} failed", driver_path))?;
        output_file
            .write_all(&buffer[..nbytes])
            .with_context(|| format!("Write output file {:?} failed", out_path))?;
        if nbytes < buffer.len() {
            break;
        }
    }

    Ok(())
}
