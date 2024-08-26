use crate::db::connectors::neo4j::get_neo4j_graph;
use crate::{queries, RedisOps};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::UserStream;

/// Represents total counts of relationships of a user.
#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct UserCounts {
    pub tags: u32,
    pub posts: u32,
    pub following: u32,
    pub followers: u32,
    pub friends: u32,
}

impl RedisOps for UserCounts {}

impl Default for UserCounts {
    fn default() -> Self {
        Self::new()
    }
}

impl UserCounts {
    pub fn new() -> Self {
        Self {
            tags: 0,
            posts: 0,
            followers: 0,
            following: 0,
            friends: 0,
        }
    }

    /// Retrieves counts by user ID, first trying to get from Redis, then from Neo4j if not found.
    pub async fn get_by_id(
        user_id: &str,
    ) -> Result<Option<UserCounts>, Box<dyn std::error::Error + Send + Sync>> {
        match Self::try_from_index_json(&[user_id]).await? {
            Some(counts) => Ok(Some(counts)),
            None => Self::get_from_graph(user_id).await,
        }
    }

    /// Retrieves the counts from Neo4j.
    pub async fn get_from_graph(
        user_id: &str,
    ) -> Result<Option<UserCounts>, Box<dyn std::error::Error + Send + Sync>> {
        let mut result;
        {
            let graph = get_neo4j_graph()?;
            let query = queries::user_counts(user_id);

            let graph = graph.lock().await;
            result = graph.execute(query).await?;
        }

        if let Some(row) = result.next().await? {
            if !row.get("user_exists").unwrap_or(false) {
                return Ok(None);
            }
            let counts = Self {
                following: row.get("following_count").unwrap_or_default(),
                followers: row.get("followers_count").unwrap_or_default(),
                friends: row.get("friends_count").unwrap_or_default(),
                posts: row.get("posts_count").unwrap_or_default(),
                tags: row.get("tags_count").unwrap_or_default(),
            };
            counts.put_index_json(&[user_id]).await?;
            UserStream::add_to_mostfollowed_sorted_set(user_id, &counts).await?;
            Ok(Some(counts))
        } else {
            Ok(None)
        }
    }

    pub async fn delete(user_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Delete user_details on Redis
        Self::remove_from_index_multiple_json(&[&[user_id]]).await?;

        Ok(())
    }
}
