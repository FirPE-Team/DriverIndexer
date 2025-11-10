#[cfg(test)]
mod Tests {
    use crate::driver_index::{DriverArch, DriverIndex, InfInfo};
    use crate::utils::utils::compare_version;
    use std::cmp::Ordering;
    use std::env::temp_dir;
    use std::path::Path;

    #[test]
    // 版本号对比测试
    fn version_compare_test() {
        assert!(matches!(
            compare_version("1.0.0", "2.0.0"),
            Ok(Ordering::Less)
        ));

        assert!(matches!(
            compare_version("1.0.5", "1.0.50"),
            Ok(Ordering::Less)
        ));

        assert!(matches!(
            compare_version("1.1.0", "1.1.5"),
            Ok(Ordering::Less)
        ));
    }

    #[test]
    // DriverIndex 结构体测试
    fn driver_index_test() {
        // 创建测试用的InfInfo
        let inf_info1 = InfInfo {
            path: String::from(r"driver1\net.inf"),
            class: String::from("Net"),
            arch: vec![DriverArch::NTamd64, DriverArch::NTx86],
            date: String::from("2023-01-01"),
            version: String::from("1.0.0.0"),
            hwid: vec![String::from(r"PCI\VEN_8086&DEV_1234")],
            cid: vec![String::from(r"PCI\VEN_8086&DEV_1234&SUBSYS_12345678")],
        };

        let inf_info2 = InfInfo {
            path: String::from(r"driver2\audio.inf"),
            class: String::from("MEDIA"),
            arch: vec![DriverArch::NTamd64],
            date: String::from("2023-02-01"),
            version: String::from("2.0.0.0"),
            hwid: vec![String::from(r"PCI\VEN_1234&DEV_5678")],
            cid: vec![String::from(r"PCI\VEN_1234&DEV_5678&SUBSYS_87654321")],
        };

        // 创建DriverIndex
        let driver_index = DriverIndex::new(1024, vec![inf_info1.clone(), inf_info2.clone()]);

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

    #[test]
    // InfInfo 测试 - 基本属性测试
    fn inf_info_basic_test() {
        let inf_info = InfInfo {
            path: String::from(r"test\driver.inf"),
            class: String::from("DISPLAY"),
            arch: vec![DriverArch::NTamd64],
            date: String::from("2023-03-01"),
            version: String::from("3.0.0.0"),
            hwid: vec![String::from(r"PCI\VEN_8086&DEV_0042")],
            cid: vec![String::from(r"PCI\VEN_8086&DEV_0042&SUBSYS_12345678")],
        };

        assert_eq!(inf_info.path, r"test\driver.inf");
        assert_eq!(inf_info.class, "DISPLAY");
        assert_eq!(inf_info.arch.len(), 1);
        assert_eq!(inf_info.arch[0], DriverArch::NTamd64);
        assert_eq!(inf_info.date, "2023-03-01");
        assert_eq!(inf_info.version, "3.0.0.0");
        assert_eq!(inf_info.hwid.len(), 1);
        assert_eq!(inf_info.hwid[0], r"PCI\VEN_8086&DEV_0042");
    }

    // 测试代码：将全部硬件ID信息输出位csv格式
    #[test]
    fn output_csv() {
        let index_path = Path::new(r"D:\Project\FirPE\EFI\boot\drivers\Network.index");
        let out_path = index_path.parent().unwrap().join(format!(
            "{}.csv",
            index_path.file_stem().unwrap().to_string_lossy()
        ));
        let index = DriverIndex::from_json(index_path).unwrap();

        let mut result = String::new();

        // 添加CSV表头
        result.push_str("HardwareID,Path,Class,Arch,Date,Version\n");

        for driver in &index.drivers {
            // 如果没有硬件ID，则跳过
            if driver.hwid.is_empty() {
                continue;
            }

            let mut all_hwid = driver.hwid.clone();
            all_hwid.append(&mut driver.cid.clone());

            // 为每个硬件ID创建一行
            for hwid in &all_hwid {
                // 将架构列表转换为字符串，用分号分隔
                let arch_str = driver
                    .arch
                    .iter()
                    .map(|a| a.clone().display().to_string())
                    .collect::<Vec<_>>()
                    .join(";");

                // 添加一行CSV数据
                result.push_str(&format!(
                    "{},{},{},{},{},{}\n",
                    hwid,           // 硬件ID
                    driver.path,    // 驱动路径
                    driver.class,   // 驱动类别
                    arch_str,       // 驱动架构
                    driver.date,    // 驱动日期
                    driver.version  // 驱动版本
                ));
            }
        }

        // 将结果写入文件
        std::fs::write(&out_path, result).expect("Unable to write file");
        println!("CSV文件已生成: {}", out_path.display());
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
