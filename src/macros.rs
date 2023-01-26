/// Make a POST request sending and expecting JSON.
/// if JSON deser fails, emit a `WARN` level tracing event
#[macro_export]
macro_rules! json_post {
    ($client:expr, $url:expr, $params:expr,) => {
        json_post!($client, $url, $params)
    };

    ($client:expr, $url:expr, $params:expr) => {
    {
        let url = $url;
        let resp = $client.post(url.clone()).json($params).send().await?;
        let text = resp.text().await?;

        let result = serde_json::from_str(&text).map_err(Into::<$crate::client::ClientError>::into);

        if result.is_err() {
            tracing::warn!(
                method = "POST",
                url = %url,
                params = serde_json::to_string(&$params).unwrap().as_str(),
                response = text.as_str(),
                "Unexpected response from server"
            );
        }
        result
    }
}}

#[macro_export]
/// Make a GET request sending and expecting JSON.
/// if JSON deser fails, emit a `WARN` level tracing event
macro_rules! json_get {
    ($client:expr, $url:expr, $expected:ty,) => {
        json_get!($client, $url, $expected)
    };
    ($client:expr, $url:expr, $expected:ty) => {{
        let unit = ();
        json_get!($client, $url, $expected, unit)
    }};
    ($client:expr, $url:expr, $expected:ty, $body:ident,) => {
        json_get!($client, $url, $expected, $body)
    };
    ($client:expr, $url:expr, $expected:ty, $body:ident) => {{
        let url = $url;
        let resp = $client.get(url.clone()).json(&$body).send().await?;
        let text = resp.text().await?;

        let result = serde_json::from_str::<$expected>(&text).map_err(Into::<$crate::client::ClientError>::into);

        if result.is_err() {
            tracing::warn!(
                method = "GET",
                url = %url,
                response = text.as_str(),
                "Unexpected response from server"
            );
        }
        result
    }};
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use reqwest::Url;
    use tracing_test::traced_test;

    use crate::ClientError;

    struct MockClient<'a>(&'a str);
    impl<'a> MockClient<'a> {
        fn get(self, _: Url) -> Self {
            self
        }
        fn post(self, _: Url) -> Self {
            self
        }
        fn json<S: serde::Serialize>(self, _: &S) -> Self {
            self
        }
        async fn send(self) -> Result<MockClient<'a>, ()>
        where
            Self: 'static,
        {
            Ok(self)
        }
        async fn text(self) -> Result<String, ()> {
            Ok(self.0.to_owned())
        }
    }

    #[tokio::test]
    #[traced_test]
    async fn test_json_get_warn() -> Result<(), ()> {
        let url = reqwest::Url::from_str("http://example.com").unwrap();
        json_get!(MockClient("hello world"), url.clone(), u64).unwrap_err();
        assert!(logs_contain("Unexpected response from server"));
        assert!(logs_contain("hello world"));

        Ok(())
    }

    #[tokio::test]
    #[traced_test]
    async fn test_json_get_ok() -> Result<(), ()> {
        let url = reqwest::Url::from_str("http://example.com").unwrap();
        let num = json_get!(MockClient("1312"), url.clone(), u64).unwrap();
        assert!(num == 1312);
        assert!(!logs_contain("Unexpected response from server"));

        Ok(())
    }

    #[tokio::test]
    #[traced_test]
    async fn test_json_post_warn() -> Result<(), ()> {
        let url = reqwest::Url::from_str("http://example.com").unwrap();
        let f: Result<u8, ClientError> = json_post!(MockClient("hello world"), url.clone(), &1312);
        assert!(f.is_err());
        assert!(logs_contain("Unexpected response from server"));
        assert!(logs_contain("hello world"));

        Ok(())
    }

    #[tokio::test]
    #[traced_test]
    async fn test_json_post_ok() -> Result<(), ()> {
        let url = reqwest::Url::from_str("http://example.com").unwrap();
        let num: u64 = json_post!(MockClient("1312"), url.clone(), &1312).unwrap();
        assert!(num == 1312);
        assert!(!logs_contain("Unexpected response from server"));

        Ok(())
    }
}
