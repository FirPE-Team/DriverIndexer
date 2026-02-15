#[cfg(test)]
mod Tests {
    use crate::driver_index::{DriverArch, DriverIndex, HardwareEntry, InfInfo};
    use crate::hardware::enumerate_hardware;
    use crate::utils::utils::{check_catalog_signature, compare_version};
    use std::cmp::Ordering;
    use std::env::temp_dir;
    use std::path::Path;

    // 版本号对比测试
    #[test]
    fn version_compare_test() {
        assert!(matches!(compare_version("1.0.0", "2.0.0"), Ordering::Less));
        assert!(matches!(compare_version("1.0.5", "1.0.50"), Ordering::Less));
        assert!(matches!(compare_version("1.1.0", "1.1.5"), Ordering::Less));
    }

    // 签名测试
    #[test]
    fn check_catalog_sign_test() {
        let catalog_path = Path::new(r"C:\Windows\explorer.exe");
        assert!(check_catalog_signature(catalog_path));
    }

    #[test]
    // DriverIndex 结构体测试
    fn driver_index_test() {
        // 创建测试用的InfInfo
        let inf_info1 = InfInfo {
            path: String::from(r"driver1\net.inf"),
            class: String::from("Net"),
            date: String::from("2023-01-01"),
            version: String::from("1.0.0.0"),
            hardware: vec![HardwareEntry {
                desc: "".to_string(),
                arch: DriverArch::NTx86,
                min_os_version: String::new(),
                hardware_id: String::from(r"PCI\VEN_8086&DEV_1234"),
                compatible_ids: vec![String::from(r"PCI\VEN_8086&DEV_1234&SUBSYS_12345678")],
                feature_score: 0,
            }],
            signature: 0,
        };

        let inf_info2 = InfInfo {
            path: String::from(r"driver2\audio.inf"),
            class: String::from("MEDIA"),
            signature: 0,
            date: String::from("2023-02-01"),
            version: String::from("2.0.0.0"),
            hardware: vec![HardwareEntry {
                desc: "".to_string(),
                arch: DriverArch::NTamd64,
                min_os_version: String::new(),
                hardware_id: String::from(r"PCI\VEN_1234&DEV_5678"),
                compatible_ids: vec![String::from(r"PCI\VEN_1234&DEV_5678&SUBSYS_87654321")],
                feature_score: 0,
            }],
        };

        // 创建DriverIndex
        let driver_index = DriverIndex::new(
            1024,
            1694400000,
            None,
            vec![inf_info1.clone(), inf_info2.clone()],
        );

        // 测试get_driver_index_info方法
        let info_str = driver_index.get_driver_index_info();
        assert!(info_str.contains("Driver Index Info"));
        assert!(info_str.contains("Driver Size:"));
        assert!(info_str.contains("Driver Count:"));
        // assert!(info_str.contains("Driver Classes: [\"Net\", \"MEDIA\"]"));

        // 测试JSON序列化和反序列化
        let temp_path = temp_dir();
        let json_path = temp_path.join("test_index.json");

        // 测试JSON序列化
        assert!(driver_index.to_json().is_ok());
    }

    #[test]
    // DriverArch 枚举测试
    fn driver_arch_test() {
        assert_eq!(DriverArch::NTx86.display(), "NTx86");
        assert_eq!(DriverArch::NTamd64.display(), "NTamd64");
        assert_eq!(DriverArch::NTia64.display(), "NTia64");
        assert_eq!(DriverArch::NTarm.display(), "NTarm");
        assert_eq!(DriverArch::NTarm64.display(), "NTarm64");
        assert_eq!(DriverArch::Nt.display(), "Nt");
    }

    // #[test]
    fn parse_inf_test() {
        let inf_path = Path::new(r"D:\UserData\Desktop\test\netrtl8188gu\netrtl8188gu.inf");
        let inf_info = InfInfo::parse_inf(inf_path.parent().unwrap(), inf_path).unwrap();
        println!("{:#?}", inf_info);
    }

    #[test]
    fn get_device_info_test() {
        let hardware_list = enumerate_hardware(None, true).unwrap();
        println!("API Device Info count: {}", hardware_list.len());
        println!("{:#?}", hardware_list);

        // create_dir_all(&*TEMP_PATH);
        // let devcon = Devcon::new().unwrap();
        // let device_info = devcon.get_hardware_device_info(Some("net")).unwrap();
        // println!("Devcon Device Info count: {}", device_info.len());

        // 对比API和Devcon获取的硬件信息
        // for api_hardware in &hardware_list {
        //     let devcon_hardware = device_info
        //         .iter()
        //         .find(|dev| dev.device_instance_path == api_hardware.device_instance_path);
        //
        //     assert!(devcon_hardware.is_some());
        //     let devcon_hardware = devcon_hardware.unwrap();
        //
        //     // assert_eq!(api_hardware.name, devcon_hardware.name);
        //     assert_eq!(api_hardware.hardware_id, devcon_hardware.hardware_id);
        //     assert_eq!(api_hardware.compatible_id, devcon_hardware.compatible_id);
        // }

        // println!("{:#?}", device_info);
    }

