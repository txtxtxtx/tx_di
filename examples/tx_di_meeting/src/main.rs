use livekit_api::services::room::{CreateRoomOptions, RoomClient};
#[tokio::main]
async fn main() {
    unsafe {
        env::set_var("LIVEKIT_API_KEY","key1");
        env::set_var("LIVEKIT_API_SECRET","abc123");
    }
    let room_service = RoomClient::new("http://localhost:7880").unwrap();

    let room = room_service
        .create_room("my_room", CreateRoomOptions::default())
        .await
        .unwrap();

    println!("Created room: {:?}", room);

    let token = create_token().unwrap();
    println!("token: {}",token)
}


use livekit_api::access_token;
use std::env;

fn create_token() -> Result<String, access_token::AccessTokenError> {
    let api_key = env::var("LIVEKIT_API_KEY").expect("LIVEKIT_API_KEY is not set");
    let api_secret = env::var("LIVEKIT_API_SECRET").expect("LIVEKIT_API_SECRET is not set");

    let token = access_token::AccessToken::with_api_key(&api_key, &api_secret)
        .with_identity("rust-bot")
        .with_name("Rust Bot")
        .with_grants(access_token::VideoGrants {
            room_join: true,
            room: "my-room".to_string(),
            ..Default::default()
        })
        .to_jwt();
    return token
}