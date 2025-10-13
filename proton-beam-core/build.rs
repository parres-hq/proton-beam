use std::io::Result;

fn main() -> Result<()> {
    // Configure prost to derive additional traits
    let mut config = prost_build::Config::new();

    // Derive Eq for all messages (PartialEq is already derived by prost)
    config.type_attribute(".", "#[derive(Eq)]");

    // Compile the protobuf schema
    config.compile_protos(&["proto/nostr.proto"], &["proto/"])?;
    Ok(())
}
