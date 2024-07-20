use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{ProfileCounts, ProfileDetails, ProfileTag, Relationship};

/// Represents a Pubky user profile with relational data including tags, counts, and relationship with a viewer.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct ProfileView {
    details: ProfileDetails,
    counts: ProfileCounts,
    tags: Vec<ProfileTag>,
    viewer: Relationship,
}

impl Default for ProfileView {
    fn default() -> Self {
        Self::new()
    }
}

impl ProfileView {
    pub fn new() -> Self {
        Self {
            details: ProfileDetails::new(),
            counts: ProfileCounts::new(),
            tags: vec![ProfileTag::new()],
            viewer: Relationship::new(),
        }
    }

    /// Retrieves a profile by user ID, checking the cache first and then the graph database.
    pub async fn get_by_id(
        user_id: &str,
        viewer_id: Option<&str>,
    ) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        // TODO: Figure how to get all concurrently without Axum complaining.
        let details = match ProfileDetails::get_by_id(user_id).await? {
            Some(details) => details,
            None => return Ok(None),
        };
        let counts = ProfileCounts::get_by_id(user_id).await?;
        let viewer = Relationship::get_by_id(user_id, viewer_id)
            .await?
            .unwrap_or_default();

        Ok(Some(Self {
            details,
            counts,
            viewer,
            tags: vec![ProfileTag::new()], //TODO
        }))
    }
}
