use age::{
    secrecy::ExposeSecret,
    x25519::{Identity, Recipient},
    Decryptor, Encryptor,
};
use std::io::{Read, Write};
use std::path::Path;

/// Generate a new age keypair.
/// Returns the public key string (bech32-encoded recipient) and the secret key string.
pub fn generate_key() -> Result<(String, String), anyhow::Error> {
    let identity = Identity::generate();
    let recipient = identity.to_public();

    let secret_str = identity.to_string().expose_secret().clone();
    let public_str = recipient.to_string();

    Ok((public_str, secret_str))
}

/// Parse a secret key (Identity) from a string.
pub fn parse_secret_key(key_str: &str) -> Result<Identity, anyhow::Error> {
    // Clean up: trim whitespace, skip comment lines, take first non-empty line
    let cleaned = key_str
        .lines()
        .map(|l| l.trim())
        .find(|l| !l.is_empty() && !l.starts_with('#'))
        .unwrap_or("");

    if cleaned.is_empty() {
        anyhow::bail!("No key found. Paste your AGE-SECRET-KEY-1... here.");
    }

    if !cleaned.starts_with("AGE-SECRET-KEY-1") {
        anyhow::bail!(
            "Key must start with AGE-SECRET-KEY-1. Make sure you are pasting the secret key, not the public key."
        );
    }

    cleaned
        .parse::<Identity>()
        .map_err(|e| anyhow::anyhow!("Invalid key: {}", e))
}

/// Load a secret key from a file.
pub fn load_secret_key(key_path: &Path) -> Result<Identity, anyhow::Error> {
    let content = std::fs::read_to_string(key_path)?;
    parse_secret_key(content.trim())
}

/// Save a secret key to a file with restricted permissions.
pub fn save_secret_key(key_path: &Path, key_str: &str) -> Result<(), anyhow::Error> {
    if let Some(parent) = key_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::write(key_path, key_str)?;
        let mut perms = std::fs::metadata(key_path)?.permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(key_path, perms)?;
    }

    #[cfg(not(unix))]
    {
        std::fs::write(key_path, key_str)?;
    }

    Ok(())
}

