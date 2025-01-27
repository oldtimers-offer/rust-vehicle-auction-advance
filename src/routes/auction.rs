use sqlx::PgPool;
use actix_web::{web, Responder, HttpResponse, HttpRequest};
use crate::models::{CreateAuction, PlaceBid};
use redis::AsyncCommands;
use bigdecimal::BigDecimal;
use std::str::FromStr;
use chrono::Utc;

pub async fn create_auction(
    pool: web::Data<PgPool>,
    redis_client: web::Data<redis::Client>,
    req: HttpRequest,
    form: web::Json<CreateAuction>,
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

    // Retrieve the user_name associated with the session_code from Redis
    let user_name: Option<String> = match redis_conn
        .get(format!("session:{}", session_code))
        .await
    {
        Ok(username) => username,
        Err(_) => return HttpResponse::InternalServerError().body("Failed to retrieve user_name"),
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


    // Verify that the current user is the owner of the vehicle
    let vehicle_owner: Option<String> = match sqlx::query_scalar::<_, String>(
        "SELECT owner_username FROM vehicles WHERE id = $1"
    )
    .bind(form.vehicle_id)
    .fetch_optional(pool.as_ref())
    .await
    {
        Ok(owner) => owner,
        Err(_) => return HttpResponse::InternalServerError().body("Failed to verify vehicle ownership"),
    };

    if vehicle_owner != Some(user_name).expect("no user name") {
        // The user is not the owner of the vehicle
        return HttpResponse::Forbidden().body("You are not the owner of this vehicle");
    }

    // Check if there is an existing auction for the same vehicle
    let existing_auction: Option<(i32, bool)> = sqlx::query_as(
        "SELECT id, closed FROM auctions WHERE vehicle_id = $1 AND closed = FALSE"
    )
    .bind(form.vehicle_id)
    .fetch_optional(pool.as_ref())
    .await
    .expect("Failed to query auctions");

    if let Some(_) = existing_auction {
        // Auction already exists and is not closed
        return HttpResponse::BadRequest().body("An auction is already open for this vehicle.");
    }

    // Proceed to create the auction
    if let Err(_) = sqlx::query(
        "INSERT INTO auctions (vehicle_id, starting_price, end_time) VALUES ($1, $2, $3)"
    )
    .bind(form.vehicle_id)
    .bind(form.starting_price)
    .bind(form.end_time)
    .execute(pool.as_ref())
    .await
    {
        return HttpResponse::InternalServerError().body("Failed to create auction");
    }

    HttpResponse::Ok().body("Auction Created")
}

pub async fn place_bid(
    pool: web::Data<PgPool>,
    redis_client: web::Data<redis::Client>,
    req: HttpRequest,
    form: web::Json<PlaceBid>,
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
        Ok(username) => username,
        Err(_) => return HttpResponse::InternalServerError().body("Failed to retrieve username"),
    };

    let user_name = match user_name {
        Some(name) => name,
        None => return HttpResponse::Unauthorized().body("Invalid or expired session"),
    };

    // Fetch the starting price and end time of the auction
    let auction_details = match sqlx::query!(
        "SELECT starting_price, end_time FROM auctions WHERE id = $1",
        form.auction_id
    )
    .fetch_optional(pool.as_ref())
    .await
    {
        Ok(Some(auction)) => auction,
        Ok(None) => return HttpResponse::NotFound().body("Auction not found"),
        Err(_) => return HttpResponse::InternalServerError().body("Failed to fetch auction details"),
    };

    let starting_price = auction_details.starting_price;
    let end_time = auction_details.end_time;

    // Ensure that the current time is before the auction's end time
    let now = Utc::now().naive_utc();
    if now > end_time {
        return HttpResponse::BadRequest().body("The auction has already ended");
    }

    // Get the current highest bid for the auction
    let current_highest_bid: BigDecimal = match sqlx::query_scalar!(
        "SELECT MAX(bid_amount) FROM bids WHERE auction_id = $1",
        form.auction_id
    )
    .fetch_optional(pool.as_ref())
    .await
    {
        Ok(Some(bid)) => bid.unwrap_or_else(|| BigDecimal::from(0)),
        Ok(None) => BigDecimal::from(0),
        Err(_) => return HttpResponse::InternalServerError().body("Failed to fetch current highest bid"),
    };

    // Determine the minimum valid bid
    let min_required_bid = if current_highest_bid > BigDecimal::from(0) {
        &current_highest_bid + BigDecimal::from(500)
    } else {
        starting_price.clone()
    };

    let bid_amount = match BigDecimal::from_str(&form.bid_amount.to_string()) {
        Ok(amount) => amount,
        Err(_) => return HttpResponse::InternalServerError().body("Failed to parse bid amount"),
    };

    // Validate the bid amount
    if bid_amount < starting_price {
        return HttpResponse::BadRequest().body(format!(
            "Your bid must be at least the starting price of {}",
            starting_price
        ));
    }

    if bid_amount < min_required_bid {
        return HttpResponse::BadRequest().body(format!(
            "Your bid must be at least 500 higher than the current highest bid of {}",
            current_highest_bid
        ));
    }

    // Place the bid
    if let Err(_) = sqlx::query!(
        "INSERT INTO bids (auction_id, bid_amount, bidder_username) VALUES ($1, $2, $3)",
        form.auction_id,
        bid_amount,
        user_name
    )
    .execute(pool.as_ref())
    .await
    {
        return HttpResponse::InternalServerError().body("Failed to place bid");
    };

    HttpResponse::Ok().body("Bid Placed")
}



