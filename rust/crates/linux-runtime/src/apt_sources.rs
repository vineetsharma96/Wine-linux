use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use androidpc_runtime_api::RuntimeError;

const PORTS: &str = "https://ports.ubuntu.com/ubuntu-ports/";
type RepositoryTarget = (String, String, String, String);

fn ports(uri: &str) -> Option<&'static str> {
    let (scheme, location) = uri.split_once("://")?;
    if !matches!(scheme, "http" | "https") {
        return None;
    }
    match location.trim_end_matches('/') {
        "archive.ubuntu.com/ubuntu" | "security.ubuntu.com/ubuntu"
        | "ports.ubuntu.com/ubuntu-ports" => Some(PORTS),
        _ => None,
    }
}

fn malformed(message: &str) -> RuntimeError {
    RuntimeError::InitializationFailed(format!("Ubuntu APT sources: {message}"))
}

fn source_options(options: impl Iterator<Item = String>) -> String {
    let mut options: Vec<_> = options.collect();
    options.sort_unstable();
    options.join(" ")
}

// Match equivalent legacy records to the Deb822 entries Ubuntu prefers.
// Unknown fields or options stay independent so their policy is preserved.
fn deb822_targets(input: &str) -> HashSet<RepositoryTarget> {
    let mut targets = HashSet::new();
    for stanza in input.split("\n\n") {
        let fields: Option<HashMap<_, _>> = stanza.lines()
            .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
            .map(|line| line.split_once(':').map(|(name, value)| (name.to_ascii_lowercase(), value.trim())))
            .collect();
        let Some(fields) = fields else { continue };
        if fields.keys().any(|key| !matches!(
            key.as_str(), "types" | "uris" | "suites" | "components" | "signed-by" | "architectures" | "enabled"
        )) || fields.get("enabled").is_some_and(|value| *value != "yes")
            || !fields.get("uris").is_some_and(|value| value.split_ascii_whitespace().any(|uri| uri == PORTS))
        {
            continue;
        }
        let options = source_options([("signed-by", "signed-by"), ("architectures", "arch")]
            .into_iter()
            .filter_map(|(field, option)| fields.get(field).map(|value| {
                let value = value.split_ascii_whitespace().collect::<Vec<_>>().join(",");
                format!("{option}={value}")
            })));
        for kind in fields.get("types").unwrap_or(&"").split_ascii_whitespace() {
            for suite in fields.get("suites").unwrap_or(&"").split_ascii_whitespace() {
                for component in fields.get("components").unwrap_or(&"").split_ascii_whitespace() {
                    targets.insert((kind.to_owned(), suite.to_owned(), component.to_owned(), options.clone()));
                }
            }
        }
    }
    targets
}

fn legacy(input: &str, preferred: &HashSet<RepositoryTarget>) -> Result<String, RuntimeError> {
    let mut seen = HashSet::new();
    let mut output = String::new();
    for line in input.split_inclusive('\n') {
        let body = line.split('#').next().unwrap_or_default();
        let mut fields = body.split_ascii_whitespace();
        let Some(kind @ ("deb" | "deb-src")) = fields.next() else {
            output.push_str(line);
            continue;
        };
        let mut uri = fields.next().ok_or_else(|| malformed("missing URI in sources.list"))?;
        if uri.starts_with('[') {
            while !uri.ends_with(']') {
                uri = fields.next().ok_or_else(|| malformed("unterminated options in sources.list"))?;
            }
            uri = fields.next().ok_or_else(|| malformed("missing URI in sources.list"))?;
        }
        let Some(replacement) = ports(uri) else {
            output.push_str(line);
            continue;
        };
        let suite = fields.next().ok_or_else(|| malformed("missing suite in sources.list"))?;
        let offset = uri.as_ptr() as usize - line.as_ptr() as usize;
        let options = line[..offset].trim().strip_prefix(kind).unwrap_or_default().trim();
        let options = source_options(options.trim_matches(['[', ']']).split_ascii_whitespace().map(str::to_owned));
        let mut components: Vec<_> = fields.collect();
        let covered = !components.is_empty() && components.iter().all(|component| {
            preferred.contains(&(kind.to_owned(), suite.to_owned(), (*component).to_owned(), options.clone()))
        });
        let record = format!("{}{replacement}{}", &line[..offset], &line[offset + uri.len()..]);
        components.sort_unstable();
        if !covered && seen.insert((kind, options, suite, components)) {
            output.push_str(&record);
        } else if let Some((_, comment)) = record.split_once('#') {
            output.push('#');
            output.push_str(comment);
        }
    }
    Ok(output)
}

