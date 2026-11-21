// SPDX-License-Identifier: GPL-3.0-only
#include <windows.h>
#include <bit7z/bit7zlibrary.hpp>
#include <bit7z/bitarchivereader.hpp>
#include <bit7z/bitfilecompressor.hpp>
#include <algorithm>
#include <cstdint>
#include <cstring>
#include <limits>
#include <memory>
#include <stdexcept>
#include <streambuf>

struct BridgeError { std::int32_t code; char message[2048]; };
struct BridgeEntry {
    const wchar_t* path;
    std::size_t path_length;
    std::uint64_t size;
    std::uint32_t is_directory;
    std::uint32_t is_symlink;
};

// std::system_error uses the Windows ANSI code page for localized messages.
// Convert without allocating while handling an exception (including bad_alloc).
static void error_message(BridgeError* error, const char* message) noexcept {
    wchar_t wide[2048];
    const int length = MultiByteToWideChar(CP_ACP, 0, message, -1, wide, 2048);
    if (!length || !WideCharToMultiByte(CP_UTF8, 0, wide, length, error->message,
                                      sizeof(error->message), nullptr, nullptr)) {
        std::strncpy(error->message, message, sizeof(error->message) - 1);
    }
    error->message[sizeof(error->message) - 1] = '\0';
}

// All exported functions catch exceptions, including allocation failures. The error
// buffer belongs to Rust; no C++ allocation or exception crosses the ABI.
template<class F> int protect(BridgeError* error, F&& function) noexcept {
    try { function(); return 0; }
    catch (const bit7z::BitException& ex) {
        error->code = ex.code().value();
        error_message(error, ex.what());
    } catch (const std::exception& ex) {
        error->code = -1;
        error_message(error, ex.what());
    } catch (...) {
        error->code = -1;
        error_message(error, "Unknown native exception");
    }
    error->message[sizeof(error->message) - 1] = '\0';
    return -1;
}

// A seekable view of a file region. Positions are relative to the region, and
// neither reads nor seeks can reach the executable prefix or driver footer.
class RangeBuffer final : public std::streambuf {
    HANDLE file_ = INVALID_HANDLE_VALUE;
    std::uint64_t offset_, length_, position_ = 0;
public:
    RangeBuffer(const wchar_t* path, std::uint64_t offset, std::uint64_t length)
        : offset_(offset), length_(length) {
        file_ = CreateFileW(path, GENERIC_READ, FILE_SHARE_READ, nullptr, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, nullptr);
        if (file_ == INVALID_HANDLE_VALUE) throw std::runtime_error("Cannot open archive region");
        LARGE_INTEGER size{};
        if (!GetFileSizeEx(file_, &size) || size.QuadPart < 0 || offset > static_cast<std::uint64_t>(size.QuadPart)
            || length > static_cast<std::uint64_t>(size.QuadPart) - offset || length == 0) {
            CloseHandle(file_); file_ = INVALID_HANDLE_VALUE;
            throw std::runtime_error("Archive region is outside the source file");
        }
    }
    ~RangeBuffer() override { if (file_ != INVALID_HANDLE_VALUE) CloseHandle(file_); }
protected:
    std::streamsize xsgetn(char* destination, std::streamsize requested) override {
        if (requested <= 0) return 0;
        auto remaining = (std::min)(static_cast<std::uint64_t>(requested), length_ - position_);
        std::streamsize total = 0;
        while (remaining) {
            LARGE_INTEGER where{}; where.QuadPart = static_cast<LONGLONG>(offset_ + position_);
            if (!SetFilePointerEx(file_, where, nullptr, FILE_BEGIN)) throw std::runtime_error("Archive seek failed");
            DWORD count = 0;
            const DWORD amount = static_cast<DWORD>((std::min)(remaining, std::uint64_t{1} << 30));
            if (!ReadFile(file_, destination + total, amount, &count, nullptr)) throw std::runtime_error("Archive read failed");
            position_ += count; total += count; remaining -= count;
            if (!count) break;
        }
        return total;
    }
    int_type underflow() override {
        char byte;
        const auto old = position_;
        if (xsgetn(&byte, 1) != 1) return traits_type::eof();
        position_ = old;
        return traits_type::to_int_type(byte);
    }
    int_type uflow() override {
        char byte;
        return xsgetn(&byte, 1) == 1 ? traits_type::to_int_type(byte) : traits_type::eof();
    }
    pos_type seekoff(off_type delta, std::ios_base::seekdir direction, std::ios_base::openmode mode) override {
        if (!(mode & std::ios_base::in)) return pos_type(off_type(-1));
        const auto base = direction == std::ios_base::beg ? 0 : direction == std::ios_base::end ? length_ : position_;
        if (delta < 0) {
            const auto distance = static_cast<std::uint64_t>(-(delta + 1)) + 1;
            if (distance > base) return pos_type(off_type(-1));
            position_ = base - distance;
        } else {
            if (static_cast<std::uint64_t>(delta) > length_ - base) return pos_type(off_type(-1));
            position_ = base + static_cast<std::uint64_t>(delta);
        }
        return pos_type(static_cast<off_type>(position_));
    }
    pos_type seekpos(pos_type position, std::ios_base::openmode mode) override {
        return seekoff(static_cast<off_type>(position), std::ios_base::beg, mode);
    }
};

