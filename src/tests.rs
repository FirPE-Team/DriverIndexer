#![allow(unused_imports)]
#![allow(unreachable_code)]
#![allow(unused_must_use)]

mod Tests {
    use crate::command::create_index::InfInfo;
    use crate::utils::devcon::Devcon;
    use crate::utils::drvstoreAPI::DriverStore;
    use crate::utils::setupAPI;
    use crate::utils::setupAPI::get_class_description;
    use crate::utils::sevenZIP::sevenZip;
    use crate::utils::util::{compareVersion, getDriveSpace, getFileList, isOfflineSystem};
    use encoding::Encoding;
    use std::fs::File;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};
    use std::{env, thread};

    // 文件解压测试
    #[test]
    fn unzipTest() {
        use crate::utils::sevenZIP::sevenZip;

        let basePath =
            PathBuf::from(r"C:\Users\Administrator.W10-20201229857\Desktop\USB无线网卡驱动.zip");
        let outPath = PathBuf::from(r"C:\Users\Administrator.W10-20201229857\Desktop\outPath");

        let zip = sevenZip::new().unwrap();
        println!("{:?}", zip.extractFilesFromPath(&basePath, None, "", &outPath));
    }

    // 文件遍历测试
    #[test]
    fn fileListTest() {
        use crate::utils::util::getFileList;

        // let basePath = PathBuf::from(r"C:\Users\Administrator.W10-20201229857\Desktop");
        let basePath = PathBuf::from(r"C:\Users\Administrator.W10-20201229857\Desktop\Network");

        let fileList = getFileList(&basePath, "*.inf").unwrap();
        println!("{:?}", fileList);
        println!("{:?}", fileList.len());
    }

    // 多国语言支持
    #[test]
    fn Language() {
        use windows::Win32::Globalization::GetUserDefaultUILanguage;

        unsafe {
            let langID = GetUserDefaultUILanguage();

            // 2052为简体中文
            println!("{:?}", langID);
        }
        return;

        use fluent_templates::{static_loader, Loader};
        use unic_langid::{langid, LanguageIdentifier};

        const US_ENGLISH: LanguageIdentifier = langid!("en-US");
        const ZH_CHINEXE: LanguageIdentifier = langid!("zh-CN");

        static_loader! {
            static LOCALES = {
                locales: "./src/i18n",
                fallback_language: "en-US",
            };
        }

        assert_eq!("Hello World!", LOCALES.lookup(&US_ENGLISH, "hello-world"));
        assert_eq!("你好，世界!", LOCALES.lookup(&ZH_CHINEXE, "hello-world"));

        println!("{}", LOCALES.lookup(&ZH_CHINEXE, "hello-world"));

        // println!("{}", getLocaleText("hello-world"));
        // println!("{}", getLocaleText("greeting", Some(hash_map!("name".to_string() => "Alice".into()))));
    }

    // INF解析测试
    #[test]
    fn parsingInfFileTest() {
        use crate::command::create_index::InfInfo;

        let infPath = PathBuf::from(
            r"D:\UserData\Desktop\ETDI2C.inf",
        );

        println!(
            "{:?}",
            InfInfo::parsingInfFile(&infPath.parent().unwrap(), &infPath).unwrap()
        );
    }

    // 正则表达式测试
    #[test]
    fn reTest() {
        // use regex::RegexSet;
        // use regex::RegexSetBuilder;
        //
        // let reSet = RegexSetBuilder::new(&["USB", "45646"])
        //     .case_insensitive(true)
        //     .build()
        //     .unwrap();
        // let _aaa = reSet.matches("USB SADFASDF SDAFFDAS 45646");

        // let bbb mut = aaa.into_iter();
        // println!("{:?}", bbb.next());

        // for item in aaa.into_iter() {
        // println!("{:?}", aaa.matched(item));
        // println!("{:?}", item.next());
        // }
    }

    // 驱动匹配测试
    #[test]
    fn matchDriverTest() {
        use crate::command::create_index::InfInfo;
        use crate::command::load_driver::getMatchInfo;
        use crate::utils::util::getFileList;

        Devcon::new()
            .unwrap()
            .removeDevice(r"USB\VID_0BDA&PID_B711&REV_0200")
            .unwrap();

        let basePath = PathBuf::from(
            r"C:\Users\Administrator.W10-20201229857\Desktop\Network\USB无线网卡驱动",
        );
        // let basePath = PathBuf::from(r"C:\Users\Administrator.W10-20201229857\Desktop\Network");

        let infList = getFileList(&basePath, "*.inf").unwrap();
        let infInfoList = InfInfo::parsingInfFileList(&basePath, &infList);

        let devcon = Devcon::new().unwrap();

        // 扫描以发现新的硬件
        // devcon.rescan().unwrap();
        unsafe {
            setupAPI::rescan();
        }
        // 获取真实硬件id信息
        let hwIdList = devcon.getRealIdInfo(None).unwrap();
        // 获取有问题的硬件id信息
        // let hwIdList = devcon.getProblemIdInfo(hwIdList).unwrap();
        // println!("{:#?}", hwIdList);

        // 匹配硬件id
        let time1 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let matchInfo = getMatchInfo(&hwIdList, &infInfoList, None);
        let time2 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

        println!("{:#?}", matchInfo);
        println!("匹配耗时：{:?}", time2 - time1);
    }

    // 驱动加载测试
    #[test]
    fn loadDriverTest() {
        use crate::utils::devcon::Devcon;

        Devcon::new()
            .unwrap()
            .removeDevice(r"USB\VID_0BDA&PID_B711&REV_0200")
            .unwrap();

        // let basePath = PathBuf::from(r"C:\Users\Administrator.W10-20201229857\Desktop\Network");
        let basePath = PathBuf::from(
            r"C:\Users\Administrator.W10-20201229857\Desktop\Network\USB无线网卡驱动.zip",
        );

        // let index = None;
        // let index = Some(PathBuf::from(r"C:\Users\Administrator.W10-20201229857\Desktop\Network\USB无线网卡驱动.json"));

        // loadDriver(&basePath, None,index, false, None, false);
    }

    // 驱动整理测试
    #[test]
    fn classifyDriverTest() {
        use crate::command::classify_driver::classify_driver;

        let basePath = PathBuf::from(r"D:\UserData\Desktop\万能网卡驱动-驱动精灵");
        classify_driver(&basePath);
    }

    // 版本号对比测试
    #[test]
    fn versionMatches() {
        println!("{:?}", compareVersion("1.0.0.9", "1.0.1.9"));

        // let mut info = InfInfo::parsingIndex(Path::new(
        //     r"D:\UserData\Desktop\ETDI2C.inf"
        // )).unwrap();
        // info.sort_by(|b, a| compareVersiopn(&*a.Version, &*b.Version));
    }

    // 编码测试
    #[test]
    fn encodingTest() {
        use chardet::{charset2encoding, detect};
        use encoding::label::encoding_from_whatwg_label;
        use encoding::DecoderTrap;
        use std::fs::File;
        use std::io::Read;

        let basePath = PathBuf::from(r"C:\Users\Administrator.W10-20201229857\Desktop");
        let infPath = basePath.join(r"Network\Wlan\Atheros\OEM\\Dell\athw10x.inf");
        // let infPath = basePath.join(r"Network\Lan\Realtek\Mod\20180105\netrtwlane.inf");

        // println!("{}", InfInfo::parsingInfFile(&basePath, &infPath).unwrap().driverList.len());

        // 读取inf文件（使用UFT-16）
        let mut file = File::open(&infPath).unwrap();
        let mut fileBuf: Vec<u8> = Vec::new();
        file.read_to_end(&mut fileBuf).unwrap();

        let result = detect(&fileBuf);
        let coder = encoding_from_whatwg_label(charset2encoding(&result.0)).unwrap();
        let utf8reader = coder.decode(&fileBuf, DecoderTrap::Ignore).expect("Error");
        println!("{:?}", utf8reader);

        // let infContent = UTF_16LE.decode(&*fileBuf, DecoderTrap::Strict).unwrap();
        // println!("=================");
        // println!("{:?}", infContent);
    }

    // 通配符支持
    #[test]
    fn wildcard() {
        use crate::utils::util::getFileList;

        let path = PathBuf::from(r"D:\Project\FirPE\WinPE插件");
        let fileName = path.file_name().unwrap().to_str().unwrap();
        if fileName.contains("*") || fileName.contains("?") {
            println!(
                "{}",
                getFileList(&PathBuf::from(&path.parent().unwrap()), fileName)
                    .unwrap()
                    .len()
            );
        }
        for item in getFileList(&path, "*.7z").unwrap() {
            println!("{}", item.to_str().unwrap());
        }
    }

    // 环境变量测试
    #[test]
    fn EnvTest() {
        use std::env;

        for (key, value) in env::vars() {
            println!("  {}  =>  {}", key, value);
        }
    }

    // 多线程测试
    #[test]
    fn multithreadingTest() {
        use std::thread;

        // 模拟有20个元素
        let mut list = [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
        ]
            .iter();

        let t1 = thread::spawn(move || {
            for _n in 0..2 {
                let value = list.next();
                if value.is_some() {
                    println!("{}", value.unwrap());
                }
            }
            list
        });

        t1.join().unwrap();
    }

    // 系统位宽判断测试
    #[test]
    fn getSystemBitWidthTest() {
        match std::mem::size_of::<&char>() {
            4 => println!("x32"),
            8 => println!("x64"),
            _ => {}
        }
    }

    // newdevAPI测试
    #[test]
    fn newdevAPITest() {
        use crate::utils::newdevAPI::*;

        unsafe {
            let result = updateDriverForPlugAndPlayDevices(
                &PathBuf::from(
                    r"C:\Users\Administrator.W10-20201229857\Desktop\Network\USB无线网卡驱动\netrtl8188gu.inf",
                ),
                &r"USB\VID_0BDA&PID_B711".to_string(),
            );
            println!("{:?}", result);
        }
    }

    // drvstoreAPI测试
    #[test]
    fn importDriverTest() {
        let systemDrive = Path::new(r"D:\Project\FirPE\Mount");
        let systemRoot = systemDrive.join("Windows");
        let installInfPath = Path::new(r"D:\UserData\Desktop\netrtl8188gu\netrtl8188gu.inf");

        unsafe {
            let driverStore = DriverStore::new().unwrap();
            if isOfflineSystem(Path::new(systemDrive)).unwrap() {
                println!("离线导入驱动");
                println!("{:?}", driverStore.offline_add_driver(installInfPath, &*systemRoot, systemDrive, 0, 9));
            } else {
                // 在线导入驱动
                let handle = driverStore.open_store(&*systemRoot, systemDrive).unwrap();
                println!("打开驱动库: {:?}", handle);
                println!("导入驱动: {:?}", driverStore.import_driver_to_store(handle, &installInfPath, 9, 0));
                println!("关闭驱动库: {:?}", driverStore.close_store(handle).is_ok());
            }
        }
    }

    #[test]
    fn removeDriverTest() {
        let systemDrive = Path::new(r"D:\Project\FirPE\Mount");
        let systemRoot = systemDrive.join("Windows");
        let installInfPath = Path::new(r"D:\UserData\Desktop\netrtl8188gu\netrtl8188gu.inf");

        unsafe {
            let driverStore = DriverStore::new().unwrap();
            if isOfflineSystem(Path::new(systemDrive)).unwrap() {
                println!("离线删除驱动");
                // if let Some(infPath) = findInfFullPath(systemDrive, installInfPath.file_name().unwrap().to_str().unwrap()) {
                //     println!("{:?}", driverStore.offline_delete_driver(&infPath, &systemRoot, systemDrive, 0));
                // }
            } else {
                println!("在线删除驱动");
                let handle = driverStore.open_store(&*systemRoot, systemDrive).unwrap();
                println!("打开驱动库: {:?}", handle);
                println!("删除驱动: {:?}", driverStore.delete_driver(handle, installInfPath, 0));
                println!("关闭驱动库: {:?}", driverStore.close_store(handle).is_ok());
            }
        }
    }

    // drvstoreAPI测试
    #[test]
    fn drvstoreAPITest() {
        let systemDrive = Path::new(r"C:\");
        let systemRoot = systemDrive.join("Windows");
        let installInfPath = Path::new(r"D:\UserData\Desktop\netrtl8188gu\netrtl8188gu.inf");

        unsafe {
            let driverStore = DriverStore::new().unwrap();
            let handle = driverStore.open_store(&systemRoot, systemDrive).unwrap();
            println!("打开驱动库: {:?}", handle);

            // let (path, info_opt) = driverStore.find_driver_package(handle, &systemRoot.join("inf").join("1394.inf").to_str().unwrap(), 9).unwrap();
            // println!("{:#?}", path);
            // println!("{:#?}", info_opt);

            // for item in getFileList(&*systemRoot.join("inf"), "*.inf").unwrap() {
            //     if let Some((path, info_opt)) = driverStore.find_driver_package(handle, &item, 9) {
            //         println!("{}", path);
            //         println!("{:?}", info_opt);
            //     }
            // }

            // println!("{:?}", driverStore.copy_driver(handle, Path::new(r"C:\Windows\System32\DriverStore\FileRepository\netrtl8188gu.inf_amd64_2a540f1c52f7ddb1\netrtl8188gu.inf"), 9, Path::new(r"D:\UserData\Desktop\123")));

            let handle = driverStore.open_driver(Path::new(r"C:\Windows\System32\DriverStore\FileRepository\netrtl8188gu.inf_amd64_2a540f1c52f7ddb1\netrtl8188gu.inf"), 9).unwrap();
            println!("{}", handle);
            println!("{:#?}", driverStore.get_version_info(handle).unwrap());
            driverStore.close_package(handle);

            driverStore.close_store(handle);
        }
    }

    #[test]
    fn writeRegTest() {
        // use winreg::enums::HKEY_LOCAL_MACHINE;
        // use winreg::RegKey;

        // 关闭驱动数字验证
        // HKLM\SYSTEM\Setup\SystemSetupInProgress=#1

        // 恢复驱动数字验证（默认）
        // HKLM\SYSTEM\Setup\SystemSetupInProgress=#0

        // let setup = RegKey::predef(HKEY_LOCAL_MACHINE)
        //     .open_subkey(r"SYSTEM\Setup")
        //     .unwrap();

        // setup
        //     .set_value("SystemSetupInProgress", &(1 as u32))
        //     .unwrap();

        // let value: u32 = setup.get_value("SystemSetupInProgress").unwrap();
        // println!("{:?}", value);
    }

    #[test]
    fn getOfflineArchTest() {
        println!("{:?}", crate::utils::util::getArchCode(Path::new(r"C:\")));
    }

    #[test]
    fn loadOfflineDriverTest() {
        let mut candidates = Vec::new();
        for letter in b'C'..=b'Z' {
            let drive = format!("{}:\\Windows", letter as char);
            let path = Path::new(&drive);
            if path.exists() && path.join("System32").join("ntoskrnl.exe").exists() {
                candidates.push(path.to_path_buf());
            }
        }
        println!("{:?}", candidates);
    }

    #[test]
    fn ejectDriveTest() {
        // println!("{:?}", getDriveType(Path::new("F:")));
        // println!("{:?}", getDriveBus(Path::new("G:")));
        // println!("{:?}", getDriveBus(Path::new("h:")));
        // println!("{:?}", ejectDrive(Path::new("G:\\")));
        println!("{:?}", getDriveSpace(Path::new("F:")));
    }

    #[test]
    fn guidTest() {
        unsafe { println!("{:?}", get_class_description("4D36E972-E325-11CE-BFC1-08002BE10318")); }
    }
}
