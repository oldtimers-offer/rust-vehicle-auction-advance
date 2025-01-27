use sqlx::PgPool;
use actix_web::{web, Responder, HttpResponse};
use crate::models::{UserRegister, UserLogin};
use argon2::{Argon2, PasswordHash, PasswordVerifier, password_hash::SaltString, PasswordHasher};
use uuid::Uuid; // To generate unique session codes
use redis::AsyncCommands;


pub async fn user_register(
    pool: web::Data<PgPool>,
    form: web::Json<UserRegister>,
) -> impl Responder {
    // Check if the username already exists
    let username_exists: (bool,) = match sqlx::query_as::<_, (bool,)>(
        "SELECT EXISTS(SELECT 1 FROM users WHERE username = $1)"
    )
    .bind(&form.username)
    .fetch_one(pool.as_ref())
    .await
    {
        Ok(result) => result,
        Err(_) => return HttpResponse::InternalServerError().body("Database query error"),
    };

    if username_exists.0 {
        // Username already exists
        return HttpResponse::BadRequest().body("Username already taken");
    }

    // Hash the password using Argon2
    let salt = SaltString::generate(&mut rand::thread_rng());
    let hashed_password = Argon2::default()
        .hash_password(form.password.as_bytes(), &salt)
        .unwrap()
        .to_string();

    // Insert the new user into the database
    if let Err(_) = sqlx::query("INSERT INTO users (username, password) VALUES ($1, $2)")
        .bind(&form.username)
        .bind(hashed_password)
        .execute(pool.as_ref())
        .await
    {
        return HttpResponse::InternalServerError().body("Failed to register user");
    }

    HttpResponse::Ok().body("User Registered")
}

pub async fn user_login(
    pool: web::Data<PgPool>,
    redis_client: web::Data<redis::Client>,
    form: web::Json<UserLogin>,
) -> impl Responder {
    let user = sqlx::query!("SELECT password FROM users WHERE username = $1", form.username)
        .fetch_one(pool.as_ref())
        .await;

    match user {
        Ok(record) => {
            match PasswordHash::new(&record.password) {
                Ok(parsed_hash) => {
                    if Argon2::default().verify_password(form.password.as_bytes(), &parsed_hash).is_ok() {
                        // Generate a unique session code
                        let session_code = Uuid::new_v4().to_string();

                        // Save the session code in Redis with a time-to-live (TTL)
                        let mut redis_conn = redis_client.get_multiplexed_async_connection().await.expect("Failed to connect to Redis");
                        let _: () = redis_conn
                            .set_ex(format!("session:{}", session_code), &form.username, 3600) // TTL = 3600 seconds (1 hour)
                            .await
                            .expect("Failed to save session in Redis");

                        // Return the session code to the user
                        HttpResponse::Ok().json(format!("Login successful. Session code: {}", session_code))
                    } else {
                        HttpResponse::Unauthorized().body("Invalid credentials")
                    }
                }
                Err(_) => {
                    eprintln!("Failed to parse password hash for user: {}", form.username);
                    HttpResponse::Unauthorized().body("Invalid password hash")
                }
            }
        }
        Err(sqlx::Error::RowNotFound) => {
            eprintln!("User not found: {}", form.username);
            HttpResponse::Unauthorized().body("Invalid credentials")
        }
        Err(err) => {
            eprintln!("Unexpected database error: {:?}", err);
            HttpResponse::InternalServerError().body("Database query error")
        }
    }
}




