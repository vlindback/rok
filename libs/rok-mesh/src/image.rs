//  image.rs
//

use std::io::Cursor;
use zune_core::colorspace::ColorSpace;
use zune_core::options::DecoderOptions;
use zune_jpeg::JpegDecoder;

pub struct ImageData {
    pub width: u32,
    pub height: u32,
    pub rgba8: Vec<u8>,
}

pub(crate) fn decode_image(bytes: &[u8], mime: Option<&str>) -> Result<ImageData, String> {
    let is_png = match mime {
        Some("image/png") => true,
        Some("image/jpeg") | Some("image/jpg") => false,
        _ => {
            // Sniff magic bytes if MIME is absent or unknown
            if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
                true
            } else if bytes.starts_with(&[0xFF, 0xD8]) {
                false
            } else {
                return Err("Unsupported or unrecognized image format".to_string());
            }
        }
    };

    if is_png {
        decode_png(bytes)
    } else {
        decode_jpeg(bytes)
    }
}

// JPG

fn decode_jpeg(bytes: &[u8]) -> Result<ImageData, String> {
    // Hint to the decoder that we prefer RGBA
    // I found that jgpeg_set_out_colorspace does not guarantee and can silently fail.
    // Thats why we force it below.
    let options = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGBA);

    let cursor = Cursor::new(bytes);
    let mut decoder = JpegDecoder::new_with_options(cursor, options);

    let buf = decoder
        .decode()
        .map_err(|e| format!("JPEG decoding failed: {:?}", e))?;

    let info = decoder
        .info()
        .ok_or_else(|| "Failed to fetch JPEG metadata".to_string())?;

    let color_space = decoder
        .output_colorspace()
        .ok_or_else(|| "Failed to fetch output colorspace".to_string())?;

    // Check what the decoder ACTUALLY produced and normalize if necessary
    let rgba_data = match color_space {
        ColorSpace::RGBA => buf, // The hint worked!
        ColorSpace::RGB => {
            // Decoder fell back to RGB; manually inject the alpha channel
            let mut rgba = Vec::with_capacity((buf.len() / 3) * 4);
            for chunk in buf.chunks_exact(3) {
                rgba.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
            }
            rgba
        }
        ColorSpace::Luma => {
            // Decoder fell back to Grayscale
            let mut rgba = Vec::with_capacity(buf.len() * 4);
            for &gray in &buf {
                rgba.extend_from_slice(&[gray, gray, gray, 255]);
            }
            rgba
        }
        other => {
            return Err(format!(
                "JPEG decoded into unhandled colorspace: {:?}",
                other
            ));
        }
    };

    // Verify that buffer fits image
    debug_assert_eq!(
        info.width as usize * info.height as usize * 4,
        rgba_data.len()
    );

    Ok(ImageData {
        width: info.width as u32,
        height: info.height as u32,
        rgba8: rgba_data,
    })
}

// PNG

fn decode_png(bytes: &[u8]) -> Result<ImageData, String> {
    let cursor = Cursor::new(bytes);
    let mut decoder = png::Decoder::new(cursor);

    // Request the PNG decoder to expand low bit depths and convert to 8-bit channels
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);

    let mut reader = decoder
        .read_info()
        .map_err(|e| format!("Failed to read PNG info: {:?}", e))?;

    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| format!("PNG decoding failed: {:?}", e))?;

    // Shrink buffer to actual decoded size
    buf.truncate(info.buffer_size());

    // Normalize PNG color types to RGBA8
    let rgba_data = match info.color_type {
        png::ColorType::Rgba => buf, // Already RGBA8
        png::ColorType::Rgb => {
            // Convert RGB to RGBA by injecting a full opacity alpha channel (255)
            let mut rgba = Vec::with_capacity((buf.len() / 3) * 4);
            for chunk in buf.chunks_exact(3) {
                rgba.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
            }
            rgba
        }
        png::ColorType::Grayscale => {
            // Convert Grayscale to RGBA (Luminance copied across R, G, B)
            let mut rgba = Vec::with_capacity(buf.len() * 4);
            for &gray in &buf {
                rgba.extend_from_slice(&[gray, gray, gray, 255]);
            }
            rgba
        }
        png::ColorType::GrayscaleAlpha => {
            // Convert Grayscale + Alpha to RGBA
            let mut rgba = Vec::with_capacity((buf.len() / 2) * 4);
            for chunk in buf.chunks_exact(2) {
                let gray = chunk[0];
                let alpha = chunk[1];
                rgba.extend_from_slice(&[gray, gray, gray, alpha]);
            }
            rgba
        }
        png::ColorType::Indexed => {
            return Err("Indexed PNG color space mapping failed expansion".to_string());
        }
    };

    // Verify that buffer fits image
    debug_assert_eq!(
        info.width as usize * info.height as usize * 4,
        rgba_data.len()
    );

    Ok(ImageData {
        width: info.width as u32,
        height: info.height as u32,
        rgba8: rgba_data,
    })
}
