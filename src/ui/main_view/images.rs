use super::state::MainView;
use crate::services::{AppPaths, LibraryService};
use gpui_kit::Context;
use gpui_kit::prelude::*;
use std::sync::Arc;

impl MainView {
    pub(crate) fn load_image(
        &self,
        item_id: &str,
        server_id: &str,
        url: &str,
        cx: &mut Context<Self>,
    ) {
        let item_id = item_id.to_string();
        let server_id = server_id.to_string();
        let url = url.to_string();
        let paths = match AppPaths::from_environment() {
            Ok(paths) => paths,
            Err(_) => return,
        };
        let service = match LibraryService::new(&paths) {
            Ok(service) => service,
            Err(_) => return,
        };
        let view = cx.entity();

        cx.spawn(async move |_this, cx| {
            let result = cx
                .background_spawn(async move { service.image(&server_id, &url) })
                .await;
            view.update(cx, move |this, cx| {
                match result {
                    Ok(image) => {
                        this.images.insert(item_id, Arc::new(image));
                    }
                    Err(err) => {
                        this.error = Some(err);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }
}
