use anyhow::Result;

use super::{DbClient, NewServer, ServerConnectionInfo, ServerRecord};

impl DbClient {
    pub async fn insert_server(&self, server: &NewServer) -> Result<()> {
        sqlx::query(
            "INSERT INTO servers (id, name, ip_address, status, secret_key) VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(&server.id)
        .bind(&server.name)
        .bind(&server.ip_address)
        .bind(&server.status)
        .bind(&server.secret_key)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn upsert_server(&self, server: &NewServer) -> Result<()> {
        sqlx::query(
            "INSERT INTO servers (id, name, ip_address, status, secret_key) VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(id) DO UPDATE SET
              name = excluded.name,
              ip_address = excluded.ip_address,
              status = excluded.status,
              secret_key = excluded.secret_key",
        )
        .bind(&server.id)
        .bind(&server.name)
        .bind(&server.ip_address)
        .bind(&server.status)
        .bind(&server.secret_key)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_server_secret(&self, server_id: &str) -> Result<Option<String>> {
        let secret =
            sqlx::query_scalar::<_, String>("SELECT secret_key FROM servers WHERE id = ?1")
                .bind(server_id)
                .fetch_optional(&self.pool)
                .await?;

        Ok(secret)
    }

    pub async fn list_servers(&self) -> Result<Vec<ServerRecord>> {
        let rows = sqlx::query_as::<_, (String, String, String, String)>(
            "SELECT id, name, ip_address, status FROM servers ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(id, name, ip_address, status)| ServerRecord {
                id,
                name,
                ip_address,
                status,
            })
            .collect())
    }

    pub async fn get_server_connection_info(
        &self,
        server_id: &str,
    ) -> Result<Option<ServerConnectionInfo>> {
        let row = sqlx::query_as::<_, (String, String, String)>(
            "SELECT id, ip_address, secret_key FROM servers WHERE id = ?1",
        )
        .bind(server_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(
            row.map(|(id, ip_address, secret_key)| ServerConnectionInfo {
                id,
                ip_address,
                secret_key,
            }),
        )
    }
}
