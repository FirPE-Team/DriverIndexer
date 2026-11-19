# Dism++ driver management reference

## Evidence and boundaries

Reference: the user-supplied `refs/Dism++10.1.1002_反向还原`.
SHA-256 identities:

* `Dism++x64.exe`: `16bbdb339173d25b4332b377da96e80809aabfe6739cf35d5e70484f08cfdc42`
* `Config/amd64/CBSHost.dll`: `6a1075f4fab4acd2722e8946cba7b23508d120367c84aae3e83284124227013a`

The supplied Ghidra C files describe static decompilation, not original source or
executed behavior. Additional CBSHost observations below come from PE import,
exception-table and Capstone instruction inspection. They are not REA Evidence
records. Runtime validation is recorded separately in `driver-management-validation.md`.
No reference executable, DLL, full pseudocode corpus or third-party driver is
included in the product or committed as part of this work.

## Recovered behavior

| Behavior | Static evidence (preferred image base) |
| --- | --- |
| Public add/remove/list marshal to a helper, opcodes 12/13/14 | main `0x14001f620`, `0x14001f694`, `0x14001f708`, `0x14002a064` |
| Default third-party enumeration uses flag 2; include inbox uses 0 | main `0x1400b6014`; CBSHost `0x180009188` |
| Enumerate only files in Windows/INF, resolve repository INF, skip/log unresolved entries | CBSHost `0x180009188`, `0x180007c14` |
| UI groups class description, then provider; leaf is INF plus four-part version | main `0x1400a5304` |
| Critical is membership in 17 class GUIDs, not the version-info flags | CBSHost `0x1800096f8`, table `0x180020320` |
| Size is a directory traversal, not the API structure's size field | main `0x1400a5600`, `0x140076c10` |
| Recursive case-insensitive INF discovery | main `0x1400a63bc` |
| Already in store succeeds; open package, enumerate a driver entry; none -> 0x8007000a; import; optionally reflect critical | CBSHost `0x180008a34`, callback `0x180008c60` |
| Add processes every INF and separates success / not applicable / failure | main `0x1400a65b8` |
| Export whole repository package directory under sanitized localized class description | main `0x1400a5aa0`, copier `0x1400774e8` / `0x140077180` |
| Remove uses published name and requires an OEM prefix; online path removes matching devices, then unreflects critical and deletes | main `0x1400a6024`; CBSHost `0x180008c6c` |
| Online DriverStoreOpenW uses null root and null registry namespace | CBSHost `0x180007a24` |
| Offline second argument is a registry namespace, not an image drive letter | CBSHost `0x180007a24`, namespace derives from SYSTEM hive path |
| Version info has separate pre-Win8 and sized layouts | CBSHost `0x18000961c` and `0x18000d874` |

The 17 critical class GUIDs are encoded literally in the implementation and
checked in tests. Missing auxiliary reflect/unreflect entrypoints are tolerated
by the reference; failures of these auxiliary calls do not undo a successful
main operation. Required import/delete/enumeration failures remain errors.

## Intentional CLI adaptations

The CLI retains list/import/export/remove and its existing filters. It exposes
published and original names, supports either with `--inf`, returns nonzero on
partial failures, and reports per-item errors without dropping subsequent work.
Inbox packages may be listed/exported but cannot bypass OEM removal validation.
Hardware matching remains a separate optional extension against current devices.

Target code preserves legacy DLL selection and the x86/x64/ARM64 architecture
mapping. Automated tests use fake backends; destructive integration is explicitly
opted into and restricted to the user-designated disposable offline image.
