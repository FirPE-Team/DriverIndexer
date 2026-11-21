# Native source provenance

These sources are vendored so compiling the bridge never downloads C++ dependencies.
Only source/include/license directories needed by this integration are retained.
Upstream files are unmodified.

| Directory | Upstream | Version / commit | License |
| --- | --- | --- | --- |
| bit7z | https://github.com/rikyoz/bit7z | v4.1.0 / c81c6c1cbf44e148cd4b06f4bb69d7ea1e299742 | MPL-2.0; see bit7z/LICENSE |
| 7zip | https://github.com/rikyoz/7-Zip | v26.01 / a261194961e092a3ef0449f6981dde7060cf7750 | see 7zip/DOC/License.txt, copying.txt, unRarLicense.txt |
| filesystem | https://github.com/rikyoz/filesystem | b99c2aebd5ddd6fb2f190731ba80b949fc3842b5 | MIT; see filesystem/LICENSE |

The 7-Zip sources supply interface headers only. Compression codecs remain in the
application-provided 7z.dll (currently 26.00); this crate does not compile a second
codec implementation. bit7z sources and the bridge are compiled together using
the same definitions, C++14, native Windows strings, and ghc::filesystem.