pub async fn close_auction(pool: web::Data<PgPool>, req: HttpRequest, path: web::Path<i32>, redis_client: web::Data<redis::Client>) -> impl Responder {
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
        Ok(username) => username,
        Err(_) => return HttpResponse::InternalServerError().body("Failed to retrieve username"),
    };

    let user_name = match user_name {
        Some(name) => name,
        None => return HttpResponse::Unauthorized().body("Invalid or expired session"),
    };

    // Fetch the vehicle owner from the auctions table
    let vehicle_owner = match sqlx::query!(
        "SELECT v.owner_username FROM vehicles v INNER JOIN auctions a ON v.id = a.vehicle_id WHERE a.id = $1",
        *path
    )
    .fetch_one(pool.as_ref())
    .await
    {
        Ok(record) => record.owner_username,
        Err(_) => return HttpResponse::InternalServerError().body("Failed to fetch vehicle owner"),
    };

    // Ensure the current user is the owner of the vehicle
    if vehicle_owner != user_name {
        return HttpResponse::Forbidden().body("You are not the owner of the vehicle");
    }

    // Ensure the auction is open (not closed)
    let auction_closed = match sqlx::query!(
        "SELECT closed FROM auctions WHERE id = $1",
        *path
    )
    .fetch_optional(pool.as_ref())
    .await
    {
        Ok(Some(record)) => record.closed,  // Extract the `closed` field from the record
        Ok(None) => return HttpResponse::NotFound().body("Auction not found"),
        Err(_) => return HttpResponse::InternalServerError().body("Failed to fetch auction status"),
    };

    if auction_closed.expect("Auction Closed missing") {
        return HttpResponse::BadRequest().body("The auction is already closed");
    }

    // Find the highest bid for the auction
    let highest_bidder: Option<String> = match sqlx::query!(
        "SELECT bidder_username FROM bids WHERE auction_id = $1 ORDER BY bid_amount DESC LIMIT 1",
        *path
    )
    .fetch_optional(pool.as_ref())
    .await
    {
        Ok(Some(record)) => Some(record.bidder_username),
        Ok(None) => None,
        Err(_) => return HttpResponse::InternalServerError().body("Failed to fetch highest bidder"),
    };

    // If no bids, the auction cannot be closed
    let highest_bidder = match highest_bidder {
        Some(bidder) => bidder,
        None => return HttpResponse::BadRequest().body("No bids have been placed for this auction"),
    };

    // Update the vehicle owner to the highest bidder
    if let Err(_) = sqlx::query!(
        "UPDATE vehicles SET owner_username = $1 WHERE id = (SELECT vehicle_id FROM auctions WHERE id = $2)",
        highest_bidder,
        *path
    )
    .execute(pool.as_ref())
    .await
    {
        return HttpResponse::InternalServerError().body("Failed to update vehicle owner");
    }

    // Close the auction
    if let Err(_) = sqlx::query!(
        "UPDATE auctions SET closed = TRUE WHERE id = $1",
        *path
    )
    .execute(pool.as_ref())
    .await
    {
        return HttpResponse::InternalServerError().body("Failed to close auction");
    }

    HttpResponse::Ok().body("Auction Closed and Vehicle Ownership Transferred")
}