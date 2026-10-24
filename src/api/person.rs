use super::http::get_json;
use super::mapping::map_item;
use super::{get_items, image_types, item_fields};
use crate::models::{ApiItem, MediaItem, SavedServer};
use reqwest::blocking::Client;

fn person_fields() -> String {
    ["Overview", "PrimaryImageAspectRatio"].join(",")
}

pub(crate) fn get_person_item(
    client: &Client,
    server: &SavedServer,
    user_id: &str,
    person_id: &str,
) -> Result<MediaItem, String> {
    let path = format!("Users/{user_id}/Items/{person_id}");
    let emby_params = vec![
        ("Fields", person_fields()),
        ("EnableImageTypes", image_types()),
    ];
    let jellyfin_params = vec![
        ("userId", user_id.to_string()),
        ("fields", person_fields()),
        ("enableImageTypes", image_types()),
    ];
    let raw: ApiItem = get_json(client, server, &path, &emby_params)
        .or_else(|_| get_json(client, server, &path, &jellyfin_params))?;
    Ok(map_item(server, raw))
}

pub(crate) fn get_person_filmography(
    client: &Client,
    server: &SavedServer,
    user_id: &str,
    person_id: &str,
    include_item_types: &[&str],
) -> Result<Vec<MediaItem>, String> {
    let emby_path = format!("Persons/{person_id}/Items");
    let mut emby_params = vec![
        ("UserId", user_id.to_string()),
        ("Recursive", "true".to_string()),
        ("SortBy", "SortName".to_string()),
        ("SortOrder", "Ascending".to_string()),
        ("Fields", item_fields()),
        ("EnableImages", "true".to_string()),
        ("EnableImageTypes", image_types()),
        ("ImageTypeLimit", "1".to_string()),
        ("EnableUserData", "true".to_string()),
    ];
    let mut jellyfin_params = vec![
        ("userId", user_id.to_string()),
        ("personIds", person_id.to_string()),
        ("recursive", "true".to_string()),
        ("sortBy", "SortName".to_string()),
        ("sortOrder", "Ascending".to_string()),
        ("fields", item_fields()),
        ("enableImages", "true".to_string()),
        ("enableImageTypes", image_types()),
        ("imageTypeLimit", "1".to_string()),
        ("enableUserData", "true".to_string()),
    ];
    if !include_item_types.is_empty() {
        emby_params.push(("IncludeItemTypes", include_item_types.join(",")));
        jellyfin_params.push(("includeItemTypes", include_item_types.join(",")));
    }
    get_items(client, server, &emby_path, &emby_params)
        .or_else(|_| get_items(client, server, "Items", &jellyfin_params))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ItemsResponse;

    fn test_server() -> SavedServer {
        SavedServer {
            id: "id".to_string(),
            name: "name".to_string(),
            url: "http://127.0.0.1:8096".to_string(),
            username: "user".to_string(),
            user_id: "user-id".to_string(),
            access_token: "token".to_string(),
            active: true,
            saved_at: 0,
            use_system_proxy: true,
            proxy_url: None,
            icon_url: None,
            icon_name: None,
        }
    }

    #[test]
    fn person_item_json_maps_to_media_item() {
        let raw = r#"{
            "Id": "person-1",
            "Name": "Keira Knightley",
            "Type": "Person",
            "Overview": "English actress and model.",
            "PrimaryImageAspectRatio": 0.68,
            "ImageTags": { "Primary": "tag-1" }
        }"#;

        let item: ApiItem = serde_json::from_str(raw).unwrap();
        let mapped = map_item(&test_server(), item);

        assert_eq!(mapped.id, "person-1");
        assert_eq!(mapped.name, "Keira Knightley");
        assert_eq!(mapped.item_type, "Person");
        assert_eq!(
            mapped.overview.as_deref(),
            Some("English actress and model.")
        );
        let url = mapped.primary_image_url.unwrap();
        assert!(url.contains("Items/person-1/Images/Primary"));
        assert!(url.contains("fillWidth=320"));
    }

    #[test]
    fn filmography_response_parses_items_with_total_count() {
        let raw = r#"{
            "Items": [
                {
                    "Id": "movie-1",
                    "Name": "Pride & Prejudice",
                    "Type": "Movie",
                    "ProductionYear": 2005,
                    "UserData": { "IsFavorite": true, "Played": true }
                },
                {
                    "Id": "series-1",
                    "Name": "Pirates of the Caribbean",
                    "Type": "Series",
                    "ProductionYear": 2003
                }
            ],
            "TotalRecordCount": 7
        }"#;

        let response: ItemsResponse = serde_json::from_str(raw).unwrap();

        assert_eq!(response.total_record_count(), Some(7));
        let items = response.into_items();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].production_year, Some(2005));
        assert_eq!(items[1].item_type.as_deref(), Some("Series"));
        assert!(
            items[0]
                .user_data
                .as_ref()
                .is_some_and(|data| data.is_favorite && data.played)
        );
    }
}
