# template
-opening-brace = {"{"}
-closing-brace = {"}"}

template =
 {-opening-brace}bin{-closing-brace} V{-opening-brace}version{-closing-brace}

 USAGE: {-opening-brace}usage{-closing-brace} [ARGUMENTS]

 OPTIONS:
  {-opening-brace}unified{-closing-brace}

 SUBCOMMANDS:
  {-opening-brace}subcommands{-closing-brace}

help-message = Prints help information
version-message = Prints version information

help = Prints this message or the help of the given subcommand(s)

on-debug = Turn on debug mode
opened-debug = Debug mode is open. The log is kept at { $path }

# subcommand

## create-index
create-index = Create a driver index. Index format: JSON
save-index-path = Index file save location

## load-driver
load-driver = Install the matching driver. Automatically match the driver in the compressed package, decompress and install
package-path = Compressed package path
index-path = index file path
package-password = Compressed package password
match-all-device = Match all device
driver-category = Set the install driver category
only-unzip = Only unzip the driver without installing
offline-import = Offline import driver

## import-driver
import-driver = Import drivers
match-device = Match the native device

## remove-driver
remove-driver = Remove drivers

## classify-driver
classify-driver = sort out the driver

## create-driver
create-driver = Create a driver package. Packages are packaged with programs and drivers for easy distribution
driver-package-program-path = Driver package program path

# validator
path-not-exist = The path does not exist, please make sure the entered directory exists
dir-not-exist = The directory does not exist, please make sure the entered directory exists
not-driver-category = The driver category is incorrect, please enter the correct driver category
not-system-path = The system disk is invalid, make sure that the drive letter you entered exists in the operating system
