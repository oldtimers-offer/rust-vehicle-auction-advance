use actix_web::{test, App, web};
use actix_web::http::header::HeaderValue;
use redis::Client;
use sqlx::{PgPool, Executor};
use vehicle_auctions::routes::vehicle::{create_vehicle, list_vehicles}; // Replace with your app module path
use vehicle_auctions::models::{CreateVehicle, Vehicle};
use vehicle_auctions::routes::user::user_login;
use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
use serde_json::json;
use bigdecimal::{BigDecimal, FromPrimitive};

#[actix_web::test]
async fn test_create_vehicle_success() {
    // Setup test database connection
    dotenv::dotenv().ok();
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let pool = PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to database");

       // sqlx::query!("DELETE FROM users").execute(&pool).await.unwrap();

        let salt = SaltString::generate(&mut rand::thread_rng());
        let hashed_password = Argon2::default()
            .hash_password("password5".as_bytes(), &salt)
            .unwrap()
            .to_string();

    // Insert a mock user
    sqlx::query!(
        "INSERT INTO users (username, password) VALUES ($1, $2)",
        "testuser4",
        hashed_password
    ).execute(&pool)
    .await
    .unwrap();

    let redis_client_url = std::env::var("REDIS_URL").expect("REDIS_URL must be set");
    let redis_client = Client::open(redis_client_url).expect("Failed to connect to Redis");


    // Setup the app
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(redis_client.clone()))
            .route("/login", web::post().to(user_login))
            .route("/create_vehicle", web::post().to(create_vehicle)),
    )
    .await;

    let req = test::TestRequest::post()
            .uri("/login")
            .set_json(json!({ "username": "testuser4", "password": "password5" }))
            .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

        let body: String = test::read_body_json(resp).await;
        assert!(body.contains("Login successful"));

        // Verify session in Redis
        let session_key = format!("{}", body.split(':').last().unwrap().trim());
        

    // Mock request
    let form = CreateVehicle {
        name: "Test Vehicle".to_string(),
        description: "A vehicle for testing".to_string(),
        starting_price: 1000.01,
    };

    let req2 = test::TestRequest::post()
        .uri("/create_vehicle")
        .insert_header((
            "Session-Code",
            HeaderValue::from_str(&session_key).unwrap(), // Convert String to HeaderValue
        ))
        .set_json(&form)
        .to_request();

    let resp2 = test::call_service(&app, req2).await;

    // Assert response
    assert_eq!(resp2.status(), 200, "Expected HTTP 200 OK");
    let body2 = test::read_body(resp2).await;
    assert_eq!(body2, "Vehicle Created");
}


#[actix_web::test]
async fn test_list_vehicles_postgres() {
    // Setup PostgreSQL pool
    dotenv::dotenv().ok();
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let pool = PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to database");

    // Ensure the table exists
    pool.execute(
        r#"
        CREATE TABLE IF NOT EXISTS vehicles (
            id SERIAL PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT NOT NULL,
            starting_price NUMERIC(10, 2) NOT NULL
        );
    "#,
    )
    .await
    .unwrap();

   let starting_price1 = BigDecimal::from_f64(10000.0).unwrap();
   let starting_price2 = BigDecimal::from_f64(20000.0).unwrap();

   //sqlx::query!("DELETE FROM vehicles").execute(&pool).await.unwrap();

    // Insert mock vehicles
    sqlx::query!(
        "INSERT INTO vehicles (name, description, starting_price, owner_username) VALUES ($1, $2, $3, $4)",
        "Car 1",
        "Description 1",
        starting_price1,
        "testuser4"
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query!(
        "INSERT INTO vehicles (name, description, starting_price, owner_username) VALUES ($1, $2, $3, $4)",
        "Car 2",
        "Description 2",
        starting_price2,
        "testuser4"
    )
    .execute(&pool)
    .await
    .unwrap();

    // Initialize the app
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/list_vehicles", web::get().to(list_vehicles)),
    )
    .await;

    let req = test::TestRequest::get().uri("/list_vehicles").to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200, "Expected 200 OK");
    let vehicles: Vec<Vehicle> = test::read_body_json(resp).await;

    assert_eq!(vehicles.len(), 3);
    assert_eq!(vehicles[1].name, "Car 1");
    assert_eq!(vehicles[2].name, "Car 2");
}