    #[test]
    // InfInfo 测试 - 基本属性测试
    fn inf_info_basic_test() {
        // let inf_info = InfInfo {
        //     path: String::from(r"test\driver.inf"),
        //     class: String::from("DISPLAY"),
        //     arch: vec![DriverArch::NTamd64],
        //     date: String::from("2023-03-01"),
        //     version: String::from("3.0.0.0"),
        //     hwid: vec![String::from(r"PCI\VEN_8086&DEV_0042")],
        //     cid: vec![String::from(r"PCI\VEN_8086&DEV_0042&SUBSYS_12345678")],
        // };
        //
        // assert_eq!(inf_info.path, r"test\driver.inf");
        // assert_eq!(inf_info.class, "DISPLAY");
        // assert_eq!(inf_info.arch.len(), 1);
        // assert_eq!(inf_info.arch[0], DriverArch::NTamd64);
        // assert_eq!(inf_info.date, "2023-03-01");
        // assert_eq!(inf_info.version, "3.0.0.0");
        // assert_eq!(inf_info.hwid.len(), 1);
        // assert_eq!(inf_info.hwid[0], r"PCI\VEN_8086&DEV_0042");
    }

    // drvstoreAPI测试
    // #[test]
    // fn importDriverTest() {
    //     let systemDrive = Path::new(r"D:\Project\FirPE\Mount");
    //     let systemRoot = systemDrive.join("Windows");
    //     let installInfPath = Path::new(r"D:\UserData\Desktop\netrtl8188gu\netrtl8188gu.inf");
    //
    //     let driverStore = DriverStore::new(None).unwrap();
    //     if is_offline_system(Path::new(systemDrive)).unwrap() {
    //         println!("离线导入驱动");
    //         // Windows 10、Window 11
    //         // println!("{:?}", driverStore.offline_add_driver(installInfPath, &*systemRoot, systemDrive, 0, 9));
    //
    //         // Windows 7
    //         let handle = driverStore.open_store(&*systemRoot, systemDrive).unwrap();
    //         println!("打开驱动库: {:?}", handle);
    //         println!(
    //             "导入驱动: {:?}",
    //             driverStore.import_driver_to_store(handle, &installInfPath, 9, 0)
    //         );
    //         println!("关闭驱动库: {:?}", driverStore.close_store(handle).is_ok());
    //     } else {
    //         println!("在线导入驱动");
    //         let handle = driverStore.open_store(&*systemRoot, systemDrive).unwrap();
    //         println!("打开驱动库: {:?}", handle);
    //         println!(
    //             "导入驱动: {:?}",
    //             driverStore.import_driver_to_store(handle, &installInfPath, 9, 0)
    //         );
    //         println!("关闭驱动库: {:?}", driverStore.close_store(handle).is_ok());
    //     }
    // }

    // #[test]
    // fn removeDriverTest() {
    //     let systemDrive = Path::new(r"D:\Project\FirPE\Mount");
    //     let systemRoot = systemDrive.join("Windows");
    //     let installInfPath = Path::new(r"D:\UserData\Desktop\netrtl8188gu\netrtl8188gu.inf");
    //
    //     let driverStore = DriverStore::new(None).unwrap();
    //     if is_offline_system(Path::new(systemDrive)).unwrap() {
    //         println!("离线删除驱动");
    //         // if let Some(infPath) = findInfFullPath(systemDrive, installInfPath.file_name().unwrap().to_str().unwrap()) {
    //         //     println!("{:?}", driverStore.offline_delete_driver(&infPath, &systemRoot, systemDrive, 0));
    //         // }
    //     } else {
    //         println!("在线删除驱动");
    //         let handle = driverStore.open_store(&*systemRoot, systemDrive).unwrap();
    //         println!("打开驱动库: {:?}", handle);
    //         println!(
    //             "删除驱动: {:?}",
    //             driverStore.delete_driver(handle, installInfPath, 0)
    //         );
    //         println!("关闭驱动库: {:?}", driverStore.close_store(handle).is_ok());
    //     }
    // }
}
