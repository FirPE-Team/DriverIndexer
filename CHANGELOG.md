# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.3.0] - 2026-03-16

### Added

- Add option to include system drivers
- Add option to only include system drivers

### Fixed

- Improve import driver match device option

## [2.2.1] - 2026-02-26

### Fixed

- Improve parse inf FeatureScore error

### Changed

- Upgrade 7zip to 26.00

## [2.2.0] - 2026-02-15

### Added

- Supports zstd compressed driver index file
- Support option to not verify driver index file
- Supports automatic use if the program directory contains 7z.exe
- If the timestamp in the driver package does not match the timestamp in the index file, a CRC32 checksum will be used
- Skip Index Validation Option

### Fixed

- Improve driver matching algorithm
- Improve API calls
- Improve enumeration system disk path
- Improved performance compared to previous versions

### Removed

- Remove driver index and save it within the driver package

## [2.1.0] - 2025-12-25

### Added

- Driver package password parameter encryption
- Password support for self-extracting driver packages (32 characters only)
- Install driver description info
- Install driver exclude class parameter

### Fixed

- Improve the path for recognizing colons
- Improved handling of offline system driver DLL usage
- Improved support for drive paths
- Improved 7-zip error handling
- Improved English translation format
- Improved driver decompression when the driver package has no root directory
- Improved driver matching performance
- Improved driver installation and decompression error message
- Improved index creation failure message
- Improved setupapi encapsulation
- Improve the matching function and output information
- Improved driver package cannot recognize the driver index
- Improved more error exit codes

## [2.0.0] - 2025-11-27

### Added

- Driver management, supporting online/offline import/export and viewing
- Driver index structure, driver signature and other fields
- Command and options for ejecting driverless device virtual CD-ROM drives
- Device scan command
- Command for installing offline system drivers
- Driver and index file match verification
- Added option to force driver installation
- Command line program prompt
- Multiple languages: Traditional Chinese

### Changed

- Parse INF using the setup API
- Use the setup API to obtain hardware information
- Automatically generate an index when drivers and indexes do not match
- DEBUG option function is now set to debug option; the original DEBUG output log is changed to the log option
- Update 7-Zip to 25.1.0.0

### Fixed

- Improved driver installation by specifying hardware ID parameters (originally INF hardware ID)
- Improved driving matching algorithm
- Improved duplicate matching of compatible device information
- Improved driver matching performance
- Improved index file fields
- Improved internal error handling logic (using anyhow)
- Improved localized text
- Improved driver category, version, and date case extraction

### Removed

- Devcon program

## [1.1.0] - 2024-09-01

### Added

- Create-driver command to create driver package program
- Support for driver package password

### Changed

- Updated 7-zip 24.8.0.0
- Optimized INF hardware ID acquisition logic (line-by-line parsing)
- Optimized creating index without content prompt
- Optimized support for Windows NT5 environment

### Fixed

- No output when only decompressing the driver package

## [1.0.0] - 2021-05-12

### Changed

- Optimize logical structure of the program
- Optimize CLI
- Optimize log files
- Optimize VC-LTL support

### Fixed

- Some drivers loading error
- Specified driver category is invalid

## [0.7.0] - 2021-05-12

### Added

- VC-LTL to compile

### Changed

- Use API to install driver
- Optimized programming language
- Optimized code structure

## [0.6.0] - 2021-04-29

### Added

- Multi threaded install driver

### Changed

- Update Devcon version
- Optimize program performance
- Optimized program structure
- Optimize program output

## [0.5.0] - 2021-04-14

### Added

- Multi language support

### Changed

- Install driver rule
- Optimize install driver

## [0.4.0] - 2021-04-09

### Added

- Drive date information
- Driver version information

### Changed

- Change the index file name to be optional
- Optimize the performance of parsing INF
- Optimization search for INF files
- Optimize code structure
- Optimize loading driver

### Fixed

- Uppercase suffixes cannot be recognized.

## [0.3.0] - 2025-04-07

## [0.2.0] - 2021-03-29
