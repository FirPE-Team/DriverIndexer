use crate::utils::drvstore::DriverStore;
use crate::utils::setupapi::SetupAPI;
use crate::utils::utils::{copy_dir, get_file_list};
use anyhow::{Context, Result};
use std::fs::create_dir_all;
use std::path::Path;

/// 分类驱动
///
/// # 参数
///
/// - `driver_path` - 驱动目录路径
/// - `output_path` - 输出目录路径
/// - `rename` - 是否重命名驱动
///
/// # 返回值
///
/// - `Result<()>` - 分类结果
pub fn organize_driver(driver_path: &Path, output_path: &Path, rename: bool) -> Result<()> {
    // 遍历INF文件
    let inf_list = get_file_list(driver_path, "*.inf")?;

    // 创建驱动存储
    let driver_store = DriverStore::new(None).with_context(|| "Failed to create driver store")?;

    for inf_path in inf_list.iter() {
        // 打开驱动
        let driver_handle = driver_store
            .open_driver(inf_path, 0)
            .with_context(|| format!("Failed to open driver for {:?}", inf_path))?;

        // 获取驱动信息
        let driver_info = driver_store
            .get_version_info(driver_handle)
            .with_context(|| format!("Failed to get version info for {:?}", inf_path))?;

        // 获取驱动类描述
        let class_description = SetupAPI::get_class_description_from_guid(&driver_info.class_guid)
            .with_context(|| {
                format!(
                    "Failed to get class description for {:?}",
                    driver_info.class_guid
                )
            })?;

        // 获取驱动供应商名称
        let provider_name = driver_info.provider_name.trim().to_string();

        // 关闭驱动
        driver_store.close_package(driver_handle).ok();

        // 构建目标路径(驱动类\供应商\驱动名)
        let driver_root = inf_path.parent().unwrap();
        let driver_name = if rename {
            inf_path.file_stem().unwrap()
        } else {
            driver_root.file_name().unwrap()
        };
        let target = output_path
            .join(class_description)
            .join(provider_name)
            .join(driver_name);

        // 创建目标目录
        create_dir_all(&target).with_context(|| format!("Failed to create dir {:?}", target))?;
        // 复制驱动文件
        copy_dir(driver_root, &target)
            .with_context(|| format!("Failed to copy dir {:?}", driver_root))?;
    }
    Ok(())
}
