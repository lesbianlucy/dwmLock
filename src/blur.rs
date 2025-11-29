pub fn blur_buffer(pixels: &mut [u8], width: usize, height: usize, radius: usize) {
    if radius == 0 || width == 0 || height == 0 {
        return;
    }

    let mut buffer = vec![0u8; pixels.len()];

    for y in 0..height {
        let row_offset = y * width * 4;
        for x in 0..width {
            let mut accum = [0u32; 4];
            let start = x.saturating_sub(radius);
            let end = (x + radius).min(width - 1);
            let count = (end - start + 1) as u32;

            for ix in start..=end {
                let idx = row_offset + ix * 4;
                for channel in 0..4 {
                    accum[channel] += pixels[idx + channel] as u32;
                }
            }

            let idx = row_offset + x * 4;
            for channel in 0..4 {
                buffer[idx + channel] = (accum[channel] / count) as u8;
            }
        }
    }

    for x in 0..width {
        for y in 0..height {
            let mut accum = [0u32; 4];
            let start = y.saturating_sub(radius);
            let end = (y + radius).min(height - 1);
            let count = (end - start + 1) as u32;

            for iy in start..=end {
                let idx = (iy * width + x) * 4;
                for channel in 0..4 {
                    accum[channel] += buffer[idx + channel] as u32;
                }
            }

            let idx = (y * width + x) * 4;
            for channel in 0..4 {
                pixels[idx + channel] = (accum[channel] / count) as u8;
            }
        }
    }
}
