use crate::tools::ExecOutput;

pub fn execute(params: &serde_json::Value) -> Result<ExecOutput, String> {
    let url = params
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'url' parameter for web_fetch".to_string())?;

    // Use blocking reqwest in spawn_blocking context
    let response = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("OhMyWu/1.0")
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?
        .get(url)
        .send()
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        return Err(format!("HTTP {}: {}", status.as_u16(), url));
    }

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let body = response
        .text()
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    // Strip HTML tags for readability if it's HTML
    let text = if content_type.contains("text/html") {
        strip_html(&body)
    } else {
        body
    };

    Ok(ExecOutput {
        output: Some(format!(
            "URL: {}\nStatus: {}\nContent-Type: {}\n\n{}",
            url, status.as_u16(), content_type, text
        )),
        stderr: None,
        exit_code: 0,
    })
}

fn strip_html(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    let mut in_script = false;
    let mut in_style = false;

    let chars: Vec<char> = html.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if in_tag || in_script || in_style {
            if i + 2 < len && chars[i] == '<' {
                // Check for </script> or </style>
                let snippet: String = chars[i..len.min(i + 9)].iter().collect();
                if snippet.starts_with("</script>") {
                    in_script = false;
                    i += 9;
                    in_tag = true; // still in the closing tag
                    continue;
                }
                if snippet.starts_with("</style>") {
                    in_style = false;
                    i += 8;
                    in_tag = true;
                    continue;
                }
            }

            if chars[i] == '>' {
                in_tag = false;
            }
            i += 1;
            continue;
        }

        if chars[i] == '<' {
            // Check for script/style opening
            let snippet: String = chars[i..len.min(i + 6)].iter().collect();
            if snippet.starts_with("<script") {
                in_script = true;
                in_tag = true;
                i += 1;
                continue;
            }
            if snippet.starts_with("<style") {
                in_style = true;
                in_tag = true;
                i += 1;
                continue;
            }
            in_tag = true;
            i += 1;
            continue;
        }

        // Collapse whitespace
        if chars[i].is_whitespace() {
            if !result.ends_with(' ') {
                result.push(' ');
            }
        } else {
            result.push(chars[i]);
        }
        i += 1;
    }

    result.trim().to_string()
}
