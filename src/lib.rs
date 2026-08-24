pub mod compression;

pub use compression::{
    CompressionEffort, CompressionOptions, CompressionResult, OutputFormat, ResultState,
    compress_batch, compress_one, format_bytes, is_supported_path,
};
