use std::sync::Mutex;

use crate::abi::generated_types::{SDL_AudioFormat, SDL_AudioStream};
use crate::audio::{convert::convert_audio_buffer, frame_size, is_supported_format, silence_value};

struct StreamBuffers {
    pending_input: Vec<u8>,
    output: Vec<u8>,
}

struct AudioStreamImpl {
    src_format: SDL_AudioFormat,
    src_channels: u8,
    src_rate: i32,
    dst_format: SDL_AudioFormat,
    dst_channels: u8,
    dst_rate: i32,
    buffers: Mutex<StreamBuffers>,
}

unsafe fn stream_from_ptr<'a>(stream: *mut SDL_AudioStream) -> Option<&'a AudioStreamImpl> {
    (!stream.is_null()).then(|| &*(stream as *mut AudioStreamImpl))
}

fn flush_locked(stream: &AudioStreamImpl, buffers: &mut StreamBuffers) -> Result<(), &'static str> {
    let source_frame =
        frame_size(stream.src_format, stream.src_channels).ok_or("Unsupported audio format")?;
    if source_frame == 0 {
        return Ok(());
    }
    if !buffers.pending_input.is_empty() && buffers.pending_input.len() % source_frame != 0 {
        let remainder = source_frame - (buffers.pending_input.len() % source_frame);
        buffers
            .pending_input
            .extend(std::iter::repeat(silence_value(stream.src_format)).take(remainder));
    }

    if buffers.pending_input.is_empty() {
        return Ok(());
    }

    let converted = convert_audio_buffer(
        &buffers.pending_input,
        stream.src_format,
        stream.src_channels,
        stream.src_rate,
        stream.dst_format,
        stream.dst_channels,
        stream.dst_rate,
    )?;
    buffers.output.extend_from_slice(&converted);
    buffers.pending_input.clear();
    Ok(())
}

#[no_mangle]
pub unsafe extern "C" fn SDL_NewAudioStream(
    src_format: SDL_AudioFormat,
    src_channels: u8,
    src_rate: i32,
    dst_format: SDL_AudioFormat,
    dst_channels: u8,
    dst_rate: i32,
) -> *mut SDL_AudioStream {
    if src_channels == 0 || dst_channels == 0 || src_rate <= 0 || dst_rate <= 0 {
        let _ = crate::core::error::set_error_message("Invalid audio stream specification");
        return std::ptr::null_mut();
    }
    if !is_supported_format(src_format) || !is_supported_format(dst_format) {
        let _ = crate::core::error::set_error_message("Unsupported audio stream format");
        return std::ptr::null_mut();
    }

    let stream = Box::new(AudioStreamImpl {
        src_format,
        src_channels,
        src_rate,
        dst_format,
        dst_channels,
        dst_rate,
        buffers: Mutex::new(StreamBuffers {
            pending_input: Vec::new(),
            output: Vec::new(),
        }),
    });
    Box::into_raw(stream) as *mut SDL_AudioStream
}

#[no_mangle]
pub unsafe extern "C" fn SDL_AudioStreamPut(
    stream: *mut SDL_AudioStream,
    buf: *const u8,
    len: libc::c_int,
) -> libc::c_int {
    let Some(stream) = stream_from_ptr(stream) else {
        return crate::core::error::invalid_param_error("stream");
    };
    if len < 0 {
        return crate::core::error::set_error_message("Audio stream length is invalid");
    }
    if len > 0 && buf.is_null() {
        return crate::core::error::invalid_param_error("buf");
    }

    let source_frame = match frame_size(stream.src_format, stream.src_channels) {
        Some(value) => value,
        None => return crate::core::error::set_error_message("Unsupported audio stream format"),
    };
    let mut buffers = match stream.buffers.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };

    if len > 0 {
        let input = std::slice::from_raw_parts(buf, len as usize);
        buffers.pending_input.extend_from_slice(input);
    }

    let aligned_len = buffers.pending_input.len() - (buffers.pending_input.len() % source_frame);
    if aligned_len > 0 {
        let converted = match convert_audio_buffer(
            &buffers.pending_input[..aligned_len],
            stream.src_format,
            stream.src_channels,
            stream.src_rate,
            stream.dst_format,
            stream.dst_channels,
            stream.dst_rate,
        ) {
            Ok(bytes) => bytes,
            Err(message) => return crate::core::error::set_error_message(message),
        };
        buffers.output.extend_from_slice(&converted);
        buffers.pending_input.drain(..aligned_len);
    }

    0
}

#[no_mangle]
pub unsafe extern "C" fn SDL_AudioStreamGet(
    stream: *mut SDL_AudioStream,
    buf: *mut u8,
    len: libc::c_int,
) -> libc::c_int {
    let Some(stream) = stream_from_ptr(stream) else {
        return crate::core::error::invalid_param_error("stream");
    };
    if len < 0 {
        return crate::core::error::set_error_message("Audio stream length is invalid");
    }
    if len > 0 && buf.is_null() {
        return crate::core::error::invalid_param_error("buf");
    }

    let mut buffers = match stream.buffers.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    let amount = (len as usize).min(buffers.output.len());
    if amount > 0 {
        std::slice::from_raw_parts_mut(buf, amount).copy_from_slice(&buffers.output[..amount]);
        buffers.output.drain(..amount);
    }
    amount as libc::c_int
}

#[no_mangle]
pub unsafe extern "C" fn SDL_AudioStreamAvailable(stream: *mut SDL_AudioStream) -> libc::c_int {
    let Some(stream) = stream_from_ptr(stream) else {
        return crate::core::error::invalid_param_error("stream");
    };
    let buffers = match stream.buffers.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    buffers.output.len().min(i32::MAX as usize) as libc::c_int
}

#[no_mangle]
pub unsafe extern "C" fn SDL_AudioStreamFlush(stream: *mut SDL_AudioStream) -> libc::c_int {
    let Some(stream) = stream_from_ptr(stream) else {
        return crate::core::error::invalid_param_error("stream");
    };
    let mut buffers = match stream.buffers.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    match flush_locked(stream, &mut buffers) {
        Ok(()) => 0,
        Err(message) => crate::core::error::set_error_message(message),
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_AudioStreamClear(stream: *mut SDL_AudioStream) {
    if let Some(stream) = stream_from_ptr(stream) {
        let mut buffers = match stream.buffers.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        buffers.pending_input.clear();
        buffers.output.clear();
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_FreeAudioStream(stream: *mut SDL_AudioStream) {
    if !stream.is_null() {
        drop(Box::from_raw(stream as *mut AudioStreamImpl));
    }
}
