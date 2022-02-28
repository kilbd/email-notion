use mailparse::*;

pub struct ParsedEmail {
    pub body: String,
    pub files: Vec<File>,
    pub from: String,
    pub images: Vec<File>,
    pub message_id: Option<String>,
    pub subject: String,
}

pub struct File {
    pub data: Vec<u8>,
    pub id: Option<String>,
    pub name: String,
}

pub fn parse_email(data: &[u8]) -> Result<ParsedEmail, MailParseError> {
    let parsed = parse_mail(data)?;
    let from_addr: String;
    match &addrparse_header(parsed.headers.get_first_header("From").unwrap())?[0] {
        MailAddr::Single(info) => {
            from_addr = info.addr.to_string();
        }
        _ => panic!("Message malformed: multiple From addresses."),
    }
    let subject = parsed.headers.get_first_value("Subject").unwrap();
    let message_id = parsed.headers.get_first_value("Message-ID");
    let mut result = ParsedEmail {
        body: String::from(""),
        files: vec![],
        from: from_addr,
        images: vec![],
        message_id,
        subject,
    };
    process_subparts(&parsed.subparts, &mut result)?;
    Ok(result)
}

fn process_subparts(
    subparts: &[ParsedMail],
    result: &mut ParsedEmail,
) -> Result<(), MailParseError> {
    for part in subparts {
        // Capturing any text part so it's not saved as a file
        if part.ctype.mimetype.starts_with("text") {
            // We're only using plain text for Notion task
            if part.ctype.mimetype == "text/plain" {
                result.body = part.get_body()?;
            }
        } else if part.ctype.mimetype.starts_with("image") {
            let name = &part.ctype.params["name"];
            let id = part.headers.get_first_value("Content-ID");
            result.images.push(File {
                id,
                name: name.to_string(),
                data: part.get_body_raw().unwrap(),
            })
        } else if !part.ctype.mimetype.starts_with("multipart") {
            // I'm assuming anything not text or a multipart type is a file
            // I need to save.
            let name = &part.ctype.params["name"];
            result.files.push(File {
                data: part.get_body_raw().unwrap(),
                id: None,
                name: name.to_string(),
            })
        } else {
            // This should only be parts with a content type of multipart.
            // Multipart content should have subparts.
            process_subparts(&part.subparts, result)?;
        }
    }
    Ok(())
}
