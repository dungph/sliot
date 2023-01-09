use std::{collections::BTreeMap, net::Ipv4Addr, time::Duration};

use async_std::sync::Mutex;
use base58::FromBase58;
use serde::Deserialize;
use serde_json::Value;
use sqlx::query;
use tide::{Request, Response};

use crate::database::DB;

static WAITER: Mutex<BTreeMap<Vec<u8>, BTreeMap<String, Value>>> = Mutex::new(BTreeMap::new());
pub async fn set_wait(device: &[u8], properties: BTreeMap<String, Value>) {
    WAITER
        .lock()
        .await
        .entry(device.to_owned())
        .or_default()
        .extend(properties);
}
async fn wait(device: &[u8]) -> Option<BTreeMap<String, Value>> {
    WAITER.lock().await.remove(device)
}
pub async fn new_device(mut req: Request<()>) -> tide::Result {
    #[derive(Deserialize)]
    struct Input {
        pubkey: String,
        username: String,
        title: String,
        local_ip: String,
        schema: Value,
    }
    if let Ok(Input {
        pubkey,
        username,
        title,
        local_ip,
        schema,
    }) = req.body_json().await
    {
        if let Ok(pubkey) = pubkey.from_base58() {
            query!(
                r#"
                insert into device (device_pubkey, device_accepted, device_title, device_local_ip, device_schema)
                values($1, $2, $3, $4, $5)
                on conflict (device_pubkey)
                do update 
                set device_title = $3, device_local_ip = $4, device_schema = $5
                ;
            "#,
                pubkey,
                false,
                title,
                local_ip,
                schema,
            )
            .execute(&*DB)
            .await?;
            query!(
                r#"
                    insert into link_account_device(account_username, device_pubkey)
                    values ($1, $2)
                    on conflict (account_username, device_pubkey) do nothing
                "#,
                username,
                pubkey
            )
            .execute(&*DB)
            .await?;
            Ok(Response::builder(200).build())
        } else {
            Ok(Response::builder(400).build())
        }
    } else {
        Ok(Response::builder(400).build())
    }
}
pub async fn put_schema(mut req: Request<()>) -> tide::Result {
    #[derive(Deserialize)]
    struct Input {
        schema: Value,
    }
    if let Ok(pubkey) = req.param("device")?.from_base58() {
        if let Ok(Input { schema }) = req.body_json().await {
            let _result = query!(
                r#"
                update device
                set device_schema = $2 
                where device_pubkey = $1
            "#,
                pubkey,
                schema,
            )
            .execute(&*DB)
            .await?;
        }
    }
    Ok(Response::builder(200).build())
}
pub async fn put_data(mut req: Request<()>) -> tide::Result {
    #[derive(Deserialize)]
    struct Input {
        properties: BTreeMap<String, Value>,
    }

    if let Ok(pubkey) = req.param("device")?.from_base58() {
        if let Ok(Input { properties }) = req.body_json().await {
            for (k, v) in properties {
                query!(
                    r#"
                    insert into property(device_pubkey, property_name, property_value)
                    values($1, $2, $3)
                    on conflict (device_pubkey, property_name)
                    do update
                    set property_value = $3
                    "#,
                    pubkey,
                    k,
                    v
                )
                .execute(&*DB)
                .await?;
            }
        }
    }
    Ok(Response::builder(200).build())
}
pub async fn wait_data(req: Request<()>) -> tide::Result {
    if let Ok(pubkey) = req.param("device")?.from_base58() {
        let values = wait(&pubkey).await;
        Ok(Response::builder(200)
            .body(serde_json::to_value(values)?)
            .build())
    } else {
        Ok(Response::builder(400).build())
    }
}
pub async fn put_local_ip(mut req: Request<()>) -> tide::Result {
    #[derive(Deserialize)]
    struct Input {
        ip: String,
    }
    if let Ok(pubkey) = req.param("device")?.from_base58() {
        if let Ok(Input { ip }) = req.body_json().await {
            let ip = ip.parse::<Ipv4Addr>()?;
            query!(
                r#"
                    update device
                    set device_local_ip = $2
                    where device_pubkey = $1
                    "#,
                pubkey,
                ip.to_string()
            )
            .execute(&*DB)
            .await?;
        }
    }
    Ok(Response::builder(200).build())
}
