use crate::register::RegisterBuilder;
use crate::route::Route;
use http::Request;
use http_body_util::{BodyExt, Full};
use hyper::client::conn::http2::SendRequest;
use krpc_common::{KrpcMsg, RpcError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct KrpcClient {
    route: Route,
}

impl KrpcClient {
    pub fn build(register_builder: RegisterBuilder) -> KrpcClient {
        let map = Arc::new(RwLock::new(HashMap::new()));
        let register = register_builder.init(map.clone());
        let cli = KrpcClient {
            route: Route::new(map, register),
        };
        return cli;
    }

    pub async fn invoke<Req, Res>(&self, msg: KrpcMsg) -> Result<Res, RpcError>
    where
        Req: Send + Sync + Serialize,
        Res: Send + Sync + Serialize + for<'a> Deserialize<'a> + Default,
    {
        let mut sender: SendRequest<Full<bytes::Bytes>> = self
            .route
            .get_socket_sender(&msg.class_name, &msg.version)
            .await
            .map_err(|e| RpcError::Client(e.to_string()))?;
        let req = Request::builder()
            .header("unique_identifier", msg.unique_identifier)
            .header("version", msg.version)
            .header("class_name", msg.class_name)
            .header("method_name", msg.method_name)
            .body(Full::<bytes::Bytes>::from(msg.req))
            .map_err(|e| RpcError::Client(e.to_string()))?;
        let mut res = sender
            .send_request(req)
            .await
            .map_err(|e| RpcError::Client(e.to_string()))?;
        let res: Result<String, RpcError> = serde_json::from_slice(
            res.frame()
                .await
                .unwrap()
                .map_err(|e| RpcError::Client(e.to_string()))?
                .data_ref()
                .unwrap()
                .as_ref(),
        )
        .map_err(|e| RpcError::Client(e.to_string()))?;
        let res: Result<Res, RpcError> = match res {
            Ok(data) => Ok(serde_json::from_slice(&data.as_bytes())
                .map_err(|e| RpcError::Client(e.to_string()))?),
            Err(err) => Err(err),
        };
        return res;
    }
}
