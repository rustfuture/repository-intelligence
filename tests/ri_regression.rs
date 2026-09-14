use repository_intelligence::{EmbeddingProvider, Index};
use std::{
    fs,
    io::Write,
    net::{TcpListener, TcpStream},
    path::PathBuf,
    process::Command,
    sync::Arc,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

fn temp_dir(label: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("repository-intelligence-{label}-{suffix}"));
    fs::create_dir_all(&path).expect("create temporary directory");
    path
}

fn hex(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
fn loading_an_external_index_does_not_admit_sensitive_files() {
    let root = temp_dir("sensitive-index");
    let index_path = root.join("external.ri");
    let index = format!(
        "RI_INDEX_V1\nrevision\t\nfile\t{}\t{}\n",
        hex("credentials.json"),
        hex("do-not-index-this-secret")
    );
    fs::write(&index_path, index).expect("write external index");

    let loaded = Index::load_from(&index_path).expect("load external index");
    assert!(
        loaded.search("do-not-index-this-secret", 5).is_empty(),
        "load_from must apply the same sensitive-file policy as a full scan"
    );
}

#[test]
fn loading_duplicate_file_records_does_not_leave_stale_terms() {
    let root = temp_dir("duplicate-index");
    let index_path = root.join("external.ri");
    let index = format!(
        "RI_INDEX_V1\nrevision\t\nfile\t{}\t{}\nfile\t{}\t{}\n",
        hex("src.rs"),
        hex("stale-secret"),
        hex("src.rs"),
        hex("public-symbol")
    );
    fs::write(&index_path, index).expect("write duplicate index");

    match Index::load_from(&index_path) {
        Err(_) => {}
        Ok(loaded) => {
            assert!(
                loaded.search("stale-secret", 5).is_empty(),
                "duplicate records must not retain terms from an overwritten file"
            );
            assert_eq!(loaded.search("public-symbol", 5).len(), 1);
        }
    }
}

struct LexicalTrapEmbedding;

impl EmbeddingProvider for LexicalTrapEmbedding {
    fn name(&self) -> &str {
        "lexical-trap-test"
    }

    fn dimension(&self) -> usize {
        1
    }

    fn embed(&self, text: &str) -> Vec<f32> {
        if text == "anchor" {
            vec![1.0]
        } else if text.contains("anchor") {
            vec![0.0]
        } else {
            vec![1.0]
        }
    }
}

#[test]
fn answer_context_keeps_a_lexical_anchor_in_the_returned_evidence() {
    let root = temp_dir("lexical-anchor");
    fs::write(root.join("z.rs"), "anchor target\n").expect("write lexical anchor");
    fs::write(root.join("a.rs"), "unrelated\n").expect("write semantic distractor");

    let mut index = Index::with_embedding(Arc::new(LexicalTrapEmbedding));
    index.rebuild(&root).expect("build trap index");
    let context = index
        .answer_context("anchor", 1)
        .expect("lexical anchor should make the question answerable");
    assert!(
        context.contains("[z.rs:1]"),
        "answer context must include the lexical anchor it used as its guard"
    );
    assert!(
        !context.contains("[a.rs:1]"),
        "a semantic-only distractor must not replace the lexical anchor"
    );
}

#[test]
fn malformed_percent_encoding_does_not_crash_http_server() {
    let root = temp_dir("malformed-query");
    fs::write(root.join("lib.rs"), "pub fn searchable() {}\n").expect("write source");
    let listener = TcpListener::bind("127.0.0.1:0").expect("reserve local port");
    let address = listener.local_addr().expect("read local address");
    drop(listener);

    let mut child = Command::new(env!("CARGO_BIN_EXE_repository-intelligence"))
        .args([
            "--serve",
            &address.to_string(),
            root.to_str().expect("utf8 temp path"),
        ])
        .spawn()
        .expect("start HTTP server");
    {
        let mut connected = None;
        for _ in 0..50 {
            match TcpStream::connect(address) {
                Ok(stream) => {
                    connected = Some(stream);
                    break;
                }
                Err(_) => thread::sleep(Duration::from_millis(20)),
            }
        }
        let mut stream = connected.expect("server should start");
        stream
            .write_all(b"GET /search?q=%e\xc3\xa9 HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .expect("send malformed query");
        thread::sleep(Duration::from_millis(100));
        assert!(
            child.try_wait().expect("inspect HTTP server").is_none(),
            "malformed percent encoding must not panic the HTTP process"
        );
    }
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn git_sync_handles_git_quoted_paths() {
    let root = temp_dir("quoted-path");
    let filename = PathBuf::from("tab\tname.rs");
    let file = root.join(&filename);
    fs::write(&file, "staleunique\n").expect("write initial source");

    let git = |args: &[&str]| {
        Command::new("git")
            .args(["-C", root.to_str().expect("utf8 temp path")])
            .args(args)
            .output()
            .expect("run git")
    };
    assert!(git(&["init", "-q"]).status.success());
    assert!(git(&["config", "user.email", "test@example.invalid"])
        .status
        .success());
    assert!(
        git(&["config", "user.name", "Repository Intelligence Tests"])
            .status
            .success()
    );
    assert!(
        git(&["add", "--", filename.to_str().expect("utf8 filename")])
            .status
            .success()
    );
    assert!(git(&["commit", "-qm", "initial"]).status.success());
    let old = String::from_utf8(git(&["rev-parse", "HEAD"]).stdout)
        .expect("utf8 old revision")
        .trim()
        .to_owned();
    let mut index = Index::build(&root).expect("build initial index");
    assert_eq!(index.search("staleunique", 5).len(), 1);

    fs::write(&file, "freshunique\n").expect("write changed source");
    assert!(
        git(&["add", "--", filename.to_str().expect("utf8 filename")])
            .status
            .success()
    );
    assert!(git(&["commit", "-qm", "changed"]).status.success());
    let new = String::from_utf8(git(&["rev-parse", "HEAD"]).stdout)
        .expect("utf8 new revision")
        .trim()
        .to_owned();
    index.sync_git(&root, &new).expect("sync quoted path");
    assert!(
        index.search("staleunique", 5).is_empty(),
        "the old content must be removed after an M status"
    );
    assert_eq!(index.search("freshunique", 5).len(), 1);
    assert_eq!(index.revision(), Some(new.as_str()));
    assert_ne!(old, new);
}

#[test]
fn git_sync_reconciles_dirty_snapshot_back_to_clean_revision() {
    let root = temp_dir("dirty-clean");
    let file = root.join("src.rs");
    fs::write(&file, "stablecommittedmarker\n").expect("write source");
    let git = |args: &[&str]| {
        Command::new("git")
            .args(["-C", root.to_str().expect("utf8 temp path")])
            .args(args)
            .output()
            .expect("run git")
    };
    assert!(git(&["init", "-q"]).status.success());
    assert!(git(&["config", "user.email", "test@example.invalid"])
        .status
        .success());
    assert!(
        git(&["config", "user.name", "Repository Intelligence Tests"])
            .status
            .success()
    );
    assert!(git(&["add", "src.rs"]).status.success());
    assert!(git(&["commit", "-qm", "initial"]).status.success());
    let clean = String::from_utf8(git(&["rev-parse", "HEAD"]).stdout)
        .expect("utf8 revision")
        .trim()
        .to_owned();
    let mut index = Index::build(&root).expect("build index");
    fs::write(&file, "ephemeraldirtymarker\n").expect("write dirty source");
    let dirty = format!("{clean}-dirty");
    index.sync_git(&root, &dirty).expect("sync dirty snapshot");
    assert_eq!(index.revision(), Some(dirty.as_str()));
    assert_eq!(index.search("ephemeraldirtymarker", 5).len(), 1);
    assert!(git(&["restore", "--", "src.rs"]).status.success());
    index
        .sync_git(&root, &clean)
        .expect("sync restored clean worktree");
    assert!(index.search("ephemeraldirtymarker", 5).is_empty());
    assert_eq!(index.search("stablecommittedmarker", 5).len(), 1);
    assert_eq!(index.revision(), Some(clean.as_str()));
}

#[cfg(unix)]
#[test]
fn answer_command_rejects_wrong_line_and_range_citations() {
    use std::{ffi::OsString, os::unix::fs::PermissionsExt};

    let root = temp_dir("citation-validation");
    let source = root.join("src").join("lib.rs");
    fs::create_dir_all(source.parent().expect("source parent")).expect("create source parent");
    fs::write(
        &source,
        "// filler 1\n// filler 2\n// filler 3\n// filler 4\n// filler 5\n// filler 6\n// filler 7\n// filler 8\n// filler 9\npub fn needle() {\n    true;\n}\n",
    )
    .expect("write citation fixture");

    let bin = env!("CARGO_BIN_EXE_repository-intelligence");
    let fake_bin = root.join("bin");
    fs::create_dir_all(&fake_bin).expect("create fake provider directory");
    let ollama = fake_bin.join("ollama");
    fs::write(
        &ollama,
        "#!/bin/sh\nprintf '%s\\n' 'wrong line [src/lib.rs:1]' 'wrong range [src/lib.rs:999-1000]' 'wrong end [src/lib.rs:10-999]' 'extensionless [LICENSE:999]'\n",
    )
    .expect("write fake provider");
    let mut permissions = fs::metadata(&ollama)
        .expect("read fake provider metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&ollama, permissions).expect("make fake provider executable");

    let mut path = OsString::from(&fake_bin);
    path.push(":");
    path.push(std::env::var_os("PATH").unwrap_or_default());
    let output = Command::new(bin)
        .args(["--answer", root.to_str().expect("utf8 temp path"), "needle"])
        .env("USE_OLLAMA", "1")
        .env("OLLAMA_MODEL", "fake")
        .env("PATH", path)
        .output()
        .expect("run answer command");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 answer output");
    assert_eq!(
        stdout.lines().last(),
        Some("Insufficient repository evidence to answer this question.")
    );
    assert!(!stdout.contains("[Unverified citation removed]"));
}