fn deb822(input: &str) -> Result<String, RuntimeError> {
    let mut output = String::new();
    let mut seen_records = HashSet::new();
    let mut lines = input.split_inclusive('\n').peekable();
    while let Some(line) = lines.next() {
        if line.trim().is_empty() {
            output.push_str(line);
            continue;
        }
        let mut stanza = vec![line];
        while lines.peek().is_some_and(|line| !line.trim().is_empty()) {
            stanza.push(lines.next().unwrap());
        }
        let mut uri_fields = 0;
        let mut in_uris = false;
        let mut seen_ports = false;
        let mut other_uri = false;
        let mut suites = false;
        let mut types = false;
        let mut record = String::new();
        let mut comments = String::new();
        for line in stanza {
            if line.starts_with('#') {
                record.push_str(line);
                comments.push_str(line);
                continue;
            }
            let continuation = line.starts_with([' ', '\t']);
            let (prefix, value) = if continuation {
                ("", line)
            } else if let Some((field, value)) = line.split_once(':') {
                in_uris = field.eq_ignore_ascii_case("URIs");
                uri_fields += usize::from(in_uris);
                suites |= field.eq_ignore_ascii_case("Suites");
                types |= field.eq_ignore_ascii_case("Types");
                (&line[..field.len() + 1], value)
            } else {
                return Err(malformed("invalid Deb822 field"));
            };
            if !in_uris || !value.split_ascii_whitespace().any(|uri| ports(uri).is_some()) {
                other_uri |= in_uris && !value.trim().is_empty();
                record.push_str(line);
                continue;
            }
            let mut at = 0;
            let mut changed = String::new();
            for uri in value.split_ascii_whitespace() {
                let start = uri.as_ptr() as usize - value.as_ptr() as usize;
                if let Some(canonical) = ports(uri) {
                    if !seen_ports {
                        changed.push_str(&value[at..start]);
                        changed.push_str(canonical);
                    }
                    seen_ports = true;
                } else {
                    other_uri = true;
                    changed.push_str(&value[at..start]);
                    changed.push_str(uri);
                }
                at = start + uri.len();
            }
            if !continuation || !changed.is_empty() {
                record.push_str(prefix);
                record.push_str(&changed);
                record.push_str(&value[at..]);
            }
        }
        if seen_ports && (uri_fields != 1 || !suites || !types) {
            return Err(malformed("incomplete or repeated Deb822 fields"));
        }
        let fields = record.lines().filter(|line| !line.starts_with('#')).collect::<Vec<_>>().join("\n");
        if !seen_ports || other_uri || seen_records.insert(fields) {
            output.push_str(&record);
        } else {
            output.push_str(&comments);
        }
    }
    Ok(output)
}

