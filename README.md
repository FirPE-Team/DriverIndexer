# DriverIndexer

[简体中文](README.zh.md) [English](README.md)

## Introduction

`DriverIndexer` is an efficient and intelligent command-line tool for driver management and installation. It focuses on solving the problems of slow installation speed and resource waste associated with traditional driver packages.

- ⚡ **On-demand Extraction** - Only extracts drivers required by current devices, significantly reducing installation time.
- 🎯 **Smart Matching** - Automatically identifies hardware IDs and precisely matches the best drivers.
- 📦 **Multi-format Support** - Supports various driver package formats including driver directories and 7z compressed packages.
- ⚙️ **High Performance** - Utilizes multi-threading and smart indexing technology to improve installation and management speed.
- 🔄 **Offline Support** - Supports importing, installing, and managing drivers for offline Windows systems without relying on DISM environment.
- 🖥️ **Command-line Interface** - Supports silent installation and automated deployment, facilitating integration into maintenance scripts.
- 🛠️ **Driver Management** - Provides functions for importing, exporting, and deleting system drivers.
- 🔧 **Driver Packaging** - Can merge drivers with the program into a single self-extracting executable file (EXE).

### What is the value of `DriverIndexer`?

Traditional driver package installation methods require extracting the entire driver package (which may contain several GB of data) before calling `Dpinst` or similar tools for installation. This not only consumes a lot of time but also wastes disk space.

`DriverIndexer` establishes index files to enable on-demand extraction and automatic installation based on device requirements, greatly improving installation efficiency.

### What is an index file?

Index files are the core of `DriverIndexer`. Since hardware ID lists are stored within INF files, we first need to create a correspondence between `hardware ID lists` and `driver file paths within the driver package`. This relationship is the index (in JSON format).

Through indexing, the program can quickly determine the drivers required by devices, thereby achieving precise on-demand extraction and installation.

## Software Architecture

Written in `Rust` language, it calls Windows API to obtain hardware information and install device drivers.

### Driver Matching Rules

> Three matches (to prevent some drivers from failing to install)

1. Match current system architecture
2. Match current operating system version
3. Match hardware IDs of current devices
4. Match compatible IDs of current devices

### Driver Sorting Rules

1. Signature status (Microsoft signature > Other signatures > Unsigned)
2. Match score (strongest first)
3. Driver date (newest first)
4. Driver version (newest first)

## Usage Instructions

This program is a command-line program, so you need to run it with parameters after the program name. If you double-click the program directly, it will "flash and exit". You can run it through terminals such as `cmd`, `PowerShell`, etc.  
Note: Please run the terminal with **administrator privileges**.

### Create Driver Index File

Index files are usually created when using a driver package for the first time. If the driver package content changes later, you need to rebuild the index.

`DriverIndexer.exe index <driver package/directory path> <index file save path>`

- Options

    - `--password <password>`: Specify driver package password

- Examples
    - `DriverIndexer.exe index D:\netcard D:\index.json`
    - `DriverIndexer.exe index D:\netcard.7z D:\index.json`

### Install Drivers

Use index files or directly specify driver package paths for installation.

`DriverIndexer.exe install <driver package/directory path> [index file path] [options]`

- Driver path formats: compressed packages (limited to formats supported by 7zip), directory formats.
- Supports wildcards (`*`, `?`) for matching multiple driver packages.
- Temporary indexes will be automatically created when not using indexes

- Options

    - `--password <password>`: Specify driver package password for extracting the driver package.
    - `--class <driver class>`: Specify driver class, only install drivers of the specified class.
    - `--missing-only`: Only install drivers for devices without drivers installed, by default install drivers for all matching devices.
    - `--extract-path <extraction directory>`: Only extract drivers, do not install drivers. Default extraction to temporary directory.

- Examples
    - `DriverIndexer.exe install D:\netcard`
    - `DriverIndexer.exe install D:\netcard.7z`
    - `DriverIndexer.exe install D:\netcard\*.7z`
    - `DriverIndexer.exe install D:\netcard.7z netcard.json`
    - `DriverIndexer.exe install D:\netcard\*.7z D:\netcard\*.json`

### Install Offline System Drivers

Install drivers from the offline system driver library, if no system drive is specified, search all drives for system drives.

`DriverIndexer.exe install-offline [system drive path]`

- Options
    - `--missing-only`: Only install drivers for devices without drivers installed, by default install drivers for all matching devices.
    - `--class <driver class>`: Only install drivers of the specified class.

### View Driver Index Information

View index subcommand, used to view information in driver index files.

`DriverIndexer.exe info <index file path>`

- Examples
    - `DriverIndexer.exe info D:\netcard.json`

### List Drivers

List drivers in the driver store of the current system or offline system.

