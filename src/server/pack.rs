use crate::{Result, ServerError};
use async_tempfile::TempFile;
use tokio::fs::read;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

pub async fn pack(map: &str, data: &[u8]) -> Result<Vec<u8>> {
    let mut input = TempFile::new_with_name(map.to_string()).await?;
    let output = TempFile::new_with_name(format!("out_{map}")).await?;

    input.write_all(data).await?;

    let pack_cmd = option_env!("GLTFPACK").unwrap_or("gltfpack");

    let out = Command::new(pack_cmd)
        .arg("-kn")
        .arg("-mm")
        .arg("-tc")
        .arg("-i")
        .arg(input.file_path())
        .arg("-o")
        .arg(output.file_path())
        .output()
        .await?;

    if !out.status.success() {
        return Err(ServerError::GltfPack(
            String::from_utf8_lossy(&out.stderr).into(),
        ));
    }

    Ok(read(output.file_path()).await?)
}
