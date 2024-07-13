//! tests/api/helpers.rs

use anyhow::Error;
use argon2::password_hash::SaltString;
use argon2::{Algorithm, Argon2, Params, PasswordHasher, Version};
use async_once_cell::OnceCell;
use ddc_tournement_planer::{
    configuration::{get_configuration, DatabaseSettings},
    domain::UserEmail,
    email_client::EmailClient,
    startup::{get_connection_pool, Application},
    telemetry::{get_subscriber, init_subscriber},
};
use lazy_static::lazy_static;
use once_cell::sync::Lazy;
use reqwest::Url;
use sqlx::{Connection, Executor, PgConnection, PgPool, Row};
use uuid::Uuid;
use wiremock::MockServer;

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();
    // We cannot assign the output of `get_subscriber` to a variable based on the
    // value TEST_LOG` because the sink is part of the type returned by
    // `get_subscriber`, therefore they are not the same type. We could work around
    // it, but this is the most straight-forward way of moving forward.
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    }
});

lazy_static! {
    static ref CLEANUP_DB: OnceCell<Result<(), Error>> = OnceCell::new();
}

pub struct TestUser {
    pub user_id: Uuid,
    pub username: String,
    pub password: String,
}

impl TestUser {
    pub fn generate() -> Self {
        Self {
            user_id: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
        }
    }

    async fn store(&self, pool: &PgPool) {
        let salt = SaltString::generate(&mut rand::thread_rng());
        // We don't care about the exact Argon2 parameters here
        // given that it's for testing purposes!
        let password_hash = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(15_000, 2, 1, None).unwrap(),
        )
        .hash_password(self.password.as_bytes(), &salt)
        .unwrap()
        .to_string();
        sqlx::query!(
            "INSERT INTO users (user_id, username, password_hash)
            VALUES ($1, $2, $3)",
            self.user_id,
            self.username,
            password_hash,
        )
        .execute(pool)
        .await
        .expect("Failed to create test user.");
    }
    pub async fn login(&self, app: &TestApp) -> reqwest::Response {
        app.post_login(&serde_json::json!({
            "username": &self.username,
            "password": &self.password
        }))
        .await
    }
}

pub struct TestApp {
    pub address: String,
    pub port: u16,
    pub db_pool: PgPool,
    pub email_server: MockServer,
    pub test_user: TestUser,
    pub api_client: reqwest::Client,
    pub email_client: EmailClient,
    pub db_name: String,
    pub n_retries: u8,
    pub time_delta: chrono::TimeDelta,
}

