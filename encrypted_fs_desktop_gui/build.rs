fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("../encrypted_fs_desktop_common/proto/encrypted_fs_desktop.proto")?;
    Ok(())
}
