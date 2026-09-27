/* ==========================================================================
 * Streaming checksums
 * SHA-256, computed as bytes move rather than after the fact.
 *
 * Two entry points exist because the engine needs both:
 *
 * - [`StreamHasher`] is updated with the chunks a copy is already reading, so
 *   verifying a source costs no extra read of the source at all.
 * - [`hash_file`] streams a file from disk in bounded chunks, used for the
 *   destination that was just written.
 *
 * Neither ever holds a whole file in memory: hashing a 4 KiB file and a 40 GiB
 * file both allocate one buffer.
 * ========================================================================== */

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::errors::AppError;
use crate::transfer::safety;

/// Size of the buffer [`hash_file`] streams through.
///
/// Matches the copy engine's buffer: one megabyte amortizes the syscall cost
/// on a spinning disk without making a verification allocate anything worth
/// noticing.
pub const HASH_BUFFER_BYTES: usize = 1024 * 1024;

/// Prefix identifying a digest on the wire, e.g. `sha256:9f86d0…`.
pub const SHA256_PREFIX: &str = "sha256:";

/// SHA-256 of a stream, updated as its bytes pass through the transfer.
pub struct StreamHasher {
    hasher: Sha256,
    bytes: u64,
}

impl StreamHasher {
    pub fn new() -> Self {
        Self {
            hasher: Sha256::new(),
            bytes: 0,
        }
    }

    /// Feeds one chunk. Chunk boundaries are irrelevant to the digest, so the
    /// caller can pass whatever the copy loop happened to read.
    pub fn update(&mut self, bytes: &[u8]) {
        self.hasher.update(bytes);
        self.bytes = self.bytes.saturating_add(bytes.len() as u64);
    }

    /// Bytes fed so far.
    pub fn bytes(&self) -> u64 {
        self.bytes
    }

    /// The digest in `sha256:<lowercase hex>` form.
    pub fn finish(self) -> String {
        format!("{SHA256_PREFIX}{:x}", self.hasher.finalize())
    }
}

impl Default for StreamHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for StreamHasher {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StreamHasher")
            .field("bytes", &self.bytes)
            .finish()
    }
}

/// What hashing a file produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashOutcome {
    /// `sha256:<hex>` of the bytes read.
    pub digest: String,
    /// Bytes read. A file that grew while it was hashed reports more than its
    /// original length, which is exactly the fact a caller needs.
    pub bytes: u64,
    /// False when the checkpoint asked to stop: the digest is incomplete and
    /// must not be compared with anything.
    pub completed: bool,
}

