use super::{PostCounts, PostDetails, PostView};
use crate::{db::kv::index::sorted_sets::Sorting, RedisOps};
use serde::{Deserialize, Serialize};
use std::error::Error;
use tokio::task::spawn;
use utoipa::ToSchema;

const POST_TIMELINE_KEY_PARTS: [&str; 3] = ["Posts", "Global", "Timeline"];
const POST_TOTAL_ENGAGEMENT_KEY_PARTS: [&str; 3] = ["Posts", "Global", "TotalEngagement"];
const POST_PER_USER_KEY_PARTS: [&str; 2] = ["Posts", "User"];

#[derive(Deserialize, ToSchema)]
pub enum PostStreamSorting {
    Timeline,
    TotalEngagement,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct PostStream(Vec<PostView>);

impl RedisOps for PostStream {}

impl Default for PostStream {
    fn default() -> Self {
        Self::new()
    }
}

impl PostStream {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub async fn get_global_posts(
        sorting: PostStreamSorting,
        viewer_id: Option<String>,
        skip: Option<isize>,
        limit: Option<isize>,
    ) -> Result<Option<Self>, Box<dyn Error + Send + Sync>> {
        let posts_sorted_set = match sorting {
            PostStreamSorting::TotalEngagement => {
                Self::try_from_index_sorted_set(
                    &POST_TOTAL_ENGAGEMENT_KEY_PARTS,
                    None,
                    None,
                    skip,
                    limit,
                    Sorting::Descending,
                )
                .await?
            }
            PostStreamSorting::Timeline => {
                Self::try_from_index_sorted_set(
                    &POST_TIMELINE_KEY_PARTS,
                    None,
                    None,
                    skip,
                    limit,
                    Sorting::Descending,
                )
                .await?
            }
        };

        match posts_sorted_set {
            Some(post_keys) => {
                let post_keys: Vec<String> = post_keys.into_iter().map(|(key, _)| key).collect();
                Self::from_listed_post_ids(viewer_id, &post_keys).await
            }
            None => Ok(None),
        }
    }

    pub async fn get_user_posts(
        user_id: &str,
        viewer_id: Option<String>,
        skip: Option<isize>,
        limit: Option<isize>,
    ) -> Result<Option<Self>, Box<dyn Error + Send + Sync>> {
        let key_parts = [&POST_PER_USER_KEY_PARTS[..], &[user_id]].concat();
        let post_ids = Self::try_from_index_sorted_set(
            &key_parts,
            None,
            None,
            skip,
            limit,
            Sorting::Descending,
        )
        .await?;

        if let Some(post_ids) = post_ids {
            let post_keys: Vec<String> = post_ids
                .into_iter()
                .map(|(post_id, _)| format!("{}:{}", user_id, post_id))
                .collect();

            Self::from_listed_post_ids(viewer_id, &post_keys).await
        } else {
            Ok(None)
        }
    }

    pub async fn from_listed_post_ids(
        viewer_id: Option<String>,
        post_keys: &[String],
    ) -> Result<Option<Self>, Box<dyn std::error::Error + Send + Sync>> {
        // TODO: potentially we could use a new redis_com.mget() with a single call to retrieve all
        // post views at once and build the postss on the fly.
        // But still, using tokio to create them concurrently has VERY high performance.
        let viewer_id = viewer_id.map(|id| id.to_string());
        let mut handles = Vec::with_capacity(post_keys.len());

        for post_key in post_keys {
            let (author_id, post_id) = post_key.split_once(':').unwrap_or_default();
            let author_id = author_id.to_string();
            let viewer_id = viewer_id.clone();
            let post_id = post_id.to_string();
            let handle = spawn(async move {
                PostView::get_by_id(&author_id, &post_id, viewer_id.as_deref()).await
            });
            handles.push(handle);
        }

        let mut post_views = Vec::with_capacity(post_keys.len());

        for handle in handles {
            if let Some(post_view) = handle.await?? {
                post_views.push(post_view);
            }
        }

        Ok(Some(Self(post_views)))
    }

    /// Adds the post to a Redis sorted set using the `indexed_at` timestamp as the score.
    pub async fn add_to_timeline_sorted_set(
        details: &PostDetails,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let element = format!("{}:{}", details.author, details.id);
        let score = details.indexed_at as f64;
        Self::put_index_sorted_set(&POST_TIMELINE_KEY_PARTS, &[(score, element.as_str())]).await
    }

    /// Adds the post to a Redis sorted set using the `indexed_at` timestamp as the score.
    pub async fn add_to_per_user_sorted_set(
        details: &PostDetails,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let key_parts = [&POST_PER_USER_KEY_PARTS[..], &[details.author.as_str()]].concat();
        let score = details.indexed_at as f64;
        Self::put_index_sorted_set(&key_parts, &[(score, details.id.as_str())]).await
    }

    /// Adds the post to a Redis sorted set using the total engagement as the score.
    pub async fn add_to_engagement_sorted_set(
        counts: &PostCounts,
        author_id: &str,
        post_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let element = format!("{}:{}", author_id, post_id);
        let score = counts.tags + counts.replies + counts.reposts;
        let score = score as f64;

        Self::put_index_sorted_set(
            &POST_TOTAL_ENGAGEMENT_KEY_PARTS,
            &[(score, element.as_str())],
        )
        .await
    }
}
