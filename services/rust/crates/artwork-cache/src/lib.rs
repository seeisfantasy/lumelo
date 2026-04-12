use std::path::PathBuf;

pub fn bucket_segments(cover_ref_id: &str) -> (String, String) {
    let mut padded = cover_ref_id.to_owned();
    while padded.len() < 4 {
        padded.push('0');
    }

    (padded[0..2].to_string(), padded[2..4].to_string())
}

pub fn thumb_path(root: &str, cover_ref_id: &str) -> PathBuf {
    let (first, second) = bucket_segments(cover_ref_id);

    PathBuf::from(root)
        .join("thumb")
        .join("320")
        .join(first)
        .join(second)
        .join(format!("{cover_ref_id}.jpg"))
}

pub fn source_path(root: &str, cover_ref_id: &str, extension: &str) -> PathBuf {
    let (first, second) = bucket_segments(cover_ref_id);
    let extension = extension.trim_start_matches('.').to_ascii_lowercase();

    PathBuf::from(root)
        .join("source")
        .join(first)
        .join(second)
        .join(format!("{cover_ref_id}.{extension}"))
}

#[cfg(test)]
mod tests {
    use super::{bucket_segments, source_path, thumb_path};

    #[test]
    fn buckets_are_zero_padded() {
        assert_eq!(bucket_segments("a"), ("a0".to_string(), "00".to_string()));
    }

    #[test]
    fn thumb_path_uses_expected_layout() {
        let path = thumb_path("/var/cache/lumelo/artwork", "a1b2c3");
        assert_eq!(
            path.to_string_lossy(),
            "/var/cache/lumelo/artwork/thumb/320/a1/b2/a1b2c3.jpg"
        );
    }

    #[test]
    fn source_path_uses_expected_layout() {
        let path = source_path("/var/cache/lumelo/artwork", "a1b2c3", "PNG");
        assert_eq!(
            path.to_string_lossy(),
            "/var/cache/lumelo/artwork/source/a1/b2/a1b2c3.png"
        );
    }
}
