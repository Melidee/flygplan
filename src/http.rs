use crate::{Error, error::Result};
use std::{fmt::Display, vec};

#[derive(Debug, Clone)]
pub struct Request<'a> {
    pub method: Method,
    pub resource: &'a str,
    pub headers: Headers<'a>,
    pub body: &'a [u8],
}

impl<'a> Request<'a> {
    pub fn parse(from: &'a [u8]) -> Result<Self> {
        let (first_line, rest) =
            split_slice_once(from, "\r\n".as_bytes()).ok_or(Error::ParseError)?;
        let (method, resource) = if let [method_bytes, resource_bytes, b"HTTP/1.1"] =
            split_slice(first_line, &[b' ']).as_slice()
        {
            let method_str = str::from_utf8(method_bytes).map_err(|_| Error::ParseError)?;
            let method = Method::try_from(method_str)?;
            let resource = str::from_utf8(resource_bytes).map_err(|_| Error::ParseError)?;
            (method, resource)
        } else {
            return Err(Error::ParseError);
        };
        let (headers_block, body) =
            split_slice_once(rest, "\r\n\r\n".as_bytes()).ok_or(Error::ParseError)?;
        let headers_str = str::from_utf8(headers_block).map_err(|_| Error::ParseError)?;

        let headers = Headers::from_lines(&mut headers_str.lines()).ok_or(Error::ParseError)?;

        Ok(Self {
            method,
            resource,
            headers,
            body,
        })
    }

    pub fn set_header(&mut self, header: &'a str, value: &'a str) -> &mut Self {
        self.headers.set(header, value);
        self
    }
}

impl<'a> Display for Request<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} HTTP/1.1\r\n{}\r\n{}",
            self.method,
            self.resource,
            self.headers,
            String::from_utf8_lossy(self.body)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Method {
    Get,
    Post,
}

impl TryFrom<&str> for Method {
    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        Ok(match value {
            "GET" => Self::Get,
            "POST" => Self::Post,
            _ => return Err(Error::ParseError),
        })
    }

    type Error = Error;
}

impl Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let method = match self {
            Method::Get => "GET",
            Method::Post => "POST",
        };
        write!(f, "{}", method)
    }
}

pub struct Response<'a> {
    pub status: Status,
    pub headers: Headers<'a>,
    pub body: String,
}

impl<'a> Response<'a> {
    pub fn new(status: Status) -> Self {
        Self {
            status,
            ..Default::default()
        }
    }
}

impl<'a> Display for Response<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "HTTP/1.1 {}\r\n{}\r\n{}",
            self.status, self.headers, self.body
        )
    }
}

impl<'a> Default for Response<'a> {
    fn default() -> Self {
        Self {
            status: Status::Ok200,
            headers: Headers::default(),
            body: String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Status {
    Ok200,
    NotFound404,
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let method = match self {
            Self::Ok200 => "200 OK",
            Self::NotFound404 => "404 NOT FOUND",
        };
        write!(f, "{}", method)
    }
}

fn split_slice<'a>(haystack: &'a [u8], needle: &'a [u8]) -> Vec<&'a [u8]> {
    let mut splits = vec![];
    let mut i = 0;
    let mut last_start = 0;
    while i < haystack.len() - needle.len() {
        let selection = if let Some(selection) = haystack.get(i..i + needle.len()) {
            selection
        } else {
            break;
        };
        if selection == needle {
            splits.push(&haystack[last_start..i]);
            i += needle.len();
            last_start = i;
        }
        i += 1;
    }
    splits.push(&haystack[last_start..]);
    return splits;
}

fn split_slice_once<'a>(haystack: &'a [u8], needle: &'a [u8]) -> Option<(&'a [u8], &'a [u8])> {
    for i in 0..(haystack.len() - needle.len()) {
        let selection = if let Some(selection) = haystack.get(i..i + needle.len()) {
            selection
        } else {
            break;
        };
        if selection == needle {
            return Some((&haystack[0..i], &haystack[i + needle.len()..]));
        }
    }
    return None;
}

#[derive(Debug, Default, Clone)]
pub struct Headers<'a> {
    headers: Vec<(&'a str, &'a str)>,
}

impl<'a> Headers<'a> {
    pub fn new() -> Self {
        Headers { headers: vec![] }
    }

    fn from_lines<'b: 'a>(lines: &mut impl Iterator<Item = &'b str>) -> Option<Self> {
        let mut header_map = vec![];
        for line in lines {
            if line.is_empty() {
                break;
            }
            let mut parts = line.split(": ");
            let header = parts.next()?;
            let value = parts.next()?;
            header_map.push((header, value));
        }
        Some(Self {
            headers: header_map,
        })
    }

    pub fn set(&mut self, header: &'a str, value: &'a str) {
        self.headers.push((header, value));
    }
}

impl<'a> Display for Headers<'a> {
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

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Url<'a> {
    pub scheme: &'a str,
    pub username: &'a str,
    pub password: &'a str,
    pub host: &'a str,
    pub port: u16,
    pub path: &'a str,
    pub query: Vec<(&'a str, &'a str)>,
    pub fragment: &'a str,
}

impl<'a> Url<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    fn parse(mut value: &'a str) -> Option<Self> {
        let mut url = Url::default();
        (url.scheme, value) = value.split_once("://").unwrap_or(("", value));
        let userpair;
        (userpair, value) = value.split_once("@").unwrap_or(("", value));
        println!("{}", userpair);
        (url.username, url.password) = userpair.split_once(":").unwrap_or((userpair, ""));
        (value, url.fragment) = value.split_once("#").unwrap_or((value, ""));
        let query;
        (value, query) = value.split_once("?").unwrap_or((value, ""));
        url.query = query
            .split("&")
            .map(|pair| pair.split_once("="))
            .collect::<Option<Vec<_>>>()?;
        let hostpair;
        (hostpair, url.path) = value
            .find("/")
            .map(|idx| value.split_at(idx))
            .unwrap_or((value, ""));
        (url.host, url.port) = hostpair
            .split_once(":")
            .map(|(host, port)| (host, port.parse().unwrap_or(0)))
            .unwrap_or((hostpair, 0u16));
        Some(url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_line_request() {}

    #[test]
    fn parse_full_url() {
        let url = "abc://username:password@example.com:123/path/data?key=value#fragid";
        let parsed = Url::parse(url).unwrap();
        let expected = Url {
            scheme: "abc",
            username: "username",
            password: "password",
            host: "example.com",
            port: 123,
            path: "/path/data",
            query: vec![("key", "value")],
            fragment: "fragid",
        };
        assert_eq!(parsed, expected)
    }

    #[test]
    fn parse_path_query_frag_url() {
        let url = "/path/data?key=value#fragid";
        let parsed = Url::parse(url).unwrap();
        let expected = Url {
            scheme: "",
            username: "",
            password: "",
            host: "",
            port: 0,
            path: "/path/data",
            query: vec![("key", "value")],
            fragment: "fragid",
        };
        assert_eq!(parsed, expected)
    }
}
