use crate::command::create_index::create_index;
use crate::utils::sevenzip::SevenZip;
use crate::TEMP_PATH;
use anyhow::{anyhow, Context, Result};
use bincode::error::{DecodeError, EncodeError};
use bincode::{config, Decode, Encode};
use rust_i18n::t;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::{env, fs};

/// 缓冲区大小（512KB）
pub const BUFFER_SIZE: usize = 1024 * 512;

/// 定义魔数
const MAGIC_SIGNATURE: &[u8; 8] = b"DRV_PKG!";

/// 固定长度：`u64(8)` + `u64(8)` + `[u8;8](64)` + `[u8;8](8)`
const FOOTER_SIZE: i64 = 88;

/// 嵌入驱动文件头
#[derive(Serialize, Deserialize, Encode, Decode, Debug)]
pub struct EmbedDriverFooter {
    /// 压缩包的起始偏移量 (相对于文件开头)
    archive_offset: u64,
    /// 压缩包的大小
    archive_size: u64,
    /// 压缩包密码（可选）
    /// 预留 64 字节。如果实际密码长度不足，用 \0 填充。
    /// 如果没有密码，整个数组用 \0 填充。
    #[serde(with = "serde_bytes")]
    password: [u8; 64],
    /// 魔数
    magic: [u8; 8],
}

impl EmbedDriverFooter {
    /// 创建一个新的嵌入驱动文件头
    ///
    /// # 参数
    /// - `offset` 压缩包的起始偏移量 (相对于文件开头)
    /// - `size` 压缩包的大小
    ///
    /// # 返回值
    /// 一个新的 `EmbedDriverFooter` 实例
    pub(crate) fn new(offset: u64, size: u64, password: Option<&str>) -> Self {
        Self {
            archive_offset: offset,
            archive_size: size,
            password: password.map_or([0u8; 64], |p| {
                let mut pwd = p.as_bytes().to_vec();
                pwd.resize(64, 0);
                pwd.try_into().unwrap()
            }),
            magic: *MAGIC_SIGNATURE,
        }
    }

    /// 转换为字节
    pub fn to_bytes(&self) -> Result<Vec<u8>, EncodeError> {
        let config = config::standard().with_fixed_int_encoding();
        bincode::encode_to_vec(self, config)
    }

    /// 将字节解析为当前数据
    pub fn from_bytes(bytes: &[u8]) -> Result<(Self, usize), DecodeError> {
        let config = config::standard().with_fixed_int_encoding();
        bincode::decode_from_slice(bytes, config)
    }

    /// 获取压缩包密码
    ///
    /// # 返回值
    /// - `Some(&str)`: 压缩包密码
    /// - `None`: 没有密码
    pub fn get_password(&self) -> Option<&str> {
        if self.password.iter().all(|&b| b == 0) {
            None
        } else {
            Some(
                std::str::from_utf8(&self.password)
                    .unwrap()
                    .trim_end_matches(char::from(0)),
            )
        }
    }
}

/// 创建驱动包程序
///
/// # 参数
/// - `driver_path` 驱动路径（可以是驱动目录或驱动包文件）
/// - `out_path` 输出路径（驱动包程序）
/// - `password` 压缩包密码（可选）
///
/// # 返回值
/// 如果函数成功，则返回 `true`；否则返回 `false`。
///
/// # 注意
/// 自身程序需要进行加壳处理，否则7z无法处理压缩包程序
pub fn pack_driver_program(
    driver_path: &Path,
    out_path: &Path,
    password: Option<&str>,
) -> Result<()> {
    let zip = SevenZip::new().with_context(|| "Initialize 7z Failed")?;
    let mut driver_path = driver_path.to_path_buf();

    if driver_path.is_file() {
        // 检查驱动路径是否为驱动包文件
        if zip.is_driver_package(&driver_path, None).is_err() {
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
            .is_err()
            || !temp_archive_path.exists()
        {
            // 打包失败
            return Err(anyhow!(t!("pack-driver-failed")));
        }
        driver_path = temp_archive_path;
    }

    // 获取当前程序路径
    let current_exe = env::current_exe().with_context(|| "Get current exe path failed")?;

    // 复制主程序
    fs::copy(&current_exe, out_path).with_context(|| {
        format!(
            "Copy current exe {} to {} failed",
            current_exe.display(),
            out_path.display()
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

    // 获取偏移量 (Host EXE 的大小)
    let archive_offset = current_exe
        .metadata()
        .with_context(|| "Get current exe metadata failed")?
        .len();

    // 获取压缩包大小
    let archive_size = driver_path
        .metadata()
        .with_context(|| format!("Get driver file {} metadata failed", driver_path.display()))?
        .len();

    // 循环读取并写入资源文件
    loop {
        let nbytes = source_file
            .read(&mut buffer)
            .with_context(|| format!("Read driver file {} failed", driver_path.display()))?;
        output_file
            .write_all(&buffer[..nbytes])
            .with_context(|| format!("Write output file {} failed", out_path.display()))?;
        if nbytes < buffer.len() {
            break;
        }
    }

    // 写入驱动包文件头
    let footer = EmbedDriverFooter::new(archive_offset, archive_size, password);
    output_file
        .write_all(
            footer
                .to_bytes()
                .with_context(|| "Serialize footer failed".to_string())?
                .as_slice(),
        )
        .with_context(|| format!("Write output file {} failed", out_path.display()))?;

    Ok(())
}

/// 检查当前程序是否为内置驱动包
///
/// # 返回值
/// 如果当前程序为内置驱动包，则返回压缩包的起始偏移量；否则返回 `None`。
pub fn check_if_bundled() -> Option<EmbedDriverFooter> {
    // 获取当前执行文件路径
    let current_exe = env::current_exe().ok()?;
    let mut file = File::open(&current_exe).ok()?;

    // 移动文件指针到倒数 FOOTER_SIZE 字节的位置
    // 注意：如果有任何 IO 错误（比如文件太小不足 FOOTER_SIZE 个字节），直接返回 None
    if file.seek(SeekFrom::End(-FOOTER_SIZE)).is_err() {
        return None;
    }

    // 读取字节
    let mut buffer = vec![0u8; FOOTER_SIZE as usize];
    if file.read_exact(&mut buffer).is_err() {
        return None;
    }

    if let Ok((footer, _size)) = EmbedDriverFooter::from_bytes(&buffer) {
        // 校验魔数
        if &footer.magic == MAGIC_SIGNATURE {
            return Some(footer);
        }
    }

    None
}
