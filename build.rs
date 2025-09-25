use spirv_builder::{MetadataPrintout, SprivBuilder};

fn main() -> Result<(), Box<dyn std::error::Error>>{
    SprivBuilder::new("shaders", "spirv-unknown-opengl4.5").print_metadata(MetadataPrintout::Full).build()?;
    Ok(())
}
