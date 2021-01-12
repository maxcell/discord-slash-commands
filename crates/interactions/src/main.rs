use aws_lambda_events::encodings::Body;
use discord_interactions::{
    reply_with, validate_discord_signature, DiscordEvent, EventType, InteractionResponseType,
};
use ed25519_dalek::PublicKey;
use http::StatusCode;
use lazy_static::lazy_static;
use netlify_lambda_http::{
    lambda::{lambda, Context},
    IntoResponse, Request, Response,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
// use sodiumoxide::crypto::sign;
use std::env;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

lazy_static! {
    static ref PUB_KEY: PublicKey =
        PublicKey::from_bytes(&hex::decode(env::var("DISCORD_PUBLIC_KEY").unwrap()).unwrap())
            .unwrap();
}

#[lambda(http)]
#[tokio::main]
async fn main(event: Request, _: Context) -> Result<impl IntoResponse, Error> {
    if validate_discord_signature(event.headers(), event.body(), &PUB_KEY) {
        Ok(handle(event))
    } else {
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::Text(
                json!({ "error": format!("failed to verify the thing!") }).to_string(),
            ))
            .map_err(|_e| panic!("whatever"))
    }
}
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(tag = "name", rename_all = "lowercase")]
enum Event {
    Role {
        id: String,
        options: Vec<RoleOption>,
    },
    #[serde(other)]
    Unknown,
}
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(tag = "name")]
enum RoleOption {
    #[serde(rename = "i-want-to-be-a")]
    IWantToBeA { value: String },
}

fn handle(event: Request) -> Response<Body> {
    dbg!(event.body());
    let parsed: Result<DiscordEvent<Event>, _> = serde_json::from_slice(event.body());
    let response = match parsed {
        Ok(event) => match event.event_type {
            EventType::Ping => pong(),
            EventType::ApplicationCommand => handle_event(event),
            _ => panic!("unhandled"),
        },
        Err(e) => {
            println!("{:?}", e);
            reply("failed to parse".to_string())
        }
    };
    dbg!(&response);
    response
}

fn handle_event(event: DiscordEvent<Event>) -> Response<Body> {
    println!("{:?}", event);
    match event.data {
        Some(Event::Role { id, options }) => {
            let role_requested = options.iter().find(|value| match value {
                RoleOption::IWantToBeA { .. } => true,
            });
            match (role_requested, event.member) {
                (
                    Some(RoleOption::IWantToBeA { value }),
                    Some(discord_interactions::GuildMember { user, .. }),
                ) => {
                    dbg!(user.id);
                    // TODO: set role on user if in safelist to self-assign
                    // otherwise send role request to mod channel?
                    reply(value.to_owned())
                }
                (None, _) => reply("must request a role".to_string()),
                (_, None) => reply("no member to apply role to".to_string()),
            }
        }
        Some(Event::Unknown) => reply("unknown_command".to_string()),
        None => reply("no data for command".to_string()),
    }
}

// panics
fn reply(content: String) -> Response<Body> {
    Response::builder()
        .body(Body::Text(
            serde_json::to_string(&reply_with(
                InteractionResponseType::ChannelMessageWithSource,
                content,
            ))
            .expect("expected valid json response"),
        ))
        .expect("body to have not failed")
}

fn pong() -> Response<Body> {
    Response::builder()
        .body(Body::Text(
            serde_json::to_string(&reply_with(
                InteractionResponseType::Pong,
                "pong".to_string(),
            ))
            .expect("expected valid json response"),
        ))
        .expect("body to have not failed")
}
