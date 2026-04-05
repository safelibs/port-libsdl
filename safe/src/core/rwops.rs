use std::ffi::CStr;
use std::ptr;

use crate::abi::generated_types::{
    SDL_RWops, SDL_bool, Sint64, Uint16, Uint32, Uint64, Uint8, SDL_RWOPS_MEMORY,
    SDL_RWOPS_MEMORY_RO, SDL_RWOPS_STDFILE,
};
use crate::core::system::sdl_to_bool;

#[repr(C)]
struct FileBackend {
    file: *mut libc::FILE,
    autoclose: bool,
}

unsafe fn new_rwops() -> *mut SDL_RWops {
    Box::into_raw(Box::new(std::mem::zeroed::<SDL_RWops>()))
}

unsafe fn free_rwops(context: *mut SDL_RWops) {
    drop(Box::from_raw(context));
}

unsafe fn file_backend(context: *mut SDL_RWops) -> *mut FileBackend {
    (*context).hidden.unknown.data1 as *mut FileBackend
}

unsafe extern "C" fn stdio_size(context: *mut SDL_RWops) -> Sint64 {
    let backend = file_backend(context);
    let current = libc::ftell((*backend).file);
    if current < 0 {
        return -1;
    }
    if libc::fseek((*backend).file, 0, libc::SEEK_END) != 0 {
        return -1;
    }
    let size = libc::ftell((*backend).file);
    let _ = libc::fseek((*backend).file, current, libc::SEEK_SET);
    size as Sint64
}

unsafe extern "C" fn stdio_seek(
    context: *mut SDL_RWops,
    offset: Sint64,
    whence: libc::c_int,
) -> Sint64 {
    let backend = file_backend(context);
    if libc::fseek((*backend).file, offset as libc::c_long, whence) != 0 {
        return -1;
    }
    libc::ftell((*backend).file) as Sint64
}

unsafe extern "C" fn stdio_read(
    context: *mut SDL_RWops,
    ptr_: *mut libc::c_void,
    size: usize,
    maxnum: usize,
) -> usize {
    let backend = file_backend(context);
    libc::fread(ptr_, size, maxnum, (*backend).file)
}

unsafe extern "C" fn stdio_write(
    context: *mut SDL_RWops,
    ptr_: *const libc::c_void,
    size: usize,
    num: usize,
) -> usize {
    let backend = file_backend(context);
    libc::fwrite(ptr_, size, num, (*backend).file)
}

unsafe extern "C" fn stdio_close(context: *mut SDL_RWops) -> libc::c_int {
    let backend = Box::from_raw(file_backend(context));
    let result = if backend.autoclose {
        libc::fclose(backend.file)
    } else {
        0
    };
    free_rwops(context);
    if result == 0 {
        0
    } else {
        -1
    }
}

unsafe extern "C" fn mem_size(context: *mut SDL_RWops) -> Sint64 {
    ((*context).hidden.mem.stop as usize - (*context).hidden.mem.base as usize) as Sint64
}

unsafe extern "C" fn mem_seek(
    context: *mut SDL_RWops,
    offset: Sint64,
    whence: libc::c_int,
) -> Sint64 {
    let base = (*context).hidden.mem.base as isize;
    let here = (*context).hidden.mem.here as isize;
    let stop = (*context).hidden.mem.stop as isize;
    let target = match whence {
        libc::SEEK_SET => base + offset as isize,
        libc::SEEK_CUR => here + offset as isize,
        libc::SEEK_END => stop + offset as isize,
        _ => return -1,
    };
    if target < base || target > stop {
        return -1;
    }
    (*context).hidden.mem.here = target as *mut Uint8;
    (target - base) as Sint64
}