impl TestApp {
    /// Extract the confirmation links embedded in the request to the email API.
    // ToDo: added this function for later use. Remove allow(dead_code), when function is in use or remove function.
    #[allow(dead_code)]
    pub fn get_email_links(&self, email_request: &wiremock::Request) -> SubscriberLinks {
        // Parse the body as JSON, starting from raw bytes
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();
        // Extract the link from one of the request fields.
        let get_link = |s: &str| {
            let confirmation_links: Vec<reqwest::Url> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .map(|l| reqwest::Url::parse(l.as_str()).unwrap())
                .filter(|l| l.path() == "/subscriptions/confirm")
                .collect();
            let unsubscribe_links: Vec<reqwest::Url> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .map(|l| reqwest::Url::parse(l.as_str()).unwrap())
                .filter(|l| l.path() == "/subscriptions/unsubscribe")
                .collect();
            assert_eq!(unsubscribe_links.len(), 1);
            assert!(confirmation_links.len() <= 1);
            let link_transform = |mut l: Url| {
                // Let's make sure we don't call random APIs on the web
                assert_eq!(l.host_str().unwrap(), "127.0.0.1");
                // Let's rewrite the URL to include the port
                l.set_port(Some(self.port)).unwrap();
                l
            };
            let confirmation = if confirmation_links.len() == 1 {
                Some(link_transform(confirmation_links[0].clone()))
            } else {
                None
            };
            let unsubscribe = link_transform(unsubscribe_links[0].clone());
            EmailLinks {
                confirmation,
                unsubscribe,
            }
        };

        let html = get_link(&body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(&body["TextBody"].as_str().unwrap());
        SubscriberLinks { html, plain_text }
    }

    /// follow a email link
    // ToDo: added this function for later use. Remove allow(dead_code), when function is in use or remove function.
    #[allow(dead_code)]
    pub async fn click_email_link(&self, email_link: Url) -> reqwest::Response {
        self.api_client
            .get(email_link)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    /// Extract the reciever email from the request to the email API.
    // ToDo: added this function for later use. Remove allow(dead_code), when function is in use or remove function.
    #[allow(dead_code)]
    pub fn get_reciever_email(&self, email_request: &wiremock::Request) -> UserEmail {
        // Parse the body as JSON, starting from raw bytes
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();
        // get reciever from body
        let reciever_email = body["To"].as_str().unwrap();
        let reciever_email = UserEmail::parse(reciever_email.to_owned()).unwrap();
        reciever_email
    }

    /// helper for sending a POST /login request
    pub async fn post_login<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .post(&format!("{}/login", &self.address))
            // This 'reqwest' method makes sure that the body is URL-encoded
            // and the 'Content-Type' header is set accordingly.
            .form(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    /// helper to get Response from url
    pub async fn get_response_from_url(&self, path: &str) -> reqwest::Response {
        self.api_client
            .get(&format!("{}{}", self.address, path))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    /// helper to get login html
    // Out tests will only look at the HTML page, therefore
    // we do not expose the underlying reqwest::Response
    pub async fn get_login_html(&self) -> String {
        self.get_response_from_url("/login")
            .await
            .text()
            .await
            .unwrap()
    }

    /// helper to get restricted dashboard
    pub async fn get_restricted_dashboard(&self) -> reqwest::Response {
        self.get_response_from_url("/restricted/dashboard").await
    }

    /// helper to get restricted dashboard html
    pub async fn get_restricted_dashboard_html(&self) -> String {
        self.get_restricted_dashboard().await.text().await.unwrap()
    }

    /// helper to get restricted change password
    pub async fn get_change_password(&self) -> reqwest::Response {
        self.get_response_from_url("/restricted/password").await
    }

    /// helper to get restricted change password html
    pub async fn get_change_password_html(&self) -> String {
        self.get_change_password().await.text().await.unwrap()
    }

    /// helper to change restricted password
    pub async fn post_change_password<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .post(&format!("{}/restricted/password", self.address))
            .form(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    /// helper to log out
    pub async fn post_logout(&self) -> reqwest::Response {
        self.api_client
            .post(&format!("{}/restricted/logout", self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    // ToDo: added this function for later use. Remove allow(dead_code), when function is in use or remove function.
    #[allow(dead_code)]
    pub async fn num_rows_of_table(&self, table_name: &str) -> i64 {
        // Prepare the query to count rows in the specified table
        let query = format!("SELECT COUNT(*) as count FROM {}", table_name);

        // Execute the query
        let row = sqlx::query(&query).fetch_one(&self.db_pool).await.unwrap();

        // Extract the count from the row
        row.get("count")
    }
}

// Little helper function to assert redirected location
pub fn assert_is_redirect_to(response: &reqwest::Response, location: &str) {
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("Location").unwrap(), location);
}

/// Spin up an instance of our application
/// and returns its address (i.e. http://localhost:XXXX)
pub async fn spawn_app() -> TestApp {
    // The first time `initialize` is invoked the code in `TRACING` is executed.
    // All other invocations will instead skip execution.
    Lazy::force(&TRACING);
    if let Err(r) = CLEANUP_DB.get_or_init(cleanup_db()).await {
        panic!("clean up of test databases failed:\n{}", r);
    }

    // Launch a mock server to stand in for Postmark's API
    let email_server = MockServer::start().await;

    // Randomise configuration to ensure test isolation
    let configuration = {
        let mut c = get_configuration().expect("Failed to read configuration.");
        // use different database for each test case
        c.database.database_name = Uuid::new_v4().to_string();
        // use a random OS port
        c.application.port = 0;
        // use the mock server as email API
        c.emailclient.base_url = email_server.uri();
        // reduce n_retries to shorten test time
        c.emailclient.n_retries = 3;
        // reduce execute_retry_after_milliseconds to 1000ms to shorten test time
        c.emailclient.execute_retry_after_milliseconds = 1000;
        c
    };

    // Create and migrate the database
    configure_database(&configuration.database).await;

    let application = Application::build(configuration.clone())
        .await
        .expect("Failed to build application");
    let application_port = application.port();
    let _ = tokio::spawn(application.run_until_stopped());

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .cookie_store(true)
        .build()
        .unwrap();

    let time_delta = chrono::TimeDelta::milliseconds(
        configuration.emailclient.execute_retry_after_milliseconds as i64,
    );

    let test_app = TestApp {
        address: format!("http://127.0.0.1:{}", application_port),
        port: application_port,
        db_pool: get_connection_pool(&configuration.database),
        email_server,
        test_user: TestUser::generate(),
        api_client: client,
        n_retries: configuration.emailclient.n_retries,
        email_client: configuration.emailclient.client(),
        db_name: configuration.database.database_name,
        time_delta,
    };
    test_app.test_user.store(&test_app.db_pool).await;
    test_app
}

async fn configure_database(config: &DatabaseSettings) -> PgPool {
    // Create database
    let mut connection = PgConnection::connect_with(&config.without_db())
        .await
        .expect("Failed to connect to Postgres");

    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database.");

    // Migrate database
    let connection_pool = PgPool::connect_with(config.with_db())
        .await
        .expect("Failed to connect to Psotgres.");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database.");

    connection_pool
}

async fn cleanup_db() -> Result<(), Error> {
    let database = get_configuration()?.database;
    // Connect to postgres without db
    let mut connection = PgConnection::connect_with(&database.without_db()).await?;

    let rows = connection
        .fetch_all("SELECT datname FROM pg_database WHERE datistemplate = false")
        .await?;

    for row in rows {
        let database_name: String = row.try_get("datname")?;
        if Uuid::parse_str(&database_name).is_ok() {
            // database is Uuid -> test database -> delete it
            let query: &str = &format!(r#"DROP DATABASE IF EXISTS "{}" ( FORCE ) "#, database_name);
            connection.execute(query).await?;
        }
    }
    Ok(())
}

/// Confirmation and unsubscribe links embedded in the request to the email API.
#[derive(PartialEq, Eq, Debug)]
pub struct EmailLinks {
    pub confirmation: Option<reqwest::Url>,
    pub unsubscribe: reqwest::Url,
}

/// Links embedded in the request to the email API in Html and text of email.
#[derive(PartialEq, Eq, Debug)]
pub struct SubscriberLinks {
    pub html: EmailLinks,
    pub plain_text: EmailLinks,
}
