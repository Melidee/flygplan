use crate::{Error, error::Result};
use std::{borrow::Cow, fmt::Display, vec};

#[derive(Debug, Clone, PartialEq)]
pub struct Request<'a> {
    pub method: Method,
    pub resource: Url<'a>,
    pub headers: Headers<'a>,
    pub body: &'a [u8],
}

impl<'a> Request<'a> {
    pub fn parse(from: &'a [u8]) -> Result<Self> {
        let (first_line, header_and_body) = split_slice_once(from, b"\r\n").ok_or(
            Error::ParseError("HTTP request is formatted incorrectly".into()),
        )?;

        let (method, resource) = str::from_utf8(first_line)
            .map_err(|_| Error::ParseError("HTTP request line is not UTF-8".into()))?
            .split_once(' ')
            .ok_or(Error::ParseError("".into()))
            .and_then(|(method_str, url_status)| {
                let (url_str, status) = url_status.split_once(' ').unwrap_or((url_status, ""));
                if status != "HTTP/1.1" {
                    return Err(Error::ParseError(format!("invalid http status `{status}`")));
                }
                let method = Method::try_from(method_str)?;
                let url = Url::parse(url_str)
                    .ok_or(Error::ParseError(format!("failed to parse url {url_str}")))?;
                Ok((method, url))
            })?;

        let (header_bytes, body) =
            split_slice_once(header_and_body, b"\r\n\r\n").unwrap_or((header_and_body, &[]));

        let mut header_lines = str::from_utf8(header_bytes)
            .map_err(|_| Error::ParseError("HTTP headers are not UTF-8".into()))?
            .split("\r\n");
        let headers = Headers::from_lines(&mut header_lines)?;
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
    if haystack.len() < needle.len() {
        return None;
    }
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
    None
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
            _ => return Err(Error::ParseError(format!("invalid HTTP method {value}"))),
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
    SeeOther303,
    BadRequest400,
    NotFound404,
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let method = match self {
            Self::Ok200 => "200 OK",
            Self::SeeOther303 => "303 See Other",
            Self::BadRequest400 => "400 Bad Request",
            Self::NotFound404 => "404 NOT FOUND",
        };
        write!(f, "{}", method)
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Headers<'a> {
    headers: Vec<(Cow<'a, str>, Cow<'a, str>)>,
}

impl<'a> Headers<'a> {
    pub fn new() -> Self {
        Headers { headers: vec![] }
    }

    fn from_lines<'b: 'a>(lines: &mut impl Iterator<Item = &'b str>) -> Result<Self> {
        let mut header_map = vec![];
        for line in lines {
            if line.is_empty() {
                break;
            }
            let mut parts = line.split(": ");
            let header = parts
                .next()
                .ok_or(Error::ParseError("failed to parse header `{line}`".into()))?;
            let value = parts
                .next()
                .ok_or(Error::ParseError("failed to parse header `{line}`".into()))?;
            header_map.push((header.into(), value.into()));
        }
        Ok(Self {
            headers: header_map,
        })
    }

    pub fn set(&mut self, header: &'a str, value: &'a str) {
        self.headers.push((header.into(), value.into()));
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
    pub scheme: Cow<'a, str>,
    pub username: Cow<'a, str>,
    pub password: Cow<'a, str>,
    pub host: Cow<'a, str>,
    pub port: u16,
    pub path: Cow<'a, str>,
    pub query_params: Params<'a>,
    pub fragment: Cow<'a, str>,
}

impl<'a> Url<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    fn parse(value: &'a str) -> Option<Self> {
        let (scheme, mut value) = value.split_once("://").unwrap_or(("", value));
        let userpair;
        (userpair, value) = value.split_once("@").unwrap_or(("", value));
        let (username, password) = userpair.split_once(":").unwrap_or((userpair, ""));
        let (mut value, fragment) = value.split_once("#").unwrap_or((value, ""));
        let query;
        (value, query) = value.split_once("?").unwrap_or((value, ""));
        let query_params = Params::parse_query_params(query).unwrap_or_default();
        let (hostpair, path) = value
            .find("/")
            .map(|idx| value.split_at(idx))
            .unwrap_or((value, ""));
        let (host, port) = hostpair
            .split_once(":")
            .map(|(host, port)| (host, port.parse().unwrap_or(0)))
            .unwrap_or((hostpair, 0u16));
        Some(Url {
            scheme: scheme.into(),
            username: username.into(),
            password: password.into(),
            host: host.into(),
            port,
            path: path.into(),
            query_params,
            fragment: fragment.into(),
        })
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
            .find(|(k, _v)| &key == k)
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

