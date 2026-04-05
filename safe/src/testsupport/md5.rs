use std::os::raw::c_ulong;
use std::slice;

use crate::testsupport::SDLTest_Md5Context;

const INIT_STATE: [u32; 4] = [0x6745_2301, 0xefcd_ab89, 0x98ba_dcfe, 0x1032_5476];
const S: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9,
    14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15,
    21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];
const K: [u32; 64] = [
    0xd76a_a478,
    0xe8c7_b756,
    0x2420_70db,
    0xc1bd_ceee,
    0xf57c_0faf,
    0x4787_c62a,
    0xa830_4613,
    0xfd46_9501,
    0x6980_98d8,
    0x8b44_f7af,
    0xffff_5bb1,
    0x895c_d7be,
    0x6b90_1122,
    0xfd98_7193,
    0xa679_438e,
    0x49b4_0821,
    0xf61e_2562,
    0xc040_b340,
    0x265e_5a51,
    0xe9b6_c7aa,
    0xd62f_105d,
    0x0244_1453,
    0xd8a1_e681,
    0xe7d3_fbc8,
    0x21e1_cde6,
    0xc337_07d6,
    0xf4d5_0d87,
    0x455a_14ed,
    0xa9e3_e905,
    0xfcef_a3f8,
    0x676f_02d9,
    0x8d2a_4c8a,
    0xfffa_3942,
    0x8771_f681,
    0x6d9d_6122,
    0xfde5_380c,
    0xa4be_ea44,
    0x4bde_cfa9,
    0xf6bb_4b60,
    0xbebf_bc70,
    0x289b_7ec6,
    0xeaa1_27fa,
    0xd4ef_3085,
    0x0488_1d05,
    0xd9d4_d039,
    0xe6db_99e5,
    0x1fa2_7cf8,
    0xc4ac_5665,
    0xf429_2244,
    0x432a_ff97,
    0xab94_23a7,
    0xfc93_a039,
    0x655b_59c3,
    0x8f0c_cc92,
    0xffef_f47d,
    0x8584_5dd1,
    0x6fa8_7e4f,
    0xfe2c_e6e0,
    0xa301_4314,
    0x4e08_11a1,
    0xf753_7e82,
    0xbd3a_f235,
    0x2ad7_d2bb,
    0xeb86_d391,
];

fn get_state(ctx: &SDLTest_Md5Context) -> [u32; 4] {
    [
        ctx.buf[0] as u32,
        ctx.buf[1] as u32,
        ctx.buf[2] as u32,
        ctx.buf[3] as u32,
    ]
}

fn set_state(ctx: &mut SDLTest_Md5Context, state: [u32; 4]) {
    for (slot, value) in ctx.buf.iter_mut().zip(state) {
        *slot = value as c_ulong;
    }
}

fn bit_count(ctx: &SDLTest_Md5Context) -> u64 {
    ((ctx.i[1] as u64) << 32) | (ctx.i[0] as u32 as u64)
}

fn set_bit_count(ctx: &mut SDLTest_Md5Context, bits: u64) {
    ctx.i[0] = (bits as u32) as c_ulong;
    ctx.i[1] = ((bits >> 32) as u32) as c_ulong;
}

fn transform(state: &mut [u32; 4], block: &[u8; 64]) {
    let mut a = state[0];
    let mut b = state[1];
    let mut c = state[2];
    let mut d = state[3];
    let mut m = [0u32; 16];
    for (index, chunk) in block.chunks_exact(4).enumerate() {
        m[index] = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
    }
    for i in 0..64usize {
        let (f, g) = if i < 16 {
            ((b & c) | ((!b) & d), i)
        } else if i < 32 {
            ((d & b) | ((!d) & c), (5 * i + 1) % 16)
        } else if i < 48 {
            (b ^ c ^ d, (3 * i + 5) % 16)
        } else {
            (c ^ (b | !d), (7 * i) % 16)
        };
        let next = a
            .wrapping_add(f)
            .wrapping_add(K[i])
            .wrapping_add(m[g])
            .rotate_left(S[i]);
        a = d;
        d = c;
        c = b;
        b = b.wrapping_add(next);
    }
    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
}

unsafe fn process_update(ctx: &mut SDLTest_Md5Context, input: &[u8]) {
    let mut state = get_state(ctx);
    let mut count = bit_count(ctx);
    let mut offset = ((count >> 3) & 0x3f) as usize;
    count = count.wrapping_add((input.len() as u64) * 8);
    set_bit_count(ctx, count);

    for &byte in input {
        ctx.in_[offset] = byte;
        offset += 1;
        if offset == 64 {
            let block = ctx.in_;
            transform(&mut state, &block);
            offset = 0;
        }
    }
    set_state(ctx, state);
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_Md5Init(mdContext: *mut SDLTest_Md5Context) {
    let Some(ctx) = mdContext.as_mut() else {
        return;
    };
    *ctx = std::mem::zeroed();
    set_state(ctx, INIT_STATE);
    set_bit_count(ctx, 0);
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_Md5Update(
    mdContext: *mut SDLTest_Md5Context,
    inBuf: *mut u8,
    inLen: u32,
) {
    let Some(ctx) = mdContext.as_mut() else {
        return;
    };
    if inBuf.is_null() || inLen == 0 {
        return;
    }
    process_update(ctx, slice::from_raw_parts(inBuf, inLen as usize));
}

#[no_mangle]
pub unsafe extern "C" fn SDLTest_Md5Final(mdContext: *mut SDLTest_Md5Context) {
    let Some(ctx) = mdContext.as_mut() else {
        return;
    };
    let original_bits = bit_count(ctx);
    let index = ((original_bits >> 3) & 0x3f) as usize;
    let pad_len = if index < 56 { 56 - index } else { 120 - index };
    let mut padding = vec![0u8; pad_len];
    if let Some(first) = padding.first_mut() {
        *first = 0x80;
    }
    process_update(ctx, &padding);
    let length_bytes = original_bits.to_le_bytes();
    process_update(ctx, &length_bytes);
    let state = get_state(ctx);
    for (chunk, value) in ctx.digest.chunks_exact_mut(4).zip(state) {
        chunk.copy_from_slice(&value.to_le_bytes());
    }
}
