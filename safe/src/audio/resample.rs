use std::f64::consts::PI;
use std::sync::OnceLock;

const RESAMPLER_ZERO_CROSSINGS: usize = 5;
const RESAMPLER_BITS_PER_SAMPLE: usize = 16;
const RESAMPLER_SAMPLES_PER_ZERO_CROSSING: usize = 1 << ((RESAMPLER_BITS_PER_SAMPLE / 2) + 1);
const RESAMPLER_FILTER_SIZE: usize =
    (RESAMPLER_SAMPLES_PER_ZERO_CROSSING * RESAMPLER_ZERO_CROSSINGS) + 1;

struct FilterTable {
    filter: Box<[f32]>,
    difference: Box<[f32]>,
}

fn bessel(x: f64) -> f64 {
    let xdiv2 = x / 2.0;
    let mut i0 = 1.0f64;
    let mut factorial = 1.0f64;
    let mut i = 1i32;

    loop {
        let diff = xdiv2.powi(i * 2) / factorial.powi(2);
        if diff < 1.0e-21 {
            break;
        }
        i0 += diff;
        i += 1;
        factorial *= i as f64;
    }

    i0
}

fn filter_table() -> &'static FilterTable {
    static TABLE: OnceLock<FilterTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut filter = vec![0.0f32; RESAMPLER_FILTER_SIZE];
        let mut difference = vec![0.0f32; RESAMPLER_FILTER_SIZE];
        let len_minus_one = (RESAMPLER_FILTER_SIZE - 1) as f64;
        let len_minus_one_div_two = len_minus_one / 2.0;
        let beta = 0.1102 * (80.0 - 8.7);
        let bessel_beta = bessel(beta);

        filter[0] = 1.0;
        for i in 1..RESAMPLER_FILTER_SIZE {
            let scaled = ((i as f64 - len_minus_one) / 2.0) / len_minus_one_div_two;
            let kaiser = bessel(beta * (1.0 - scaled.powi(2)).sqrt()) / bessel_beta;
            filter[RESAMPLER_FILTER_SIZE - i] = kaiser as f32;
        }

        for i in 1..RESAMPLER_FILTER_SIZE {
            let x = (i as f64 / RESAMPLER_SAMPLES_PER_ZERO_CROSSING as f64) * PI;
            filter[i] *= (x.sin() / x) as f32;
            difference[i - 1] = filter[i] - filter[i - 1];
        }
        difference[RESAMPLER_FILTER_SIZE - 1] = 0.0;

        FilterTable {
            filter: filter.into_boxed_slice(),
            difference: difference.into_boxed_slice(),
        }
    })
}

fn output_frames(input_frames: usize, src_rate: i32, dst_rate: i32) -> usize {
    if input_frames == 0 || src_rate <= 0 || dst_rate <= 0 {
        return 0;
    }

    usize::try_from((input_frames as u128 * dst_rate as u128) / src_rate as u128)
        .unwrap_or(usize::MAX)
}

fn input_sample(
    input: &[f32],
    channels: usize,
    input_frames: usize,
    frame: isize,
    channel: usize,
) -> f32 {
    if frame < 0 || frame as usize >= input_frames {
        0.0
    } else {
        input[frame as usize * channels + channel]
    }
}

pub(crate) fn resample_interleaved_f32_into(
    input: &[f32],
    channels: usize,
    src_rate: i32,
    dst_rate: i32,
    output: &mut Vec<f32>,
) {
    if channels == 0 || src_rate <= 0 || dst_rate <= 0 {
        output.clear();
        return;
    }
    if src_rate == dst_rate {
        output.clear();
        output.extend_from_slice(input);
        return;
    }

    let input_frames = input.len() / channels;
    let output_frames = output_frames(input_frames, src_rate, dst_rate);
    if input_frames == 0 || output_frames == 0 {
        output.clear();
        return;
    }

    let tables = filter_table();
    output.clear();
    output.resize(output_frames * channels, 0.0);

    for out_frame in 0..output_frames {
        let src_index = (out_frame as i64 * src_rate as i64 / dst_rate as i64) as isize;
        let src_fraction = (out_frame as i64 * src_rate as i64 % dst_rate as i64) as usize;
        let interpolation_left = src_fraction as f32 / dst_rate as f32;
        let filter_index_left =
            src_fraction * RESAMPLER_SAMPLES_PER_ZERO_CROSSING / dst_rate as usize;
        let interpolation_right = 1.0 - interpolation_left;
        let filter_index_right = (dst_rate as usize - src_fraction)
            * RESAMPLER_SAMPLES_PER_ZERO_CROSSING
            / dst_rate as usize;

        for channel in 0..channels {
            let mut out_sample = 0.0f32;
            let mut tap = 0usize;

            while filter_index_left + tap * RESAMPLER_SAMPLES_PER_ZERO_CROSSING
                < RESAMPLER_FILTER_SIZE
            {
                let filter_slot = filter_index_left + tap * RESAMPLER_SAMPLES_PER_ZERO_CROSSING;
                let input_sample = input_sample(
                    input,
                    channels,
                    input_frames,
                    src_index - tap as isize,
                    channel,
                );
                let weight = tables.filter[filter_slot]
                    + interpolation_left * tables.difference[filter_slot];
                out_sample += input_sample * weight;
                tap += 1;
            }

            let mut tap = 0usize;
            while filter_index_right + tap * RESAMPLER_SAMPLES_PER_ZERO_CROSSING
                < RESAMPLER_FILTER_SIZE
            {
                let filter_slot = filter_index_right + tap * RESAMPLER_SAMPLES_PER_ZERO_CROSSING;
                let input_sample = input_sample(
                    input,
                    channels,
                    input_frames,
                    src_index + 1 + tap as isize,
                    channel,
                );
                let weight = tables.filter[filter_slot]
                    + interpolation_right * tables.difference[filter_slot];
                out_sample += input_sample * weight;
                tap += 1;
            }

            output[out_frame * channels + channel] = out_sample;
        }
    }
}
