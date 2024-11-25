use crate::models::tag::global::TagGlobal;
use crate::models::tag::stream::{
    HotTag, HotTags, HotTagsInput, TagStreamReach, TaggedType, Taggers,
};
use crate::routes::v0::endpoints::{HOT_TAGS_BY_REACH_ROUTE, HOT_TAGS_ROUTE, TAG_TAGGERS_ROUTE};
use crate::types::Pagination;
use crate::{Error, Result};
use axum::extract::{Path, Query};
use axum::Json;
use chrono::Utc;
use log::{error, info};
use serde::Deserialize;
use utoipa::OpenApi;

#[derive(Deserialize, Debug)]
pub struct HotTagsQuery {
    taggers_limit: Option<usize>,
    from: Option<i64>,
    to: Option<i64>,
    tagged_type: Option<TaggedType>,
    #[serde(flatten)]
    pagination: Pagination,
}

#[utoipa::path(
    get,
    path = HOT_TAGS_ROUTE,
    params(
        ("taggers_limit" = Option<usize>, Query, description = "Retrieve N user_id for each tag"),
        ("skip" = Option<usize>, Query, description = "Skip N tags"),
        ("limit" = Option<usize>, Query, description = "Retrieve N tag"),
        ("from" = Option<i64>, Query, description = "Retrieve hot tags from this timestamp"),
        ("to" = Option<i64>, Query, description = "Retrieve hot tags up to this timestamp"),
        ("tagged_type" = Option<TaggedType>, Query, description = "Retrieve hot tags by the type of entities tagged with it"),
    ),
    tag = "Global hot Tags",
    responses(
        // TODO: Add hot tags
        (status = 200, description = "Retrieve hot tags", body = Vec<HotTag>),
        (status = 404, description = "Hot tags not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn hot_tags_handler(Query(query): Query<HotTagsQuery>) -> Result<Json<HotTags>> {
    info!(
        "GET {HOT_TAGS_ROUTE} skip:{:?}, limit:{:?}, taggers_limit: {:?}, tagged_type: {:?}, from: {:?}, to: {:?}",
        query.pagination.skip, query.pagination.limit, query.taggers_limit, query.tagged_type, query.from, query.to
    );

    let skip = query.pagination.skip.unwrap_or(0);
    let limit = query.pagination.limit.unwrap_or(40);
    let taggers_limit = query.taggers_limit.unwrap_or(20);
    let from = query.from.unwrap_or(0);
    let to = query.to.unwrap_or(Utc::now().timestamp_millis());
    let tagged_type = query.tagged_type;

    let input = HotTagsInput {
        from,
        to,
        skip,
        limit,
        taggers_limit,
        tagged_type,
    };

    match HotTags::get_global_hot_tags(&input).await {
        Ok(Some(hot_tags)) => Ok(Json(hot_tags)),
        Ok(None) => Err(Error::TagsNotFound {
            reach: String::from("GLOBAL"),
        }),
        Err(source) => Err(Error::InternalServerError { source }),
    }
}

#[derive(Deserialize, Debug)]
pub struct TagTaggersQuery {
    pagination: Pagination,
    user_id: Option<String>,
    reach: Option<TagStreamReach>,
}

#[utoipa::path(
    get,
    path = TAG_TAGGERS_ROUTE,
    tag = "Tag Taggers",
    params(
        ("label" = String, Path, description = "Tag name"),
        ("reach" = TagStreamReach, Path, description = "Reach type: Follower | Following | Friends"),
        ("user_id" = Option<String>, Query, description = "User ID to base reach on"),
    ),
    responses(
        (status = 200, description = "Taggers", body = Vec<String>),
        (status = 404, description = "Tag not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn tag_taggers_handler(
    Path(label): Path<String>,
    Query(query): Query<TagTaggersQuery>,
) -> Result<Json<Vec<String>>> {
    info!(
        "GET {TAG_TAGGERS_ROUTE} label:{}, query: {:?}",
        label, query
    );

    match TagGlobal::get_tag_taggers(
        label.clone(),
        query.user_id,
        query.reach,
        query.pagination.skip,
        query.pagination.limit,
    )
    .await
    {
        Ok(Some(post)) => Ok(Json(post)),
        Ok(None) => Err(Error::TagsNotFound { reach: label }),
        Err(source) => Err(Error::InternalServerError { source }),
    }
}

#[derive(Deserialize)]
pub struct TagsByReachPath {
    user_id: String,
    reach: Option<TagStreamReach>,
}

#[utoipa::path(
    get,
    path = HOT_TAGS_BY_REACH_ROUTE,
    tag = "Hot Tags by reach",
    params(
        ("user_id" = String, Path, description = "User Pubky ID"),
        ("reach" = TagStreamReach, Path, description = "Reach type: Follower | Following | Friends"),
        ("taggers_limit" = Option<usize>, Query, description = "Retrieve N user_id for each tag"),
        ("skip" = Option<usize>, Query, description = "Skip N tags"),
        ("limit" = Option<usize>, Query, description = "Retrieve N tag"),
        ("from" = Option<i64>, Query, description = "Retrieve hot tags from this timestamp"),
        ("to" = Option<i64>, Query, description = "Retrieve hot tags up to this timestamp"),
        ("tagged_type" = Option<TaggedType>, Query, description = "Retrieve hot tags by the type of entities tagged with it"),
    ),
    responses(
        (status = 200, description = "Retrieve tags by reach cluster", body = Vec<HotTag>),
        (status = 404, description = "Hot tags not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn tags_by_reach_handler(
    Path(path): Path<TagsByReachPath>,
    Query(query): Query<HotTagsQuery>,
) -> Result<Json<HotTags>> {
    info!(
        "GET {HOT_TAGS_BY_REACH_ROUTE} user_id: {:?}, reach: {:?}, query: {:?}",
        path.user_id, path.reach, query
    );

    let reach = path.reach.unwrap_or(TagStreamReach::Following);
    let user_id = path.user_id;

    let skip = query.pagination.skip.unwrap_or(0);
    let limit = query.pagination.limit.unwrap_or(40);
    let taggers_limit = query.taggers_limit.unwrap_or(20);
    let from = query.from.unwrap_or(0);
    let to = query.to.unwrap_or(Utc::now().timestamp_millis());
    let tagged_type = query.tagged_type;

    let input = HotTagsInput {
        from,
        to,
        skip,
        limit,
        taggers_limit,
        tagged_type,
    };

    match HotTags::get_hot_tags_by_reach(user_id, reach, &input).await {
        Ok(Some(hot_tags)) => Ok(Json(hot_tags)),
        Ok(None) => Ok(Json(HotTags(vec![]))),
        Err(source) => {
            error!("Internal Server ERROR: {:?}", source);
            Err(Error::InternalServerError { source })
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(hot_tags_handler, tag_taggers_handler, tags_by_reach_handler),
    components(schemas(HotTags, HotTag, Taggers))
)]
pub struct TagGlobalApiDoc;
