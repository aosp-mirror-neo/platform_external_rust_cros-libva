// Copyright 2022 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::num::TryFromIntError;
use std::rc::Rc;

use log::error;

use crate::bindings;
use crate::display::Display;
use crate::generic_value::GenericValue;
use crate::va_check;
use crate::GenericValueError;
use crate::VaError;

/// A configuration for a given [`Display`].
pub struct Config {
    display: Rc<Display>,
    id: bindings::VAConfigID,
}

#[derive(Debug)]
pub enum QuerySurfaceAttributesError {
    VaError(VaError),
    GenericValueError(GenericValueError),
}

#[derive(Debug)]
pub enum QueryConfigAttributesError {
    VaError(VaError),
    IntConversionError(TryFromIntError),
}

impl std::fmt::Display for QuerySurfaceAttributesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::VaError(e) => write!(f, "error while calling vaQuerySurfaceAttributes: {e}"),
            Self::GenericValueError(e) => write!(f, "error while converting attribute: {e}"),
        }
    }
}

impl From<VaError> for QuerySurfaceAttributesError {
    fn from(error: VaError) -> Self {
        Self::VaError(error)
    }
}

impl From<GenericValueError> for QuerySurfaceAttributesError {
    fn from(error: GenericValueError) -> Self {
        Self::GenericValueError(error)
    }
}

impl std::error::Error for QuerySurfaceAttributesError {}

impl std::fmt::Display for QueryConfigAttributesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::VaError(e) => write!(f, "Verror while calling vaQueryConfigAttributes: {e}"),
            Self::IntConversionError(e) => write!(f, "error while converting an integer: {e}"),
        }
    }
}

impl From<VaError> for QueryConfigAttributesError {
    fn from(error: VaError) -> Self {
        Self::VaError(error)
    }
}

impl From<TryFromIntError> for QueryConfigAttributesError {
    fn from(error: TryFromIntError) -> Self {
        Self::IntConversionError(error)
    }
}

impl std::error::Error for QueryConfigAttributesError {}

impl Config {
    /// Creates a Config by wrapping around the `vaCreateConfig` call. This is just a helper for
    /// [`Display::create_config`].
    pub(crate) fn new(
        display: Rc<Display>,
        mut attrs: Vec<bindings::VAConfigAttrib>,
        profile: bindings::VAProfile::Type,
        entrypoint: bindings::VAEntrypoint::Type,
    ) -> Result<Self, VaError> {
        let mut config_id = 0u32;

        // Safe because `self` represents a valid `VADisplay`.
        //
        // The `attrs` vector is also properly initialized and its actual size is passed to
        // `vaCreateConfig`, so it is impossible to write past the end of its storage by mistake.
        unsafe {
            va_check!(bindings::vaCreateConfig(
                display.handle(),
                profile,
                entrypoint,
                attrs.as_mut_ptr(),
                attrs.len() as i32,
                &mut config_id,
            ))
        }?;

        Ok(Self {
            display,
            id: config_id,
        })
    }

    /// Returns the ID of this config.
    pub(crate) fn id(&self) -> bindings::VAConfigID {
        self.id
    }

    // Queries surface attributes for this config.
    //
    // This function queries for all supported attributes for this configuration. In particular, if
    // the underlying hardware supports the creation of VA surfaces in various formats, then this
    // function will enumerate all pixel formats that are supported.
    fn query_surface_attributes(&mut self) -> Result<Vec<bindings::VASurfaceAttrib>, VaError> {
        // Safe because `self` represents a valid VAConfig. We first query how
        // much space is needed by the C API by passing in NULL in the first
        // call to `vaQuerySurfaceAttributes`.
        let attrs_len: std::os::raw::c_uint = 0;
        unsafe {
            va_check!(bindings::vaQuerySurfaceAttributes(
                self.display.handle(),
                self.id,
                std::ptr::null_mut(),
                &attrs_len as *const _ as *mut std::os::raw::c_uint,
            ))
        }?;

        let mut attrs = Vec::with_capacity(attrs_len as usize);
        // Safe because we allocate a vector with the required capacity as
        // returned by the initial call to vaQuerySurfaceAttributes. We then
        // pass a valid pointer to it.
        unsafe {
            va_check!(bindings::vaQuerySurfaceAttributes(
                self.display.handle(),
                self.id,
                attrs.as_mut_ptr(),
                &attrs_len as *const _ as *mut std::os::raw::c_uint,
            ))
        }?;

        // Safe because vaQuerySurfaceAttributes will have written to
        // exactly attrs_len entries in the vector.
        unsafe {
            attrs.set_len(attrs_len as usize);
        }

        Ok(attrs)
    }

    /// Query the surface attributes of type `attr_type`. The attribute may or may not be defined by
    /// the driver.
    pub fn query_surface_attributes_by_type(
        &mut self,
        attr_type: bindings::VASurfaceAttribType::Type,
    ) -> Result<Vec<GenericValue>, QuerySurfaceAttributesError> {
        let surface_attributes = self.query_surface_attributes()?;

        surface_attributes
            .into_iter()
            .filter(|attr| attr.type_ == attr_type)
            .map(|attr| {
                GenericValue::try_from(attr.value)
                    .map_err(QuerySurfaceAttributesError::GenericValueError)
            })
            .collect()
    }

    /// Query the current config attributes.
    pub fn query_config_attributes(
        &self,
    ) -> Result<Vec<bindings::VAConfigAttrib>, QueryConfigAttributesError> {
        // Safe because `self` represents a valid `VADisplay`.
        let mut max_num_config_attributes: usize =
            unsafe { bindings::vaMaxNumConfigAttributes(self.display.handle()) }.try_into()?;

        let mut profile = bindings::VAProfile::VAProfileNone;
        let mut entrypoint = bindings::VAEntrypoint::VAEntrypointVLD;
        let mut attrib_list = Vec::with_capacity(max_num_config_attributes);
        attrib_list.resize(
            max_num_config_attributes,
            bindings::VAConfigAttrib::default(),
        );
        let mut num_attribs = 0;

        // Safe because `self` represents a valid `VADisplay` and `VAConfig`. Additionally, all
        // pointers passed below point to data that was just allocated above. In particular, the
        // libva documentation requires that for the attribute list, the caller passes a pointer to
        // an array that can hold at least vaMaxNumConfigAttributes() elements, and that allocation
        // happened with the Vec::with_capacity() call above.
        unsafe {
            va_check!(bindings::vaQueryConfigAttributes(
                self.display.handle(),
                self.id,
                &mut profile,
                &mut entrypoint,
                attrib_list.as_mut_ptr(),
                &mut num_attribs,
            ))
        }?;

        attrib_list.truncate(num_attribs.try_into()?);

        Ok(attrib_list)
    }
}

impl Drop for Config {
    fn drop(&mut self) {
        // Safe because `self` represents a valid Config.
        let status =
            unsafe { va_check!(bindings::vaDestroyConfig(self.display.handle(), self.id)) };

        if status.is_err() {
            error!("vaDestroyConfig failed: {}", status.unwrap_err());
        }
    }
}