/// Encrypt plaintext bytes using the given public key (bech32 recipient string).
/// Returns the encrypted bytes in ASCII-armoured format.
pub fn encrypt(public_key: &str, plaintext: &[u8]) -> Result<Vec<u8>, anyhow::Error> {
    // Ensure the `age` binary path is known to the `age` crate.
    // The crate respects the `AGE_BINARY` environment variable.
    if let Ok(age_path) = crate::util::locate_age() {
        std::env::set_var("AGE_BINARY", age_path);
    }

    let recipient: Recipient = public_key
        .parse()
        .map_err(|e: &'static str| anyhow::anyhow!("Invalid public key: {}", e))?;

    let encryptor = Encryptor::with_recipients(vec![Box::new(recipient)])
        .ok_or_else(|| anyhow::anyhow!("no recipients"))?;

    let mut encrypted = Vec::new();
    let mut writer = encryptor.wrap_output(&mut encrypted)?;
    writer.write_all(plaintext)?;
    writer.finish()?;

    Ok(encrypted)
}

/// Decrypt ASCII-armoured ciphertext using the given secret key (Identity).
/// Returns the decrypted plaintext bytes.
pub fn decrypt(identity: &Identity, ciphertext: &[u8]) -> Result<Vec<u8>, anyhow::Error> {
    // Ensure the `age` binary path is known to the `age` crate.
    if let Ok(age_path) = crate::util::locate_age() {
        std::env::set_var("AGE_BINARY", age_path);
    }

    let decryptor = match Decryptor::new(ciphertext)? {
        Decryptor::Recipients(d) => d,
        _ => anyhow::bail!("unsupported encryption type (expected public-key encryption)"),
    };

    let mut plaintext = Vec::new();
    let mut reader = decryptor.decrypt(std::iter::once(identity as &dyn age::Identity))?;
    reader.read_to_end(&mut plaintext)?;

    Ok(plaintext)
}

/// Encrypt a file and write the encrypted output to a destination path.
#[allow(dead_code)]
pub fn encrypt_file(public_key: &str, source: &Path, dest: &Path) -> Result<(), anyhow::Error> {
    let plaintext = std::fs::read(source)?;
    let encrypted = encrypt(public_key, &plaintext)?;
    std::fs::write(dest, encrypted)?;
    Ok(())
}

/// Decrypt a file and write the plaintext output to a destination path.
#[allow(dead_code)]
pub fn decrypt_file(identity: &Identity, source: &Path, dest: &Path) -> Result<(), anyhow::Error> {
    let ciphertext = std::fs::read(source)?;
    let plaintext = decrypt(identity, &ciphertext)?;
    std::fs::write(dest, plaintext)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let (public_key, secret_str) = generate_key().unwrap();
        let identity = parse_secret_key(&secret_str).unwrap();

        let plaintext = b"hello, this is a secret note";
        let encrypted = encrypt(&public_key, plaintext).unwrap();

        assert_ne!(encrypted, plaintext);

        let header = String::from_utf8_lossy(&encrypted);
        assert!(header.starts_with("age-encryption.org"));

        let decrypted = decrypt(&identity, &encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_wrong_key_fails() {
        let (_, secret_str_a) = generate_key().unwrap();
        let (_, secret_str_b) = generate_key().unwrap();

        let identity_a = parse_secret_key(&secret_str_a).unwrap();
        let identity_b = parse_secret_key(&secret_str_b).unwrap();

        let public_a = identity_a.to_public().to_string();
        let plaintext = b"secret data";
        let encrypted = encrypt(&public_a, plaintext).unwrap();

        let result = decrypt(&identity_b, &encrypted);
        assert!(result.is_err());
    }

    #[test]
    fn test_save_load_key() {
        let dir = std::env::temp_dir().join("nvage_test_keys");
        std::fs::create_dir_all(&dir).unwrap();
        let key_path = dir.join("key.txt");

        let (public_key, secret_str) = generate_key().unwrap();
        save_secret_key(&key_path, &secret_str).unwrap();

        let loaded = load_secret_key(&key_path).unwrap();
        let loaded_public = loaded.to_public().to_string();
        assert_eq!(loaded_public, public_key);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_security_no_plaintext_in_sync_destination() {
        let (public_key, secret_str) = generate_key().unwrap();
        let identity = parse_secret_key(&secret_str).unwrap();

        let notes = vec![
            (
                "note-1.md.age",
                "---\nid: a3f7b2c1-9e4d-4b8a-b6f2-1c3d5e7f9a0b\ncreated: 2026-03-29T10:15:00Z\n---\n# Meeting notes\n\nDiscussed the new feature rollout plan.\nAction items:\n- Update the docs\n- Email the team\n",
            ),
            (
                "note-2.md.age",
                "---\nid: b4e8c3d2-0f5e-5c9b-c7g3-2d4e6f8g0b1c\ncreated: 2026-03-30T14:00:00Z\n---\n# Shopping list\n\n- Milk\n- Eggs\n- Bread\n- Coffee beans\n",
            ),
            (
                "note-3.md.age",
                "---\nid: c5f9d4e3-1g6f-6d0c-d8h4-3e5f7g9h1c2d\ncreated: 2026-04-01T09:30:00Z\n---\n# Project ideas\n\nWhat if we added encrypted sync? Users could share notes between devices without anyone else reading them.\n\nKey requirements:\n- age encryption\n- Git as transport\n- No plaintext ever leaves the device\n",
            ),
        ];

        let sync_dir = std::env::temp_dir().join("nvage_sync_test");
        let _ = std::fs::remove_dir_all(&sync_dir);
        std::fs::create_dir_all(&sync_dir).unwrap();

        for (filename, content) in &notes {
            let encrypted = encrypt(&public_key, content.as_bytes()).unwrap();
            let dest = sync_dir.join(filename);
            std::fs::write(&dest, encrypted).unwrap();
        }

        for entry in std::fs::read_dir(&sync_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let data = std::fs::read(&path).unwrap();
            let text = String::from_utf8_lossy(&data);

            assert!(
                text.starts_with("age-encryption.org"),
                "File {:?} does not look like an age-encrypted file",
                path
            );

            let decrypted = decrypt(&identity, &data)
                .expect("Failed to decrypt file — key mismatch or corruption");

            let original_note = notes
                .iter()
                .find(|(name, _)| path.file_name().unwrap() == *name)
                .unwrap();
            assert_eq!(
                String::from_utf8_lossy(&decrypted),
                original_note.1,
                "Decrypted content does not match original for {:?}",
                path
            );

            for (_, original_content) in &notes {
                let distinctive_phrases = vec![
                    "Meeting notes",
                    "Shopping list",
                    "Project ideas",
                    "Milk",
                    "Coffee beans",
                    "encrypted sync",
                    "age encryption",
                ];
                for phrase in distinctive_phrases {
                    assert!(
                        !text.contains(phrase),
                        "Plaintext phrase '{}' found in encrypted file {:?}",
                        phrase,
                        path
                    );
                }
            }
        }

        std::fs::remove_dir_all(&sync_dir).ok();
    }
}
