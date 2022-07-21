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
        let resp: reqwest::Response = $client.post(url.clone()).json($params).send().await?;
        let text = resp.text().await?;

        let result = serde_json::from_str(&text).map_err(Into::into);

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
        let url = $url;
        let resp = $client.get(url.clone()).send().await?;
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
