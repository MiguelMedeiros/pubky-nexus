use crate::db::connectors::redis::get_redis_conn;
use redis::AsyncCommands;
use std::error::Error;

/// Adds elements to a Redis sorted set.
///
/// This function adds elements to the specified Redis sorted set. If the set doesn't exist,
/// it creates a new sorted set.
///
/// # Argumentsf64
///
/// * `prefix` - A string slice representing the prefix for the Redis keys.
/// * `key` - A string slice representing the key under which the sorted set is stored.
/// * `values` - A slice of tuples where each tuple contains a reference to a string slice representing
///              the element and a f64 representing the score of the element.
///
/// # Errors
///
/// Returns an error if the operation fails.
pub async fn put(
    prefix: &str,
    key: &str,
    items: &[(f64, &str)],
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if items.is_empty() {
        return Ok(());
    }

    let index_key = format!("{}:{}", prefix, key);
    let mut redis_conn = get_redis_conn().await?;

    redis_conn.zadd_multiple(&index_key, items).await?;

    Ok(())
}

/// Retrieves a range of elements from a Redis sorted set.
///
/// This function retrieves elements from a specified Redis sorted set based on a score range.
/// The range is defined by `min_score` and `max_score` parameters, where `min_score` and `max_score`
/// specify the inclusive lower and upper bounds of the scores.
///
/// # Arguments
///
/// * `prefix` - A string slice representing the prefix for the Redis keys.
/// * `key` - A string slice representing the key under which the sorted set is stored.
/// * `min_score` - The minimum score for the range (inclusive).
/// * `max_score` - The maximum score for the range (inclusive).
/// * `limit` - The maximum number of elements to retrieve.
///
/// # Returns
///
/// Returns a vector of tuples containing the elements and their scores.
///
/// # Errors
///
/// Returns an error if the operation fails.
pub async fn get_range(
    prefix: &str,
    key: &str,
    min_score: Option<f64>,
    max_score: Option<f64>,
    limit: Option<usize>,
) -> Result<Option<Vec<(String, f64)>>, Box<dyn Error + Send + Sync>> {
    let mut redis_conn = get_redis_conn().await?;
    let index_key = format!("{}:{}", prefix, key);

    let min_score = min_score.unwrap_or(f64::MIN);
    let max_score = max_score.unwrap_or(f64::MAX);

    // ZRANGE with the WITHSCORES option retrieves both the elements and their scores
    let elements: Vec<(String, f64)> = redis_conn
        .zrangebyscore_withscores(index_key, min_score, max_score)
        .await?;

    match elements.len() {
        0 => Ok(None),
        _ => match limit {
            Some(l) => Ok(Some(elements.into_iter().take(l).collect())),
            None => Ok(Some(elements)),
        },
    }
}
