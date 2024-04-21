fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("../encryptedfs_desktop_common/proto/encryptedfs_desktop.proto")?;
    println!("cargo:rerun-if-changed=../encryptedfs_desktop_common/migrations");
    Ok(())
}
