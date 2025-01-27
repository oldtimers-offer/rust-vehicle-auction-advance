// FIRST WAY OF TESTING

// #[actix_web::test]
// async fn test_user_register_success() {
//     // Set up an in-memory or test database
//     let pool = PgPool::connect("postgres://your_username:your_password@localhost:5434/vehicle_auctions")
//         .await
//         .expect("Failed to connect to database");

//     // Ensure the database is clean
//     sqlx::query!("DELETE FROM users").execute(&pool).await.unwrap();

//     let app = test::init_service(
//         App::new()
//             .app_data(web::Data::new(pool.clone()))
//             .route("/users/register", web::post().to(user_register)),
//     )
//     .await;

//     let req = test::TestRequest::post()
//         .uri("/users/register")
//         .set_json(json!({ "username": "testuser", "password": "testpassword" }))
//         .to_request();

//     let resp = test::call_service(&app, req).await;
//     assert_eq!(resp.status(), 200);

//     let body = test::read_body(resp).await;
//     assert_eq!(body, "User Registered");
// }

// #[actix_web::test]
// async fn test_user_register_existing_username() {
//     let pool = PgPool::connect("postgres://your_username:your_password@localhost:5434/vehicle_auctions")
//         .await
//         .expect("Failed to connect to database");

//     sqlx::query!("DELETE FROM users").execute(&pool).await.unwrap();
//     sqlx::query!("INSERT INTO users (username, password) VALUES ($1, $2)", "testuser2", "hashedpassword")
//         .execute(&pool)
//         .await
//         .unwrap();

//     let app = test::init_service(
//         App::new()
//             .app_data(web::Data::new(pool.clone()))
//             .route("/users/register", web::post().to(user_register)),
//     )
//     .await;

//     let req = test::TestRequest::post()
//         .uri("/users/register")
//         .set_json(json!({ "username": "testuser2", "password": "testpassword" }))
//         .to_request();

//     let resp = test::call_service(&app, req).await;
//     assert_eq!(resp.status(), 400);

//     let body = test::read_body(resp).await;
//     assert_eq!(body, "Username already taken");
// }

// #[actix_web::test]
// async fn test_user_login_success() {
//     let pool = PgPool::connect("postgres://your_username:your_password@localhost:5434/vehicle_auctions")
//         .await
//         .expect("Failed to connect to database");

//     sqlx::query!("DELETE FROM users").execute(&pool).await.unwrap();

//     let salt = SaltString::generate(&mut rand::thread_rng());
//     let hashed_password = Argon2::default()
//         .hash_password("testpassword".as_bytes(), &salt)
//         .unwrap()
//         .to_string();

//     sqlx::query!("INSERT INTO users (username, password) VALUES ($1, $2)", "testuser3", hashed_password)
//         .execute(&pool)
//         .await
//         .unwrap();

//     let redis_client = Client::open("redis://127.0.0.1:6380").expect("Failed to connect to Redis");
//     let mut redis_conn = redis_client.get_multiplexed_async_connection().await.unwrap();

//     let app = test::init_service(
//         App::new()
//             .app_data(web::Data::new(pool.clone()))
//             .app_data(web::Data::new(redis_client.clone()))
//             .route("/login", web::post().to(user_login)),
//     )
//     .await;

//     let req = test::TestRequest::post()
//         .uri("/login")
//         .set_json(json!({ "username": "testuser3", "password": "testpassword" }))
//         .to_request();

//     let resp = test::call_service(&app, req).await;
//     assert_eq!(resp.status(), 200);

//     let body: String = test::read_body_json(resp).await;
//     assert!(body.contains("Login successful"));

//     // Verify session in Redis
//     let session_key = format!("session:{}", body.split(':').last().unwrap().trim());
//     let username: String = redis_conn.get(session_key).await.unwrap();
//     assert_eq!(username, "testuser3");
// }



// SECOND WAY OF TESTING

#[cfg(test)]
mod tests {
    use actix_web::{test, web, App};
    use redis::Client;
    use sqlx::PgPool;
    use serde_json::json;
    use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
    use redis::AsyncCommands;
    use vehicle_auctions::routes::user::{user_register, user_login};
    

    #[actix_web::test]
    async fn test_user_register_success() {
        dotenv::dotenv().ok();
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let pool = PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to database");

        // Ensure the database is clean
        //sqlx::query!("DELETE FROM users").execute(&pool).await.unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .route("/users/register", web::post().to(user_register)),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/users/register")
            .set_json(json!({ "username": "testuser", "password": "testpassword" }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body = test::read_body(resp).await;
        assert_eq!(body, "User Registered");
    }

    #[actix_web::test]
    async fn test_user_register_existing_username() {
        dotenv::dotenv().ok();
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let pool = PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to database");

        //sqlx::query!("DELETE FROM users").execute(&pool).await.unwrap();
        sqlx::query!(
            "INSERT INTO users (username, password) VALUES ($1, $2)",
            "testuser2",
            "hashedpassword"
        )
        .execute(&pool)
        .await
        .unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .route("/users/register", web::post().to(user_register)),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/users/register")
            .set_json(json!({ "username": "testuser2", "password": "testpassword" }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);

        let body = test::read_body(resp).await;
        assert_eq!(body, "Username already taken");
    }

    #[actix_web::test]
    async fn test_user_login_success() {
        dotenv::dotenv().ok();
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let pool = PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to database");
        

        //sqlx::query!("DELETE FROM users").execute(&pool).await.unwrap();

        let salt = SaltString::generate(&mut rand::thread_rng());
        let hashed_password = Argon2::default()
            .hash_password("testpassword".as_bytes(), &salt)
            .unwrap()
            .to_string();

        sqlx::query!(
            "INSERT INTO users (username, password) VALUES ($1, $2)",
            "testuser3",
            hashed_password
        )
        .execute(&pool)
        .await
        .unwrap();
        let redis_client_url = std::env::var("REDIS_URL").expect("REDIS_URL must be set");
        let redis_client = Client::open(redis_client_url).expect("Failed to connect to Redis");
        let mut redis_conn = redis_client.get_multiplexed_async_connection().await.unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .app_data(web::Data::new(redis_client.clone()))
                .route("/login", web::post().to(user_login)),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/login")
            .set_json(json!({ "username": "testuser3", "password": "testpassword" }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let body: String = test::read_body_json(resp).await;
        assert!(body.contains("Login successful"));

        // Verify session in Redis
        let session_key = format!("session:{}", body.split(':').last().unwrap().trim());
        let username: String = redis_conn.get(session_key).await.unwrap();
        assert_eq!(username, "testuser3");
    }
}