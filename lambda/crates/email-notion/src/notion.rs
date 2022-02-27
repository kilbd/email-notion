use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct UserData {
    pub id: String,
    pub person: Option<UserEmail>,
}

#[derive(Deserialize, Serialize)]
pub struct UserEmail {
    pub email: String,
}

#[derive(Deserialize)]
pub struct NotionApiUserResponse {
    pub results: Vec<UserData>,
}

#[derive(Serialize)]
pub struct PersonData {
    pub id: String,
    #[serde(rename = "type")]
    pub data_type: String,
    pub person: UserEmail,
}

#[derive(Serialize)]
pub struct TextContent {
    pub content: String,
}

#[derive(Serialize)]
pub struct TypedData {
    #[serde(rename = "type")]
    pub data_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub people: Option<Vec<PersonData>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<TextContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<Vec<TypedData>>,
}

impl TypedData {
    fn new() -> Self {
        TypedData {
            data_type: String::from("none"),
            database_id: None,
            people: None,
            text: None,
            title: None,
        }
    }

    pub fn database(id: String) -> Self {
        let mut data = TypedData::new();
        data.data_type = String::from("database_id");
        data.database_id = Some(id);
        data
    }

    pub fn people(people: PersonData) -> Self {
        let mut data = TypedData::new();
        data.data_type = String::from("people");
        data.people = Some(vec![people]);
        data
    }

    pub fn text(text: TextContent) -> Self {
        let mut data = TypedData::new();
        data.data_type = String::from("text");
        data.text = Some(text);
        data
    }

    pub fn title(title: TypedData) -> Self {
        let mut data = TypedData::new();
        data.data_type = String::from("title");
        data.title = Some(vec![title]);
        data
    }
}

#[derive(Serialize)]
pub struct Properties {
    #[serde(rename = "Name")]
    pub name: TypedData,
    #[serde(rename = "Assign")]
    pub assign: TypedData,
}

#[derive(Serialize)]
pub struct ParagraphContent {
    pub text: Vec<TypedData>,
}

#[derive(Serialize)]
pub struct BlockData {
    pub object: String,
    #[serde(rename = "type")]
    pub data_type: String,
    pub paragraph: ParagraphContent,
}

impl BlockData {
    pub fn new(text_content: String) -> Self {
        BlockData {
            object: String::from("block"),
            data_type: String::from("paragraph"),
            paragraph: ParagraphContent {
                text: vec![TypedData::text(TextContent {
                    content: text_content,
                })],
            },
        }
    }
}

#[derive(Serialize)]
pub struct TaskData {
    pub parent: TypedData,
    pub properties: Properties,
    pub children: Vec<BlockData>,
}
