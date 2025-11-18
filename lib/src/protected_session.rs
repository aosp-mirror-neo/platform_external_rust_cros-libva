// Copyright 2025 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

//! Wrappers around `ProtectedSession` types.

use std::num::NonZero;
use std::ptr;
use std::rc::Rc;
use std::slice;

use log::error;

use crate::bindings;
use crate::buffer::Buffer;
use crate::display::Display;
use crate::va_check;
use crate::Config;
use crate::Context;
use crate::VaError;

/// Structure for tracking VA-API Protected Session.
pub struct ProtectedSession {
    /// The display from which the protected session was created.
    display: Rc<Display>,
    /// The ID of the protected session.
    id: bindings::VAProtectedSessionID,
    /// The ID of the context this protected session is attached to.
    context_id: Option<bindings::VAContextID>,
}

impl ProtectedSession {
    /// Creates a `ProtectedSession` by wrapping a `vaCreateProtectedSession` call. This is just a
    /// helper for [`Display::create_protected_session`].
    pub(crate) fn new(display: Rc<Display>, config: &Config) -> Result<Self, VaError> {
        let mut session_id = 0;
        // Safe because `display` is a valid display handle and `config` is a valid VAConfig.
        unsafe {
            va_check!(bindings::vaCreateProtectedSession(
                display.handle(),
                config.id(),
                &mut session_id
            ))
        }?;

        Ok(Self {
            display,
            id: session_id,
            context_id: None,
        })
    }

    /// Attach the protected session to a `context`.
    pub fn attach(&mut self, context: &Context) -> Result<(), VaError> {
        if self.context_id.is_some() {
            log::error!(
                "Please detach the existing attached context before attaching a new context"
            );
            return Err(VaError::non_va_func(
                NonZero::new(bindings::VA_STATUS_ERROR_OPERATION_FAILED as i32).unwrap(),
            ));
        }

        // Safe because `display` is a valid display handle and `context` is a valid VAContext.
        unsafe {
            va_check!(bindings::vaAttachProtectedSession(
                self.display.handle(),
                context.id(),
                self.id
            ))
        }?;

        self.context_id = Some(context.id());
        Ok(())
    }

    /// Detach the protected session from the context it is attached to.
    pub fn detach(&mut self) -> Result<(), VaError> {
        if self.context_id.is_none() {
            return Ok(());
        }
        // Safe because `display` is a valid display handle and we checked that `context_id` is not `None`.
        unsafe {
            va_check!(bindings::vaDetachProtectedSession(
                self.display.handle(),
                self.context_id.unwrap(),
            ))
        }?;

        self.context_id = None;
        Ok(())
    }

    /// Execute a function on the protected session.
    pub fn execute(&self, buffer: &ProtectedSessionExecuteBuffer) -> Result<(), VaError> {
        // Safe because `self` represents a valid `VAProtectedSessionID`. `buffer` is also a valid
        // pointer to a `VAProtectedSessionExecuteBuffer` struct.
        unsafe {
            va_check!(bindings::vaProtectedSessionExecute(
                self.display.handle(),
                self.id,
                buffer.id()
            ))
        }
    }

    /// Create a `ProtectedSessionExecuteBuffer` to be used with `execute`.
    /// ProtectedSessionBuffer has a pointer to data for input and output for the execution.
    /// The data must outlive the created ProtectedSessionExecuteBuffer.
    pub fn create_execute_buffer<'a>(
        &self,
        function_id: u32,
        input: ProtectedSessionBuffer<'a>,
        output: ProtectedSessionBuffer<'a>,
        vtag: u64,
    ) -> Result<ProtectedSessionExecuteBuffer<'a>, VaError> {
        let mut execute_buffer_struct_in = bindings::VAProtectedSessionExecuteBuffer {
            function_id,
            input: input.c_params,
            output: output.c_params,
            status: 0,
            vtag,
            va_reserved: Default::default(),
        };
        let struct_in_ptr = &mut execute_buffer_struct_in as *mut _ as *mut std::ffi::c_void;
        let struct_in_size = std::mem::size_of_val(&execute_buffer_struct_in);

        let mut buffer_id = 0;
        // When creating a buffer for VAProtectedSessionExecuteBuffer. The protected session id is
        // used instead of a context id.
        // Safe because `self` represents a valid `VAProtectedSessionID`. `struct_in_ptr` is also a
        // valid pointer to a `VAProtectedSessionExecuteBuffer` struct.
        unsafe {
            va_check!(bindings::vaCreateBuffer(
                self.display.handle(),
                self.id,
                bindings::VABufferType::VAProtectedSessionExecuteBufferType,
                struct_in_size as u32,
                1u32,
                struct_in_ptr,
                &mut buffer_id,
            ))
        }?;

        Ok(ProtectedSessionExecuteBuffer::new(
            self.display.clone(),
            execute_buffer_struct_in,
            buffer_id,
            input,
            output,
        ))
    }
}

