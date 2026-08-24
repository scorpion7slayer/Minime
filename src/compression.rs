use std::{
    fs,
    io::{BufReader, Cursor, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Result, anyhow, bail};
use image::{
    AnimationDecoder, ColorType, DynamicImage, GenericImageView, ImageDecoder, ImageEncoder,
    ImageFormat, ImageReader,
    codecs::{
        bmp::BmpEncoder,
        farbfeld::FarbfeldEncoder,
        gif::GifDecoder,
        png::{CompressionType, FilterType, PngDecoder, PngEncoder},
        qoi::QoiEncoder,
        tiff::TiffEncoder,
        webp::{WebPDecoder, WebPEncoder},
    },
};

const MAX_INPUT_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Auto,
    WebP,
    Png,
    Qoi,
    Tiff,
    Bmp,
    Farbfeld,
}

impl OutputFormat {
    pub const ALL: [Self; 7] = [
        Self::Auto,
        Self::WebP,
        Self::Png,
        Self::Qoi,
        Self::Tiff,
        Self::Bmp,
        Self::Farbfeld,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::WebP => "WebP",
            Self::Png => "PNG",
            Self::Qoi => "QOI",
            Self::Tiff => "TIFF",
            Self::Bmp => "BMP",
            Self::Farbfeld => "FF",
        }
    }

    pub const fn description(self) -> &'static str {
        match self {
            Self::Auto => "Allège sans changer les pixels",
            Self::WebP => "Petit fichier, transparence préservée",
            Self::Png => "Le choix passe-partout",
            Self::Qoi => "Rapide à lire et à écrire",
            Self::Tiff => "Pour l’archive et la retouche",
            Self::Bmp => "Pour les anciens logiciels",
            Self::Farbfeld => "RGBA 16 bits, volontairement simple",
        }
    }

    pub const fn description_en(self) -> &'static str {
        match self {
            Self::Auto => "Makes files smaller without changing pixels",
            Self::WebP => "Small files with transparency intact",
            Self::Png => "The dependable all-rounder",
            Self::Qoi => "Fast to read and write",
            Self::Tiff => "For archives and editing",
            Self::Bmp => "For older software",
            Self::Farbfeld => "Straightforward 16-bit RGBA",
        }
    }

    pub const fn id(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::WebP => "webp",
            Self::Png => "png",
            Self::Qoi => "qoi",
            Self::Tiff => "tiff",
            Self::Bmp => "bmp",
            Self::Farbfeld => "farbfeld",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Some(match id {
            "auto" => Self::Auto,
            "webp" => Self::WebP,
            "png" => Self::Png,
            "qoi" => Self::Qoi,
            "tiff" => Self::Tiff,
            "bmp" => Self::Bmp,
            "farbfeld" => Self::Farbfeld,
            _ => return None,
        })
    }

    const fn extension(self) -> Option<&'static str> {
        match self {
            Self::Auto => None,
            Self::WebP => Some("webp"),
            Self::Png => Some("png"),
            Self::Qoi => Some("qoi"),
            Self::Tiff => Some("tiff"),
            Self::Bmp => Some("bmp"),
            Self::Farbfeld => Some("ff"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionEffort {
    Fast,
    Balanced,
    Maximum,
}

impl CompressionEffort {
    pub const ALL: [Self; 3] = [Self::Fast, Self::Balanced, Self::Maximum];

    pub const fn id(self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Balanced => "balanced",
            Self::Maximum => "maximum",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Some(match id {
            "fast" => Self::Fast,
            "balanced" => Self::Balanced,
            "maximum" => Self::Maximum,
            _ => return None,
        })
    }

    const fn oxipng_preset(self) -> u8 {
        match self {
            Self::Fast => 2,
            Self::Balanced => 4,
            Self::Maximum => 6,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompressionOptions {
    pub output_format: OutputFormat,
    pub output_dir: Option<PathBuf>,
    pub reject_larger: bool,
    pub effort: CompressionEffort,
}

impl Default for CompressionOptions {
    fn default() -> Self {
        Self {
            output_format: OutputFormat::Auto,
            output_dir: None,
            reject_larger: true,
            effort: CompressionEffort::Balanced,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultState {
    Saved,
    Unchanged,
    Failed,
}

#[derive(Debug, Clone)]
pub struct CompressionResult {
    pub input_path: PathBuf,
    pub output_path: Option<PathBuf>,
    pub original_bytes: u64,
    pub output_bytes: u64,
    pub output_format: Option<OutputFormat>,
    pub state: ResultState,
    pub message: String,
}

impl CompressionResult {
    pub fn bytes_saved(&self) -> u64 {
        self.original_bytes.saturating_sub(self.output_bytes)
    }

    pub fn savings_percent(&self) -> f32 {
        if self.original_bytes == 0 {
            0.0
        } else {
            self.bytes_saved() as f32 / self.original_bytes as f32 * 100.0
        }
    }

    fn failed(path: &Path, original_bytes: u64, error: &anyhow::Error) -> Self {
        Self {
            input_path: path.to_path_buf(),
            output_path: None,
            original_bytes,
            output_bytes: original_bytes,
            output_format: None,
            state: ResultState::Failed,
            message: format!("{error:#}"),
        }
    }
}

#[derive(Debug)]
struct Decoded {
    image: DynamicImage,
    icc_profile: Option<Vec<u8>>,
    source_format: ImageFormat,
}

#[derive(Debug)]
struct Candidate {
    bytes: Vec<u8>,
    format: OutputFormat,
}

pub fn is_supported_path(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| {
                matches!(
                    extension.to_ascii_lowercase().as_str(),
                    "png"
                        | "apng"
                        | "jpg"
                        | "jpeg"
                        | "jfif"
                        | "webp"
                        | "gif"
                        | "bmp"
                        | "tif"
                        | "tiff"
                        | "tga"
                        | "dds"
                        | "qoi"
                        | "ico"
                        | "pnm"
                        | "ppm"
                        | "pgm"
                        | "pam"
                        | "pbm"
                        | "ff"
                )
            })
            .unwrap_or(false)
}

pub fn compress_batch(paths: Vec<PathBuf>, options: CompressionOptions) -> Vec<CompressionResult> {
    paths
        .into_iter()
        .map(|path| compress_one(&path, &options))
        .collect()
}

pub fn compress_one(path: &Path, options: &CompressionOptions) -> CompressionResult {
    let original_bytes = fs::metadata(path)
        .map(|metadata| metadata.len())
        .unwrap_or(0);

    match compress_one_inner(path, options, original_bytes) {
        Ok(result) => result,
        Err(error) => CompressionResult::failed(path, original_bytes, &error),
    }
}

fn compress_one_inner(
    path: &Path,
    options: &CompressionOptions,
    original_bytes: u64,
) -> Result<CompressionResult> {
    if !is_supported_path(path) {
        bail!("Minime ne peut pas ouvrir ce format, ou le fichier a été déplacé");
    }
    if original_bytes > MAX_INPUT_BYTES {
        bail!("Cette image dépasse la limite de sécurité de 512 Mio");
    }

    let source =
        fs::read(path).with_context(|| format!("Lecture impossible : {}", path.display()))?;
    let decoded = decode_source(&source)?;
    let candidate = build_candidate(&source, &decoded, options.output_format, options.effort)?;

    if candidate.bytes.len() >= source.len() && options.reject_larger {
        return Ok(CompressionResult {
            input_path: path.to_path_buf(),
            output_path: None,
            original_bytes,
            output_bytes: original_bytes,
            output_format: Some(candidate.format),
            state: ResultState::Unchanged,
            message: "Aucun fichier plus léger trouvé — original conservé".into(),
        });
    }

    verify_exact_pixels(&decoded.image, &candidate.bytes)
        .context("La vérification pixel par pixel a échoué")?;

    let output_path = destination_path(path, options.output_dir.as_deref(), candidate.format)?;
    persist_atomically(&output_path, &candidate.bytes)?;

    Ok(CompressionResult {
        input_path: path.to_path_buf(),
        output_path: Some(output_path),
        original_bytes,
        output_bytes: candidate.bytes.len() as u64,
        output_format: Some(candidate.format),
        state: ResultState::Saved,
        message: "Pixels vérifiés, fichier enregistré".into(),
    })
}

fn decode_source(source: &[u8]) -> Result<Decoded> {
    let mut reader = ImageReader::new(Cursor::new(source))
        .with_guessed_format()
        .context("Format d’image non reconnu")?;
    let source_format = reader
        .format()
        .ok_or_else(|| anyhow!("Format d’image non reconnu"))?;

    reject_animation(source, source_format)?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(65_535);
    limits.max_image_height = Some(65_535);
    limits.max_alloc = Some(MAX_INPUT_BYTES);
    reader.limits(limits);

    let mut decoder = reader.into_decoder().context("Décodage impossible")?;
    let icc_profile = decoder
        .icc_profile()
        .context("Lecture du profil colorimétrique impossible")?;
    let orientation = decoder
        .orientation()
        .context("Lecture de l’orientation impossible")?;
    let mut image = DynamicImage::from_decoder(decoder).context("Décodage impossible")?;
    image.apply_orientation(orientation);

    Ok(Decoded {
        image,
        icc_profile,
        source_format,
    })
}

fn reject_animation(source: &[u8], format: ImageFormat) -> Result<()> {
    let animated = match format {
        ImageFormat::Gif => {
            let decoder =
                GifDecoder::new(BufReader::new(Cursor::new(source))).context("GIF illisible")?;
            decoder.into_frames().take(2).count() > 1
        }
        ImageFormat::WebP => WebPDecoder::new(BufReader::new(Cursor::new(source)))
            .context("WebP illisible")?
            .has_animation(),
        ImageFormat::Png => PngDecoder::new(BufReader::new(Cursor::new(source)))
            .context("PNG illisible")?
            .is_apng()
            .context("PNG animé illisible")?,
        _ => false,
    };

    if animated {
        bail!("Minime ne convertit pas encore les images animées — l’original est intact");
    }
    Ok(())
}

fn build_candidate(
    source: &[u8],
    decoded: &Decoded,
    output_format: OutputFormat,
    effort: CompressionEffort,
) -> Result<Candidate> {
    if output_format == OutputFormat::Auto {
        return auto_candidate(source, decoded, effort);
    }

    let bytes = encode(decoded, output_format, effort)?;
    Ok(Candidate {
        bytes,
        format: output_format,
    })
}

fn auto_candidate(
    source: &[u8],
    decoded: &Decoded,
    effort: CompressionEffort,
) -> Result<Candidate> {
    let mut candidates = Vec::with_capacity(2);

    let png = if decoded.source_format == ImageFormat::Png {
        optimize_png(source, effort).context("Optimisation PNG impossible")?
    } else {
        encode_png(decoded, effort)?
    };
    candidates.push(Candidate {
        bytes: png,
        format: OutputFormat::Png,
    });

    if supports_8_bit_lossless(&decoded.image)
        && let Ok(webp) = encode_webp(decoded)
    {
        candidates.push(Candidate {
            bytes: webp,
            format: OutputFormat::WebP,
        });
    }

    candidates
        .into_iter()
        .min_by_key(|candidate| candidate.bytes.len())
        .ok_or_else(|| anyhow!("Aucune sortie sans perte disponible"))
}

fn encode(decoded: &Decoded, format: OutputFormat, effort: CompressionEffort) -> Result<Vec<u8>> {
    match format {
        OutputFormat::Auto => unreachable!("Auto est résolu avant l’encodage"),
        OutputFormat::WebP => encode_webp(decoded),
        OutputFormat::Png => encode_png(decoded, effort),
        OutputFormat::Qoi => encode_qoi(decoded),
        OutputFormat::Tiff => encode_tiff(decoded),
        OutputFormat::Bmp => encode_bmp(decoded),
        OutputFormat::Farbfeld => encode_farbfeld(decoded),
    }
}

fn encode_png(decoded: &Decoded, effort: CompressionEffort) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut encoder =
        PngEncoder::new_with_quality(&mut output, CompressionType::Best, FilterType::Adaptive);
    if let Some(profile) = &decoded.icc_profile {
        encoder
            .set_icc_profile(profile.clone())
            .context("Le profil colorimétrique ne peut pas être conservé en PNG")?;
    }
    encoder
        .write_image(
            decoded.image.as_bytes(),
            decoded.image.width(),
            decoded.image.height(),
            decoded.image.color().into(),
        )
        .context("Encodage PNG impossible")?;

    optimize_png(&output, effort).context("Optimisation PNG impossible")
}

fn optimize_png(source: &[u8], effort: CompressionEffort) -> Result<Vec<u8>> {
    let options = oxipng::Options::from_preset(effort.oxipng_preset());
    oxipng::optimize_from_memory(source, &options).map_err(Into::into)
}

fn encode_webp(decoded: &Decoded) -> Result<Vec<u8>> {
    if !supports_8_bit_lossless(&decoded.image) {
        bail!("WebP lossless ne conserve pas les images 16 bits dans Minime");
    }

    let mut output = Vec::new();
    let mut encoder = WebPEncoder::new_lossless(&mut output);
    if let Some(profile) = &decoded.icc_profile {
        encoder
            .set_icc_profile(profile.clone())
            .context("Le profil colorimétrique ne peut pas être conservé en WebP")?;
    }
    encoder
        .write_image(
            decoded.image.as_bytes(),
            decoded.image.width(),
            decoded.image.height(),
            decoded.image.color().into(),
        )
        .context("Encodage WebP impossible")?;
    Ok(output)
}

fn encode_qoi(decoded: &Decoded) -> Result<Vec<u8>> {
    if !supports_8_bit_lossless(&decoded.image) {
        bail!("QOI ne peut pas conserver cette profondeur de couleur");
    }
    if decoded.icc_profile.is_some() {
        bail!("QOI ne peut pas conserver le profil colorimétrique de cette image");
    }

    let rgba = decoded.image.to_rgba8();
    let mut output = Vec::new();
    QoiEncoder::new(&mut output)
        .write_image(
            rgba.as_raw(),
            rgba.width(),
            rgba.height(),
            ColorType::Rgba8.into(),
        )
        .context("Encodage QOI impossible")?;
    Ok(output)
}

fn encode_tiff(decoded: &Decoded) -> Result<Vec<u8>> {
    let mut output = Cursor::new(Vec::new());
    let mut encoder = TiffEncoder::new(&mut output);
    if let Some(profile) = &decoded.icc_profile {
        encoder
            .set_icc_profile(profile.clone())
            .context("Le profil colorimétrique ne peut pas être conservé en TIFF")?;
    }

    match decoded.image.color() {
        ColorType::La8 => {
            let rgba = decoded.image.to_rgba8();
            encoder.write_image(
                rgba.as_raw(),
                rgba.width(),
                rgba.height(),
                ColorType::Rgba8.into(),
            )
        }
        ColorType::La16 => {
            let rgba = DynamicImage::ImageRgba16(decoded.image.to_rgba16());
            encoder.write_image(
                rgba.as_bytes(),
                rgba.width(),
                rgba.height(),
                ColorType::Rgba16.into(),
            )
        }
        _ => encoder.write_image(
            decoded.image.as_bytes(),
            decoded.image.width(),
            decoded.image.height(),
            decoded.image.color().into(),
        ),
    }
    .context("Encodage TIFF impossible")?;

    Ok(output.into_inner())
}

fn encode_bmp(decoded: &Decoded) -> Result<Vec<u8>> {
    if !supports_8_bit_lossless(&decoded.image) {
        bail!("BMP ne peut pas conserver cette profondeur de couleur");
    }
    if decoded.icc_profile.is_some() {
        bail!("BMP ne peut pas conserver le profil colorimétrique de cette image");
    }

    let rgba = decoded.image.to_rgba8();
    let mut output = Vec::new();
    BmpEncoder::new(&mut output)
        .write_image(
            rgba.as_raw(),
            rgba.width(),
            rgba.height(),
            ColorType::Rgba8.into(),
        )
        .context("Encodage BMP impossible")?;
    Ok(output)
}

fn encode_farbfeld(decoded: &Decoded) -> Result<Vec<u8>> {
    if decoded.icc_profile.is_some() {
        bail!("Farbfeld ne peut pas conserver le profil colorimétrique de cette image");
    }

    let rgba = DynamicImage::ImageRgba16(decoded.image.to_rgba16());
    let mut output = Vec::new();
    FarbfeldEncoder::new(&mut output)
        .write_image(
            rgba.as_bytes(),
            rgba.width(),
            rgba.height(),
            ColorType::Rgba16.into(),
        )
        .context("Encodage Farbfeld impossible")?;
    Ok(output)
}

fn supports_8_bit_lossless(image: &DynamicImage) -> bool {
    matches!(
        image.color(),
        ColorType::L8 | ColorType::La8 | ColorType::Rgb8 | ColorType::Rgba8
    )
}

fn verify_exact_pixels(source: &DynamicImage, encoded: &[u8]) -> Result<()> {
    let output = decode_source(encoded)?;
    if source.dimensions() != output.image.dimensions() {
        bail!("Les dimensions diffèrent après encodage");
    }
    if source.to_rgba16().as_raw() != output.image.to_rgba16().as_raw() {
        bail!("Les pixels diffèrent après encodage");
    }
    Ok(())
}

fn destination_path(
    input: &Path,
    output_dir: Option<&Path>,
    format: OutputFormat,
) -> Result<PathBuf> {
    let extension = format
        .extension()
        .ok_or_else(|| anyhow!("Format de sortie non résolu"))?;
    let directory = output_dir
        .map(Path::to_path_buf)
        .or_else(|| input.parent().map(Path::to_path_buf))
        .ok_or_else(|| anyhow!("Dossier de sortie introuvable"))?;
    fs::create_dir_all(&directory)
        .with_context(|| format!("Création impossible : {}", directory.display()))?;

    let stem = input
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("image");
    let first = directory.join(format!("{stem}.minime.{extension}"));
    if !first.exists() {
        return Ok(first);
    }

    for index in 2..10_000 {
        let candidate = directory.join(format!("{stem}.minime-{index}.{extension}"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    bail!("Impossible de trouver un nom de sortie disponible")
}

fn persist_atomically(path: &Path, bytes: &[u8]) -> Result<()> {
    let directory = path
        .parent()
        .ok_or_else(|| anyhow!("Dossier de sortie introuvable"))?;
    let mut temporary = tempfile::NamedTempFile::new_in(directory)
        .with_context(|| format!("Création temporaire impossible : {}", directory.display()))?;
    temporary
        .write_all(bytes)
        .context("Écriture temporaire impossible")?;
    temporary.flush().context("Finalisation impossible")?;
    temporary
        .persist_noclobber(path)
        .map_err(|error| error.error)
        .with_context(|| format!("Écriture impossible : {}", path.display()))?;
    Ok(())
}

pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["o", "Kio", "Mio", "Gio", "Tio"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[unit])
    } else if value >= 10.0 {
        format!("{value:.0} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba, codecs::png::PngEncoder};

    fn write_uncompressed_png(path: &Path) -> DynamicImage {
        let image = ImageBuffer::from_fn(256, 192, |x, y| {
            Rgba([
                (x % 32) as u8 * 7,
                (y % 24) as u8 * 9,
                ((x + y) % 17) as u8 * 11,
                255,
            ])
        });
        let dynamic = DynamicImage::ImageRgba8(image);
        let mut output = Vec::new();
        PngEncoder::new_with_quality(
            &mut output,
            CompressionType::Uncompressed,
            FilterType::NoFilter,
        )
        .write_image(
            dynamic.as_bytes(),
            dynamic.width(),
            dynamic.height(),
            dynamic.color().into(),
        )
        .unwrap();
        fs::write(path, output).unwrap();
        dynamic
    }

    fn write_uncompressed_png_16(path: &Path) -> DynamicImage {
        let image = ImageBuffer::from_fn(48, 32, |x, y| {
            Rgba([
                (x * 1_117) as u16,
                (y * 1_733) as u16,
                ((x + y) * 701) as u16,
                65_535,
            ])
        });
        let dynamic = DynamicImage::ImageRgba16(image);
        let mut output = Vec::new();
        PngEncoder::new_with_quality(
            &mut output,
            CompressionType::Uncompressed,
            FilterType::NoFilter,
        )
        .write_image(
            dynamic.as_bytes(),
            dynamic.width(),
            dynamic.height(),
            dynamic.color().into(),
        )
        .unwrap();
        fs::write(path, output).unwrap();
        dynamic
    }

    fn write_tiny_png(path: &Path) -> DynamicImage {
        let image = ImageBuffer::from_pixel(1, 1, Rgba([31_u8, 89, 144, 255]));
        let dynamic = DynamicImage::ImageRgba8(image);
        let mut output = Vec::new();
        PngEncoder::new_with_quality(&mut output, CompressionType::Best, FilterType::Adaptive)
            .write_image(
                dynamic.as_bytes(),
                dynamic.width(),
                dynamic.height(),
                dynamic.color().into(),
            )
            .unwrap();
        fs::write(path, output).unwrap();
        dynamic
    }

    #[test]
    fn auto_compression_is_smaller_and_pixel_exact() {
        let directory = tempfile::tempdir().unwrap();
        let source_path = directory.path().join("sample.png");
        let source_image = write_uncompressed_png(&source_path);

        let result = compress_one(&source_path, &CompressionOptions::default());

        assert_eq!(result.state, ResultState::Saved, "{}", result.message);
        assert!(result.output_bytes < result.original_bytes);
        let output = fs::read(result.output_path.unwrap()).unwrap();
        verify_exact_pixels(&source_image, &output).unwrap();
    }

    #[test]
    fn explicit_qoi_output_is_pixel_exact() {
        let directory = tempfile::tempdir().unwrap();
        let source_path = directory.path().join("source.png");
        let source_image = write_uncompressed_png(&source_path);
        let options = CompressionOptions {
            output_format: OutputFormat::Qoi,
            output_dir: None,
            reject_larger: false,
            effort: CompressionEffort::Balanced,
        };

        let result = compress_one(&source_path, &options);

        assert_eq!(result.state, ResultState::Saved, "{}", result.message);
        assert_eq!(result.output_format, Some(OutputFormat::Qoi));
        let output = fs::read(result.output_path.unwrap()).unwrap();
        verify_exact_pixels(&source_image, &output).unwrap();
    }

    #[test]
    fn manual_conversion_can_write_a_larger_exact_file() {
        let directory = tempfile::tempdir().unwrap();
        let source_path = directory.path().join("tiny.png");
        let source_image = write_tiny_png(&source_path);
        let options = CompressionOptions {
            output_format: OutputFormat::Tiff,
            output_dir: None,
            reject_larger: false,
            effort: CompressionEffort::Balanced,
        };

        let result = compress_one(&source_path, &options);

        assert_eq!(result.state, ResultState::Saved, "{}", result.message);
        assert!(result.output_bytes > result.original_bytes);
        let output = fs::read(result.output_path.unwrap()).unwrap();
        verify_exact_pixels(&source_image, &output).unwrap();

        let skipped = compress_one(
            &source_path,
            &CompressionOptions {
                reject_larger: true,
                ..options
            },
        );
        assert_eq!(skipped.state, ResultState::Unchanged);
        assert!(skipped.output_path.is_none());
    }

    #[test]
    fn extended_outputs_are_pixel_exact() {
        let directory = tempfile::tempdir().unwrap();
        let source_path = directory.path().join("source.png");
        let source_image = write_uncompressed_png(&source_path);

        for output_format in [
            OutputFormat::Tiff,
            OutputFormat::Bmp,
            OutputFormat::Farbfeld,
        ] {
            let options = CompressionOptions {
                output_format,
                output_dir: None,
                reject_larger: false,
                effort: CompressionEffort::Balanced,
            };
            let result = compress_one(&source_path, &options);

            assert_eq!(result.state, ResultState::Saved, "{}", result.message);
            assert_eq!(result.output_format, Some(output_format));
            let output = fs::read(result.output_path.unwrap()).unwrap();
            verify_exact_pixels(&source_image, &output).unwrap();
        }
    }

    #[test]
    fn sixteen_bit_outputs_are_exact_and_bmp_is_refused() {
        let directory = tempfile::tempdir().unwrap();
        let source_path = directory.path().join("source-16.png");
        let source_image = write_uncompressed_png_16(&source_path);

        for output_format in [OutputFormat::Tiff, OutputFormat::Farbfeld] {
            let result = compress_one(
                &source_path,
                &CompressionOptions {
                    output_format,
                    output_dir: None,
                    reject_larger: false,
                    effort: CompressionEffort::Balanced,
                },
            );

            assert_eq!(result.state, ResultState::Saved, "{}", result.message);
            let output = fs::read(result.output_path.unwrap()).unwrap();
            verify_exact_pixels(&source_image, &output).unwrap();
        }

        let bmp = compress_one(
            &source_path,
            &CompressionOptions {
                output_format: OutputFormat::Bmp,
                output_dir: None,
                reject_larger: false,
                effort: CompressionEffort::Balanced,
            },
        );
        assert_eq!(bmp.state, ResultState::Failed);
        assert!(bmp.message.contains("profondeur de couleur"));
    }

    #[test]
    fn additional_input_extensions_are_accepted() {
        let directory = tempfile::tempdir().unwrap();

        for extension in ["dds", "ff", "pam", "pbm"] {
            let path = directory.path().join(format!("sample.{extension}"));
            fs::write(&path, b"fixture").unwrap();
            assert!(is_supported_path(&path), "extension refusée : {extension}");
        }
    }

    #[test]
    fn generated_names_never_overwrite_existing_files() {
        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("photo.jpg");
        fs::write(directory.path().join("photo.minime.webp"), b"existing").unwrap();

        let path = destination_path(&input, None, OutputFormat::WebP).unwrap();

        assert_eq!(
            path.file_name().and_then(|name| name.to_str()),
            Some("photo.minime-2.webp")
        );
    }

    #[test]
    fn bytes_are_formatted_for_people() {
        assert_eq!(format_bytes(999), "999 o");
        assert_eq!(format_bytes(1_536), "1.5 Kio");
        assert_eq!(format_bytes(12 * 1024 * 1024), "12 Mio");
    }
}
