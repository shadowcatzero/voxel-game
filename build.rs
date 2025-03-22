use spirv_builder::{Capability, MetadataPrintout, SpirvBuilder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    SpirvBuilder::new("./shader", "spirv-unknown-spv1.5")
        .capability(Capability::ImageQuery)
        .print_metadata(MetadataPrintout::Full)
        .build()?;
    Ok(())
}
