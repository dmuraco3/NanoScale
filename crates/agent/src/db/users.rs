use anyhow::Result;

use super::{DbClient, NewUser, UserRecord};

impl DbClient {
    pub async fn users_count(&self) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?;

        Ok(count)
    }

    pub async fn insert_user(&self, user: &NewUser) -> Result<()> {
        sqlx::query("INSERT INTO users (id, username, password_hash) VALUES (?1, ?2, ?3)")
            .bind(&user.id)
            .bind(&user.username)
            .bind(&user.password_hash)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn find_user_by_username(&self, username: &str) -> Result<Option<UserRecord>> {
        let row = sqlx::query_as::<_, (String, String)>(
            "SELECT id, password_hash FROM users WHERE username = ?1",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|(id, password_hash)| UserRecord { id, password_hash }))
    }
}
