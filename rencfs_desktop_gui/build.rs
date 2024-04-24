fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("../rencfs_desktop_common/proto/rencfs_desktop.proto")?;
    Ok(())
}
