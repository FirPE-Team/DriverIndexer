use crate::driver_index::{DriverIndex, InfInfo};
use crate::utils::console::{write_console, ConsoleType};
use crate::utils::sevenzip::SevenZip;
use crate::utils::utils::get_file_list;
use crate::TEMP_PATH;
use anyhow::{anyhow, Context, Result};
use rust_i18n::t;
use std::fs;

use std::path::Path;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::time::UNIX_EPOCH;
use threadpool::ThreadPool;

/// 创建索引文件
///
/// # 参数
///
/// - `drive_path` - 驱动路径
/// - `password` - 驱动密码
/// - `save_index_path` - 保存索引路径
///
/// # 返回值
///
/// - `Result<()>` - 创建索引文件结果
pub fn create_index(
    drive_path: &Path,
    password: Option<&str>,
    save_path: &Path,
) -> Result<(u32, u32, u32, u32)> {
    let zip = SevenZip::new().with_context(|| "create sevenZip failed")?;

    // INF文件父路径
    let inf_parent_path;
    // INF文件列表
    let inf_list;
    // 保存索引路径
    let index_path;

    if drive_path.is_dir() {
        // 从驱动目录中创建索引文件
        inf_list = get_file_list(drive_path, "*.inf").with_context(|| "get inf list failed")?;
        inf_parent_path = drive_path.to_path_buf();
        // 如果输入的索引路径是相对路径，则令实际路径为驱动目录所在路径
        index_path = if save_path.is_relative() {
            drive_path.join(save_path)
        } else {
            save_path.to_path_buf()
        };
    } else {
        // 从文件中创建索引文件
        inf_parent_path = TEMP_PATH.join(drive_path.file_stem().unwrap());

        // 解压全部INF文件
        if let Err(e) = zip.extract_files_from_path(drive_path, password, "*.inf", &inf_parent_path)
        {
            drop(zip);
            return Err(anyhow!("{}: {}", t!("driver-unzip-failed"), e));
        }

        // 解压全部 CAT 文件
        let _ = zip.extract_files_from_path(drive_path, password, "*.cat", &inf_parent_path);

        // 从解压目录中获取INF文件列表
        inf_list =
            get_file_list(&inf_parent_path, "*.inf").with_context(|| "get inf list failed")?;

        // 如果输入的索引路径是相对路径，则令实际实际为驱动包所在路径
        index_path = if save_path.is_relative() {
            drive_path.parent().unwrap().join(save_path)
        } else {
            save_path.to_path_buf()
        };
    }

    if inf_list.is_empty() {
        return Err(anyhow!(t!("no-inf-find")));
    }

    // 创建线程池（池大小以可用 CPU 核心数为准）
    let pool = ThreadPool::new(num_cpus::get());
    let (tx, rx) = channel();

    let base_path = Arc::new(inf_parent_path);
    let success_count = Arc::new(AtomicI32::new(0));
    let error_count = Arc::new(AtomicI32::new(0));
    let blank_count = Arc::new(AtomicI32::new(0));

    // 遍历INF文件
    for item in inf_list.clone() {
        let tx = tx.clone();
        let base_path = Arc::clone(&base_path);
        let success_count = Arc::clone(&success_count);
        let error_count = Arc::clone(&error_count);
        let blank_count = Arc::clone(&blank_count);

        // 多线程解析INF文件
        pool.execute(move || {
            match InfInfo::parse_inf(&base_path, &item) {
                Ok(info) => {
                    // 判断inf文件是否包含硬件信息
                    if info.hardware.is_empty() {
                        blank_count.fetch_add(1, Ordering::SeqCst);
                        write_console(
                            ConsoleType::Warning,
                            &format!(
                                "{}: {}",
                                t!("no-hardware"),
                                item.to_string_lossy()
                                    .trim_start_matches(&*TEMP_PATH.to_string_lossy())
                            ),
                        );
                        return;
                    }
                    success_count.fetch_add(1, Ordering::SeqCst);
                    let _ = tx.send(info);
                }
                Err(e) => {
                    error_count.fetch_add(1, Ordering::SeqCst);
                    write_console(
                        ConsoleType::Error,
                        &format!(
                            "{}: {} ({})",
                            t!("inf-parse-error"),
                            item.to_string_lossy()
                                .trim_start_matches(&*TEMP_PATH.to_string_lossy()),
                            e
                        ),
                    );
                }
            }
        });
    }

    // 释放主线程发送通道
    drop(tx);

    // 接收所有线程解析结果
    let mut inf_info_list: Vec<InfInfo> = rx.into_iter().collect();
    if inf_info_list.is_empty() {
        return Err(anyhow!(t!("create-index-failed")));
    }

    // 对INF文件列表进行排序
    InfInfo::sort_inf_list(&mut inf_info_list);

    // 计算驱动包大小
    let size = drive_path
        .metadata()
        .with_context(|| format!("get drive path metadata {}", drive_path.display()))?
        .len();

    // 获取驱动包修改时间戳
    let timestamp = drive_path
        .metadata()
        .with_context(|| format!("get drive path metadata {}", drive_path.display()))?
        .modified()
        .with_context(|| format!("get drive path modified {}", drive_path.display()))?
        .duration_since(UNIX_EPOCH)
        .with_context(|| {
            format!(
                "get drive path duration since unix epoch {}",
                drive_path.display()
            )
        })?
        .as_secs();

    // 创建索引配置文件
    let config = DriverIndex::new(size, timestamp, inf_info_list);
    let json = config
        .to_json()
        .with_context(|| format!("serialize config data to json {}", index_path.display()))?;

    // 保存索引配置文件
    fs::write(&index_path, json).with_context(|| t!("index-save-failed"))?;

    Ok((
        inf_list.len() as u32,
        success_count.load(Ordering::SeqCst) as u32,
        error_count.load(Ordering::SeqCst) as u32,
        blank_count.load(Ordering::SeqCst) as u32,
    ))
}
