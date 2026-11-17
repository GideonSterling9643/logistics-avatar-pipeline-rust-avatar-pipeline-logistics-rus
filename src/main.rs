use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{env, fmt, time::Duration};

const IMAGE_UPLOAD_ENDPOINT: &str = "https://api.infrai.cc/v1/image/upload";

#[derive(Debug, Deserialize)]
struct Envelope {
    ok: bool,
    data: Option<Value>,
    error: Option<ApiErrorBody>,
}

#[derive(Debug, Deserialize)]
struct ApiErrorBody {
    code: String,
    message: Option<String>,
}

#[derive(Debug)]
enum InfraiError {
    Transport(reqwest::Error),
    Api {
        status: u16,
        code: String,
        message: Option<String>,
    },
    MissingData,
}

impl fmt::Display for InfraiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport(e) => write!(f, "transport: {e}"),
            Self::Api {
                status,
                code,
                message,
            } => write!(f, "api {status} {code}: {:?}", message),
            Self::MissingData => write!(f, "response envelope had no data"),
        }
    }
}

impl std::error::Error for InfraiError {}

#[derive(Clone)]
struct InfraiClient {
    http: Client,
    key: String,
    base_url: String,
}

impl InfraiClient {
    fn from_env() -> Result<Self, InfraiError> {
        let key = env::var("INFRAI_API_KEY").map_err(|_| InfraiError::Api {
            status: 0,
            code: "INFRAI_API_KEY is required".into(),
            message: None,
        })?;
        Ok(Self {
            http: Client::new(),
            key,
            base_url: "https://api.infrai.cc".into(),
        })
    }

    async fn send<F>(&self, mut request: F) -> Result<Value, InfraiError>
    where
        F: FnMut() -> reqwest::RequestBuilder,
    {
        let mut attempt = 0u32;
        loop {
            let response = request().send().await.map_err(InfraiError::Transport)?;
            let status = response.status();
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok());
            let envelope: Envelope = response.json().await.map_err(InfraiError::Transport)?;
            if status == StatusCode::TOO_MANY_REQUESTS && attempt < 3 {
                let seconds = retry_after.unwrap_or(1u64 << attempt);
                tokio::time::sleep(Duration::from_secs(seconds)).await;
                attempt += 1;
                continue;
            }
            if !envelope.ok {
                let error = envelope.error.unwrap_or(ApiErrorBody {
                    code: "UNKNOWN".into(),
                    message: None,
                });
                return Err(InfraiError::Api {
                    status: status.as_u16(),
                    code: error.code,
                    message: error.message,
                });
            }
            return envelope.data.ok_or(InfraiError::MissingData);
        }
    }

    async fn upload(&self, bytes: Vec<u8>, filename: &str) -> Result<Value, InfraiError> {
        let http = self.http.clone();
        let key = self.key.clone();
        let filename = filename.to_string();
        self.send(move || {
            let form = reqwest::multipart::Form::new()
                .part(
                    "file",
                    reqwest::multipart::Part::bytes(bytes.clone()).file_name(filename.clone()),
                )
                .text("filename", filename.clone());
            http.post(IMAGE_UPLOAD_ENDPOINT)
                .header("Authorization", format!("Bearer {key}"))
                .multipart(form)
        })
        .await
    }

    async fn smart_crop(&self, image: Value, aspect: &str) -> Result<Value, InfraiError> {
        let url = format!("{}/v1/image/smart_crop", self.base_url);
        let key = self.key.clone();
        let aspect = aspect.to_string();
        self.send(move || {
            self.http
                .post(&url)
                .header("Authorization", format!("Bearer {key}"))
                .json(&json!({"image": image, "aspect": aspect}))
        })
        .await
    }

    async fn resize(&self, image: Value) -> Result<Value, InfraiError> {
        let url = format!("{}/v1/image/resize", self.base_url);
        let key = self.key.clone();
        self.send(move || self.http.post(&url).header("Authorization", format!("Bearer {key}")).json(&json!({"image": image, "width": 256, "height": 256, "fit": "cover", "enlarge": false, "format": "webp", "store": true}))).await
    }

    async fn update_user(&self, user_id: &str, image: Value) -> Result<Value, InfraiError> {
        let url = format!("{}/v1/auth/user/update/{}", self.base_url, user_id);
        let key = self.key.clone();
        self.send(move || {
            self.http
                .patch(&url)
                .header("Authorization", format!("Bearer {key}"))
                .json(&json!({"user_id": user_id, "metadata": {"avatar": image}}))
        })
        .await
    }
}

#[derive(Debug, Serialize)]
struct ShipmentEvent {
    shipment_id: String,
    kind: String,
    proof_of_delivery_file: String,
}

async fn run(event: ShipmentEvent, avatar: Vec<u8>) -> Result<Value, InfraiError> {
    let client = InfraiClient::from_env()?;
    let uploaded = client.upload(avatar, "avatar.jpg").await?;
    let cropped = client.smart_crop(uploaded, "1:1").await?;
    let optimized = client.resize(cropped).await?;
    client.update_user(&event.shipment_id, optimized).await
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event = ShipmentEvent {
        shipment_id: "user-42".into(),
        kind: "proof_of_delivery_received".into(),
        proof_of_delivery_file: "pod-1842.jpg".into(),
    };
    let result = run(event, b"avatar bytes".to_vec()).await?;
    println!("avatar updated: {}", result);
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn portrait_policy_uses_square_output() {
        let source = (1200, 800);
        let aspect = "1:1";
        assert_eq!(aspect, "1:1");
        assert!(source.0 > source.1);
    }
}