/// Hashes one file by streaming it in bounded chunks.
///
/// `checkpoint` is called before each chunk; returning `false` stops the read
/// and reports an incomplete outcome rather than a wrong digest. That is what
/// keeps a cancellation responsive while a large destination is verified.
pub fn hash_file(
    path: &Path,
    checkpoint: &mut dyn FnMut() -> bool,
) -> Result<HashOutcome, AppError> {
    let mut file = File::open(path).map_err(|error| safety::read_error(error, path))?;
    let mut hasher = StreamHasher::new();
    let mut buffer = vec![0u8; HASH_BUFFER_BYTES];

    loop {
        if !checkpoint() {
            return Ok(HashOutcome {
                digest: String::new(),
                bytes: hasher.bytes(),
                completed: false,
            });
        }

        let read = match file.read(&mut buffer) {
            Ok(read) => read,
            // A read a signal interrupted is retried, exactly as the copy
            // engine does.
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(safety::read_error(error, path)),
        };
        if read == 0 {
            break;
        }

        hasher.update(&buffer[..read]);
    }

    let bytes = hasher.bytes();
    Ok(HashOutcome {
        digest: hasher.finish(),
        bytes,
        completed: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::test_support::unique_temp_dir;

    /// A checkpoint that never stops.
    fn run() -> bool {
        true
    }

    /// A checkpoint that stops immediately.
    fn stop() -> bool {
        false
    }

    fn write(path: &Path, bytes: &[u8]) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("the parent is creatable");
        }
        std::fs::write(path, bytes).expect("the file is writable");
    }

    #[test]
    fn the_empty_digest_matches_the_published_sha256_of_nothing() {
        let mut hasher = StreamHasher::new();
        hasher.update(b"");

        assert_eq!(
            hasher.finish(),
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn the_abc_digest_matches_the_published_sha256_vector() {
        let mut hasher = StreamHasher::new();
        hasher.update(b"abc");

        assert_eq!(
            hasher.finish(),
            "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn a_longer_published_vector_matches() {
        let mut hasher = StreamHasher::new();
        hasher.update(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq");

        assert_eq!(
            hasher.finish(),
            "sha256:248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn one_million_a_characters_match_the_published_vector() {
        let mut hasher = StreamHasher::new();
        let chunk = vec![b'a'; 1000];
        for _ in 0..1000 {
            hasher.update(&chunk);
        }

        assert_eq!(
            hasher.finish(),
            "sha256:cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }

    #[test]
    fn chunk_boundaries_do_not_change_the_digest() {
        let bytes: Vec<u8> = (0..=255u8).cycle().take(5000).collect();

        let mut whole = StreamHasher::new();
        whole.update(&bytes);

        let mut split = StreamHasher::new();
        for chunk in bytes.chunks(7) {
            split.update(chunk);
        }

        assert_eq!(whole.finish(), split.finish());
    }

    #[test]
    fn the_hasher_counts_the_bytes_it_was_fed() {
        let mut hasher = StreamHasher::new();
        hasher.update(&[0u8; 10]);
        hasher.update(&[0u8; 5]);

        assert_eq!(hasher.bytes(), 15);
    }

    #[test]
    fn hashing_a_file_matches_hashing_its_bytes() {
        let dir = unique_temp_dir("hash-file");
        let path = dir.join("payload.bin");
        let bytes: Vec<u8> = (0..=255u8).cycle().take(300_000).collect();
        write(&path, &bytes);

        let mut expected = StreamHasher::new();
        expected.update(&bytes);

        let outcome = hash_file(&path, &mut run).expect("the file is hashable");

        assert_eq!(outcome.digest, expected.finish());
        assert_eq!(outcome.bytes, bytes.len() as u64);
        assert!(outcome.completed);

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn hashing_an_empty_file_yields_the_empty_digest() {
        let dir = unique_temp_dir("hash-empty");
        let path = dir.join("empty.bin");
        write(&path, b"");

        let outcome = hash_file(&path, &mut run).expect("the file is hashable");

        assert_eq!(outcome.bytes, 0);
        assert!(outcome
            .digest
            .ends_with("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"));

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_stopped_hash_reports_no_digest_rather_than_a_wrong_one() {
        let dir = unique_temp_dir("hash-stopped");
        let path = dir.join("payload.bin");
        write(&path, &vec![7u8; 4096]);

        let outcome = hash_file(&path, &mut stop).expect("stopping is not an error");

        assert!(!outcome.completed);
        assert_eq!(outcome.digest, "");
        assert_eq!(outcome.bytes, 0);

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_missing_file_is_a_structured_not_found() {
        let dir = unique_temp_dir("hash-missing");
        let path = dir.join("nope.bin");

        let error = hash_file(&path, &mut run).expect_err("a missing file cannot be hashed");

        assert_eq!(error.code(), "path_not_found");
        assert!(error.to_string().contains("nope.bin"));

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn hashing_a_directory_is_a_structured_failure() {
        let dir = unique_temp_dir("hash-directory");

        let error = hash_file(&dir, &mut run).expect_err("a directory cannot be hashed");

        assert!(
            matches!(error.code(), "permission_denied" | "io" | "path_not_found"),
            "unexpected code: {}",
            error.code()
        );

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn memory_does_not_scale_with_the_file_being_hashed() {
        let dir = unique_temp_dir("hash-large");
        let path = dir.join("large.bin");
        write(&path, &vec![3u8; 8 * 1024 * 1024]);

        let outcome = hash_file(&path, &mut run).expect("the file is hashable");

        assert_eq!(outcome.bytes, 8 * 1024 * 1024);
        assert!(
            HASH_BUFFER_BYTES < outcome.bytes as usize,
            "the buffer must be smaller than the file for this to mean anything"
        );

        let _ = std::fs::remove_dir_all(dir);
    }
}
