
use super::*;

// https://tools.ietf.org/html/rfc8422
#[derive(Clone, Debug, PartialEq)]
pub struct ExtensionUseExtendedMasterSecret {
    pub supported: bool,
}

impl ExtensionUseExtendedMasterSecret {
    pub fn extension_value(&self) -> ExtensionValue {
        ExtensionValue::UseExtendedMasterSecret
    }

    pub fn size(&self) -> usize {
        2
    }

    pub fn marshal<W: Write>(&self, writer: &mut W) -> Result<()> {
        // length
        writer.write_u16::<BigEndian>(0)?;

        Ok(writer.flush()?)
    }

    pub fn unmarshal<R: Read>(reader: &mut R) -> Result<Self> {
        let _ = reader.read_u16::<BigEndian>()?;

        Ok(ExtensionUseExtendedMasterSecret { supported: true })
    }
}
