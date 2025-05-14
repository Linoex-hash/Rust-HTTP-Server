use serde::{Deserialize, Serialize, Serializer};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct ChatMessage {
    pub comment: String,
    #[serde(skip_deserializing)]
    #[serde(serialize_with = "serialize_uuid")]
    pub username: Uuid,
    #[serde(rename = "messageType")]
    pub message_type: String,
}

fn serialize_uuid<S>(uuid: &Uuid, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&uuid.to_string())
}
