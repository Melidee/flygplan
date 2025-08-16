use crate::{Error, error::Result};
use std::{fmt::Display, vec};

#[derive(Debug, Clone, PartialEq)]
pub struct Request<'a> {
    pub method: Method,
    pub resource: Url<'a>,
    pub headers: Headers<'a>,
    pub body: &'a [u8],
}

impl<'a> Request<'a> {
    pub fn parse(from: &'a [u8]) -> Result<Self> {
        let (request_parts, body) =
            split_slice_once(from, "\r\n\r\n".as_bytes()).ok_or(Error::ParseError)?;
        let (first_line, header_str) = str::from_utf8(request_parts)
            .map_err(|_| Error::ParseError)?
            .split_once("\r\n")
            .ok_or(Error::ParseError)?;
        let mut splits = first_line.split(" ");
        let method = splits
            .next()
            .map(|method| Method::try_from(method).ok())
            .flatten()
            .ok_or(Error::ParseError)?;
        let resource = splits
            .next()
            .map(|url| Url::parse(url))
            .flatten()
            .ok_or(Error::ParseError)?;
        let headers =
            Headers::from_lines(&mut header_str.split("\r\n")).ok_or(Error::ParseError)?;
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

// used in HTTP request parsing
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
    BadRequest400,
    NotFound404,
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let method = match self {
            Self::Ok200 => "200 OK",
            Self::BadRequest400 => "400 Bad Request",
            Self::NotFound404 => "404 NOT FOUND",
        };
        write!(f, "{}", method)
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
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
    pub query_params: Params<'a>,
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
        (url.username, url.password) = userpair.split_once(":").unwrap_or((userpair, ""));
        (value, url.fragment) = value.split_once("#").unwrap_or((value, ""));
        let query;
        (value, query) = value.split_once("?").unwrap_or((value, ""));
        url.query_params = Params::parse_query_params(query).unwrap_or_default();
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

impl<'a> Display for Url<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}{}{}{}{}{}{}{}{}{}{}",
            self.scheme,
            if !self.scheme.is_empty() { "://" } else { "" },
            self.username,
            if !self.password.is_empty() { ":" } else { "" },
            self.password,
            if !self.username.is_empty() { "@" } else { "" },
            self.host,
            if self.port != 0 {
                format!(":{}", self.port)
            } else {
                "".to_string()
            },
            self.path,
            self.query_params,
            if !self.fragment.is_empty() { "#" } else { "" },
            self.fragment
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Params<'a> {
    params: Vec<(&'a str, &'a str)>,
}

impl<'a> Params<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn push(&mut self, pair: (&'a str, &'a str)) {
        self.params.push(pair);
    }

    pub fn get(&self, key: &'a str) -> Option<String> {
        self.params
            .iter()
            .filter(|(k, _v)| &key == k)
            .next()
            .map(|(_k, v)| v.to_string())
    }

    pub fn parse_query_params(query: &'a str) -> Option<Self> {
        let params = query
            .split("&")
            .map(|pair| pair.split_once("="))
            .collect::<Option<Vec<_>>>()?;
        Some(Self { params })
    }
}

impl<'a> Display for Params<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.params.is_empty() {
            return write!(f, "");
        }
        write!(
            f,
            "?{}",
            self.params
                .iter()
                .map(|(key, val)| format!("{}={}", key, val))
                .collect::<Vec<_>>()
                .join("&"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_line_request() {
        let request = "GET / HTTP/1.1\r\n\r\n".as_bytes();
        let parsed = Request::parse(request).expect("failed to parse request");
        let expected = Request {
            method: Method::Get,
            resource: Url::parse("/").unwrap(),
            headers: Headers::default(),
            body: &[],
        };
        assert_eq!(parsed, expected);
    }

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
            query_params: Params {
                params: vec![("key", "value")],
            },
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
            query_params: Params {
                params: vec![("key", "value")],
            },
            fragment: "fragid",
        };
        assert_eq!(parsed, expected)
    }

    #[test]
    fn parse_path_only() {
        let url = "/ameliaa";
        let parsed = Url::parse(url).unwrap();
        let expected = Url {
            scheme: "",
            username: "",
            password: "",
            host: "",
            port: 0,
            path: "/ameliaa",
            query_params: Params::new(),
            fragment: "",
        };
        assert_eq!(parsed, expected)
    }

    #[test]
    fn format_full_url() {
        let url = Url {
            scheme: "",
            username: "",
            password: "",
            host: "",
            port: 0,
            path: "/path/data",
            query_params: Params {
                params: vec![("key", "value")],
            },
            fragment: "fragid",
        };
        let formatted = &url.to_string();
        let expected = "/path/data?key=value#fragid";
        assert_eq!(formatted, expected)
    }
}
