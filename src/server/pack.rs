use crate::{Result, ServerError};
use async_tempfile::TempFile;
use tokio::fs::read;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tracing::instrument;

#[instrument(skip(data))]
pub async fn pack(map: &str, data: &[u8]) -> Result<Vec<u8>> {
    let mut input = TempFile::new_with_name(map.to_string()).await?;
    let output = TempFile::new_with_name(format!("out_{map}")).await?;

    input
        .write_all(data)
        .await
        .map_err(|err| ServerError::File {
            path: input.file_path().into(),
            err,
        })?;

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
        .await
        .map_err(|err| ServerError::Pack {
            err,
            input: input.file_path().into(),
            output: output.file_path().into(),
            binary: pack_cmd.into(),
        })?;

    if !out.status.success() {
        return Err(ServerError::GltfPack(
            String::from_utf8_lossy(&out.stderr).into(),
        ));
    }

    read(output.file_path())
        .await
        .map_err(|err| ServerError::File {
            path: output.file_path().into(),
            err,
        })
}
