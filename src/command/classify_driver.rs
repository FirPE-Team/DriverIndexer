use crate::utils::drvstoreAPI::DriverStore;
use crate::utils::setupAPI::get_class_description;
use crate::utils::util::{copy_dir, getFileList};
use std::error::Error;
use std::fs::create_dir_all;
use std::path::Path;

pub fn classify_driver(driver_path: &Path, output_path: &Path, rename_driver: bool) -> Result<(), Box<dyn Error>> {
    // 遍历INF文件
    let infList = getFileList(driver_path, "*.inf")?;

    unsafe {
        for infPath in infList.iter() {
            let driver_store = DriverStore::new()?;
            let driver_handle = driver_store.open_driver(infPath, 0)?;

            let driver_info = driver_store.get_version_info(driver_handle)?;
            let class_description = get_class_description(driver_info.class_guid)?;
            let provider_name = driver_info.provider_name.trim();
            driver_store.close_package(driver_handle).ok();

            let driver_root = infPath.parent().unwrap();
            let driver_name = if rename_driver {
                infPath.file_stem().unwrap()
            } else {
                driver_root.file_name().unwrap()
            };
            let target = output_path.join(class_description).join(&provider_name).join(driver_name);

            // println!("Moved `{:?}` => `{:?}`", driver_root, target);
            create_dir_all(&target)?;
            copy_dir(driver_root, &target)?;
        }
    }
    Ok(())
}
