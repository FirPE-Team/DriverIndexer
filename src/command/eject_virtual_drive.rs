use crate::utils::console::{write_console, ConsoleType};
use crate::utils::utils::{eject_drive, get_drive_bus, get_drive_space, get_drive_type};
use anyhow::{anyhow, Result};
use rust_i18n::t;
use std::fs;

use std::path::Path;

/// 卸载免驱设备
pub fn eject_virtual_drive() -> Result<()> {
    let mut count = 0;

    for letter in b'C'..=b'Z' {
        let drive = format!("{}:", letter as char);
        let path = Path::new(&drive);

        if path.exists() && is_driver_cd(path) {
            count += 1;
            match eject_drive(path) {
                Ok(_) => {
                    write_console(
                        ConsoleType::Success,
                        &t!("ejecting-virtual-drive", drive = drive.clone()),
                    );
                }
                Err(_e) => {
                    write_console(ConsoleType::Error, &t!("ejecting-virtual-drive"));
                }
            };
        }
    }

    if count == 0 {
        return Err(anyhow!(t!("not-found-virtual-drive")));
    }

    Ok(())
}

/// 判断指定盘符是否为免驱设备虚拟的CDROM盘符
///
/// # 参数
/// - `drive_path`: 盘符
///
/// # 返回值
/// - `true`: 是
/// - `false`: 不是
pub fn is_driver_cd(drive_path: &Path) -> bool {
    // 判断是否为CDROM
    if get_drive_type(drive_path) != 5 {
        return false;
    }

    // 判断总线是否为USB
    if get_drive_bus(drive_path) != Some(7) {
        return false;
    }

    // 判断容量是否小于32MB
    if get_drive_space(drive_path).is_some_and(|space| space > 32 * 1024 * 1024) {
        return false;
    }

    // 判断是否存在exe驱动安装包
    if !fs::read_dir(drive_path)
        .ok()
        .map(|entries| {
            entries.flatten().any(|entry| {
                entry
                    .path()
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("exe"))
            })
        })
        .unwrap_or(false)
    {
        return false;
    };

    true
}
