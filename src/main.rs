use repository_intelligence::{
    current_git_revision, format_evidence,
    llm::{AgyProvider, OllamaProvider, Provider},
    Evidence, Index, RetrievalMode,
};
use std::{
    env,
    io::{Read, Write},
    net::TcpListener,
    path::Path,
    sync::{Arc, RwLock},
};

fn main() {
    let mut args = env::args().skip(1);
    let mode = args.next().unwrap_or_else(|| ".".into());
    match mode.as_str() {
        "--answer" => answer_command(&mut args),
        "--serve" => {
            let address = args.next().unwrap_or_else(|| "127.0.0.1:8080".into());
            let repository = args.next().unwrap_or_else(|| ".".into());
            serve(&address, Path::new(&repository));
        }
        "--analytics" => {
            let repository = args.next().unwrap_or_else(|| ".".into());
            let index = Index::build(Path::new(&repository)).expect("index repository");
            println!("{}", index.analytics());
        }
        "--index" => {
            let repository = args.next().unwrap_or_else(|| ".".into());
            let index_path = args
                .next()
                .unwrap_or_else(|| ".repository-intelligence/index.ri".into());
            let index = Index::build(Path::new(&repository)).expect("index repository");
            index.save_to(Path::new(&index_path)).expect("save index");
            println!(
                "saved={} files={} chunks={} commit={}",
                index_path,
                index.file_count(),
                index.chunk_count(),
                index.revision().unwrap_or("unknown")
            );
        }
        "--load-index" => {
            let index_path = args
                .next()
                .unwrap_or_else(|| ".repository-intelligence/index.ri".into());
            let query = args.collect::<Vec<_>>().join(" ");
            let index = Index::load_from(Path::new(&index_path)).expect("load index");
            for item in index.search_evidence(&query, 10, RetrievalMode::Hybrid) {
                println!(
                    "{}\t{:.6}\t{}",
                    item.citation(),
                    item.score,
                    item.text.replace('\n', " ")
                );
            }
        }
        "--semantic" | "--hybrid" => {
            let retrieval = if mode == "--semantic" {
                RetrievalMode::Semantic
            } else {
                RetrievalMode::Hybrid
            };
            let repository = args.next().unwrap_or_else(|| ".".into());
            let query = args.collect::<Vec<_>>().join(" ");
            let index = Index::build(Path::new(&repository)).expect("index repository");
            for item in index.search_evidence(&query, 10, retrieval) {
                println!(
                    "{}\t{:.6}\t{}",
                    item.citation(),
                    item.score,
                    item.text.replace('\n', " ")
                );
            }
        }
        "--commits" => {
            let repository = args.next().unwrap_or_else(|| ".".into());
            let query = args.collect::<Vec<_>>().join(" ");
            let index = Index::build(Path::new(&repository)).expect("index repository");
            for commit in index.search_commits(&query, 10) {
                println!("{}\t{}\t{}", commit.sha, commit.date, commit.subject);
            }
        }
        _ => {
            let query = args.collect::<Vec<_>>().join(" ");
            if query.trim().is_empty() {
                eprintln!("usage: repository-intelligence <repo> <query...>");
                std::process::exit(2);
            }
            let index = Index::build(Path::new(&mode)).expect("index repository");
            for hit in index.search(&query, 10) {
                println!("{}:{}\t{}", hit.path.display(), hit.line, hit.text.trim());
            }
        }
    }
}

fn answer_command(args: &mut impl Iterator<Item = String>) {
    let repository = args.next().unwrap_or_else(|| ".".into());
    let question = args.collect::<Vec<_>>().join(" ");
    let index = Index::build(Path::new(&repository)).expect("index repository");
    let evidence_items = index.build_evidence(&question, 5);
    if evidence_items.is_empty()
        || index
            .search_evidence(&question, 1, RetrievalMode::Lexical)
            .is_empty()
    {
        println!(
            "commit={}\nmodel=none\nduration_ms=0\ncost_usd=0\nInsufficient repository evidence to answer this question.",
            index.revision().unwrap_or("unknown")
        );
        return;
    }
    let evidence = format_evidence(&evidence_items);
    let model = env::var("AGY_MODEL").unwrap_or_else(|_| "gemini-3.8-flash-low".to_owned());
    let provider: Box<dyn Provider> = if env::var("USE_OLLAMA").is_ok() {
        Box::new(OllamaProvider {
            model: env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3".to_owned()),
        })
    } else {
        Box::new(AgyProvider { model })
    };
    let mut answer = provider
        .answer(&question, &evidence)
        .expect("run LLM provider");

    let mut verified_text = String::new();
    let mut verified_citations = 0;
    for line in answer.text.lines() {
        let mut valid = true;
        for word in line.split_whitespace() {
            let cleaned = word.trim_matches(|c: char| {
                !c.is_alphanumeric() && c != '.' && c != ':' && c != '/' && c != '_' && c != '-'
            });
            if let Some((path, range)) = parse_citation(cleaned) {
                if !evidence_items
                    .iter()
                    .any(|item| citation_is_supported(item, &path, range.0, range.1))
                {
                    valid = false;
                    break;
                }
                verified_citations += 1;
            }
        }
        if valid {
            verified_text.push_str(line);
        } else {
            verified_text.push_str("[Unverified citation removed]");
        }
        verified_text.push('\n');
    }
    answer.text = if verified_citations == 0 {
        "Insufficient repository evidence to answer this question.".to_owned()
    } else {
        verified_text.trim().to_owned()
    };
    println!(
        "commit={}\nmodel={}\nduration_ms={}\ncost_usd={}\n{}",
        index.revision().unwrap_or("unknown"),
        answer.model,
        answer.duration_ms,
        answer
            .cost_usd
            .map(|cost| cost.to_string())
            .unwrap_or_else(|| "unknown".to_owned()),
        answer.text
    );
}