fn atomic_write(path: &Path, bytes: &[u8], permissions: fs::Permissions) -> io::Result<()> {
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let suffix = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let tmp = path.with_extension(format!("androidpc-{}-{suffix}.tmp", std::process::id()));
    let mut writer = OpenOptions::new().write(true).create_new(true).open(&tmp)?;
    let result = (|| {
        writer.set_permissions(permissions)?;
        writer.write_all(bytes)?;
        writer.sync_all()?;
        drop(writer);
        fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(tmp);
    }
    result
}

#[cfg(target_os = "android")]
fn ensure_guest_tls_roots(etc: &Path) -> Result<(), RuntimeError> {
    use std::os::unix::fs::PermissionsExt;

    // Ubuntu Base omits ca-certificates. Seed the standard location so the
    // guest can fetch its own ca-certificates package over authenticated TLS.
    let mut certs = etc.to_path_buf();
    for part in ["ssl", "certs"] {
        certs.push(part);
        match fs::symlink_metadata(&certs) {
            Ok(metadata) if metadata.is_dir() => {}
            Ok(_) => return Err(malformed("unsafe certificate directory")),
            Err(error) if error.kind() == io::ErrorKind::NotFound => fs::create_dir(&certs)?,
            Err(error) => return Err(error.into()),
        }
    }
    let bundle = certs.join("ca-certificates.crt");
    match fs::symlink_metadata(&bundle) {
        Ok(metadata) if metadata.is_file() && metadata.len() > 0 => return Ok(()),
        Ok(_) => return Err(malformed("invalid existing TLS certificate bundle")),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    for root in ["/apex/com.android.conscrypt/cacerts", "/system/etc/security/cacerts"] {
        let entries = match fs::read_dir(root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        let mut pem = Vec::new();
        for entry in entries {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let bytes = fs::read(entry.path())?;
                if bytes.starts_with(b"-----BEGIN CERTIFICATE-----") {
                    pem.extend(bytes);
                    pem.push(b'\n');
                }
            }
        }
        if !pem.is_empty() {
            atomic_write(&bundle, &pem, fs::Permissions::from_mode(0o644))?;
            return Ok(());
        }
    }
    Err(malformed("Android system TLS certificates were unavailable"))
}

pub(crate) fn configure_ports(root: &Path) -> Result<(), RuntimeError> {
    let etc = root.join("etc");
    let apt = etc.join("apt");
    let snippets = apt.join("sources.list.d");
    for dir in [root, &etc, &apt, &snippets] {
        match fs::symlink_metadata(dir) {
            Ok(metadata) if metadata.is_dir() => {}
            Err(error) if dir == snippets && error.kind() == io::ErrorKind::NotFound => {}
            _ => return Err(malformed("expected a real guest APT directory")),
        }
    }

    let candidates = [(snippets.join("ubuntu.sources"), true), (apt.join("sources.list"), false)];
    let mut files = Vec::new();
    let mut preferred = HashSet::new();
    for (path, is_deb822) in candidates {
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_file() => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            _ => return Err(malformed("expected a regular source file")),
        };
        let original = fs::read_to_string(&path)?;
        let new = if is_deb822 {
            let source = deb822(&original)?;
            preferred = deb822_targets(&source.replace("\r\n", "\n"));
            source
        } else {
            legacy(&original, &preferred)?
        };
        let backup = path.with_file_name(format!(
            "{}.androidpc-original", path.file_name().unwrap().to_string_lossy()
        ));
        let has_backup = match fs::symlink_metadata(&backup) {
            Ok(metadata) if metadata.is_file() => true,
            Err(error) if error.kind() == io::ErrorKind::NotFound => false,
            _ => return Err(malformed("unsafe backup path")),
        };
        files.push((path, backup, has_backup, metadata.permissions(), original, new));
    }
    let has_entries = files.iter().any(|(_, _, _, _, _, source)| {
        source.lines().any(|line| !line.trim().is_empty() && !line.trim_start().starts_with('#'))
    });
    if !has_entries {
        let mut has_custom_sources = false;
        if snippets.is_dir() {
            for entry in fs::read_dir(&snippets)? {
                let path = entry?.path();
                if path.file_name().is_some_and(|name| name != "ubuntu.sources")
                    && matches!(path.extension().and_then(|extension| extension.to_str()), Some("list" | "sources"))
                {
                    has_custom_sources = true;
                    break;
                }
            }
        }
        if !has_custom_sources {
            return Err(malformed("no source entries found; restore the guest's source configuration"));
        }
    }
    #[cfg(target_os = "android")]
    if files.iter().any(|(_, _, _, _, _, source)| source.contains(PORTS)) {
        ensure_guest_tls_roots(&etc)?;
        configure_ports_tls(&apt)?;
    }
    for (path, backup, has_backup, permissions, original, new) in files {
        if original != new {
            if !has_backup {
                atomic_write(&backup, original.as_bytes(), permissions.clone())?;
            }
            atomic_write(&path, new.as_bytes(), permissions)?;
        }
    }
    Ok(())
}

#[cfg(target_os = "android")]
fn configure_ports_tls(apt: &Path) -> Result<(), RuntimeError> {
    use std::os::unix::fs::PermissionsExt;

    // Ubuntu Base 26.04 includes APT's OpenSSL transport but lacks the default
    // OpenSSL trust path. The guest's ca-certificates package can replace this
    // bundle in place, without changing the per-host APT configuration.
    let config_dir = apt.join("apt.conf.d");
    match fs::symlink_metadata(&config_dir) {
        Ok(metadata) if metadata.is_dir() => {}
        Ok(_) => return Err(malformed("unsafe APT configuration directory")),
        Err(error) if error.kind() == io::ErrorKind::NotFound => fs::create_dir(&config_dir)?,
        Err(error) => return Err(error.into()),
    }
    let path = config_dir.join("15androidpc-ubuntu-ports");
    let config = b"Acquire::https::ports.ubuntu.com::CAInfo \"/etc/ssl/certs/ca-certificates.crt\";\n";
    let preview = b"Acquire::https::CAInfo::ports.ubuntu.com \"/etc/ssl/certs/ca-certificates.crt\";\n";
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.is_file() => {
            let existing = fs::read(&path)?;
            if existing == config {
                return Ok(());
            }
            if existing != preview {
                return Err(malformed("the Android PC TLS configuration path is already in use"));
            }
        }
        Ok(_) => return Err(malformed("the Android PC TLS configuration path is already in use")),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    atomic_write(&path, config, fs::Permissions::from_mode(0o644))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_pockets_options_and_custom_hosts() {
        let input = "Types: deb\nURIs: http://archive.ubuntu.com/ubuntu https://custom.test/ubuntu\n http://ports.ubuntu.com/ubuntu-ports\nSuites: resolute resolute-updates\nComponents: main universe\nSigned-By: /x\n\nTypes: deb\nURIs: http://security.ubuntu.com/ubuntu\nSuites: resolute-security\nComponents: main\n";
        let result = deb822(input).unwrap();
        assert_eq!(result.matches(PORTS).count(), 2);
        assert!(result.contains("https://custom.test/ubuntu\nSuites: resolute resolute-updates"));
        assert!(result.contains("Suites: resolute-security\nComponents: main"));
        assert_eq!(deb822(&result).unwrap(), result);
    }

    #[test]
    fn deduplicates_legacy_entries_and_preserves_comments() {
        let input = "deb [arch=arm64] http://archive.ubuntu.com/ubuntu resolute main # main\n\
                     deb [arch=arm64] https://ports.ubuntu.com/ubuntu-ports/ resolute main # alias\n\
                     deb https://archive.ubuntu.com.example/ubuntu resolute main\n";
        let result = legacy(input, &HashSet::new()).unwrap();
        assert_eq!(result.matches(PORTS).count(), 1);
        assert!(result.contains("# alias\n"));
        assert!(result.contains("https://archive.ubuntu.com.example/ubuntu"));
        assert_eq!(legacy(&result, &HashSet::new()).unwrap(), result);
    }

    #[test]
    fn rejects_malformed_official_entries() {
        assert!(deb822("Types: deb\nURIs: http://archive.ubuntu.com/ubuntu\n").is_err());
        assert!(legacy("deb [arch=arm64 http://archive.ubuntu.com/ubuntu resolute main\n", &HashSet::new()).is_err());
    }

    #[test]
    fn deduplicates_equivalent_legacy_and_deb822_targets() {
        let source = format!("Types: deb\nURIs: {PORTS}\nSuites: resolute\nComponents: main universe\nSigned-By: /keys/ubuntu.gpg\n");
        let input = "deb [signed-by=/keys/ubuntu.gpg] http://archive.ubuntu.com/ubuntu resolute universe main\n\
                     deb [signed-by=/keys/other.gpg] http://archive.ubuntu.com/ubuntu resolute main\n";
        let result = legacy(input, &deb822_targets(&source)).unwrap();
        assert_eq!(result, format!("deb [signed-by=/keys/other.gpg] {PORTS} resolute main\n"));
    }

    #[cfg(not(target_os = "android"))]
    #[test]
    fn keeps_the_original_and_validates_all_sources_before_writing() {
        let root = std::env::temp_dir().join(format!("androidpc-apt-sources-{}", std::process::id()));
        let apt = root.join("etc/apt");
        let snippets = apt.join("sources.list.d");
        fs::create_dir_all(&snippets).unwrap();
        assert!(configure_ports(&root).is_err());
        let official = apt.join("sources.list");
        let invalid = snippets.join("ubuntu.sources");
        let custom = snippets.join("third-party.list");
        fs::write(&official, "# Ubuntu uses ubuntu.sources\n").unwrap();
        assert!(configure_ports(&root).is_err());
        let original = "deb http://archive.ubuntu.com/ubuntu resolute main\n";
        fs::write(&official, original).unwrap();
        fs::write(&custom, original).unwrap();
        fs::write(&invalid, "Types: deb\nURIs: http://security.ubuntu.com/ubuntu\n").unwrap();
        assert!(configure_ports(&root).is_err());
        assert_eq!(fs::read_to_string(&official).unwrap(), original);
        fs::remove_file(&invalid).unwrap();

        configure_ports(&root).unwrap();
        configure_ports(&root).unwrap();
        let migrated = format!("deb {PORTS} resolute main\n");
        assert_eq!(fs::read_to_string(&official).unwrap(), migrated);
        assert_eq!(fs::read_to_string(&custom).unwrap(), original);
        assert_eq!(fs::read_to_string(apt.join("sources.list.androidpc-original")).unwrap(), original);
        fs::remove_dir_all(root).unwrap();
    }
}
