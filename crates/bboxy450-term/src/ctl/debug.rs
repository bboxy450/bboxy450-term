use super::*;

pub(super) fn read(
    client: &mut Client,
    request: bbt_proto::gui::DebugRequest,
    path: Option<&Path>,
) -> Result<Value, String> {
    let data = client.debug(request)?;
    let Some(path) = path else { return Ok(data) };
    let mut bytes =
        serde_json::to_vec_pretty(&data).map_err(|_| "debug snapshot serialization failed")?;
    bytes.push(b'\n');
    use std::os::unix::fs::OpenOptionsExt;
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
    {
        Ok(mut file) => file
            .write_all(&bytes)
            .map_err(|_| "debug snapshot could not be written")?,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            // A retry can reuse its own identical artifact; never overwrite or
            // follow an existing symlink, including one to a key/capability file.
            let metadata = std::fs::symlink_metadata(path)
                .map_err(|_| "debug snapshot destination unavailable")?;
            if !metadata.is_file()
                || metadata.file_type().is_symlink()
                || metadata.len() != bytes.len() as u64
            {
                return Err(
                    "debug snapshot destination exists with different content; choose a new file"
                        .into(),
                );
            }
            let mut existing = Vec::new();
            std::fs::OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NOFOLLOW)
                .open(path)
                .and_then(|file| file.take(bytes.len() as u64 + 1).read_to_end(&mut existing))
                .map_err(|_| "debug snapshot destination unavailable")?;
            if existing != bytes {
                return Err(
                    "debug snapshot destination exists with different content; choose a new file"
                        .into(),
                );
            }
        }
        Err(_) => {
            return Err(
                "debug snapshot destination unavailable; choose a writable new file".into(),
            );
        }
    }
    Ok(
        json!({"path":path,"bytes":bytes.len(),"sampled_at_ms":data["sampled_at_ms"],
        "seats":data["seats"].as_array().map_or(0,Vec::len),"omitted_seats":data["omitted_seats"]}),
    )
}
