use windows::Win32::Devices::DeviceAndDriverInstallation::{
    CM_Locate_DevNodeW, CM_Reenumerate_DevNode,
    CM_LOCATE_DEVNODE_NORMAL,
    CONFIGRET,
};

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
