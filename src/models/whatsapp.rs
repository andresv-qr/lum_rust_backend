use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum MessageType {
    Text,
    Interactive,
    Image,
    Document,
    #[serde(other)]
    #[default]
    Unsupported,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WebhookVerification {
    #[serde(rename = "hub.mode")]
    pub hub_mode: Option<String>,
    #[serde(rename = "hub.verify_token")]
    pub hub_verify_token: Option<String>,
    #[serde(rename = "hub.challenge")]
    pub hub_challenge: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Text {
    pub body: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ButtonReply {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ListReply {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Interactive {
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub button_reply: Option<ButtonReply>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_reply: Option<ListReply>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Image {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    #[serde(default)]
    pub mime_type: String,
    #[serde(default)]
    pub sha256: String,
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Document {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    pub filename: String,
    #[serde(default)]
    pub mime_type: String,
    #[serde(default)]
    pub sha256: String,
    pub id: String,
    // This field is added to be able to get the sender's number
    // in the handler, as it's not directly available in the Document object
    // but is present in the parent Message object.
    #[serde(skip_serializing)] 
    pub from: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Message {
    pub from: String,
    pub id: String,
    pub timestamp: String,
    #[serde(default)]
    pub text: Text,
    #[serde(default)]
    pub interactive: Option<Interactive>,
    #[serde(default)]
    pub image: Option<Image>,
    #[serde(default)]
    pub document: Option<Document>,
    #[serde(rename = "type")]
    pub message_type: MessageType,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Profile {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Contact {
    pub profile: Profile,
    pub wa_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Metadata {
    pub display_phone_number: String,
    pub phone_number_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Value {
    pub messaging_product: String,
    #[serde(default)]
    pub metadata: Metadata,
    #[serde(default)]
    pub contacts: Vec<Contact>,
    #[serde(default)]
    pub messages: Vec<Message>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Change {
    pub value: Value,
    pub field: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Entry {
    pub id: String,
    pub changes: Vec<Change>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WebhookPayload {
    pub object: String,
    pub entry: Vec<Entry>,
}

// Structs for sending interactive messages

#[derive(Debug, Serialize, Clone)]
pub struct Row {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct Section {
    pub title: String,
    pub rows: Vec<Row>,
}

#[derive(Debug, Serialize, Clone)]
pub struct Button {
    #[serde(rename = "type")]
    pub r#type: String, // e.g., "reply"
    pub reply: ButtonReply,
}

impl Button {
    pub fn new(r#type: &str, reply: ButtonReply) -> Self {
        Self { r#type: r#type.to_string(), reply }
    }
}


#[derive(Debug, Serialize, Clone)]
pub struct Action {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buttons: Option<Vec<Button>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub button: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sections: Option<Vec<Section>>,
}

impl Action {
    pub fn new_for_buttons(buttons: Vec<Button>) -> Self {
        Self {
            buttons: Some(buttons),
            button: None,
            sections: None,
        }
    }

    pub fn new_for_list(button_text: &str, sections: Vec<Section>) -> Self {
        Self {
            buttons: None,
            button: Some(button_text.to_string()),
            sections: Some(sections),
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct InteractiveBody {
    pub text: String,
}

impl InteractiveBody {
    pub fn new(text: &str) -> Self {
        Self { text: text.to_string() }
    }
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum InteractiveType {
    Button,
    List,
}

#[derive(Debug, Serialize, Clone)]
pub struct InteractiveMessage {
    #[serde(rename = "type")]
    pub r#type: InteractiveType,
    pub body: InteractiveBody,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<Action>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footer: Option<Text>,
}

impl InteractiveMessage {
        pub fn new_for_button(body: InteractiveBody, action: Action, footer: Option<Text>) -> Self {
        Self { r#type: InteractiveType::Button, body, action: Some(action), footer }
    }

    pub fn new_for_list(body: InteractiveBody, action: Action, footer: Option<Text>) -> Self {
        Self { r#type: InteractiveType::List, body, action: Some(action), footer }
    }

    pub fn new(r#type: InteractiveType, body: InteractiveBody, action: Option<Action>, footer: Option<Text>) -> Self {
        Self { r#type, body, action, footer }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct InteractiveMessageRequest {
    pub messaging_product: String,
    pub to: String,
    #[serde(rename = "type")]
    pub message_type: String, // Should be "interactive"
    pub interactive: InteractiveMessage,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ImageMedia {
    pub link: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ImageMessageRequest {
    pub messaging_product: String,
    pub to: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub image: ImageMedia,
}

#[derive(Debug, Serialize, Clone)]
pub struct TextMessageRequest {
    pub messaging_product: String,
    pub to: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub text: Text,
}
