use actix_web::{web, App, HttpServer};
use sqlx::{Pool, Postgres};
use sqlx::migrate;
use redis::Client;
mod routes;
mod models;
use crate::routes::user::{user_register, user_login} ;
use crate::routes::vehicle::{create_vehicle, list_vehicles, delete_vehicle} ;
use crate::routes::auction::{create_auction, place_bid, close_auction} ;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load environment variables (e.g., database URL)
    dotenv::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let redis_client_url = std::env::var("REDIS_URL").expect("REDIS_URL must be set");

    // Initialize SQLx connection pool
    let pool = Pool::<Postgres>::connect(&database_url).await.expect("Failed to create pool");
    //let redis_client
    let redis_client = Client::open(redis_client_url).expect("Failed to create redis_client");

    // Run database migrations
    let _migrate = migrate!("./migrations").run(&pool).await.expect("Failed to run migrations");

    // Start Actix Web server
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(redis_client.clone()))
            .configure(routes)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

fn routes(cfg: &mut web::ServiceConfig) {
    cfg
        .service(web::scope("/users")
            .route("/register", web::post().to(user_register))
            .route("/login", web::post().to(user_login)))
        .service(web::scope("/vehicles")
            .route("/create", web::post().to(create_vehicle))
            .route("/list", web::get().to(list_vehicles))
            .route("/delete/{id}", web::delete().to(delete_vehicle)))
        .service(web::scope("/auctions")
            .route("/create", web::post().to(create_auction))
            .route("/bid", web::post().to(place_bid))
            .route("/close/{id}", web::post().to(close_auction)));
}
