# Driver archive verification baselines

The JSON files record relative file paths, uncompressed sizes, and SHA-256 hashes
from the two user-provided archives in `testDrive`. They were generated before
replacing the backend, by fully extracting each archive using the repository's
original x64 7-Zip 26.00 CLI and hashing the resulting files.

* `VMwareVMCI.7z`: 4 files, 118,867 uncompressed bytes.
* `DisplayVmWare.7z`: 35 files, 60,644,245 uncompressed bytes.

The original driver archives and their extracted contents are not committed.
Tests compare exact path sets and hashes for full and directory extraction, and
the matching subset for recursive INF/CAT extraction. Index checks also assert
the known INF path, two hardware entries per package, and the primary VMware
PCI hardware ID. All generated files use temporary workspaces.

```powershell
cargo test --test driver_archives -- --ignored
cargo test --target i686-pc-windows-msvc --test driver_archives -- --ignored
```

Set `DRIVERINDEXER_TEST_DRIVE` to override the input directory. The tests never
execute generated bundled EXEs and never install drivers. They intentionally
fail when requested archives are absent instead of reporting a false pass.
