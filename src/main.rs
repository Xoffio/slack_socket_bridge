use clap::Parser;
use reqwest::StatusCode;
use slack_morphism::prelude::*;
use std::{env, sync::Arc};
use tracing::Level;

#[derive(Parser, Debug)]
#[command(name = "slack_socket_bridge")]
struct Args {
    // Log level
    #[arg(short, long, value_name = "LOG LEVEL")]
    log_level: Option<tracing::Level>,
}

async fn _test_interaction_events_function(
    event: SlackInteractionEvent,
    _client: Arc<SlackHyperClient>,
    _states: SlackClientEventsUserState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing::debug!("{:#?}", event);
    Ok(())
}

async fn handle_command_events(
    event: SlackCommandEvent,
    _client: Arc<SlackHyperClient>,
    _states: SlackClientEventsUserState,
) -> Result<SlackCommandEventResponse, Box<dyn std::error::Error + Send + Sync>> {
    tracing::debug!("{:#?}", event);

    let event_value = serde_json::to_value(event)?;

    let client = match reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
    {
        Ok(c) => c,
        Err(err) => {
            tracing::error!("Failed to create webhook client: {}", err);
            return Err(Box::new(err));
        }
    };

    println!("{:#?}", &event_value);

    if let Ok(webhook_url) = env::var("WEBHOOK_URL_CMD_PROD") {
        tracing::debug!("Tring to send a message to webhook {}", &webhook_url);
        let res_msg = match client.post(&webhook_url).json(&event_value).send().await {
            Ok(res) => {
                tracing::info!("webhook result status: {}", res.status());

                if res.status().is_success() {
                    let res_text = res.text().await;

                    if let Ok(text) = res_text {
                        Ok(text)
                    } else {
                        Err("Internal error. The action may have executed, but I was unable to retrieve the result.".to_string())
                    }
                } else {
                    Err(format!(
                        "Internal Error. Bridge could't connect to webhook. Resturned code: {}",
                        res.status()
                    ))
                }
            }
            Err(err) => {
                let error_msg = format!("Failed to send message to webhook. Error: {}", err);
                tracing::warn!("{}", &error_msg);
                Err(error_msg)
            }
        };

        // For now I will skip errors in production in case the user is using the dev webhook.
        if let Ok(text) = res_msg {
            return Ok(SlackCommandEventResponse::new(
                SlackMessageContent::new().with_text(text),
            ));
        }
    }

    if let Ok(webhook_url) = env::var("WEBHOOK_URL_CMD_DEV") {
        tracing::debug!("Tring to send a message to webhook {}", &webhook_url);
        let res_msg = match client.post(&webhook_url).json(&event_value).send().await {
            Ok(res) => {
                tracing::info!("webhook result status: {}", res.status());

                if res.status().is_success() {
                    let res_text = res.text().await;

                    if let Ok(text) = res_text {
                        text
                    } else {
                        "Internal error. The action may have executed, but I was unable to retrieve the result.".to_string()
                    }
                } else {
                    format!(
                        "Internal Error. Bridge could't connect to webhook. Resturned code: {}",
                        res.status()
                    )
                }
            }
            Err(err) => {
                let error_msg = format!("Failed to send message to webhook. Error: {}", err);
                tracing::warn!("{}", &error_msg);
                error_msg
            }
        };

        return Ok(SlackCommandEventResponse::new(
            SlackMessageContent::new().with_text(res_msg),
        ));
    }

    Ok(SlackCommandEventResponse::new(
        SlackMessageContent::new().with_text("Failed to talk to backend. slack_socket_bridge didn't find any ENV variable webhook token.".into()),
    ))
}

async fn handle_push_events(
    event: SlackPushEventCallback,
    _client: Arc<SlackHyperClient>,
    _states: SlackClientEventsUserState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing::debug!("{:#?}", event);

    let event_value = serde_json::to_value(event)?;

    let client = match reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
    {
        Ok(c) => c,
        Err(err) => {
            tracing::error!("Failed to create webhook client: {}", err);
            return Err(Box::new(err));
        }
    };

    if let Ok(webhook_url) = env::var("WEBHOOK_URL_PROD") {
        tracing::debug!("Tring to send a message to webhook {}", &webhook_url);
        match client.post(&webhook_url).json(&event_value).send().await {
            Ok(res) => {
                tracing::info!("webhook result status: {}", res.status());
            }
            Err(err) => {
                tracing::warn!("Failed to send message to webhook. Error: {}", err);
            }
        }
    }

    if let Ok(webhook_url) = env::var("WEBHOOK_URL_DEV") {
        tracing::debug!("Tring to send a message to webhook {}", &webhook_url);
        match client.post(&webhook_url).json(&event_value).send().await {
            Ok(res) => {
                tracing::info!("webhook result status: {}", res.status());
            }
            Err(err) => {
                tracing::warn!("Failed to send message to webhook. Error: {}", err);
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let log_level = args.log_level.unwrap_or(Level::INFO);
    tracing_subscriber::fmt().with_max_level(log_level).init();

    let app_token_value: SlackApiTokenValue = match env::var("SLACK_SOCKET_TOKEN") {
        Ok(tkn) => tkn.into(),
        Err(err) => {
            tracing::error!(
                "Failed to get environment variable SLACK_SOCKET_TOKEN. Error: {}",
                err
            );
            std::process::exit(1);
        }
    };
    let app_token: SlackApiToken = SlackApiToken::new(app_token_value);

    // Client for the websocket connection
    let socket_client = Arc::new(SlackClient::new(
        SlackClientHyperConnector::new().expect("Failed creting slack client"),
    ));

    // Add callbacks
    let socket_mode_callbacks = SlackSocketModeListenerCallbacks::new()
        .with_command_events(handle_command_events)
        // .with_interaction_events(test_interaction_events_function)
        .with_push_events(handle_push_events);

    let listener_environment = Arc::new(SlackClientEventsListenerEnvironment::new(
        socket_client.clone(),
    ));

    let socket_mode_listener = SlackClientSocketModeListener::new(
        &SlackClientSocketModeConfig::new(),
        listener_environment.clone(),
        socket_mode_callbacks,
    );

    // Register an app token to listen for events,
    match socket_mode_listener.listen_for(&app_token).await {
        Ok(_) => (),
        Err(err) => {
            tracing::error!("Failed setting socket to listen. Error {}", err);
            std::process::exit(1);
        }
    }

    // Listen indefinitely
    socket_mode_listener.serve().await;
}
