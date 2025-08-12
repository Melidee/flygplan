use std::{collections::HashMap, fmt::Display};

pub struct Request {
    pub method: Method,
    pub resource: String,
    pub headers: Headers,
    pub body: String,
}

impl Request {
    pub fn parse(from: &str) -> Option<Self> {
        let mut lines = from.split("\r\n");
        let mut parts = lines.next()?.split(" ");
        let method = parts.next()?.try_into().ok()?;
        let resource = parts.next()?.to_string();
        if parts.count() != 1 {
            return None;
        }
        let headers = Headers::from_lines(&mut lines)?;
        let body = lines.collect::<Vec<&str>>().join("\r\n");
        Some(Self {
            method,
            resource,
            headers,
            body,
        })
    }
}

impl Display for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} HTTP/1.1\r\n{}\r\n{}",
            self.method, self.resource, self.headers, self.body
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Method {
    Get,
    Post,
}

impl TryFrom<&str> for Method {
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(match value {
            "GET" => Self::Get,
            "POST" => Self::Post,
            _ => return Err(()),
        })
    }

    type Error = ();
}

impl Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let method = match self {
            Method::Get => "GET",
            Method::Post => "GET",
        };
        write!(f, "{}", method)
    }
}

pub struct Response {
    pub status: Status,
    pub headers: Headers,
    pub body: String,
}

impl Response {
    pub fn new(status: Status) -> Self {
        Self {
            status,
            ..Default::default()
        }
    }
}

impl Display for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "HTTP/1.1 {}\r\n{}\r\n{}",
            self.status, self.headers, self.body
        )
    }
}

impl Default for Response {
    fn default() -> Self {
        Self {
            status: Status::Ok,
            headers: Headers::default(),
            body: String::new(),
        }
    }
}

pub enum Status {
    Ok,
    NotFound,
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let method = match self {
            Self::Ok => "200 OK",
            Self::NotFound => "404 NOT FOUND",
        };
        write!(f, "{}", method)
    }
}

#[derive(Debug, Default, Clone)]
pub struct Headers {
    headers: HashMap<String, String>,
}

impl Headers {
    fn from_lines<'a>(lines: &mut impl Iterator<Item = &'a str>) -> Option<Self> {
        let mut header_map = HashMap::new();
        for line in lines {
            if line.is_empty() {
                break;
            }
            let mut parts = line.split(": ");
            let header = parts.next()?.to_string();
            let value = parts.next()?.to_string();
            header_map.insert(header, value);
        }
        Some(Self {
            headers: header_map,
        })
    }

    pub fn set(&mut self, header: String, value: String) {
        self.headers.insert(header, value);
    }
}

impl Display for Headers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let formatted = self
            .headers
            .iter()
            .map(|(h, v)| format!("{h}: {v}"))
            .collect::<Vec<String>>()
            .join("\r\n");
        write!(f, "{}", formatted)
    }
} 
