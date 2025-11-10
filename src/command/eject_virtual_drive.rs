use crate::utils::console::{write_console, ConsoleType};
use crate::utils::utils::{eject_drive, is_driver_cd};
use anyhow::{anyhow, Result};
use rust_i18n::t;

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
