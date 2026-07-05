use super::http::build_url;
use crate::models::{ApiItem, SavedServer};
use reqwest::Url;

pub(crate) fn primary_image_url(server: &SavedServer, item: &ApiItem) -> Option<String> {
    item.image_tags.get("Primary")?;
    media_image_url(server, &item.id, "Primary", &[("fillWidth", "360")]).ok()
}

pub(crate) fn backdrop_url(server: &SavedServer, item: &ApiItem) -> Option<String> {
    if !item.backdrop_image_tags.is_empty() {
        return media_image_url(server, &item.id, "Backdrop", &[("fillWidth", "1600")]).ok();
    }
    if let Some(parent_id) = &item.parent_backdrop_item_id {
        if !item.parent_backdrop_image_tags.is_empty() {
            return media_image_url(server, parent_id, "Backdrop", &[("fillWidth", "1600")]).ok();
        }
    }
    None
}

pub(crate) fn logo_url(server: &SavedServer, item: &ApiItem) -> Option<String> {
    if item.image_tags.contains_key("Logo") {
        return media_image_url(server, &item.id, "Logo", &[("fillWidth", "720")]).ok();
    }
    if let Some(parent_id) = &item.parent_logo_item_id {
        if item.parent_logo_image_tag.is_some() {
            return media_image_url(server, parent_id, "Logo", &[("fillWidth", "720")]).ok();
        }
    }
    None
}

pub(crate) fn media_image_url(
    server: &SavedServer,
    item_id: &str,
    kind: &str,
    params: &[(&str, &str)],
) -> Result<String, String> {
    let mut owned = vec![
        ("api_key", server.access_token.clone()),
        ("quality", "90".to_string()),
    ];
    for (key, value) in params {
        owned.push((key, value.to_string()));
    }
    Ok(build_url(
        &server.url,
        &format!("Items/{item_id}/Images/{kind}"),
        &owned,
    )?
    .to_string())
}

pub(crate) fn media_image_url_indexed(
    server: &SavedServer,
    item_id: &str,
    kind: &str,
    index: usize,
    params: &[(&str, &str)],
) -> Result<String, String> {
    let mut owned = vec![
        ("api_key", server.access_token.clone()),
        ("quality", "90".to_string()),
    ];
    for (key, value) in params {
        owned.push((key, value.to_string()));
    }
    Ok(build_url(
        &server.url,
        &format!("Items/{item_id}/Images/{kind}/{index}"),
        &owned,
    )?
    .to_string())
}

pub(crate) fn media_playback_url(server: &SavedServer, raw_url: &str) -> Result<String, String> {
    let mut url = if raw_url.starts_with("http://") || raw_url.starts_with("https://") {
        Url::parse(raw_url).map_err(|err| format!("Invalid playback URL: {err}"))?
    } else {
        build_url(&server.url, raw_url, &[])?
    };
    let has_key = url.query_pairs().any(|(key, _)| key == "api_key");
    if !has_key {
        url.query_pairs_mut()
            .append_pair("api_key", &server.access_token);
    }
    Ok(url.to_string())
}