struct ArchiveContext {
    // Destruction order: reader, stream, buffer, then DLL.
    bit7z::Bit7zLibrary library;
    std::unique_ptr<RangeBuffer> buffer;
    std::unique_ptr<std::istream> stream;
    std::unique_ptr<bit7z::BitArchiveReader> reader;
    std::wstring entry_path;
    explicit ArchiveContext(const wchar_t* dll) : library(dll) {}
};

extern "C" {
int b7_library_check(const wchar_t* dll, BridgeError* error) noexcept {
    return protect(error, [&] { bit7z::Bit7zLibrary library(dll); });
}
int b7_open(const wchar_t* dll, const wchar_t* path, const wchar_t* password,
            std::uint32_t region, std::uint64_t offset, std::uint64_t length,
            ArchiveContext** output, BridgeError* error) noexcept {
    return protect(error, [&] {
        auto context = std::make_unique<ArchiveContext>(dll);
        if (region) {
            context->buffer = std::make_unique<RangeBuffer>(path, offset, length);
            context->stream = std::make_unique<std::istream>(context->buffer.get());
            context->reader = std::make_unique<bit7z::BitArchiveReader>(context->library, *context->stream,
                bit7z::ArchiveStartOffset::FileStart, bit7z::BitFormat::Auto, password);
        } else {
            context->reader = std::make_unique<bit7z::BitArchiveReader>(context->library, path, bit7z::BitFormat::Auto, password);
        }
        *output = context.release();
    });
}
void b7_close(ArchiveContext* context) noexcept { delete context; }
int b7_count(ArchiveContext* context, std::uint32_t* count, BridgeError* error) noexcept {
    return protect(error, [&] { *count = context->reader->itemsCount(); });
}
int b7_entry(ArchiveContext* context, std::uint32_t index, BridgeEntry* output, BridgeError* error) noexcept {
    return protect(error, [&] {
        const auto item = context->reader->itemAt(index);
        context->entry_path = item.path();
        *output = {context->entry_path.c_str(), context->entry_path.size(), item.size(), item.isDir() ? 1u : 0u, item.isSymLink() ? 1u : 0u};
    });
}
int b7_extract(ArchiveContext* context, const std::uint32_t* indices, std::uint32_t count,
               const wchar_t* destination, std::uint32_t retain, std::uint32_t skip, BridgeError* error) noexcept {
    return protect(error, [&] {
        // bit7z interprets empty indices as ALL; our public API means NONE.
        if (!count) return;
        context->reader->setRetainDirectories(retain != 0);
        context->reader->setOverwriteMode(skip ? bit7z::OverwriteMode::Skip : bit7z::OverwriteMode::None);
        const bit7z::IndicesVector selected(indices, indices + count);
        context->reader->extractTo(destination, selected);
    });
}
int b7_compress(const wchar_t* dll, const wchar_t* input, const wchar_t* output,
                std::uint32_t level, std::uint64_t solid_size, std::uint32_t threads,
                BridgeError* error) noexcept {
    return protect(error, [&] {
        bit7z::Bit7zLibrary library(dll);
        bit7z::BitFileCompressor compressor(library, bit7z::BitFormat::SevenZip);
        compressor.setCompressionLevel(static_cast<bit7z::BitCompressionLevel>(level));
        compressor.setSolidMode(solid_size != 0);
        if (solid_size) compressor.setFormatProperty(L"s", std::to_wstring(solid_size) + L"b");
        compressor.setThreadsCount(threads);
        compressor.compressDirectoryContents(input, output);
    });
}
}