fn parse_citation(value: &str) -> Option<(String, (usize, usize))> {
    let (path, range) = value.rsplit_once(':')?;
    if path.is_empty() {
        return None;
    }
    let mut numbers = range.split('-');
    let start = numbers.next()?.parse::<usize>().ok()?;
    let end = numbers.next().unwrap_or(range).parse::<usize>().ok()?;
    (start > 0 && end >= start).then_some((path.to_owned(), (start, end)))
}

fn citation_is_supported(item: &Evidence, path: &str, start: usize, end: usize) -> bool {
    item.path.to_string_lossy() == path && start >= item.start_line && end <= item.end_line
}

fn serve(address: &str, root: &Path) {
    let root = root.to_path_buf();
    let index = Arc::new(RwLock::new(Index::build(&root).expect("index repository")));
    let listener = TcpListener::bind(address).expect("bind HTTP listener");
    eprintln!("repository-intelligence listening on http://{address}");
    for stream in listener.incoming() {
        let index = Arc::clone(&index);
        let root = root.clone();
        let Ok(mut stream) = stream else { continue };
        let mut request = [0_u8; 8192];
        let Ok(size) = stream.read(&mut request) else {
            continue;
        };
        let request = String::from_utf8_lossy(&request[..size]);
        let first_line = request.lines().next().unwrap_or_default();
        let path = first_line.split_whitespace().nth(1).unwrap_or("/");
        let body = if path == "/health" {
            let index = index.read().expect("index lock");
            format!(
                "{{\"status\":\"ok\",\"commit\":{},\"files\":{},\"chunks\":{},\"embedding\":\"{}\"}}",
                index
                    .revision()
                    .map(|revision| format!("\"{}\"", json_escape(revision)))
                    .unwrap_or_else(|| "null".to_owned()),
                index.file_count(),
                index.chunk_count(),
                json_escape(index.embedding_provider())
            )
        } else if path == "/reload" {
            match current_git_revision(&root) {
                Some(revision) => {
                    let mut index = index.write().expect("index lock");
                    match index.sync_git(&root, &revision) {
                        Ok(()) => format!(
                            "{{\"status\":\"reloaded\",\"commit\":\"{}\",\"files\":{},\"chunks\":{}}}",
                            json_escape(&revision),
                            index.file_count(),
                            index.chunk_count()
                        ),
                        Err(error) => {
                            format!("{{\"error\":\"{}\"}}", json_escape(&error.to_string()))
                        }
                    }
                }
                None => "{\"error\":\"repository has no readable Git HEAD\"}".to_owned(),
            }
        } else if let Some(query) = path.strip_prefix("/commits?q=") {
            let index = index.read().expect("index lock");
            let query = decode_query(query);
            let items = index
                .search_commits(&query, 10)
                .iter()
                .map(|commit| {
                    format!(
                        "{{\"sha\":\"{}\",\"date\":\"{}\",\"subject\":\"{}\"}}",
                        json_escape(&commit.sha),
                        json_escape(&commit.date),
                        json_escape(&commit.subject)
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "{{\"query\":\"{}\",\"commits\":[{}]}}",
                json_escape(&query),
                items
            )
        } else if let Some(query) = path.strip_prefix("/search?q=") {
            let index = index.read().expect("index lock");
            let query = decode_query(query);
            let hits = index.search_evidence(&query, 10, RetrievalMode::Hybrid);
            let items = hits
                .iter()
                .map(|hit| {
                    format!(
                        "{{\"path\":\"{}\",\"start_line\":{},\"end_line\":{},\"score\":{:.6},\"source\":\"{}\",\"kind\":\"{}\",\"text\":\"{}\"}}",
                        json_escape(&hit.path.display().to_string()),
                        hit.start_line,
                        hit.end_line,
                        hit.score,
                        json_escape(&hit.source),
                        json_escape(&hit.kind),
                        json_escape(hit.text.trim())
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "{{\"query\":\"{}\",\"commit\":{},\"mode\":\"hybrid\",\"hits\":[{}]}}",
                json_escape(&query),
                index
                    .revision()
                    .map(|revision| format!("\"{}\"", json_escape(revision)))
                    .unwrap_or_else(|| "null".to_owned()),
                items
            )
        } else {
            "{\"error\":\"use /health or /search?q=term+term\"}".to_owned()
        };
        let status = if path == "/health"
            || path == "/reload"
            || path.starts_with("/search?q=")
            || path.starts_with("/commits?q=")
        {
            "200 OK"
        } else {
            "404 Not Found"
        };
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = stream.write_all(response.as_bytes());
    }
}

fn decode_query(value: &str) -> String {
    let mut decoded = Vec::with_capacity(value.len());
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => decoded.push(b' '),
            b'%' if index + 2 < bytes.len() => {
                if let (Some(high), Some(low)) =
                    (hex_nibble(bytes[index + 1]), hex_nibble(bytes[index + 2]))
                {
                    decoded.push((high << 4) | low);
                    index += 2;
                } else {
                    decoded.push(b'%');
                }
            }
            byte => decoded.push(byte),
        }
        index += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn json_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '\u{08}' => escaped.push_str("\\b"),
            '\u{0c}' => escaped.push_str("\\f"),
            character if character.is_control() => {
                escaped.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => escaped.push(character),
        }
    }
    escaped
}
