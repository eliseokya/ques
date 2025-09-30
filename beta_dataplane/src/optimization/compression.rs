//! Data compression utilities
//!
//! Compress data for storage and transmission to reduce bandwidth and storage costs.

use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::{Result, BetaDataplaneError};

/// Compression algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    /// No compression
    None,
    
    /// Gzip compression
    Gzip,
    
    /// Snappy compression (fast)
    Snappy,
    
    /// Lz4 compression (very fast)
    Lz4,
    
    /// Zstd compression (balanced)
    Zstd,
}

/// Compression level (1-9, higher = better compression but slower)
#[derive(Debug, Clone, Copy)]
pub struct CompressionLevel(u8);

impl CompressionLevel {
    /// Create new compression level
    pub fn new(level: u8) -> Self {
        Self(level.clamp(1, 9))
    }

    /// Get the level value
    pub fn value(&self) -> u8 {
        self.0
    }
}

impl Default for CompressionLevel {
    fn default() -> Self {
        Self(6)
    }
}

/// Data compressor
pub struct DataCompressor {
    /// Compression algorithm
    algorithm: CompressionAlgorithm,
    
    /// Compression level
    level: CompressionLevel,
}

impl DataCompressor {
    /// Create a new data compressor
    pub fn new(algorithm: CompressionAlgorithm, level: CompressionLevel) -> Self {
        Self { algorithm, level }
    }

    /// Compress data
    pub fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        match self.algorithm {
            CompressionAlgorithm::None => Ok(data.to_vec()),
            CompressionAlgorithm::Gzip => self.compress_gzip(data),
            CompressionAlgorithm::Snappy => self.compress_snappy(data),
            CompressionAlgorithm::Lz4 => self.compress_lz4(data),
            CompressionAlgorithm::Zstd => self.compress_zstd(data),
        }
    }

    /// Decompress data
    pub fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        match self.algorithm {
            CompressionAlgorithm::None => Ok(data.to_vec()),
            CompressionAlgorithm::Gzip => self.decompress_gzip(data),
            CompressionAlgorithm::Snappy => self.decompress_snappy(data),
            CompressionAlgorithm::Lz4 => self.decompress_lz4(data),
            CompressionAlgorithm::Zstd => self.decompress_zstd(data),
        }
    }

    /// Compress with Gzip
    fn compress_gzip(&self, data: &[u8]) -> Result<Vec<u8>> {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::new(self.level.value() as u32));
        encoder.write_all(data)
            .map_err(|e| BetaDataplaneError::internal(format!("Gzip compression failed: {}", e)))?;
        
        encoder.finish()
            .map_err(|e| BetaDataplaneError::internal(format!("Gzip finish failed: {}", e)))
    }

    /// Decompress with Gzip
    fn decompress_gzip(&self, data: &[u8]) -> Result<Vec<u8>> {
        use flate2::read::GzDecoder;
        use std::io::Read;

        let mut decoder = GzDecoder::new(data);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)
            .map_err(|e| BetaDataplaneError::internal(format!("Gzip decompression failed: {}", e)))?;
        
        Ok(decompressed)
    }

    /// Compress with Snappy
    fn compress_snappy(&self, data: &[u8]) -> Result<Vec<u8>> {
        // TODO: Implement Snappy compression when snap crate is added
        debug!("Snappy compression not yet implemented, using no compression");
        Ok(data.to_vec())
    }

    /// Decompress with Snappy
    fn decompress_snappy(&self, data: &[u8]) -> Result<Vec<u8>> {
        // TODO: Implement Snappy decompression
        debug!("Snappy decompression not yet implemented");
        Ok(data.to_vec())
    }

    /// Compress with Lz4
    fn compress_lz4(&self, data: &[u8]) -> Result<Vec<u8>> {
        // TODO: Implement Lz4 compression when lz4 crate is added
        debug!("Lz4 compression not yet implemented, using no compression");
        Ok(data.to_vec())
    }

    /// Decompress with Lz4
    fn decompress_lz4(&self, data: &[u8]) -> Result<Vec<u8>> {
        // TODO: Implement Lz4 decompression
        debug!("Lz4 decompression not yet implemented");
        Ok(data.to_vec())
    }

    /// Compress with Zstd
    fn compress_zstd(&self, data: &[u8]) -> Result<Vec<u8>> {
        // TODO: Implement Zstd compression when zstd crate is added
        debug!("Zstd compression not yet implemented, using no compression");
        Ok(data.to_vec())
    }

    /// Decompress with Zstd
    fn decompress_zstd(&self, data: &[u8]) -> Result<Vec<u8>> {
        // TODO: Implement Zstd decompression
        debug!("Zstd decompression not yet implemented");
        Ok(data.to_vec())
    }

    /// Calculate compression ratio
    pub fn compression_ratio(&self, original_size: usize, compressed_size: usize) -> f64 {
        if original_size > 0 {
            compressed_size as f64 / original_size as f64
        } else {
            1.0
        }
    }

    /// Estimate compression benefit
    pub fn estimate_benefit(&self, original_size: usize) -> CompressionBenefit {
        let estimated_ratio = match self.algorithm {
            CompressionAlgorithm::None => 1.0,
            CompressionAlgorithm::Gzip => 0.3, // ~70% reduction
            CompressionAlgorithm::Snappy => 0.5, // ~50% reduction
            CompressionAlgorithm::Lz4 => 0.6, // ~40% reduction (very fast)
            CompressionAlgorithm::Zstd => 0.4, // ~60% reduction
        };

        let compressed_size = (original_size as f64 * estimated_ratio) as usize;
        let bytes_saved = original_size - compressed_size;

        CompressionBenefit {
            original_size,
            compressed_size,
            bytes_saved,
            compression_ratio: estimated_ratio,
        }
    }
}

/// Compression benefit analysis
#[derive(Debug, Clone)]
pub struct CompressionBenefit {
    /// Original size in bytes
    pub original_size: usize,
    
    /// Compressed size in bytes
    pub compressed_size: usize,
    
    /// Bytes saved
    pub bytes_saved: usize,
    
    /// Compression ratio (0.0 to 1.0)
    pub compression_ratio: f64,
}

impl Default for DataCompressor {
    fn default() -> Self {
        Self::new(CompressionAlgorithm::Gzip, CompressionLevel::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_none() {
        let compressor = DataCompressor::new(CompressionAlgorithm::None, CompressionLevel::default());
        let data = b"Hello, World!";
        
        let compressed = compressor.compress(data).unwrap();
        assert_eq!(compressed, data);
        
        let decompressed = compressor.decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_compression_gzip() {
        let compressor = DataCompressor::new(CompressionAlgorithm::Gzip, CompressionLevel::new(6));
        let data = b"Hello, World! This is a test of gzip compression.".repeat(10);
        
        let compressed = compressor.compress(&data).unwrap();
        assert!(compressed.len() < data.len(), "Compressed should be smaller");
        
        let decompressed = compressor.decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }
}
