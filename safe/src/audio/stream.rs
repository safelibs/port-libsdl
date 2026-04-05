use std::sync::Mutex;

use crate::abi::generated_types::{SDL_AudioFormat, SDL_AudioStream};
use crate::audio::{
    convert::{convert_audio_buffer_reuse, AudioConvertScratch},
    frame_size, is_supported_format, silence_value,
};

struct StreamBuffers {
    pending_input: Vec<u8>,
    pending_input_start: usize,
    output: Vec<u8>,
    output_start: usize,
}

struct AudioStreamImpl {
    src_format: SDL_AudioFormat,
    src_channels: u8,
    src_rate: i32,
    src_frame: usize,
    dst_format: SDL_AudioFormat,
    dst_channels: u8,
    dst_rate: i32,
    buffers: Mutex<StreamBuffers>,
    scratch: Mutex<AudioConvertScratch>,
}

unsafe fn stream_from_ptr<'a>(stream: *mut SDL_AudioStream) -> Option<&'a AudioStreamImpl> {
    (!stream.is_null()).then(|| &*(stream as *mut AudioStreamImpl))
}

fn active_slice(buffer: &[u8], start: usize) -> &[u8] {
    &buffer[start.min(buffer.len())..]
}

fn compact_front(buffer: &mut Vec<u8>, start: &mut usize) {
    if *start == 0 {
        return;
    }
    if *start >= buffer.len() {
        buffer.clear();
        *start = 0;
        return;
    }

    let remaining = buffer.len() - *start;
    buffer.copy_within(*start.., 0);
    buffer.truncate(remaining);
    *start = 0;
}

fn maybe_compact_front(buffer: &mut Vec<u8>, start: &mut usize) {
    if *start >= 4096 && *start * 2 >= buffer.len() {
        compact_front(buffer, start);
    }
}

impl StreamBuffers {
    fn maybe_compact_pending(&mut self) {
        maybe_compact_front(&mut self.pending_input, &mut self.pending_input_start);
    }

    fn maybe_compact_output(&mut self) {
        maybe_compact_front(&mut self.output, &mut self.output_start);
    }
}

fn flush_locked(
    stream: &AudioStreamImpl,
    buffers: &mut StreamBuffers,
    scratch: &mut AudioConvertScratch,
) -> Result<(), &'static str> {
    if stream.src_frame == 0 {
        return Ok(());
    }
    let pending = active_slice(&buffers.pending_input, buffers.pending_input_start);
    if !pending.is_empty() && pending.len() % stream.src_frame != 0 {
        let remainder = stream.src_frame - (pending.len() % stream.src_frame);
        buffers
            .pending_input
            .extend(std::iter::repeat(silence_value(stream.src_format)).take(remainder));
    }

    if active_slice(&buffers.pending_input, buffers.pending_input_start).is_empty() {
        return Ok(());
    }

    convert_audio_buffer_reuse(
        active_slice(&buffers.pending_input, buffers.pending_input_start),
        stream.src_format,
        stream.src_channels,
        stream.src_rate,
        stream.dst_format,
        stream.dst_channels,
        stream.dst_rate,
        scratch,
    )?;
    buffers.output.extend_from_slice(&scratch.encoded);
    buffers.pending_input.clear();
    buffers.pending_input_start = 0;
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

    let src_frame = match frame_size(src_format, src_channels) {
        Some(frame) => frame,
        None => {
            let _ = crate::core::error::set_error_message("Unsupported audio stream format");
            return std::ptr::null_mut();
        }
    };
    let dst_frame = match frame_size(dst_format, dst_channels) {
        Some(frame) => frame,
        None => {
            let _ = crate::core::error::set_error_message("Unsupported audio stream format");
            return std::ptr::null_mut();
        }
    };

    let stream = Box::new(AudioStreamImpl {
        src_format,
        src_channels,
        src_rate,
        src_frame,
        dst_format,
        dst_channels,
        dst_rate,
        buffers: Mutex::new(StreamBuffers {
            pending_input: Vec::with_capacity(src_frame.saturating_mul(4096)),
            pending_input_start: 0,
            output: Vec::with_capacity(dst_frame.saturating_mul(4096)),
            output_start: 0,
        }),
        scratch: Mutex::new(AudioConvertScratch::default()),
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

    let mut buffers = match stream.buffers.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    buffers.maybe_compact_pending();

    if len > 0 {
        let input = std::slice::from_raw_parts(buf, len as usize);
        buffers.pending_input.extend_from_slice(input);
    }

    let pending_len = active_slice(&buffers.pending_input, buffers.pending_input_start).len();
    let aligned_len = pending_len - (pending_len % stream.src_frame);
    if aligned_len > 0 {
        let mut scratch = match stream.scratch.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let conversion_result = {
            let start = buffers.pending_input_start;
            let end = start + aligned_len;
            convert_audio_buffer_reuse(
                &buffers.pending_input[start..end],
                stream.src_format,
                stream.src_channels,
                stream.src_rate,
                stream.dst_format,
                stream.dst_channels,
                stream.dst_rate,
                &mut scratch,
            )
        };
        if let Err(message) = conversion_result {
            return crate::core::error::set_error_message(message);
        }
        buffers.output.extend_from_slice(&scratch.encoded);
        buffers.pending_input_start += aligned_len;
        buffers.maybe_compact_pending();
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
    let start = buffers.output_start;
    let end = buffers.output.len();
    let amount = (len as usize).min(end.saturating_sub(start));
    if amount > 0 {
        std::slice::from_raw_parts_mut(buf, amount)
            .copy_from_slice(&buffers.output[start..start + amount]);
        buffers.output_start += amount;
        buffers.maybe_compact_output();
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
    active_slice(&buffers.output, buffers.output_start)
        .len()
        .min(i32::MAX as usize) as libc::c_int
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
    let mut scratch = match stream.scratch.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    match flush_locked(stream, &mut buffers, &mut scratch) {
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
        buffers.pending_input_start = 0;
        buffers.output.clear();
        buffers.output_start = 0;
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_FreeAudioStream(stream: *mut SDL_AudioStream) {
    if !stream.is_null() {
        drop(Box::from_raw(stream as *mut AudioStreamImpl));
    }
}
