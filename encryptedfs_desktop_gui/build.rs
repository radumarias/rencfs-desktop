fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("../encryptedfs_desktop_common/proto/encryptedfs_desktop.proto")?;
    Ok(())
}