impl Drop for ProtectedSession {
    fn drop(&mut self) {
        self.detach()
            .unwrap_or_else(|e| error!("vaDetachProtectedSession failed: {}", e));
        // Safe because `self` represents a valid `VAProtectedSessionID`.
        unsafe {
            va_check!(bindings::vaDestroyProtectedSession(
                self.display.handle(),
                self.id
            ))
        }
        .unwrap_or_else(|e| error!("vaDestroyProtectedSession failed: {}", e));
    }
}

/// Wrapper over the `VAProtectedSessionBuffer` ffi type.
pub struct ProtectedSessionBuffer<'a> {
    c_params: bindings::VAProtectedSessionBuffer,
    data: &'a [u8],
}

impl<'a> ProtectedSessionBuffer<'a> {
    pub fn new(max_data_size: u32, data_size: u32, data: &'a [u8]) -> Self {
        Self {
            c_params: bindings::VAProtectedSessionBuffer {
                max_data_size,
                data_size,
                data: if data.is_empty() {
                    ptr::null_mut()
                } else {
                    data.as_ptr() as *mut _
                },
                va_reserved: Default::default(),
            },
            data,
        }
    }
}

/// A buffer to be passed to `ProtectedSession::execute`.
pub struct ProtectedSessionExecuteBuffer<'a> {
    display: Rc<Display>,
    c_params: bindings::VAProtectedSessionExecuteBuffer,
    buffer_id: bindings::VABufferID,
    input: ProtectedSessionBuffer<'a>,
    output: ProtectedSessionBuffer<'a>,
}

impl<'a> ProtectedSessionExecuteBuffer<'a> {
    /// Creates a new `ProtectedSessionExecuteBuffer`.
    fn new(
        display: Rc<Display>,
        c_params: bindings::VAProtectedSessionExecuteBuffer,
        buffer_id: bindings::VABufferID,
        input: ProtectedSessionBuffer<'a>,
        output: ProtectedSessionBuffer<'a>,
    ) -> Self {
        Self {
            display,
            c_params,
            buffer_id,
            input,
            output,
        }
    }

    /// Returns the ID of this buffer.
    fn id(&self) -> bindings::VABufferID {
        self.buffer_id
    }

    /// Returns MappedProtectedSessionExecuteBuffer so a caller can access the
    /// output of ProtectedSessionExecuteBuffer.
    pub fn map(&'a self) -> Result<MappedProtectedSessionExecuteBuffer<'a>, VaError> {
        let mut addr = std::ptr::null_mut();

        // Safe because `self` holds a valid buffer ID and we check the return value. `addr`
        // is also a valid pointer.
        unsafe {
            va_check!(bindings::vaMapBuffer(
                self.display.handle(),
                self.id(),
                &mut addr
            ))
        }?;

        // Safe because the trusted VA-API video driver returns a valid pointer to a
        // `VAProtectedSessionExecuteBuffer` struct when `vaMapBuffer()` is successful.
        let buffer = unsafe { *(addr as *const bindings::VAProtectedSessionExecuteBuffer) };
        Ok(MappedProtectedSessionExecuteBuffer {
            buffer,
            source: self,
        })
    }
}

impl Drop for ProtectedSessionExecuteBuffer<'_> {
    fn drop(&mut self) {
        // Safe because `display` is a valid display handle and `buffer_id` is a valid buffer ID.
        let status = unsafe {
            va_check!(bindings::vaDestroyBuffer(
                self.display.handle(),
                self.buffer_id,
            ))
        };

        if status.is_err() {
            error!("vaDestroyBuffer failed: {}", status.unwrap_err());
        }
    }
}

/// A mapped `ProtectedSessionExecuteBuffer`, allowing access to its content.
pub struct MappedProtectedSessionExecuteBuffer<'a> {
    buffer: bindings::VAProtectedSessionExecuteBuffer,
    source: &'a ProtectedSessionExecuteBuffer<'a>,
}

impl<'a> MappedProtectedSessionExecuteBuffer<'a> {
    pub fn get_output(&'a self) -> &'a [u8] {
        // SAFETY: The pointer and data size are provided by a trusted VA-API video driver and are
        // expected to be valid. The resulting slice doesn't outlive `source` thanks to the
        // lifetime annotation.
        unsafe {
            slice::from_raw_parts(
                self.buffer.output.data as *const _,
                self.buffer.output.data_size as usize,
            )
        }
    }
}

impl<'a> Drop for MappedProtectedSessionExecuteBuffer<'a> {
    fn drop(&mut self) {
        // Safe because `display` is a valid display handle and `source` holds a valid buffer ID.
        let status = unsafe {
            va_check!(bindings::vaUnmapBuffer(
                self.source.display.handle(),
                self.source.id()
            ))
        };
        if status.is_err() {
            error!("vaUnmapBuffer failed: {}", status.unwrap_err());
        }
    }
}