        assert_eq!(parsed.method, Method::Get);
        assert_eq!(parsed.resource.path, "/");
        assert_eq!(parsed.headers, Headers::default());
        assert_eq!(parsed.body, b"");
    }

    #[test]
    fn parse_full_url() {
        let url = "abc://username:password@example.com:123/path/data?key=value#fragid";
        let parsed = Url::parse(url).unwrap();
        assert_eq!(parsed.scheme, "abc");
        assert_eq!(parsed.username, "username");
        assert_eq!(parsed.password, "password");
        assert_eq!(parsed.host, "example.com");
        assert_eq!(parsed.port, 123);
        assert_eq!(parsed.path, "/path/data");
        assert_eq!(
            parsed.query_params,
            Params {
                params: vec![("key", "value")]
            }
        );
        assert_eq!(parsed.fragment, "fragid");
    }

    #[test]
    fn parse_path_query_frag_url() {
        let url = "/path/data?key=value#fragid";
        let parsed = Url::parse(url).unwrap();
        assert_eq!(parsed.scheme, "");
        assert_eq!(parsed.username, "");
        assert_eq!(parsed.password, "");
        assert_eq!(parsed.host, "");
        assert_eq!(parsed.port, 0);
        assert_eq!(parsed.path, "/path/data");
        assert_eq!(
            parsed.query_params,
            Params {
                params: vec![("key", "value")]
            }
        );
        assert_eq!(parsed.fragment, "fragid");
    }

    #[test]
    fn parse_host_only() {
        let url = "example.com";
        let parsed = Url::parse(url).unwrap();

        assert_eq!(parsed.scheme, "");
        assert_eq!(parsed.host, "example.com");
        assert_eq!(parsed.port, 0);
        assert_eq!(parsed.path, "");
        assert_eq!(parsed.query_params, Params::default());
        assert_eq!(parsed.fragment, "");
    }

    #[test]
    fn parse_host_port_and_path() {
        let url = "example.com:8080/path";
        let parsed = Url::parse(url).unwrap();

        assert_eq!(parsed.host, "example.com");
        assert_eq!(parsed.port, 8080);
        assert_eq!(parsed.path, "/path");
    }

    #[test]
    fn parse_userinfo_without_password() {
        let url = "ftp://user@example.com/path";
        let parsed = Url::parse(url).unwrap();

        assert_eq!(parsed.scheme, "ftp");
        assert_eq!(parsed.username, "user");
        assert_eq!(parsed.password, "");
        assert_eq!(parsed.host, "example.com");
        assert_eq!(parsed.path, "/path");
    }

    #[test]
    fn parse_single_slash_url() {
        let url = "/";
        let parsed = Url::parse(url).unwrap();

        assert_eq!(parsed.scheme, "");
        assert_eq!(parsed.username, "");
        assert_eq!(parsed.password, "");
        assert_eq!(parsed.host, "");
        assert_eq!(parsed.path, "/");
    }

    #[test]
    fn parse_path_only() {
        let url = "/ameliaa";
        let parsed = Url::parse(url).unwrap();
        assert_eq!(parsed.scheme, "");
        assert_eq!(parsed.username, "");
        assert_eq!(parsed.password, "");
        assert_eq!(parsed.host, "");
        assert_eq!(parsed.port, 0);
        assert_eq!(parsed.path, "/ameliaa");
        assert_eq!(parsed.fragment, "");
    }

    #[test]
    fn format_full_url() {
        let url = "abc://username:password@example.com:123/path/data?key=value#fragid";
        let parsed = Url::parse(url).unwrap();
        let formatted = &parsed.to_string();
        assert_eq!(url, formatted)
    }
}
