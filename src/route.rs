use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};

use crate::{
    handler::{
        create_post_handler, delete_post_handler, edit_post_handler, get_post_handler,
        health_checker_handler, post_list_handler,
    },
    AppState,
};

pub fn create_router(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/v1/api/healthchecker", get(health_checker_handler))
        .route("/v1/api/posts", post(create_post_handler))
        .route("/v1/api/posts", get(post_list_handler))
        .route(
            "/v1/api/posts/:id",
            get(get_post_handler)
                .patch(edit_post_handler)
                .delete(delete_post_handler),
        )
        .with_state(app_state)
}
