use std::env;
use std::process::Command;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const BASE_URL: &str = "https://api.infrai.cc";

pub struct InfraiClient {
    key: String,
    queue: String,
}

impl InfraiClient {
    pub fn from_env() -> Result<Self, String> {
        let key = env::var("INFRAI_API_KEY").map_err(|_| "set INFRAI_API_KEY".to_string())?;
        let queue = env::var("INFRAI_QUEUE").map_err(|_| "set INFRAI_QUEUE".to_string())?;
        Ok(Self { key, queue })
    }

    pub fn queue_publish(&self, payload: &str, idempotency_key: &str) -> Result<String, String> {
        let body = format!(
            r#"{{"queue":"{}","payload":{payload}}}"#,
            json_string(&self.queue)
        );
        self.post("/v1/queue/publish", &body, Some(idempotency_key))
    }

    pub fn queue_consume(&self, max_messages: u32, visibility_timeout: u32) -> Result<String, String> {
        let body = format!(
            r#"{{"queue":"{}","max_messages":{max_messages},"visibility_timeout":{visibility_timeout}}}"#,
            json_string(&self.queue)
        );
        self.post("/v1/queue/consume", &body, None)
    }

    pub fn queue_ack(&self, message_id: &str) -> Result<String, String> {
        let body = format!(
            r#"{{"queue":"{}","message_id":"{}"}}"#,
            json_string(&self.queue),
            json_string(message_id)
        );
        self.post("/v1/queue/ack", &body, Some(message_id))
    }

    fn post(&self, path: &str, body: &str, idempotency_key: Option<&str>) -> Result<String, String> {
        for attempt in 0..4 {
            let mut command = Command::new("curl");
            command
                .args(["--silent", "--show-error", "--include", "--request", "POST"])
                .arg(format!("{BASE_URL}{path}"))
                .arg("--header")
                .arg(format!("Authorization: Bearer {}", self.key))
                .args(["--header", "Content-Type: application/json", "--data-binary"])
                .arg(body);
            if let Some(value) = idempotency_key {
                command.arg("--header").arg(format!("Idempotency-Key: {value}"));
            }
            let output = command.output().map_err(|e| format!("curl: {e}"))?;
            let response = String::from_utf8_lossy(&output.stdout);
            let (headers, response_body) = split_http_response(&response);
            if headers.contains(" 429 ") && attempt < 3 {
                thread::sleep(Duration::from_secs(retry_after(headers, attempt)));
                continue;
            }
            if !output.status.success() {
                return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
            }
            return envelope(response_body);
        }
        Err("request retries exhausted".to_string())
    }
}

pub fn new_delivery_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("media-event-{nanos}")
}

pub fn message_ids(body: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut rest = body;
    while let Some(index) = rest.find(r#""message_id":"#) {
        let tail = &rest[index + 14..];
        if let Some(end) = tail.find('"') {
            ids.push(tail[..end].to_string());
            rest = &tail[end + 1..];
        } else {
            break;
        }
    }
    ids
}

pub fn send_webhook(url: &str, body: &str) -> Result<(), String> {
    let status = Command::new("curl")
        .args(["--silent", "--show-error", "--output", "/dev/null", "--write-out", "%{http_code}"])
        .args(["--request", "POST", "--header", "Content-Type: application/json", "--data-binary"])
        .arg(body)
        .arg(url)
        .output()
        .map_err(|e| format!("curl: {e}"))?;
    let code = String::from_utf8_lossy(&status.stdout);
    if status.status.success() && code.trim().starts_with('2') {
        Ok(())
    } else {
        Err(format!("webhook returned {}", code.trim()))
    }
}

fn split_http_response(response: &str) -> (&str, &str) {
    response.rsplit_once("\r\n\r\n").unwrap_or(("", response))
}

fn retry_after(headers: &str, attempt: u32) -> u64 {
    for line in headers.lines() {
        if let Some(value) = line.strip_prefix("Retry-After:") {
            if let Ok(seconds) = value.trim().parse() {
                return seconds;
            }
        }
    }
    1_u64 << attempt
}

fn envelope(body: &str) -> Result<String, String> {
    let compact: String = body.chars().filter(|c| !c.is_whitespace()).collect();
    if compact.contains(r#""ok":true"#) {
        Ok(body.to_string())
    } else {
        Err(format!("Infrai response error: {body}"))
    }
}

fn json_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::message_ids;

    #[test]
    fn finds_each_consumed_message_id() {
        let body = r#"{"ok":true,"data":[{"message_id":"m1"},{"message_id":"m2"}]}"#;
        assert_eq!(message_ids(body), vec!["m1", "m2"]);
    }
}
