# DriverIndexer

[简体中文](README.zh.md) [English](README.md)

## Introduction

`DriverIndexer` is a tool for creating, reading and installing driver package indexes.

- ⚡ **On-demand Decompression** - Only decompresses drivers that match current devices, significantly improving
  installation speed
- 🎯 **Smart Matching** - Automatically identifies hardware and matches appropriate drivers
- 📦 **Multi-format Support** - Supports various driver package formats including directories and 7z archives
- 🔄 **Offline Support** - Supports offline system driver management without relying on DISM environment
- 🖥️ **Command Line Interface** - Can be installed silently for automated deployment
- 🛠️ **Driver Management** - Import, export, and remove system drivers
- 🔧 **Driver Packaging** - Merge drivers with the program into a single executable file

### What is the use of `DriverIndexer`?

Many people pack multiple drivers into one driver package for convenience in installation. Generally, installing a
driver package requires decompressing all files and then calling tools like `Dpinst` to install drivers, which is very
time-consuming and resource-intensive. The function of `DriverIndexer` is to decompress only the currently matched
drivers on demand and install them automatically.

### What is an index file?

Since hardware IDs are stored in INF files, on-demand decompression requires establishing a correspondence between the
`list of hardware IDs in INF files` and the `paths of drivers within the driver package`. This correspondence is called
an `index`. Based on the index, the required drivers for a device can be determined, enabling on-demand decompression
and installation.

### Why does the index file use the `JSON` format?

Under normal circumstances, an index in a driver package will not exceed 10MB, and this amount of data is sufficiently
handled by the common `JSON` format.

### Why can I install drivers without specifying an index file?

When no index file is specified, `DriverIndexer` decompresses all INF files in the driver package, creates an index
instantly, and finally matches drivers based on the index information.

### What is the difference with EasyDrv/DrvCeo?

`DriverIndexer` is a command-line program, which means drivers can be installed silently without interface interaction,
providing an experience similar to built-in drivers.

### Where can I get driver packages?

> We recommend downloading and collecting driver packages yourself. If needed, you can also extract driver packages from
> various driver software (generally, such driver packages are copyright-free).

The following are recommended websites for downloading driver packages (all free and unencrypted):

