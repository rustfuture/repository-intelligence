use repository_intelligence::{current_git_revision, llm::{AgyProvider, OllamaProvider, Provider}, Index};
use std::{
    env,
    io::{Read, Write},
    net::TcpListener,
    path::Path,
    sync::{Arc, RwLock},
};

fn main() {
    let mut args = env::args().skip(1);
    let root = args.next().unwrap_or_else(|| ".".into());
    if root == "--answer" {
        let repository = args.next().unwrap_or_else(|| ".".into());
        let question = args.collect::<Vec<_>>().join(" ");
        let index = Index::build(Path::new(&repository)).expect("index repository");
        let evidence = index
            .search(&question, 5)
            .iter()
            .map(|hit| format!("{}:{} {}", hit.path.display(), hit.line, hit.text.trim()))
            .collect::<Vec<_>>()
            .join("\n");
        let model = env::var("AGY_MODEL").unwrap_or_else(|_| "gemini-3.8-flash-low".to_owned());
        let provider: Box<dyn Provider> = if env::var("USE_OLLAMA").is_ok() {
            Box::new(OllamaProvider { model: env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3".to_owned()) })
        } else {
            Box::new(AgyProvider { model })
        };
        let answer = provider
            .answer(&question, &evidence)
            .expect("run LLM provider");
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
        return;
    }
    if root == "--serve" {
        let address = args.next().unwrap_or_else(|| "127.0.0.1:8080".into());
        let repository = args.next().unwrap_or_else(|| ".".into());
        serve(&address, Path::new(&repository));
        return;
    }
    if root == "--analytics" {
        let repository = args.next().unwrap_or_else(|| ".".into());
        let index = Index::build(Path::new(&repository)).expect("index repository");
        println!("{}", index.analytics());
        return;
    }
    let query = args.collect::<Vec<_>>().join(" ");
    if query.trim().is_empty() {
        eprintln!("usage: repository-intelligence <repo> <query...>");
        std::process::exit(2);
    }
    let index = Index::build(Path::new(&root)).expect("index repository");
    for hit in index.search(&query, 10) {
        println!("{}:{}\t{}", hit.path.display(), hit.line, hit.text.trim());
    }
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
                "{{\"status\":\"ok\",\"commit\":{}}}",
                index
                    .revision()
                    .map(|revision| format!("\"{}\"", json_escape(revision)))
                    .unwrap_or_else(|| "null".to_owned())
            )
        } else if path == "/reload" {
            match current_git_revision(&root) {
                Some(revision) => {
                    let mut index = index.write().expect("index lock");
                    match index.sync_git(&root, &revision) {
                        Ok(()) => format!(
                            "{{\"status\":\"reloaded\",\"commit\":\"{}\"}}",
                            json_escape(&revision)
                        ),
                        Err(error) => {
                            format!("{{\"error\":\"{}\"}}", json_escape(&error.to_string()))
                        }
                    }
                }
                None => "{\"error\":\"repository has no readable Git HEAD\"}".to_owned(),
            }
        } else if let Some(query) = path.strip_prefix("/search?q=") {
            let index = index.read().expect("index lock");
            let query = query.replace('+', " ");
            let hits = index.search(&query, 10);
            let items = hits
                .iter()
                .map(|hit| {
                    format!(
                        "{{\"path\":\"{}\",\"line\":{},\"score\":{},\"text\":\"{}\"}}",
                        json_escape(&hit.path.display().to_string()),
                        hit.line,
                        hit.score,
                        json_escape(hit.text.trim())
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "{{\"query\":\"{}\",\"commit\":{},\"hits\":[{}]}}",
                json_escape(&query),
                index
                    .revision()
                    .map(|revision| format!("\"{}\"", json_escape(revision)))
                    .unwrap_or_else(|| "null".to_owned()),
                items
            )
        } else {
            "{\"error\":\"use /health or /search?q=term\"}".to_owned()
        };
        let status = if path == "/health" || path == "/reload" || path.starts_with("/search?q=") {
            "200 OK"
        } else {
            "404 Not Found"
        };
        let response = format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
        let _ = stream.write_all(response.as_bytes());
    }
}

fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}