unsafe extern "C" fn mem_read(
    context: *mut SDL_RWops,
    ptr_: *mut libc::c_void,
    size: usize,
    maxnum: usize,
) -> usize {
    let Some(mut total_bytes) = size.checked_mul(maxnum) else {
        return 0;
    };
    if total_bytes == 0 {
        return 0;
    }
    let available = (*context).hidden.mem.stop as usize - (*context).hidden.mem.here as usize;
    if total_bytes > available {
        total_bytes = available;
    }
    if total_bytes > 0 {
        ptr::copy_nonoverlapping(
            (*context).hidden.mem.here.cast::<u8>(),
            ptr_.cast::<u8>(),
            total_bytes,
        );
        (*context).hidden.mem.here = (*context).hidden.mem.here.add(total_bytes);
    }
    total_bytes / size
}

unsafe extern "C" fn mem_write(
    context: *mut SDL_RWops,
    ptr_: *const libc::c_void,
    size: usize,
    num: usize,
) -> usize {
    if size == 0 || num == 0 {
        return 0;
    }
    let available = (*context).hidden.mem.stop as usize - (*context).hidden.mem.here as usize;
    let count = num.min(available / size);
    if count > 0 {
        ptr::copy_nonoverlapping(
            ptr_.cast::<u8>(),
            (*context).hidden.mem.here.cast::<u8>(),
            count * size,
        );
        (*context).hidden.mem.here = (*context).hidden.mem.here.add(count * size);
    }
    count
}

unsafe extern "C" fn mem_write_const(
    _context: *mut SDL_RWops,
    _ptr_: *const libc::c_void,
    _size: usize,
    _num: usize,
) -> usize {
    let _ = crate::core::error::SDL_Error(crate::abi::generated_types::SDL_errorcode_SDL_EFWRITE);
    0
}

unsafe extern "C" fn mem_close(context: *mut SDL_RWops) -> libc::c_int {
    free_rwops(context);
    0
}

unsafe fn configure_mem_rwops(
    mem: *mut Uint8,
    size: libc::c_int,
    writable: bool,
) -> *mut SDL_RWops {
    let rw = new_rwops();
    (*rw).size = Some(mem_size);
    (*rw).seek = Some(mem_seek);
    (*rw).read = Some(mem_read);
    (*rw).write = Some(if writable { mem_write } else { mem_write_const });
    (*rw).close = Some(mem_close);
    (*rw).type_ = if writable {
        SDL_RWOPS_MEMORY
    } else {
        SDL_RWOPS_MEMORY_RO
    };
    (*rw).hidden.mem.base = mem;
    (*rw).hidden.mem.here = mem;
    (*rw).hidden.mem.stop = mem.add(size as usize);
    rw
}

