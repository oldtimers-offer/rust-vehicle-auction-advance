use serde::{Deserialize, Serialize, Deserializer};
use chrono::NaiveDateTime;
use bigdecimal::BigDecimal;

fn deserialize_naive_datetime<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
where
    D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S")  // Adjust format to handle T separator
            .map_err(serde::de::Error::custom)
    }

#[derive(Deserialize)]
pub struct UserRegister {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct UserLogin {
    // #[serde(skip_deserializing)]
    // pub id: i32,
    pub username: String,
    pub password: String,
}

#[derive(Deserialize, Serialize)]
pub struct CreateVehicle {
    pub name: String,
    pub description: String,
    pub starting_price: f64,
    // #[serde(skip_deserializing)]
    // pub owner_username: i32,
}

#[derive(Serialize, Deserialize)]
pub struct Vehicle {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub starting_price: BigDecimal,
}

#[derive(Deserialize, Serialize)]
pub struct CreateAuction {
    pub vehicle_id: i32,
    pub starting_price: f64,
    #[serde(deserialize_with = "deserialize_naive_datetime")] 
    pub end_time: NaiveDateTime,
}

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
pub struct Auction {
    pub id: i32,
    pub closed: Option<bool>
}

#[derive(Deserialize, Serialize)]
pub struct PlaceBid {
    pub auction_id: i32,
    pub bid_amount: f64,
}
