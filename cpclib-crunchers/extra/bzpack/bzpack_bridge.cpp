// Copyright (c) 2024, bzpack bridge for cpclib-crunchers
// BSD 2-Clause License.

#include "bzpack_bridge.h"
#include "Compression.h"
#include "Formats.h"
#include <algorithm>
#include <cstdint>
#include <vector>

rust::Vec<uint8_t> bzpack_compress(
    rust::Slice<const uint8_t> data,
    uint8_t format_id,
    bool reverse,
    bool end_marker,
    bool extend_offset,
    bool extend_length,
    bool natural_stream)
{
    FormatOptions options = {};
    options.id = format_id;
    options.reverse = reverse ? 1 : 0;
    options.endMarker = end_marker ? 1 : 0;
    options.extendOffset = extend_offset ? 1 : 0;
    options.extendLength = extend_length ? 1 : 0;
    options.naturalStream = natural_stream ? 1 : 0;

    std::unique_ptr<Format> spFormat = Format::Create(options);
    if (!spFormat) {
        return {};
    }

    std::vector<uint8_t> inputData(data.data(), data.data() + data.size());

    if (reverse) {
        std::reverse(inputData.begin(), inputData.end());
    }

    BitStream packedStream = Compress(inputData.data(), static_cast<uint32_t>(inputData.size()), *spFormat);
    if (packedStream.Size() == 0) {
        return {};
    }

    if (reverse) {
        packedStream.Reverse();
    }

    rust::Vec<uint8_t> result;
    result.reserve(packedStream.Size());
    const uint8_t* pData = packedStream.Data();
    for (size_t i = 0; i < packedStream.Size(); i++) {
        result.push_back(pData[i]);
    }
    return result;
}
