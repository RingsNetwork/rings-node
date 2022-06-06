use crate::err::{Error, Result};
use async_trait::async_trait;
use redis::aio::Connection;
use redis::{cmd, AsyncCommands, Client, InfoDict, Value};
use serde::de::DeserializeOwned;
use serde_json::{self};
use std::collections::HashMap;
use std::hash::Hash;
use std::string::ToString;
use url::Url;

/// A cache that stores entries in a Redis.
#[derive(Clone)]
pub struct RedisCache {
    /// for display only: password (if any) will be masked
    pub display_url: String,
    client: Client,
}

/// RedisCache Taits about Storage
#[async_trait]
pub trait TRedisCacheStorage<K, V> {
    type K;
    type V;

    /// Get a cache entry by `key`.
    async fn get(&self, key: &Self::K) -> Result<Self::V>;

    /// Put `entry` in the cache under `key`.
    async fn put(&self, key: &Self::K, entry: &Self::V) -> Result<()>;

    /// Get the current storage usage, if applicable.
    async fn current_size(&self) -> Result<Option<u64>>;

    /// Get the maximum storage size, if applicable.
    async fn max_size(&self) -> Result<Option<u64>>;
}

impl RedisCache {
    /// Create a new `RedisCache`.
    pub fn new(url: &str) -> Result<RedisCache> {
        let mut parsed = Url::parse(url)?;
        // If the URL has a password set, mask it when displaying.
        if parsed.password().is_some() {
            let _ = parsed.set_password(Some("bns-redis-cache"));
        }
        Ok(RedisCache {
            display_url: parsed.to_string(),
            client: Client::open(url)?,
        })
    }

    /// Returns a connection with configured read and write timeouts.
    async fn connect(&self) -> Result<Connection> {
        Ok(self.client.get_async_connection().await?)
    }
}

#[async_trait]
impl<K, V> TRedisCacheStorage<K, V> for RedisCache
where
    K: Clone + Eq + Hash + ToString + std::marker::Sync,
    V: Clone + redis::ToRedisArgs + std::marker::Sync + DeserializeOwned,
{
    type K = K;
    type V = V;

    /// Open a connection and query for a key.
    async fn get(&self, key: &Self::K) -> Result<Self::V> {
        let cache_key = key.to_string();
        let mut con = self.connect().await?;
        match con.get(&cache_key).await? {
            Value::Nil => Err(Error::RedisCacheMiss),
            Value::Data(val) => serde_json::from_slice(&val).map_err(Error::Deserialize),
            _ => Err(Error::RedisInvalidKind),
        }
    }

    /// Open a connection and store a object in the cache.
    async fn put(&self, key: &Self::K, entry: &Self::V) -> Result<()> {
        let cache_key = key.to_string();
        let mut con = self.connect().await?;
        let _: () = redis::pipe()
            .atomic()
            .set(&cache_key, &entry)
            .expire(&cache_key, 60)
            .query_async(&mut con)
            .await?;
        Ok(())
    }

    /// Returns the current cache size. This value is aquired via
    /// the Redis INFO command (used_memory).
    async fn current_size(&self) -> Result<Option<u64>> {
        let mut con = self.connect().await?;
        let v: InfoDict = cmd("INFO").query_async(&mut con).await?;
        Ok(v.get("used_memory"))
    }

    /// Returns the maximum cache size. This value is read via
    /// the Redis CONFIG command (maxmemory). If the server has no
    /// configured limit, the result is None.
    async fn max_size(&self) -> Result<Option<u64>> {
        let mut con = self.connect().await?;
        let result: redis::RedisResult<HashMap<String, usize>> = cmd("CONFIG")
            .arg("GET")
            .arg("maxmemory")
            .query_async(&mut con)
            .await;
        match result {
            Ok(h) => Ok(h
                .get("maxmemory")
                .and_then(|&s| if s != 0 { Some(s as u64) } else { None })),
            Err(_) => Ok(None),
        }
    }
}
