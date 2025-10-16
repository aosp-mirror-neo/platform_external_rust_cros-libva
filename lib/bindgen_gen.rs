// Copyright 2022 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

/// The allow list of VA structures and enum values.
const ALLOW_LIST_TYPE: &str =
    ".*ExternalBuffers.*|.*PRIME.*|.*MPEG2.*|.*VP8.*|.*VP9.*|.*H264.*|.*HEVC.*|\
    .*JPEG.*|VACodedBufferSegment|.*AV1.*|VAEncMisc.*|VASurfaceDecodeMBErrors|\
    VADecodeErrorType|.*VAProc.*|VAEncPacked.*|VAProtected.*|VAEncryption.*";

/// The allow list of VA structures and enum values in intel protected content headers.
const INTEL_PROTECTED_CONTENT_ALLOW_LIST_TYPE: &str =
    "VACenc.*|VA_TEE_.*|VA_PROTECTED_.|OEMCrypto_.*|PAVP_.*";

// The common bindgen builder for VA-API.
pub fn vaapi_gen_builder(builder: bindgen::Builder) -> bindgen::Builder {
    builder
        .derive_default(true)
        .derive_eq(true)
        .layout_tests(false)
        .constified_enum_module("VA.*")
        .allowlist_var("VA.*")
        .allowlist_function("va.*")
        .allowlist_type(ALLOW_LIST_TYPE)
        .allowlist_type(INTEL_PROTECTED_CONTENT_ALLOW_LIST_TYPE)
}
