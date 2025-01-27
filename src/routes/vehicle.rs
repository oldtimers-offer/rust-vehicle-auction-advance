use sqlx::PgPool;
use actix_web::{web, Responder, HttpResponse, HttpRequest};
use crate::models::{CreateVehicle, Vehicle};
use redis::AsyncCommands;


pub async fn create_vehicle(
    pool: web::Data<PgPool>,
    redis_client: web::Data<redis::Client>,
    req: HttpRequest,
    form: web::Json<CreateVehicle>,
) -> impl Responder {
    // Extract the Session-Code from headers
    let session_code = match req.headers().get("Session-Code") {
        Some(code) => code.to_str().unwrap_or_default(),
        None => return HttpResponse::Unauthorized().body("Missing Session-Code header"),
    };

    // Connect to Redis
    let mut redis_conn = match redis_client.get_multiplexed_async_connection().await {
        Ok(conn) => conn,
        Err(_) => return HttpResponse::InternalServerError().body("Failed to connect to Redis"),
    };

     // Check if the session is valid
    // Retrieve the username associated with the session_code from Redis
    let user_name: Option<String> = match redis_conn
        .get(format!("session:{}", session_code))
        .await
    {
        Ok(owner_username) => owner_username,
        Err(_) => return HttpResponse::InternalServerError().body("Failed to retrieve username"),
    };

     // Check if the username exists
     let username_exists: (bool,) = match sqlx::query_as::<_, (bool,)>(
        "SELECT EXISTS(SELECT 1 FROM users WHERE username = $1)"
    )
    .bind(&user_name)
    .fetch_one(pool.as_ref())
    .await
    {
        Ok(result) => result,
        Err(_) => return HttpResponse::InternalServerError().body("Database query error"),
    };

    if !username_exists.0 {
        // Username not exists
        return HttpResponse::BadRequest().body("Username not exists");
    }

    

    match user_name {
        Some(_) => {
            // Session is valid, proceed to create the vehicle
            if let Err(_) = sqlx::query("INSERT INTO vehicles (name, description, starting_price, owner_username) VALUES ($1, $2, $3, $4)"
                )
                .bind(&form.name)
                .bind(&form.description)
                .bind(form.starting_price)
                .bind(&user_name)
                .execute(pool.as_ref())
                .await
            {
                return HttpResponse::InternalServerError().body("Failed to create vehicle");
            }

            HttpResponse::Ok().body("Vehicle Created")
        }
        None => {
            // Session is invalid or expired
            HttpResponse::Unauthorized().body("Invalid or expired session")
        }
    }
}

pub async fn list_vehicles(pool: web::Data<PgPool>) -> impl Responder {
    let result = sqlx::query_as!(
        Vehicle, 
        "SELECT id, name, description, starting_price FROM vehicles"
    )
    .fetch_all(pool.get_ref())
    .await;

    match result {
        Ok(vehicles) => HttpResponse::Ok().json(vehicles),
        Err(err) => {
            eprintln!("Error fetching vehicles: {:?}", err);
            HttpResponse::InternalServerError().body("Failed to fetch vehicles")
        }
    }
}

pub async fn delete_vehicle(
    pool: web::Data<PgPool>,
    redis_client: web::Data<redis::Client>,
    req: HttpRequest,
    path: web::Path<i32>,
) -> impl Responder {
    // Extract the Session-Code from headers
    let session_code = match req.headers().get("Session-Code") {
        Some(code) => code.to_str().unwrap_or_default(),
        None => return HttpResponse::Unauthorized().body("Missing Session-Code header"),
    };

    // Connect to Redis
    let mut redis_conn = match redis_client.get_multiplexed_async_connection().await {
        Ok(conn) => conn,
        Err(_) => return HttpResponse::InternalServerError().body("Failed to connect to Redis"),
    };

    // Retrieve the username associated with the session_code from Redis
    let user_name: Option<String> = match redis_conn
        .get(format!("session:{}", session_code))
        .await
    {
        Ok(owner_username) => owner_username,
        Err(_) => return HttpResponse::InternalServerError().body("Failed to retrieve username"),
    };

    let user_name = match user_name {
        Some(name) => name,
        None => return HttpResponse::Unauthorized().body("Invalid or expired session"),
    };

    // Check ownership
    let vehicle_id = *path;
    let owner_username: Option<String> = match sqlx::query_scalar!(
        "SELECT owner_username FROM vehicles WHERE id = $1",
        vehicle_id
    )
    .fetch_optional(pool.as_ref())
    .await
    {
        Ok(Some(username)) => Some(username),
        Ok(None) => return HttpResponse::NotFound().body("Vehicle not found"),
        Err(_) => return HttpResponse::InternalServerError().body("Database query error"),
    };

    if owner_username.as_deref() != Some(&user_name) {
        return HttpResponse::Forbidden().body("You are not the owner of this vehicle");
    }

    // Delete the vehicle
    match sqlx::query!("DELETE FROM vehicles WHERE id = $1", vehicle_id)
        .execute(pool.as_ref())
        .await
    {
        Ok(_) => HttpResponse::Ok().body("Vehicle Deleted"),
        Err(_) => HttpResponse::InternalServerError().body("Failed to delete vehicle"),
    }
}

