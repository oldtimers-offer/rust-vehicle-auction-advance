use actix_web::{test, web, App, http, http::StatusCode};
use actix_web::http::header::HeaderValue;
use sqlx::PgPool;
use redis::Client;
use chrono::NaiveDateTime;
use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
use serde_json::json;


// Import the handlers and models
use vehicle_auctions::{routes::{auction::{create_auction, place_bid, close_auction}, user::user_login, vehicle::{create_vehicle, list_vehicles}}, 
    models::{CreateAuction, CreateVehicle, Vehicle, PlaceBid, Auction}}; // Replace `your_crate_name` with your actual crate name.

#[actix_web::test]
async fn test_create_auction() {
    // Setup test database
    dotenv::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to database");

    // sqlx::query!("DELETE FROM users").execute(&pool).await.unwrap();
    // sqlx::query!("DELETE FROM vehicles").execute(&pool).await.unwrap();
    // sqlx::query!("DELETE FROM auctions").execute(&pool).await.unwrap();

    let salt = SaltString::generate(&mut rand::thread_rng());
    let hashed_password = Argon2::default()
        .hash_password("password7".as_bytes(), &salt)
        .unwrap()
        .to_string();

    // Insert a mock user
    sqlx::query!(
        "INSERT INTO users (username, password) VALUES ($1, $2)",
        "test_user_auction",
        hashed_password
    ).execute(&pool)
    .await
    .unwrap();

    // Setup Redis mock
    let redis_client_url = std::env::var("REDIS_URL").expect("REDIS_URL must be set");
    let redis_client = Client::open(redis_client_url).expect("Failed to connect to Redis");

    // Create an Actix test app
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(redis_client.clone()))
            .route("/login", web::post().to(user_login))
            .route("/create_vehicle", web::post().to(create_vehicle))
            .route("/list_vehicles", web::get().to(list_vehicles))
            .route("/create_auction", web::post().to(create_auction))
            .route("/place_bid", web::post().to(place_bid))
            .route("/close/{id}", web::post().to(close_auction)),
    )
    .await;

     

    let req = test::TestRequest::post()
            .uri("/login")
            .set_json(json!({ "username": "test_user_auction", "password": "password7" }))
            .to_request();

    let resp = test::call_service(&app, req).await;
    println!("Response1: {:?}", resp);
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
    println!("Response2: {:?}", resp2);
    assert_eq!(resp2.status(), 200, "Expected HTTP 200 OK");
    let body2 = test::read_body(resp2).await;
    assert_eq!(body2, "Vehicle Created");

    let req4 = test::TestRequest::get().uri("/list_vehicles").to_request();
    let resp4 = test::call_service(&app, req4).await;

    println!("Response4: {:?}", resp4);
    assert_eq!(resp4.status(), 200, "Expected 200 OK");
    let vehicles: Vec<Vehicle> = test::read_body_json(resp4).await;

    let id_vehicle =vehicles[0].id;
    println!("Vehicle ID: {:?}", id_vehicle);
    let end_time_str = "2025-01-28 15:00:00";
    let end_time2 = NaiveDateTime::parse_from_str(end_time_str, "%Y-%m-%d %H:%M:%S")
        .expect("Failed to parse end_time");
    println!("End time: {:?}", end_time2);

    // Prepare test data
    let create_auction_data = CreateAuction {
        vehicle_id: id_vehicle,
        starting_price: 1200.02,
        end_time: end_time2
    };

    // Send a test request
    let req3 = test::TestRequest::post()
        .uri("/create_auction")
        .insert_header((http::header::CONTENT_TYPE, "application/json"))
        .insert_header((
            "Session-Code",
            HeaderValue::from_str(&session_key).unwrap(), // Convert String to HeaderValue
        ))
        .set_json(&create_auction_data)
        .to_request();

    // Call the service
    let resp3 = test::call_service(&app, req3).await;

    // Assert the response
    println!("Response3: {:?}", resp3);
    assert_eq!(resp3.status(), 200, "Expected 200 OK");
    let body3 = test::read_body(resp3).await;
    assert_eq!(body3, "Auction Created");

    let salt2 = SaltString::generate(&mut rand::thread_rng());
    let hashed_password2 = Argon2::default()
        .hash_password("password8".as_bytes(), &salt2)
        .unwrap()
        .to_string();

     // Insert a mock user
     sqlx::query!(
        "INSERT INTO users (username, password) VALUES ($1, $2)",
        "test_user_auction_2",
        hashed_password2
    ).execute(&pool)
    .await
    .unwrap();

    let req5 = test::TestRequest::post()
    .uri("/login")
    .set_json(json!({ "username": "test_user_auction_2", "password": "password8" }))
    .to_request();

    let resp5 = test::call_service(&app, req5).await;
    assert_eq!(resp5.status(), 200);

    let body5: String = test::read_body_json(resp5).await;
    assert!(body5.contains("Login successful"));

    // Verify session in Redis
    let session_key2 = format!("{}", body5.split(':').last().unwrap().trim());

    // let id_auction:(i32, bool) = sqlx::query_as(
    //     "SELECT id, closed FROM auctions WHERE vehicle_id = $1 AND closed = FALSE"
    // )
    // .bind(id_vehicle)
    // .execute(&pool)
    // .await
    // .expect("Failed to query auctions");

    // let id_auction: Option<(i32, bool)> = sqlx::query_as(
    //     "SELECT id, closed FROM auctions WHERE vehicle_id = $1 AND closed = FALSE"
    // )
    // .bind(id_vehicle)
    // .fetch_optional(&pool)
    // .await
    // .unwrap();

    let id_auction = sqlx::query_as!(Auction,
        "SELECT id, COALESCE(closed, FALSE) as closed FROM auctions WHERE vehicle_id = $1 AND closed = FALSE",
        id_vehicle)
        .fetch_one(&pool)
        .await
        .unwrap();

    println!("ID Auction: {:?}", id_auction.id);
    // Prepare test data
    let create_bid = PlaceBid {
        auction_id: id_auction.id, 
        bid_amount: 1750.04
    };

    // Send a test request
    let req6 = test::TestRequest::post()
        .uri("/place_bid")
        .insert_header((http::header::CONTENT_TYPE, "application/json"))
        .insert_header((
            "Session-Code",
            HeaderValue::from_str(&session_key2).unwrap(), // Convert String to HeaderValue
        ))
        .set_json(&create_bid)
        .to_request();

    let resp6 = test::call_service(&app, req6).await;
    println!("Response 6: {:?}", resp6);
    dbg!(&resp6); 
    assert_eq!(resp6.status(), StatusCode::OK);

    let body6 = test::read_body(resp6).await;
    assert_eq!(body6, "Bid Placed");

    // Send a test request
    let req7 = test::TestRequest::post()
        .uri(&format!("/close/{}", id_auction.id))
        .insert_header((http::header::CONTENT_TYPE, "application/json"))
        .insert_header((
            "Session-Code",
            HeaderValue::from_str(&session_key).unwrap(), // Convert String to HeaderValue
        ))
        .to_request();

    let resp7 = test::call_service(&app, req7).await;
    println!("Response 7: {:?}", resp7);
    dbg!(&resp7); 
    assert_eq!(resp7.status(), StatusCode::OK);

    let body7 = test::read_body(resp7).await;
    assert_eq!(body7, "Auction Closed and Vehicle Ownership Transferred");
}
