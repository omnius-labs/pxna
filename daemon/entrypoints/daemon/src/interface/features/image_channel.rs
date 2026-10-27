//! The image channel: the WebSocket through which the web client wants thumbnails, sprite sheets, and display images
//! (Image Channel Design). `daemon/image-channel.schema.json` is its contract, and `openapi.yaml` lists only the path of the upgrade.
use async_trait::async_trait;
use axum_extra::extract::CookieJar;
use headers::Host;
use http::Method;

use omnius_pxna_interface::{apis, models};

use crate::interface::{ApiServer, session::Claims};

mod connection;
mod message;
mod route;
#[cfg(test)]
mod tests;

pub use route::*;

#[allow(unused_variables)]
#[async_trait]
impl apis::image_channel::ImageChannel for ApiServer {
    type Claims = Claims;

    async fn get_image_channel(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
    ) -> std::result::Result<apis::image_channel::GetImageChannelResponse, ()> {
        // 外側のルータの手書きの route が先に処理するため、この実装は呼ばれない。
        let mut error = models::ErrorResponse::new(models::ErrorKind::InternalServerError);
        error.message = Some("served by a hand-written route".to_string());
        Ok(apis::image_channel::GetImageChannelResponse::Status500_InternalServerError(error))
    }
}
