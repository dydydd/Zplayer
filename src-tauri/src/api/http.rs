use crate::models::SavedServer;
use reqwest::blocking::Client;
use reqwest::Url;
use serde::de::DeserializeOwned;
use std::time::Duration;

const CLIENT_NAME: &str = "Zplayer";
const CLIENT_VERSION: &str = "0.1.0";

pub(crate) fn http_client(use_system_proxy: bool) -> Result<Client, String> {
    http_client_with_timeout(use_system_proxy, Duration::from_secs(20))
}

pub(crate) fn http_client_with_timeout(
    use_system_proxy: bool,
    timeout: Duration,
) -> Result<Client, String> {
    let mut builder = Client::builder().timeout(timeout);
    if !use_system_proxy {
        builder = builder.no_proxy();
    }
    builder
        .build()
        .map_err(|err| format!("Failed to create HTTP client: {err}"))
}

pub(crate) fn get_json<T: DeserializeOwned>(
    client: &Client,
    server: &SavedServer,
    path: &str,
    params: &[(&str, String)],
) -> Result<T, String> {
    let original_path = path.to_string();
    let path = path.replace("{user_id}", &server.user_id);
    let mut all_params = params.to_vec();
    all_params.push(("api_key", server.access_token.clone()));
    let url = build_url(&server.url, &path, &all_params)?;
    let response = client
        .get(url)
        .header("X-Emby-Token", &server.access_token)
        .header("X-Emby-Authorization", auth_header("MediaBrowser"))
        .send()
        .map_err(|err| format!("Server request failed: {err}"))?;
    read_json(response, &original_path)
}

pub(crate) fn read_json<T: DeserializeOwned>(
    response: reqwest::blocking::Response,
    context: &str,
) -> Result<T, String> {
    let status = response.status();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    if !status.is_success() {
        let body = response.text().unwrap_or_default();
        let detail = body.trim();
        return Err(if detail.is_empty() {
            format!("{context} returned HTTP {status}.")
        } else {
            format!("{context} returned HTTP {status}: {detail}")
        });
    }
    let body = response
        .text()
        .map_err(|err| format!("Failed to read {context} response body: {err}"))?;
    serde_json::from_str(&body).map_err(|err| {
        let content_type = content_type.unwrap_or_else(|| "unknown content-type".to_string());
        let preview = response_preview(&body);
        if preview.is_empty() {
            format!("Failed to decode {context} response as JSON ({content_type}): {err}; empty response body")
        } else {
            format!("Failed to decode {context} response as JSON ({content_type}): {err}; body starts with: {preview}")
        }
    })
}

fn response_preview(body: &str) -> String {
    body.chars()
        .take(240)
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_preview_collapses_whitespace_and_limits_body() {
        let body = format!("  <html>\n  {}\n  </html>", "x".repeat(300));
        let preview = response_preview(&body);

        assert!(!preview.contains('\n'));
        assert!(preview.starts_with("<html>"));
        assert!(preview.len() <= 240);
    }
}

pub(crate) fn build_url(
    base_url: &str,
    path: &str,
    params: &[(&str, String)],
) -> Result<Url, String> {
    let root = format!("{}/", base_url.trim_end_matches('/'));
    let mut url = Url::parse(&root)
        .and_then(|url| url.join(path.trim_start_matches('/')))
        .map_err(|err| format!("Invalid server URL: {err}"))?;
    {
        let mut pairs = url.query_pairs_mut();
        for (key, value) in params {
            pairs.append_pair(key, value);
        }
    }
    Ok(url)
}

pub(crate) fn auth_header(prefix: &str) -> String {
    format!(
        "{prefix} Client=\"{CLIENT_NAME}\", Device=\"Desktop\", DeviceId=\"zplayer-desktop\", Version=\"{CLIENT_VERSION}\""
    )
}

pub(crate) fn post_to_server(
    client: &Client,
    server: &SavedServer,
    path: &str,
    body: &serde_json::Value,
) -> Result<(), String> {
    post_to_server_with_params(client, server, path, &[], body)
}

pub(crate) fn post_to_server_with_params(
    client: &Client,
    server: &SavedServer,
    path: &str,
    params: &[(&str, String)],
    body: &serde_json::Value,
) -> Result<(), String> {
    let mut all_params = params.to_vec();
    all_params.push(("api_key", server.access_token.clone()));
    let url = build_url(&server.url, path, &all_params)?;
    let response = client
        .post(url)
        .header("X-Emby-Token", &server.access_token)
        .header("X-Emby-Authorization", auth_header("MediaBrowser"))
        .header("Authorization", auth_header("MediaBrowser"))
        .json(body)
        .send()
        .map_err(|err| format!("Server request failed: {err}"))?;
    if response.status().is_success() || response.status().as_u16() == 204 {
        return Ok(());
    }
    let status = response.status();
    let text = response.text().unwrap_or_default();
    Err(format!("{path} returned HTTP {status}: {text}"))
}

pub(crate) fn post_empty_to_server_with_params(
    client: &Client,
    server: &SavedServer,
    path: &str,
    params: &[(&str, String)],
) -> Result<(), String> {
    let mut all_params = params.to_vec();
    all_params.push(("api_key", server.access_token.clone()));
    let url = build_url(&server.url, path, &all_params)?;
    let response = client
        .post(url)
        .header("X-Emby-Token", &server.access_token)
        .header("X-Emby-Authorization", auth_header("MediaBrowser"))
        .header("Authorization", auth_header("MediaBrowser"))
        .send()
        .map_err(|err| format!("Server request failed: {err}"))?;
    if response.status().is_success() || response.status().as_u16() == 204 {
        return Ok(());
    }
    let status = response.status();
    let text = response.text().unwrap_or_default();
    Err(format!("{path} returned HTTP {status}: {text}"))
}

pub(crate) fn delete_from_server(
    client: &Client,
    server: &SavedServer,
    path: &str,
) -> Result<(), String> {
    delete_from_server_with_params(client, server, path, &[])
}

pub(crate) fn delete_from_server_with_params(
    client: &Client,
    server: &SavedServer,
    path: &str,
    params: &[(&str, String)],
) -> Result<(), String> {
    let mut all_params = params.to_vec();
    all_params.push(("api_key", server.access_token.clone()));
    let url = build_url(&server.url, path, &all_params)?;
    let response = client
        .delete(url)
        .header("X-Emby-Token", &server.access_token)
        .header("X-Emby-Authorization", auth_header("MediaBrowser"))
        .header("Authorization", auth_header("MediaBrowser"))
        .send()
        .map_err(|err| format!("Server request failed: {err}"))?;
    if response.status().is_success() || response.status().as_u16() == 204 {
        return Ok(());
    }
    let status = response.status();
    let text = response.text().unwrap_or_default();
    Err(format!("{path} returned HTTP {status}: {text}"))
}
