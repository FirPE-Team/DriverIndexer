use std::error::Error;
use uuid::Uuid;
use windows::core::GUID;
use windows::Win32::Devices::DeviceAndDriverInstallation::{CM_Locate_DevNodeW, CM_Reenumerate_DevNode, SetupDiGetClassDescriptionW, CM_LOCATE_DEVNODE_NORMAL, CONFIGRET};

// https://docs.microsoft.com/zh-cn/windows-hardware/drivers/install/using-device-installation-functions
/// 获取硬件信息
/// [参考资料](https://docs.microsoft.com/zh-cn/windows/win32/api/setupapi/nf-setupapi-setupdigetclassdevsexa)
pub unsafe fn getDeviceInfo() {
    // let _hdevInfo: *mut c_void = SetupDiGetClassDevsW(null_mut(), PWSTR::NULL, HWND::NULL, DIGCF_ALLCLASSES);

    // if HANDLE::from(hdevInfo) == INVALID_HANDLE_VALUE {
    //     println!("错误码: {:?}", GetLastError());
    //     return;
    // }
    // println!("{:?}", hdevInfo);
}

/// 扫描检测硬件改动
///
/// 返回
/// - `bool`: 成功返回`true`，失败返回`false`
/// [参考资料](https://www.shuzhiduo.com/A/D854GRg3JE)
pub unsafe fn rescan() -> bool {
    let devInst: *mut u32 = &mut 0;

    let status = CM_Locate_DevNodeW(devInst, None, CM_LOCATE_DEVNODE_NORMAL);
    if status != CONFIGRET(0) {
        return false;
    }

    let status = CM_Reenumerate_DevNode(*devInst, windows::Win32::Devices::DeviceAndDriverInstallation::CM_REENUMERATE_FLAGS(0_u32));
    if status != CONFIGRET(0) {
        return false;
    }
    true
}

/// 获取驱动GUID类说明
///
/// 参数
/// - `guid`: guid 类
///
/// 返回
/// - `Ok(String)`:  成功返回类名的描述
/// - `Err(...)`：   失败则返回错误
pub unsafe fn get_class_description(guid: &str) -> Result<String, Box<dyn Error>> {
    let uuid = Uuid::parse_str(guid)?;
    let (data1, data2, data3, data4) = uuid.as_fields();
    let guid_raw = GUID { data1, data2, data3, data4: *data4 };

    let mut buf: [u16; 256] = [0; 256];
    let mut needed: u32 = 0;

    SetupDiGetClassDescriptionW(&guid_raw as *const GUID, &mut buf, Some(&mut needed))?;

    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Ok(String::from_utf16_lossy(&buf[..len]))
}
