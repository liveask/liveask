use anyhow::{bail, Result};

pub async fn ping_test_redis(pool: &deadpool_redis::Pool) -> Result<()> {
    let mut db = pool.get().await?;

    let pong = redis::cmd("PING").query_async::<_, String>(&mut db).await?;

    if pong != "PONG" {
        bail!("redis ping failed: {pong}");
    }

    Ok(())
}

pub fn create_pool(url: &str) -> Result<deadpool_redis::Pool> {
    let cfg = deadpool_redis::Config {
        url: Some(url.to_string()),
        connection: None,
        // pool: Some(deadpool_redis::PoolConfig::new(2)),
        pool: Some(deadpool_redis::PoolConfig::default()),
    };

    let pool = cfg.create_pool(None)?;

    Ok(pool)
}