- [SamDrivers](https://driveroff.net)
- [DriverPack](https://drp.su/en/foradmin?_blank)
- [3DP](https://www.3dpchip.com/3dpchip/3dp/net_down.php?_blank)
- [DriverOff](https://driveroff.net/category/dp?_blank)
- [BatPEDriver](http://forum.ru-board.com/topic.cgi?forum=62&topic=24098&start=71&limit=1&m=1#1?_blank)

## Software Architecture

Written in `Rust`, it calls `Devcon.exe` to obtain hardware information and uses Windows API to install device drivers.

### Driver Matching Rules

1. By default, only matches devices without drivers installed
2. Dedicated drivers have higher priority than generic ones
3. Higher versions have higher priority than lower versions
4. Three matching attempts (to prevent some drivers from failing to install)

## Usage Instructions

This program is a command-line program, so it needs to be run with parameters after it. Double-clicking the program
directly will cause a "flash back" phenomenon. You can run it through terminals such as `cmd` and `PowerShell`.  
Note: Please run the terminal as an **administrator**.

### Global Options

`DriverIndexer.exe [Global Options] Command Parameters`

- `--language <Language>`: Specify program language. Options are `en` (English), `zh-CN` (Simplified Chinese), `zh-TW` (
  Traditional Chinese). Default is system language.
- `--debug`: Enable debug mode. Print more debug information for troubleshooting.
- `--log <LogFilePath>`: Enable log mode. Print all running information to the specified file for troubleshooting.
- `--help`: View help information.

### Create Index

Create index subcommand, used to create driver package indexes.

`DriverIndexer.exe create-index <DriverPath> <IndexSavePath>`
`DriverIndexer.exe index <DriverPath> <IndexSavePath>`

- Options
    - `--password <DecompressionPassword>`: Specify driver package password

- Examples
    - `DriverIndexer.exe create-index D:\netcard.7z D:\index.json`
    - `DriverIndexer.exe create-index D:\netcard D:\index.json`

### View Index Information

View index subcommand, used to view information in the index file.

`DriverIndexer.exe index-info <IndexPath>`
`DriverIndexer.exe info <IndexPath>`

### Install Driver

Install driver subcommand, used to install drivers from driver packages. Supports compressed packages (limited to
formats supported by 7zip) and directory formats.

`DriverIndexer.exe install-driver <DriverPath> [-p DecompressionPassword] [--AllDevice] [--ExtractDriver] [--class DriverClass]`
`DriverIndexer.exe install <DriverPath> [-p DecompressionPassword] [--AllDevice] [--ExtractDriver] [--class DriverClass]`

- Options
    - `--password <DecompressionPassword>`: Specify driver package password
    - `--class <DriverClass>`: Specify driver class
    - `--match_device`: Match current system devices
    - `--AllDevice`: Match all devices, default is to install only devices without drivers
    - `--ExtractDriver <ExtractionDirectory>`: Only extract drivers, do not install

- Examples
    - No driver index: `DriverIndexer.exe install-driver <DriverPath>`
        - `DriverIndexer.exe install-driver D:\netcard`
        - `DriverIndexer.exe install-driver D:\netcard.7z`
        - `DriverIndexer.exe install-driver D:\netcard\*.7z`
    - With driver index: `DriverIndexer.exe install-driver <DriverPath> <IndexPath>`
        - `DriverIndexer.exe install-driver D:\netcard.7z netcard.json`
        - `DriverIndexer.exe install-driver D:\netcard\*.7z D:\netcard\*.json`

### Install Offline System Drivers

Install drivers from offline system driver library. If no system drive is specified, it will search the entire disk for
system drives. Default is to install only devices without drivers.

`DriverIndexer.exe install-offline-driver [SystemDrivePath]`

- Options
    - `--all-Device`: Match all devices
    - `--class <DriverClass>`: Install only specified class drivers

### List Drivers

List all drivers in the system, supports both online and offline systems.

`DriverIndexer.exe list-driver <SystemDrivePath>`
`DriverIndexer.exe list <SystemDrivePath>`

- Options
    - `--class <DriverClass>`: Specify driver class
    - `--provider <DriverProvider>`: Specify driver provider

### Import Driver

Import all drivers from driver package into the system, supports both online and offline systems.

`DriverIndexer.exe import-driver <SystemDrivePath> <DriverPath>`
`DriverIndexer.exe import <SystemDrivePath> <DriverPath>`

- Options
    - `--password <DecompressionPassword>`: Specify driver package password
    - `--match-device`: Match current system devices

### Export Driver

Export all drivers from the system to a specified directory, supports both online and offline systems.

`DriverIndexer.exe export-driver <SystemDrivePath> <ExportDirectory>`
`DriverIndexer.exe export <SystemDrivePath> <ExportDirectory>`

- Options
    - `--inf <DriverName>`: Specify driver name
    - `--class <DriverClass>`: Specify driver class
    - `--provider <DriverProvider>`: Specify driver provider

### Remove Driver

Remove drivers from the system, supports both online and offline systems.

`DriverIndexer.exe remove-driver <SystemDrivePath>`

- Options
    - `--inf <DriverName>`: Specify driver name
    - `--class <DriverClass>`: Specify driver class
    - `--provider <DriverProvider>`: Specify driver provider

### Organize Driver

Organize drivers in the specified directory by driver class and provider.

`DriverIndexer.exe organize-driver DriverPath`

- `DriverIndexer.exe organize-driver D:\netcard`

### Create Driver Package Program

Merge `DriverIndexer` with the driver package to generate an exe binary executable file. The generated executable file
will automatically read its own driver package and only decompress the required drivers (avoiding secondary
decompression).

> Note: The driver package cannot be password-protected, otherwise driver installation will fail.

`DriverIndexer.exe create-driver <DriverPath> <OutputPath>`

- Examples
    - Create program driver package from file
        - `DriverIndexer.exe create-driver D:\netcard.7z D:\netcard.exe`
    - Create program driver package from directory
        - `DriverIndexer.exe create-driver D:\netcard D:\netcard.exe`

## Driver Class Reference

The following are common driver class names that can be used for the `--class` parameter:

> Note:
> - Driver class names are case-insensitive, e.g., `Display` and `display` have the same effect.
> - Driver class names can be defined by driver manufacturers, so there are no restrictions on driver class names.
    Please ensure the class name is correct.

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
| Processor   | Processors                        |
| Monitor     | Monitors                          |
| Keyboard    | Keyboards                         |
| Pointer     | Mice and other pointing devices   |
| Modem       | Modems                            |
| Media       | Multimedia devices                |
| System      | System devices                    |

## Open Source License

`DriverIndexer` is open source under the GPL V3.0 license, please try to comply with the open source agreement.

## Acknowledgments

- Hydrogen
- Lightning
- Skyfree
- Red Sakuragi
- Little Duck
- Gross Profit

## Contributing

1. Fork this repository
2. Create a new Feat_xxx branch
3. Commit your code
4. Create a Pull Request