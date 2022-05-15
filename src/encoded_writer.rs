use std::io::Write;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum ByteOrder {
    BigEndian,
    LittleEndian,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum EncodedWriter<W> {
    Utf8 { writer: W },
    Utf16 { writer: W, byte_order: ByteOrder },
}

impl<W> EncodedWriter<W>
where
    W: Write,
{
    pub fn utf8(writer: W) -> Self {
        EncodedWriter::Utf8 { writer }
    }

    pub fn utf16(writer: W, byte_order: ByteOrder) -> Self {
        EncodedWriter::Utf16 { writer, byte_order }
    }

    pub fn write_all(&mut self, text: &str) -> std::io::Result<()> {
        match self {
            EncodedWriter::Utf8 { writer } => writer.write_all(text.as_bytes()),
            EncodedWriter::Utf16 { writer, byte_order } => {
                let encoded = text
                    .encode_utf16()
                    .flat_map(|codepoint| match byte_order {
                        ByteOrder::BigEndian => codepoint.to_be_bytes(),
                        ByteOrder::LittleEndian => codepoint.to_le_bytes(),
                    })
                    .collect::<Vec<u8>>();
                writer.write_all(&encoded)
            }
        }
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        match self {
            EncodedWriter::Utf8 { writer } => writer.flush(),
            EncodedWriter::Utf16 { writer, .. } => writer.flush(),
        }
    }
}