unsafe fn read_into<const N: usize>(src: *mut SDL_RWops) -> Option<[u8; N]> {
    let mut bytes = [0u8; N];
    if SDL_RWread(src, bytes.as_mut_ptr().cast(), 1, N) == N {
        Some(bytes)
    } else {
        None
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_RWFromFile(
    file: *const libc::c_char,
    mode: *const libc::c_char,
) -> *mut SDL_RWops {
    if file.is_null() {
        let _ = crate::core::error::invalid_param_error("file");
        return std::ptr::null_mut();
    }
    if mode.is_null() {
        let _ = crate::core::error::invalid_param_error("mode");
        return std::ptr::null_mut();
    }
    let fp = libc::fopen(file, mode);
    if fp.is_null() {
        let path = CStr::from_ptr(file).to_string_lossy();
        let _ = crate::core::error::set_error_message(&format!(
            "Couldn't open {}: {}",
            path,
            std::io::Error::last_os_error()
        ));
        return std::ptr::null_mut();
    }
    SDL_RWFromFP(fp.cast(), crate::abi::generated_types::SDL_bool_SDL_TRUE)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_RWFromFP(
    fp: *mut libc::c_void,
    autoclose: SDL_bool,
) -> *mut SDL_RWops {
    if fp.is_null() {
        let _ = crate::core::error::invalid_param_error("fp");
        return std::ptr::null_mut();
    }
    let rw = new_rwops();
    let backend = Box::new(FileBackend {
        file: fp.cast(),
        autoclose: sdl_to_bool(autoclose),
    });
    (*rw).size = Some(stdio_size);
    (*rw).seek = Some(stdio_seek);
    (*rw).read = Some(stdio_read);
    (*rw).write = Some(stdio_write);
    (*rw).close = Some(stdio_close);
    (*rw).type_ = SDL_RWOPS_STDFILE;
    (*rw).hidden.unknown.data1 = Box::into_raw(backend).cast();
    (*rw).hidden.unknown.data2 = std::ptr::null_mut();
    rw
}

#[no_mangle]
pub unsafe extern "C" fn SDL_RWFromMem(
    mem: *mut libc::c_void,
    size: libc::c_int,
) -> *mut SDL_RWops {
    if mem.is_null() {
        let _ = crate::core::error::invalid_param_error("mem");
        return std::ptr::null_mut();
    }
    if size <= 0 {
        let _ = crate::core::error::invalid_param_error("size");
        return std::ptr::null_mut();
    }
    configure_mem_rwops(mem.cast(), size, true)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_RWFromConstMem(
    mem: *const libc::c_void,
    size: libc::c_int,
) -> *mut SDL_RWops {
    if mem.is_null() {
        let _ = crate::core::error::invalid_param_error("mem");
        return std::ptr::null_mut();
    }
    if size <= 0 {
        let _ = crate::core::error::invalid_param_error("size");
        return std::ptr::null_mut();
    }
    configure_mem_rwops(mem.cast_mut().cast(), size, false)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_AllocRW() -> *mut SDL_RWops {
    new_rwops()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_FreeRW(area: *mut SDL_RWops) {
    if !area.is_null() {
        free_rwops(area);
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_RWsize(context: *mut SDL_RWops) -> Sint64 {
    if context.is_null() {
        let _ = crate::core::error::invalid_param_error("context");
        return -1;
    }
    (*context).size.map(|size| size(context)).unwrap_or(-1)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_RWseek(
    context: *mut SDL_RWops,
    offset: Sint64,
    whence: libc::c_int,
) -> Sint64 {
    if context.is_null() {
        let _ = crate::core::error::invalid_param_error("context");
        return -1;
    }
    (*context)
        .seek
        .map(|seek| seek(context, offset, whence))
        .unwrap_or(-1)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_RWtell(context: *mut SDL_RWops) -> Sint64 {
    SDL_RWseek(context, 0, libc::SEEK_CUR)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_RWread(
    context: *mut SDL_RWops,
    ptr_: *mut libc::c_void,
    size: usize,
    maxnum: usize,
) -> usize {
    if context.is_null() {
        let _ = crate::core::error::invalid_param_error("context");
        return 0;
    }
    (*context)
        .read
        .map(|read| read(context, ptr_, size, maxnum))
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_RWwrite(
    context: *mut SDL_RWops,
    ptr_: *const libc::c_void,
    size: usize,
    num: usize,
) -> usize {
    if context.is_null() {
        let _ = crate::core::error::invalid_param_error("context");
        return 0;
    }
    (*context)
        .write
        .map(|write| write(context, ptr_, size, num))
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_RWclose(context: *mut SDL_RWops) -> libc::c_int {
    if context.is_null() {
        let _ = crate::core::error::invalid_param_error("context");
        return -1;
    }
    (*context).close.map(|close| close(context)).unwrap_or(-1)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_LoadFile_RW(
    src: *mut SDL_RWops,
    datasize: *mut usize,
    freesrc: libc::c_int,
) -> *mut libc::c_void {
    if src.is_null() {
        let _ = crate::core::error::invalid_param_error("src");
        return std::ptr::null_mut();
    }

    let size_hint = SDL_RWsize(src);
    let mut buffer = Vec::new();
    if size_hint > 0 {
        buffer.resize(size_hint as usize, 0);
        let read = SDL_RWread(src, buffer.as_mut_ptr().cast(), 1, buffer.len());
        buffer.truncate(read);
    } else {
        let mut chunk = [0u8; 4096];
        loop {
            let read = SDL_RWread(src, chunk.as_mut_ptr().cast(), 1, chunk.len());
            if read == 0 {
                break;
            }
            buffer.extend_from_slice(&chunk[..read]);
        }
    }

    let output = crate::core::memory::SDL_malloc(buffer.len() + 1) as *mut u8;
    if output.is_null() {
        if freesrc != 0 {
            let _ = SDL_RWclose(src);
        }
        let _ = crate::core::error::out_of_memory_error();
        return std::ptr::null_mut();
    }
    ptr::copy_nonoverlapping(buffer.as_ptr(), output, buffer.len());
    *output.add(buffer.len()) = 0;
    if !datasize.is_null() {
        *datasize = buffer.len();
    }
    if freesrc != 0 {
        let _ = SDL_RWclose(src);
    }
    output.cast()
}

#[no_mangle]
pub unsafe extern "C" fn SDL_LoadFile(
    file: *const libc::c_char,
    datasize: *mut usize,
) -> *mut libc::c_void {
    let mode = b"rb\0";
    let rw = SDL_RWFromFile(file, mode.as_ptr().cast());
    if rw.is_null() {
        return std::ptr::null_mut();
    }
    SDL_LoadFile_RW(rw, datasize, 1)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_ReadU8(src: *mut SDL_RWops) -> Uint8 {
    read_into::<1>(src).map(|bytes| bytes[0]).unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_ReadLE16(src: *mut SDL_RWops) -> Uint16 {
    read_into::<2>(src).map(Uint16::from_le_bytes).unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_ReadBE16(src: *mut SDL_RWops) -> Uint16 {
    read_into::<2>(src).map(Uint16::from_be_bytes).unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_ReadLE32(src: *mut SDL_RWops) -> Uint32 {
    read_into::<4>(src).map(Uint32::from_le_bytes).unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_ReadBE32(src: *mut SDL_RWops) -> Uint32 {
    read_into::<4>(src).map(Uint32::from_be_bytes).unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_ReadLE64(src: *mut SDL_RWops) -> Uint64 {
    read_into::<8>(src).map(Uint64::from_le_bytes).unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn SDL_ReadBE64(src: *mut SDL_RWops) -> Uint64 {
    read_into::<8>(src).map(Uint64::from_be_bytes).unwrap_or(0)
}

fn write_bytes(dst: *mut SDL_RWops, bytes: &[u8]) -> usize {
    unsafe {
        if SDL_RWwrite(dst, bytes.as_ptr().cast(), 1, bytes.len()) == bytes.len() {
            1
        } else {
            0
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_WriteU8(dst: *mut SDL_RWops, value: Uint8) -> usize {
    write_bytes(dst, &[value])
}

#[no_mangle]
pub unsafe extern "C" fn SDL_WriteLE16(dst: *mut SDL_RWops, value: Uint16) -> usize {
    write_bytes(dst, &value.to_le_bytes())
}

#[no_mangle]
pub unsafe extern "C" fn SDL_WriteBE16(dst: *mut SDL_RWops, value: Uint16) -> usize {
    write_bytes(dst, &value.to_be_bytes())
}

#[no_mangle]
pub unsafe extern "C" fn SDL_WriteLE32(dst: *mut SDL_RWops, value: Uint32) -> usize {
    write_bytes(dst, &value.to_le_bytes())
}

#[no_mangle]
pub unsafe extern "C" fn SDL_WriteBE32(dst: *mut SDL_RWops, value: Uint32) -> usize {
    write_bytes(dst, &value.to_be_bytes())
}

#[no_mangle]
pub unsafe extern "C" fn SDL_WriteLE64(dst: *mut SDL_RWops, value: Uint64) -> usize {
    write_bytes(dst, &value.to_le_bytes())
}

#[no_mangle]
pub unsafe extern "C" fn SDL_WriteBE64(dst: *mut SDL_RWops, value: Uint64) -> usize {
    write_bytes(dst, &value.to_be_bytes())
}
