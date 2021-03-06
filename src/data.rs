use crate::HOSTNAME;
use std::{
    fmt::{Display, Formatter},
    net::{IpAddr, SocketAddr},
};
use warp::{filters::header, hyper::Method, Filter, Rejection};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClientInfo {
    pub hostname: &'static str,
    #[serde(with = "http_method")]
    pub method: Method,
    pub host: Option<String>,
    pub client_ip: IpAddr,
    pub forwarded_for: Vec<IpAddr>,
    pub referer: Option<String>,
    pub user_agent: Option<String>,
}

pub fn client_info(
) -> impl Filter<Extract = (ClientInfo,), Error = Rejection> + Clone {
    warp::method()
        .and(header::optional::<String>("HOST"))
        .and(header::optional::<String>("X-FORWARDED-FOR"))
        .and(header::optional::<IpAddr>("X-REAL-IP"))
        .and(header::optional::<String>("REFERER"))
        .and(header::optional::<String>("USER-AGENT"))
        .and(warp::filters::addr::remote())
        .map(
            |method: Method,
             host: Option<String>,
             x_forwarded_for: Option<String>,
             x_real_ip: Option<IpAddr>,
             referer: Option<String>,
             user_agent: Option<String>,
             remote_addr: Option<SocketAddr>| {
                let forwarded_for: Vec<_> =
                    x_forwarded_for.map_or_else(Vec::new, |value| {
                        value
                            .split(',')
                            .map(|s| s.trim())
                            .filter_map(|s| s.parse::<IpAddr>().ok())
                            .collect()
                    });
                let client_ip = forwarded_for
                    .get(0)
                    .copied()
                    .or(x_real_ip)
                    .unwrap_or_else(|| remote_addr.unwrap().ip());

                ClientInfo {
                    hostname: &*HOSTNAME,
                    method,
                    host,
                    client_ip,
                    forwarded_for,
                    referer,
                    user_agent,
                }
            },
        )
}

impl Display for ClientInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Server hostname: {}", self.hostname)?;
        writeln!(f, "Method: {}", self.method)?;
        if let Some(host) = &self.host {
            writeln!(f, "Host: {}", host)?;
        }
        writeln!(f, "IP: {}", &self.client_ip)?;

        let mut header = "Forwarded-For: ";
        for entry in &self.forwarded_for {
            write!(f, "{}", header)?;
            writeln!(f, "{}", entry)?;
            header = "               ";
        }

        if let Some(referer) = &self.referer {
            writeln!(f, "Referer: {}", referer)?;
        }
        if let Some(user_agent) = &self.user_agent {
            writeln!(f, "User-Agent: {}", user_agent)?;
        }

        Ok(())
    }
}

mod http_method {
    use serde::{Deserialize, Deserializer, Serializer};
    use warp::hyper::Method;

    pub fn serialize<S>(
        method: &Method,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(method.as_str())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Method, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Method::from_bytes(s.as_bytes()).map_err(serde::de::Error::custom)
    }
}
