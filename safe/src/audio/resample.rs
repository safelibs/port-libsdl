use std::f64::consts::PI;

const FILTER_TAPS: isize = 16;

fn output_frames(input_frames: usize, src_rate: i32, dst_rate: i32) -> usize {
    if input_frames == 0 || src_rate <= 0 || dst_rate <= 0 {
        return 0;
    }

    let numerator = (input_frames as u128) * (dst_rate as u128) + (src_rate as u128 / 2);
    usize::try_from(numerator / src_rate as u128).unwrap_or(usize::MAX)
}

fn sinc(x: f64) -> f64 {
    if x.abs() < 1.0e-12 {
        1.0
    } else {
        x.sin() / x
    }
}

fn lanczos_weight(distance: f64, cutoff: f64) -> f64 {
    let radius = FILTER_TAPS as f64;
    let abs_distance = distance.abs();
    if abs_distance >= radius {
        return 0.0;
    }

    let x = PI * distance * cutoff;
    let window = sinc(PI * distance / radius);
    cutoff * sinc(x) * window
}

pub(crate) fn resample_interleaved_f32(
    input: &[f32],
    channels: usize,
    src_rate: i32,
    dst_rate: i32,
) -> Vec<f32> {
    if channels == 0 || src_rate <= 0 || dst_rate <= 0 {
        return Vec::new();
    }
    if src_rate == dst_rate {
        return input.to_vec();
    }

    let input_frames = input.len() / channels;
    let output_frames = output_frames(input_frames, src_rate, dst_rate);
    if input_frames == 0 || output_frames == 0 {
        return Vec::new();
    }

    let cutoff = if dst_rate < src_rate {
        (dst_rate as f64 / src_rate as f64) * 0.999
    } else {
        1.0
    };
    let rate_ratio = src_rate as f64 / dst_rate as f64;
    let mut output = vec![0.0f32; output_frames * channels];

    for out_frame in 0..output_frames {
        let source_position = out_frame as f64 * rate_ratio;
        let center = source_position.floor() as isize;
        let start = center - FILTER_TAPS + 1;
        let end = center + FILTER_TAPS;

        let mut accum = vec![0.0f64; channels];
        let mut weight_sum = 0.0f64;

        for in_frame in start..=end {
            if in_frame < 0 || in_frame >= input_frames as isize {
                continue;
            }
            let distance = source_position - in_frame as f64;
            let weight = lanczos_weight(distance, cutoff);
            if weight == 0.0 {
                continue;
            }

            let base = in_frame as usize * channels;
            for channel in 0..channels {
                accum[channel] += input[base + channel] as f64 * weight;
            }
            weight_sum += weight;
        }

        let output_base = out_frame * channels;
        if weight_sum.abs() < 1.0e-12 {
            let nearest = source_position
                .round()
                .clamp(0.0, (input_frames - 1) as f64) as usize;
            let input_base = nearest * channels;
            output[output_base..output_base + channels]
                .copy_from_slice(&input[input_base..input_base + channels]);
            continue;
        }

        for channel in 0..channels {
            output[output_base + channel] = (accum[channel] / weight_sum) as f32;
        }
    }

    output
}
