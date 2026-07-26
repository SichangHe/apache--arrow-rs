#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use parquet::errors::ParquetError;
    use parquet::file::metadata::{FooterTail, ParquetMetaDataReader};

    fn footer(metadata_len: u32, magic: [u8; 4]) -> [u8; 8] {
        let mut bytes = [0; 8];
        bytes[..4].copy_from_slice(&metadata_len.to_le_bytes());
        bytes[4..].copy_from_slice(&magic);
        bytes
    }

    #[test]
    fn footer_magic_and_little_endian_length() {
        let plain = FooterTail::try_from(footer(0x7856_3412, *b"PAR1")).unwrap();
        assert_eq!(plain.metadata_length(), 0x7856_3412);
        assert!(!plain.is_encrypted_footer());

        let encrypted = FooterTail::try_from(footer(u32::MAX, *b"PARE")).unwrap();
        assert_eq!(encrypted.metadata_length(), u32::MAX as usize);
        assert!(encrypted.is_encrypted_footer());

        assert!(FooterTail::try_from(footer(28, *b"PAR0")).is_err());
    }

    #[test]
    fn short_file_fails_before_footer_decode() {
        let bytes = Bytes::from(vec![0; 7]);
        let mut reader = ParquetMetaDataReader::new();
        assert!(matches!(
            reader.try_parse(&bytes),
            Err(ParquetError::EOF(_))
        ));
        assert_eq!(reader.metadata_size(), None);
    }

    #[test]
    fn corrupt_magic_fails_before_metadata_range() {
        let bytes = Bytes::copy_from_slice(&footer(28, *b"PAR0"));
        let mut reader = ParquetMetaDataReader::new();
        let error = reader.try_parse_sized(&bytes, 100).unwrap_err();
        assert!(matches!(error, ParquetError::General(_)));
        assert_eq!(
            error.to_string(),
            "Parquet error: Invalid Parquet file. Corrupt footer"
        );
        assert_eq!(reader.metadata_size(), None);
    }

    #[test]
    fn suffix_retry_reports_footer_plus_metadata() {
        let bytes = Bytes::copy_from_slice(&footer(28, *b"PAR1"));
        let mut reader = ParquetMetaDataReader::new();
        let error = reader.try_parse_sized(&bytes, 100).unwrap_err();
        assert!(matches!(error, ParquetError::NeedMoreData(36)));
        assert_eq!(reader.metadata_size(), Some(36));
    }

    #[test]
    fn declared_metadata_beyond_original_file_is_eof() {
        let bytes = Bytes::copy_from_slice(&footer(100, *b"PAR1"));
        let mut reader = ParquetMetaDataReader::new();
        let error = reader.try_parse_sized(&bytes, 50).unwrap_err();
        assert!(matches!(error, ParquetError::EOF(_)));
        assert_eq!(
            error.to_string(),
            "EOF: Parquet file too small. Size is 50 but need 108"
        );
        assert_eq!(reader.metadata_size(), Some(108));
    }
}
