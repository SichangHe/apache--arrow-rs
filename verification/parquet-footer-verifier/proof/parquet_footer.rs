use vstd::prelude::*;

verus! {

pub const FOOTER_SIZE: u64 = 8;

pub struct FooterBytes {
    pub len0: u8,
    pub len1: u8,
    pub len2: u8,
    pub len3: u8,
    pub magic0: u8,
    pub magic1: u8,
    pub magic2: u8,
    pub magic3: u8,
}

pub struct MetadataRange {
    pub start: u64,
    pub end: u64,
    pub metadata_len: u64,
    pub encrypted: bool,
}

pub enum FooterError {
    Magic,
    FileRange,
}

pub open spec fn metadata_len_spec(len0: u8, len1: u8, len2: u8, len3: u8) -> nat {
    len0 as nat
        + (len1 as nat) * 256
        + (len2 as nat) * 65_536
        + (len3 as nat) * 16_777_216
}

pub open spec fn plain_magic_spec(magic0: u8, magic1: u8, magic2: u8, magic3: u8) -> bool {
    magic0 == 80 && magic1 == 65 && magic2 == 82 && magic3 == 49
}

pub open spec fn encrypted_magic_spec(
    magic0: u8,
    magic1: u8,
    magic2: u8,
    magic3: u8,
) -> bool {
    magic0 == 80 && magic1 == 65 && magic2 == 82 && magic3 == 69
}

pub open spec fn accepted_magic_spec(
    magic0: u8,
    magic1: u8,
    magic2: u8,
    magic3: u8,
) -> bool {
    plain_magic_spec(magic0, magic1, magic2, magic3)
        || encrypted_magic_spec(magic0, magic1, magic2, magic3)
}

fn decode_metadata_len(len0: u8, len1: u8, len2: u8, len3: u8) -> (result: u64)
    ensures
        result as nat == metadata_len_spec(len0, len1, len2, len3),
{
    len0 as u64
        + (len1 as u64) * 256
        + (len2 as u64) * 65_536
        + (len3 as u64) * 16_777_216
}

fn magic_kind(magic0: u8, magic1: u8, magic2: u8, magic3: u8) -> (result: Option<bool>)
    ensures
        match result {
            Some(false) => plain_magic_spec(magic0, magic1, magic2, magic3),
            Some(true) => encrypted_magic_spec(magic0, magic1, magic2, magic3),
            None => !accepted_magic_spec(magic0, magic1, magic2, magic3),
        },
{
    if magic0 == 80 && magic1 == 65 && magic2 == 82 && magic3 == 49 {
        Some(false)
    } else if magic0 == 80 && magic1 == 65 && magic2 == 82 && magic3 == 69 {
        Some(true)
    } else {
        None
    }
}

fn validate_footer(file_len: u64, footer: FooterBytes) -> (result: Result<MetadataRange, FooterError>)
    ensures
        match result {
            Ok(range) => {
                &&& accepted_magic_spec(
                    footer.magic0,
                    footer.magic1,
                    footer.magic2,
                    footer.magic3,
                )
                &&& range.metadata_len as nat == metadata_len_spec(
                    footer.len0,
                    footer.len1,
                    footer.len2,
                    footer.len3,
                )
                &&& range.start as nat + range.metadata_len as nat == range.end as nat
                &&& range.end as nat + FOOTER_SIZE as nat == file_len as nat
                &&& range.end <= file_len
                &&& range.encrypted == encrypted_magic_spec(
                    footer.magic0,
                    footer.magic1,
                    footer.magic2,
                    footer.magic3,
                )
            },
            Err(FooterError::Magic) => !accepted_magic_spec(
                footer.magic0,
                footer.magic1,
                footer.magic2,
                footer.magic3,
            ),
            Err(FooterError::FileRange) => {
                &&& accepted_magic_spec(
                    footer.magic0,
                    footer.magic1,
                    footer.magic2,
                    footer.magic3,
                )
                &&& (file_len as nat) < (FOOTER_SIZE as nat) + metadata_len_spec(
                    footer.len0,
                    footer.len1,
                    footer.len2,
                    footer.len3,
                )
            },
        },
{
    let encrypted = match magic_kind(footer.magic0, footer.magic1, footer.magic2, footer.magic3) {
        Some(encrypted) => encrypted,
        None => return Err(FooterError::Magic),
    };
    let metadata_len = decode_metadata_len(footer.len0, footer.len1, footer.len2, footer.len3);
    let total = metadata_len + FOOTER_SIZE;
    if total > file_len {
        return Err(FooterError::FileRange);
    }
    let start = file_len - total;
    let end = start + metadata_len;
    Ok(MetadataRange {
        start,
        end,
        metadata_len,
        encrypted,
    })
}

fn main() {
    let footer = FooterBytes {
        len0: 28,
        len1: 0,
        len2: 0,
        len3: 0,
        magic0: 80,
        magic1: 65,
        magic2: 82,
        magic3: 49,
    };
    match validate_footer(100, footer) {
        Ok(range) => {
            assert(range.start == 64);
            assert(range.end == 92);
            assert(range.metadata_len == 28);
            assert(!range.encrypted);
        },
        Err(_) => assert(false),
    }

    let bad_magic = FooterBytes {
        len0: 28,
        len1: 0,
        len2: 0,
        len3: 0,
        magic0: 80,
        magic1: 65,
        magic2: 82,
        magic3: 0,
    };
    match validate_footer(100, bad_magic) {
        Err(FooterError::Magic) => {},
        _ => assert(false),
    }

    let oversized = FooterBytes {
        len0: 28,
        len1: 0,
        len2: 0,
        len3: 0,
        magic0: 80,
        magic1: 65,
        magic2: 82,
        magic3: 49,
    };
    match validate_footer(35, oversized) {
        Err(FooterError::FileRange) => {},
        _ => assert(false),
    }
}

}
