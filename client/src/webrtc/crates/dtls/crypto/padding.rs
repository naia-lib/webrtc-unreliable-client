use block_modes::block_padding::{PadError, Padding, UnpadError};

pub(crate) enum DtlsPadding {}
/// Reference: RFC5246, 6.2.3.2
impl Padding for DtlsPadding {
    fn pad_block(block: &mut [u8], pos: usize) -> Result<(), PadError> {
        if pos == block.len() {
            return Err(PadError);
        }

        let padding_length = block.len() - pos - 1;
        if padding_length > 255 {
            return Err(PadError);
        }

        set(&mut block[pos..], padding_length as u8);

        Ok(())
    }

    fn unpad(data: &[u8]) -> Result<&[u8], UnpadError> {
        let padding_length = data.last().copied().unwrap_or(1) as usize;
        if padding_length + 1 > data.len() {
            return Err(UnpadError);
        }

        let padding_begin = data.len() - padding_length - 1;

        if data[padding_begin..data.len() - 1]
            .iter()
            .any(|&byte| byte as usize != padding_length)
        {
            return Err(UnpadError);
        }

        Ok(&data[0..padding_begin])
    }
}

/// Sets all bytes in `dst` equal to `value`
#[inline(always)]
fn set(dst: &mut [u8], value: u8) {
    // SAFETY: we overwrite valid memory behind `dst`
    // note: loop is not used here because it produces
    // unnecessary branch which tests for zero-length slices
    unsafe {
        core::ptr::write_bytes(dst.as_mut_ptr(), value, dst.len());
    }
}
