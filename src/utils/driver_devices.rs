//! Online removal of devices bound to a selected published INF.
//! Called only by the online removal backend, never by default tests.
use anyhow::{Result, anyhow};
use windows::{
    Win32::{
        Devices::{
            DeviceAndDriverInstallation::*,
            Properties::{DEVPKEY_Device_DriverInfPath, DEVPROP_TYPE_STRING, DEVPROPTYPE},
        },
        Foundation::{
            DEVPROPKEY, ERROR_INSUFFICIENT_BUFFER, ERROR_NO_MORE_ITEMS, ERROR_NOT_FOUND, HWND,
        },
    },
    core::{BOOL, GUID, PCWSTR},
};
struct DeviceSet(HDEVINFO);
impl Drop for DeviceSet {
    fn drop(&mut self) {
        let _ = unsafe { SetupDiDestroyDeviceInfoList(self.0) };
    }
}
fn property(set: HDEVINFO, data: &SP_DEVINFO_DATA, key: &DEVPROPKEY) -> Result<Option<String>> {
    let mut buf = vec![0u8; 520];
    let mut required = 0;
    let mut kind = DEVPROPTYPE(0);
    let mut result = unsafe {
        SetupDiGetDevicePropertyW(
            set,
            data,
            key,
            &mut kind,
            Some(&mut buf),
            Some(&mut required),
            0,
        )
    };
    if result
        .as_ref()
        .is_err_and(|e| e.code() == ERROR_INSUFFICIENT_BUFFER.to_hresult())
    {
        buf.resize(required as usize, 0);
        result = unsafe {
            SetupDiGetDevicePropertyW(
                set,
                data,
                key,
                &mut kind,
                Some(&mut buf),
                Some(&mut required),
                0,
            )
        };
    }
    if let Err(e) = result {
        if e.code() == ERROR_NOT_FOUND.to_hresult() {
            return Ok(None);
        }
        return Err(e.into());
    }
    if kind != DEVPROP_TYPE_STRING || required as usize > buf.len() || required % 2 != 0 {
        return Err(anyhow!("invalid driver INF device property"));
    }
    let wide: Vec<_> = buf[..required as usize]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .take_while(|c| *c != 0)
        .collect();
    Ok(Some(String::from_utf16(&wide)?))
}
pub fn remove_bound_devices(published_name: &str, published_path: &std::path::Path) -> Result<()> {
    type Uninstall =
        unsafe extern "system" fn(HWND, HDEVINFO, *const SP_DEVINFO_DATA, u32, *mut BOOL) -> i32;
    let root = crate::utils::utils::get_current_system_root()?;
    let dll = unsafe { libloading::Library::new(root.join("System32/newdev.dll")) }?;
    let uninstall: Uninstall = unsafe { *dll.get(b"DiUninstallDevice")? };
    let set =
        DeviceSet(unsafe { SetupDiGetClassDevsW(None, PCWSTR::null(), None, DIGCF_ALLCLASSES) }?);
    // Reference fallback for systems exposing a full INF path instead of DriverInfPath.
    let fallback = DEVPROPKEY {
        fmtid: GUID::from_u128(0x83da632697a640889453a1923f573b29),
        pid: 7,
    };
    let mut index = 0;
    loop {
        let mut data = SP_DEVINFO_DATA {
            cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32,
            ..Default::default()
        };
        if let Err(e) = unsafe { SetupDiEnumDeviceInfo(set.0, index, &mut data) } {
            if e.code() == ERROR_NO_MORE_ITEMS.to_hresult() {
                break;
            }
            return Err(e.into());
        }
        index += 1;
        let primary = property(set.0, &data, &DEVPKEY_Device_DriverInfPath);
        let matched = match primary {
            Ok(Some(name)) => name.eq_ignore_ascii_case(published_name),
            _ => property(set.0, &data, &fallback)?
                .is_some_and(|path| path.eq_ignore_ascii_case(&published_path.to_string_lossy())),
        };
        if matched
            && unsafe { uninstall(HWND::default(), set.0, &data, 0, std::ptr::null_mut()) } == 0
        {
            return Err(windows::core::Error::from_thread().into());
        }
    }
    Ok(())
}
