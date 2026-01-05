// Copyright 2022 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use crate::bindings;

use std::ops::{BitAnd, BitOr, BitXor, Not};

/// Gives the driver a hint of intended usage to optimize allocation (e.g. tiling).
#[derive(Debug, Clone, Copy)]
pub struct UsageHint(u32);

impl UsageHint {
    /// Surface used by video decoder.
    pub const USAGE_HINT_DECODER: Self = Self(bindings::VA_SURFACE_ATTRIB_USAGE_HINT_DECODER);
    /// Surface used by video encoder.
    pub const USAGE_HINT_ENCODER: Self = Self(bindings::VA_SURFACE_ATTRIB_USAGE_HINT_ENCODER);
    /// Surface read by video post-processing.
    pub const USAGE_HINT_VPP_READ: Self = Self(bindings::VA_SURFACE_ATTRIB_USAGE_HINT_VPP_READ);
    /// Surface written by video post-processing.
    pub const USAGE_HINT_VPP_WRITE: Self = Self(bindings::VA_SURFACE_ATTRIB_USAGE_HINT_VPP_WRITE);
    /// Surface used for display.
    pub const USAGE_HINT_DISPLAY: Self = Self(bindings::VA_SURFACE_ATTRIB_USAGE_HINT_DISPLAY);
    /// Surface used for export to third-party APIs, e.g. via `vaExportSurfaceHandle()`.
    pub const USAGE_HINT_EXPORT: Self = Self(bindings::VA_SURFACE_ATTRIB_USAGE_HINT_EXPORT);

    pub fn bits(&self) -> u32 {
        self.0
    }
}

impl BitOr for UsageHint {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl BitAnd for UsageHint {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl Not for UsageHint {
    type Output = Self;
    fn not(self) -> Self {
        Self(!self.0)
    }
}
