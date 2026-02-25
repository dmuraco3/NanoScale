use super::*;

fn new_server(id: &str, secret: &str) -> NewServer {
    NewServer {
        id: id.to_string(),
        name: format!("server-{id}"),
        ip_address: "127.0.0.1".to_string(),
        status: "online".to_string(),
        secret_key: secret.to_string(),
    }
}

fn new_project(id: &str, server_id: &str, port: i64, domain: Option<&str>) -> NewProject {
    NewProject {
        id: id.to_string(),
        server_id: server_id.to_string(),
        name: format!("project-{id}"),
        repo_url: "https://example.com/repo.git".to_string(),
        branch: "main".to_string(),
        install_command: "bun install".to_string(),
        build_command: "bun run build".to_string(),
        start_command: "bun run start".to_string(),
        output_directory: ".next/standalone".to_string(),
        env_vars: "[]".to_string(),
        port,
        domain: domain.map(ToString::to_string),
        source_provider: "manual".to_string(),
        source_repo_id: None,
    }
}

async fn temp_db() -> DbClient {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let db_path = tempdir.path().join("nanoscale.db");
    // Keep tempdir alive by leaking it for the duration of the test (each test has its own).
    // This avoids DB file disappearing while async tasks are still using it.
    std::mem::forget(tempdir);

    DbClient::initialize(&db_path.to_string_lossy())
        .await
        .expect("db init")
}

#[tokio::test]
async fn initialize_runs_migrations_and_enables_wal() {
    let db = temp_db().await;
    db.ensure_wal_mode().await.expect("wal mode");
}

#[tokio::test]
async fn servers_insert_upsert_and_secret_lookup() {
    let db = temp_db().await;

    let server_id = "srv-1";
    db.insert_server(&new_server(server_id, "secret-a"))
        .await
        .expect("insert server");

    let secret = db
        .get_server_secret(server_id)
        .await
        .expect("get secret")
        .expect("secret should exist");
    assert_eq!(secret, "secret-a");

    db.upsert_server(&new_server(server_id, "secret-b"))
        .await
        .expect("upsert server");
    let secret = db
        .get_server_secret(server_id)
        .await
        .expect("get secret")
        .expect("secret should exist");
    assert_eq!(secret, "secret-b");

    let list = db.list_servers().await.expect("list servers");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, server_id);

    let connection = db
        .get_server_connection_info(server_id)
        .await
        .expect("connection info")
        .expect("should exist");
    assert_eq!(connection.id, server_id);
    assert_eq!(connection.secret_key, "secret-b");
}

#[tokio::test]
async fn users_insert_count_and_lookup() {
    let db = temp_db().await;
    assert_eq!(db.users_count().await.expect("users_count"), 0);

    let user = NewUser {
        id: "user-1".to_string(),
        username: "admin".to_string(),
        password_hash: "hash".to_string(),
    };
    db.insert_user(&user).await.expect("insert user");
    assert_eq!(db.users_count().await.expect("users_count"), 1);

    let found = db
        .find_user_by_username("admin")
        .await
        .expect("find")
        .expect("exists");
    assert_eq!(found.id, "user-1");
    assert_eq!(found.password_hash, "hash");
    assert!(db
        .find_user_by_username("missing")
        .await
        .expect("find")
        .is_none());
}

#[tokio::test]
async fn projects_crud_and_port_domain_checks() {
    let db = temp_db().await;
    db.insert_server(&new_server("srv-1", "secret"))
        .await
        .expect("insert server");

    let next = db.next_available_project_port().await.expect("next port");
    assert_eq!(next, DbClient::min_project_port());

    let project = new_project("p1", "srv-1", next, Some("app.example.com"));
    db.insert_project(&project).await.expect("insert project");

    assert!(db.is_project_port_in_use(next).await.expect("port in use"));
    assert!(db
        .is_project_domain_in_use("app.example.com")
        .await
        .expect("domain in use"));
    assert!(!db
        .is_project_domain_in_use("missing.example.com")
        .await
        .expect("domain check"));

    let listed = db.list_projects().await.expect("list projects");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, project.id);

    let details = db
        .get_project_by_id(&project.id)
        .await
        .expect("get project")
        .expect("exists");
    assert_eq!(details.id, project.id);
    assert_eq!(details.server_id, "srv-1");
    assert_eq!(details.domain.as_deref(), Some("app.example.com"));

    let next2 = db.next_available_project_port().await.expect("next port");
    assert_eq!(next2, next + 1);

    db.delete_project_by_id(&project.id).await.expect("delete");
    assert!(db
        .get_project_by_id(&project.id)
        .await
        .expect("get")
        .is_none());
}
