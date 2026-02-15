# DriverIndexer

[简体中文](README.zh.md) | English

## Introduction

`DriverIndexer` is an efficient and intelligent command-line tool for driver management and installation. It focuses on
solving the problems of slow installation speed and resource waste associated with traditional driver packages.

- ⚡ **On-demand Extraction** - Only extracts drivers required by current devices, significantly reducing installation
  time.
- 🎯 **Smart Matching** - Automatically identifies hardware IDs and precisely matches the best drivers.
- 📦 **Multi-format Support** - Supports various driver package formats including driver directories and 7z compressed
  packages.
- ⚙️ **High Performance** - Utilizes multi-threading and smart indexing technology to improve installation and
  management speed.
- 🔄 **Offline Support** - Supports importing, installing, and managing drivers for offline Windows systems without
  relying on DISM environment.
- 🖥️ **Command-line Interface** - Supports silent installation and automated deployment, facilitating integration into
  maintenance scripts.
- 🛠️ **Driver Management** - Provides functions for importing, exporting, and deleting system drivers.
- 🔧 **Driver Packaging** - Can merge drivers with the program into a single self-extracting executable file (EXE).

### What is the value of `DriverIndexer`?

Traditional driver package installation methods require extracting the entire driver package (which may contain several
GB of data) before calling `Dpinst` or similar tools for installation. This not only consumes a lot of time but also
wastes disk space.

`DriverIndexer` establishes index files to enable on-demand extraction and automatic installation based on device
requirements, greatly improving installation efficiency.

### What is an index file?

Index files are the core of `DriverIndexer`. Since hardware ID lists are stored within INF files, we first need to
create a correspondence between `hardware ID lists` and `driver file paths within the driver package`. This relationship
is the index (in JSON format).

Through indexing, the program can quickly determine the drivers required by devices, thereby achieving precise on-demand
extraction and installation.

### Driver Package Download Websites

> We more advocate downloading and collecting driver packages yourself. If needed, you can also extract driver packages
> from various driver software (generally such driver packages are not copyrighted)

The following are recommended driver package download websites (all free, unencrypted):

