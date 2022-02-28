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
    let result = ParsedEmail {
        body: String::from(""),
        files: vec![],
        from: from_addr,
        images: vec![],
        message_id,
        subject,
    };
    Ok(result)
}