`DriverIndexer.exe list <system drive path>`

- Options
    - `--class <driver class>`: Specify driver class, only display drivers of the specified class.
    - `--provider <driver provider>`: Specify driver provider, only display drivers of the specified provider.

### Import Drivers

Import drivers into the system driver store.

`DriverIndexer.exe import <system drive path> <driver path>`

- Options
    - `--password <password>`: Specify driver package password for extracting the driver package.
    - `--match-device`: Match current system devices, by default match all devices.

### Export Drivers

Export specific drivers from the system driver store.

`DriverIndexer.exe export <system drive path> <export directory>`

- Options
    - `--inf <driver name>`: Specify driver name, only export the specified driver.
    - `--class <driver class>`: Specify driver class, only export drivers of the specified class.
    - `--provider <driver provider>`: Specify driver provider, only export drivers of the specified provider.

### Remove Drivers

Remove drivers from the system driver store.

`DriverIndexer.exe remove <system drive path>`

- Options
    - `--inf <driver name>`: Specify driver name, only remove the specified driver.
    - `--class <driver class>`: Specify driver class, only remove drivers of the specified class.
    - `--provider <driver provider>`: Specify driver provider, only remove drivers of the specified provider.
    - `--all`: Remove all drivers.

### Create Self-extracting Driver Package

Merge `DriverIndexer` with the driver package to generate a single EXE file. This EXE will run automatically and extract and install built-in drivers on demand.

> Friendly reminder: The driver package cannot be set with a password, otherwise it will cause driver installation failure.

`DriverIndexer.exe pack <driver package/directory path> <output EXE path>`

- Examples
    - `DriverIndexer.exe pack D:\netcard D:\netcard.exe`
    - `DriverIndexer.exe pack D:\netcard.7z D:\netcard.exe`

### Organize Driver Files

Classify and rename INF files in a directory according to rules such as manufacturer and class.

`DriverIndexer.exe organize <driver path> <export directory>`

- Examples
    - `DriverIndexer.exe organize D:\netcard D:\netcard-organized`

### Global Options

`DriverIndexer.exe [global options] command parameters`

| Parameter       | Short Parameter | Description                                               | Default Value |
|-----------------|-----------------|----------------------------------------------------------|---------------|
| `--debug`       | None            | Debug mode, output debug information to console          | None          |
| `--language`    | None            | Set program language (`En`, `zh-cn`, `zh-tw`, `ja-jp`, `ko-kr`) | Auto-detect   |
| `--log<log file path>` | None            | Enable logging. Print all running information to the specified file for troubleshooting. | None          |

## Driver Class Reference

The following are common driver class names that can be used for the `--class` parameter to specify driver classes.

> Note:
>
> - Driver class names are case-insensitive, such as `Display` and `display` have the same effect.
> - Driver class names can be defined by driver manufacturers, so there are no restrictions on driver class names. Please ensure the class names are correct.

| Class Name   | Description              |
|--------------|-------------------------|
| Display      | Display adapters         |
| Net          | Network adapters         |
| Media        | Sound, video and game controllers |
| System       | System devices           |
| HID          | Human Interface Devices  |
| USB          | USB controllers          |
| Bluetooth    | Bluetooth devices        |
| Printer      | Printers                 |
| Imaging      | Imaging devices          |
| SCSIAdapter  | SCSI and RAID controllers |
| DiskDrive    | Disk drives              |
| Computer     | Computer                 |
| Processor    | Processor                |
| Monitor      | Monitor                  |
| Keyboard     | Keyboard                 |
| Pointer      | Mouse and other pointing devices |
| Modem        | Modem                    |
| Media        | Multimedia devices       |
| System       | System devices           |

## Driver Package Download Websites

> We more advocate downloading and collecting driver packages yourself. If needed, you can also extract driver packages from various driver software (generally such driver packages are not copyrighted)

The following are recommended driver package download websites (all free, unencrypted)

- [SamDrivers](https://driveroff.net)
- [DriverPack](https://drp.su/en/foradmin?_blank)
- [3DP](https://www.3dpchip.com/3dpchip/3dp/net_down.php?_blank)
- [DriverOff](https://driveroff.net/category/dp?_blank)
- [BatPEDriver](http://forum.ru-board.com/topic.cgi?forum=62&topic=24098&start=71&limit=1&m=1#1?_blank)

## Open Source License

`DriverIndexer` is open source under the GPL V3.0 license, please try to comply with the open source agreement.

## Acknowledgments

- Hydrogen
- Lightning
- Skyfree
- 红毛樱木
- 小鸭子
- 毛利
- 优捷易

## Contributing

1. Fork this repository
2. Create a new Feat_xxx branch
3. Submit your code
4. Create a new Pull Request