- [SamDrivers](https://driveroff.net)
- [DriverPack](https://drp.su/en/foradmin?_blank)
- [3DP](https://www.3dpchip.com/3dpchip/3dp/net_down.php?_blank)
- [DriverOff](https://driveroff.net/category/dp?_blank)
- [BatPEDriver](http://forum.ru-board.com/topic.cgi?forum=62&topic=24098&start=71&limit=1&m=1#1?_blank)

## Software Architecture

Written in `Rust` language, it calls Windows API to obtain hardware information and install device drivers.

### Driver Matching Rules

> Three matches (to prevent some drivers from failing to install)

1. Match current system architecture
2. Match current operating system version
3. Match hardware information

    - Device hardware ID vs driver file hardware ID
    - Device hardware ID vs driver file compatible ID
    - Device compatible ID vs driver file hardware ID
    - Device compatible ID vs driver file compatible ID

### Driver Sorting Rules

1. Signature status (Microsoft signature > Other signatures > Unsigned)
2. Match score (strongest first)
3. Driver date (newest first)
4. Driver version (newest first)

## Usage Instructions

This program is a command-line program, so you need to run it with parameters after the program name. If you
double-click the program directly, it will "flash and exit". You can run it through terminals such as `cmd`,
`PowerShell`, etc.  
Note: Please run the terminal with **administrator privileges**.

### Create Driver Index File

Index files are usually created when using a driver package for the first time. If the driver package content changes
later, you need to rebuild the index.

`DriverIndexer.exe index <driver package/directory path> <index file save path>`

- Options

| **Parameter**           | **Short Parameter** | **Description**                                                    |
|-------------------------|---------------------|--------------------------------------------------------------------|
| `--password <password>` | `-p`                | Specify driver package password for extracting the driver package. |
| `--compress`            | `-c`                | Compress the index file using zstd algorithm.                      |

- Examples
    - `DriverIndexer.exe index D:\netcard D:\index.json`
    - `DriverIndexer.exe index D:\netcard.7z D:\index.json`

### Install Drivers

Use index files or directly specify driver package paths for installation.

`DriverIndexer.exe install <driver package/directory path> [options]`

- Driver path formats: compressed packages (limited to formats supported by 7zip), directory formats.
- Supports wildcards (`*`, `?`) for matching multiple driver packages.
- Temporary indexes will be automatically created when not using indexes

- Options

  | **Parameter**                | **Short Parameter** | **Description**                                                                                                                      |
    |------------------------------|---------------------|--------------------------------------------------------------------------------------------------------------------------------------|
  | `--index-path <path>`        | `-i`                | Specify index file path for faster installation. If not specified, a temporary index will be automatically created.                  |
  | `--password <password>`      | `-p`                | Specify driver package password for extracting the driver package.                                                                   |
  | `--class <class>`            | `-c`                | **Include** the specified driver class, only install drivers matching the class. Multiple classes can be specified repeatedly.       |
  | `--exclude-class <class>`    | `-e`                | **Exclude** the specified driver class, do not install drivers of the specified class. Multiple classes can be specified repeatedly. |
  | `--missing-only`             | `-m`                | Only install drivers for devices without drivers installed (i.e., devices with missing drivers).                                     |
  | `--extract-path <directory>` | `-x`                | Only extract drivers to the specified directory, do not perform installation operations. Default extraction to temporary directory.  |
  | `--skip-verify`              | `-s`                | Skip driver index file verification.                                                                                                 |
  | `--force`                    | `-f`                | Force installation, overwrite existing drivers.                                                                                      |

- Examples
    - `DriverIndexer.exe install D:\netcard`
    - `DriverIndexer.exe install D:\netcard.7z`
    - `DriverIndexer.exe install D:\netcard\*.7z`
    - `DriverIndexer.exe install D:\netcard.7z --index-path D:\netcard.json`
    - `DriverIndexer.exe install D:\netcard\*.7z --index-path D:\netcard\*.json`

### Install Offline System Drivers

Install drivers from the offline system driver library, if no system drive is specified, search all drives for system
drives.

`DriverIndexer.exe install-offline [system drive path]`

- Options

| **Parameter**             | **Short Parameter** | **Description**                                                                                                                      |
|---------------------------|---------------------|--------------------------------------------------------------------------------------------------------------------------------------|
| `--missing-only`          | `-m`                | Only install drivers for devices without drivers installed (i.e., devices with missing drivers).                                     |
| `--class <class>`         | `-c`                | **Include** the specified driver class, only install drivers matching the class. Multiple classes can be specified repeatedly.       |
| `--exclude-class <class>` | `-e`                | **Exclude** the specified driver class, do not install drivers of the specified class. Multiple classes can be specified repeatedly. |

### View Driver Index Information

View index subcommand, used to view information in driver index files.

`DriverIndexer.exe info <index file path>`

- Examples
    - `DriverIndexer.exe info D:\netcard.json`

### List Drivers

List drivers in the driver store of the current system or offline system.

`DriverIndexer.exe list <system drive path>`

- Options

| **Parameter**                    | **Short Parameter** | **Description**                                                                                                                            |
|----------------------------------|---------------------|--------------------------------------------------------------------------------------------------------------------------------------------|
| `--class <driver class>`         | `-c`                | **Include** the specified driver class, only display drivers of the specified class. Multiple classes can be specified repeatedly.         |
| `--exclude-class <driver class>` | `-e`                | **Exclude** the specified driver class, do not display drivers of the specified class. Multiple classes can be specified repeatedly.       |
| `--provider <driver provider>`   | `-p`                | **Include** the specified driver provider, only display drivers of the specified provider. Multiple providers can be specified repeatedly. |

- Examples
    - `DriverIndexer.exe list C:\`
    - `DriverIndexer.exe list C:\ --class net`
    - `DriverIndexer.exe list C:\ --exclude-class net`
    - `DriverIndexer.exe list C:\ --provider "Qualcomm, Inc."`

### Import Drivers

Import drivers into the system driver store.

`DriverIndexer.exe import <system drive path> <driver path>`

- Driver path formats: compressed packages (limited to formats supported by 7zip), directory formats.
- Supports wildcards (`*`, `?`) for matching multiple driver packages.

- Options

| **Parameter**           | **Short Parameter** | **Description**                                                    |
|-------------------------|---------------------|--------------------------------------------------------------------|
| `--password <password>` | `-p`                | Specify driver package password for extracting the driver package. |
| `--match-device`        | `-m`                | Match current system devices, default match all devices.           |

- Examples
    - `DriverIndexer.exe import C:\ D:\netcard.7z`
    - `DriverIndexer.exe import C:\ D:\netcard\*.7z`
    - `DriverIndexer.exe import C:\ D:\netcard.7z --password 123456`

### Export Drivers

Export specific drivers from the system driver store.

`DriverIndexer.exe export <system drive path> <export directory>`

- Options

| **Parameter**                    | **Short Parameter** | **Description**                                                                                                                           |
|----------------------------------|---------------------|-------------------------------------------------------------------------------------------------------------------------------------------|
| `--inf <inf file name>`          | `-i`                | Specify driver inf file name, only export the specified driver.                                                                           |
| `--class <driver class>`         | `-c`                | **Include** the specified driver class, only export drivers of the specified class. Multiple classes can be specified repeatedly.         |
| `--exclude-class <driver class>` | `-e`                | **Exclude** the specified driver class, do not export drivers of the specified class. Multiple classes can be specified repeatedly.       |
| `--provider <driver provider>`   | `-p`                | **Include** the specified driver provider, only export drivers of the specified provider. Multiple providers can be specified repeatedly. |

- Examples
    - `DriverIndexer.exe export C:\ D:\drivers`
    - `DriverIndexer.exe export C:\ D:\drivers --inf netcard.inf`
    - `DriverIndexer.exe export C:\ D:\drivers --class net`
    - `DriverIndexer.exe export C:\ D:\drivers --exclude-class net`
    - `DriverIndexer.exe export C:\ D:\drivers --provider "Qualcomm, Inc."`

### Remove Drivers

Remove drivers from the system driver store.

`DriverIndexer.exe remove <system drive path>`

- Options

| **Parameter**                  | **Short Parameter** | **Description**                                                                                                                           |
|--------------------------------|---------------------|-------------------------------------------------------------------------------------------------------------------------------------------|
| `--inf <inf file name>`        | `-i`                | Specify driver inf file name, only remove the specified driver.                                                                           |
| `--class <driver class>`       | `-c`                | **Include** the specified driver class, only remove drivers of the specified class. Multiple classes can be specified repeatedly.         |
| `--provider <driver provider>` | `-p`                | **Include** the specified driver provider, only remove drivers of the specified provider. Multiple providers can be specified repeatedly. |
| `--all`                        | `-a`                | Remove all drivers.                                                                                                                       |

- Examples
    - `DriverIndexer.exe remove C:\ --inf netcard.inf`
    - `DriverIndexer.exe remove C:\ --class net`
    - `DriverIndexer.exe remove C:\ --provider "Qualcomm, Inc."`
    - `DriverIndexer.exe remove C:\ --all`

### Create Self-extracting Driver Package

Merge `DriverIndexer` with the driver package to generate a single EXE file. This EXE will run automatically and extract
and install built-in drivers on demand.

`DriverIndexer.exe pack <driver package/directory path> <output EXE path>`

- Options

| **Parameter**           | **Short Parameter** | **Description**                                                      |
|-------------------------|---------------------|----------------------------------------------------------------------|
| `--password <password>` | `-p`                | Specify driver package password, used to encrypt the driver package. |

- Examples
    - `DriverIndexer.exe pack D:\netcard D:\netcard.exe`
    - `DriverIndexer.exe pack D:\netcard.7z D:\netcard.exe`

### Organize Driver Files

Classify drivers in a folder according to rules such as driver class, manufacturer, etc., and rename them according to
information in INF files.

`DriverIndexer.exe organize <driver path> <export directory>`

- Examples
    - `DriverIndexer.exe organize D:\netcard D:\netcard-organized`

### Password Encryption

To protect driver package passwords, provides password encryption functionality. The encrypted password can be used when
installing driver packages by specifying the `--password <encrypted password>` parameter. Password encryption uses
`AES-128` algorithm to ensure password security.

`DriverIndexer.exe encrypt <password>`

- Examples
    - `DriverIndexer.exe encrypt 12345678`
    - `DriverIndexer.exe install D:\netcard.7z --password enc:XXXXX`

### Global Options

`DriverIndexer.exe [global options] command [parameters] [options]`

| Parameter              | Short Parameter | Description                                                                              | Default Value |
|------------------------|-----------------|------------------------------------------------------------------------------------------|---------------|
| `--debug`              | None            | Debug mode, output debug information to console                                          | None          |
| `--language`           | None            | Set program language (`En`, `zh-cn`, `zh-tw`, `ja-jp`, `ko-kr`)                          | Auto-detect   |
| `--log<log file path>` | None            | Enable logging. Print all running information to the specified file for troubleshooting. | None          |

### Driver Class Reference

The following are common driver class names that can be used for the `--class` parameter, `--exclude-class` parameter to
specify driver classes.

> Note:
>
> - Driver class names are case-insensitive, such as `Display` and `display` have the same effect.
> - Driver class names can be defined by driver manufacturers, so there are no restrictions on driver class names.

    Please ensure the class names are correct.

| Class Name  | Description                       |
|-------------|-----------------------------------|
| Display     | Display adapters                  |
| Net         | Network adapters                  |
| Media       | Sound, video and game controllers |
| System      | System devices                    |
| HID         | Human Interface Devices           |
| USB         | USB controllers                   |
| Bluetooth   | Bluetooth devices                 |
| Printer     | Printers                          |
| Imaging     | Imaging devices                   |
| SCSIAdapter | SCSI and RAID controllers         |
| DiskDrive   | Disk drives                       |
| Computer    | Computer                          |
| Processor   | Processor                         |
| Monitor     | Monitor                           |
| Keyboard    | Keyboard                          |
| Pointer     | Mouse and other pointing devices  |
| Modem       | Modem                             |
| Media       | Multimedia devices                |
| System      | System devices                    |

## Build

### Environment Requirements

- Rust 1.65 or higher
- VC-LTL
- YY-Thunks

### Configuration Project

Create a new `.env` file and configure the following content:

```env
# Secret key, used for password encryption
# This key is an example, it is recommended to customize the key
SECRET_KEY = 0123456789ABCDEF0123456789ABCDEF
```

### Build Project

Use the `cargo build --release` command to compile the project. After compilation, the `DriverIndexer.exe` file can be
found in the `target/release` directory.

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
