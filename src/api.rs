use std::collections::BTreeMap;

use base58::{FromBase58, ToBase58};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tide::{Request, Response};

use crate::database::{self, *};

#[derive(Serialize)]
pub struct ApiResult<T: Serialize> {
    success: bool,
    message: String,
    payload: T,
}

impl<T: Serialize> ApiResult<T> {
    pub fn success(message: &str, payload: T) -> Self {
        Self {
            success: true,
            message: message.to_string(),
            payload,
        }
    }
    pub fn failure(message: &str, payload: T) -> Self {
        Self {
            success: false,
            message: message.to_string(),
            payload,
        }
    }
}

impl<T: Serialize> Into<tide::Result> for ApiResult<T> {
    fn into(self) -> tide::Result {
        Ok(Response::builder(200)
            .body(serde_json::to_value(&self)?)
            .build())
    }
}

pub async fn create_account(mut req: Request<()>) -> tide::Result {
    #[derive(Deserialize)]
    struct AccountInfo {
        name: String,
        username: String,
        password: String,
        owner: Option<String>,
    }

    if let Ok(AccountInfo {
        name,
        username,
        password,
        owner,
    }) = req.body_json().await
    {
        if let Some(_) = db_get_account(&username).await? {
            ApiResult::failure("Account existed", ()).into()
        } else {
            if let Some(owner) = owner {
                if let Some(owner) = db_get_account(&owner).await? {
                    db_create_account(&username, &password, &owner.account_username, &name).await?;
                    ApiResult::failure("Not implement yet!", ()).into()
                } else {
                    ApiResult::failure("Account not found", ()).into()
                }
            } else {
                if let Ok(_) = db_create_account(&username, &password, "admin", &name).await {
                    ApiResult::success("Succes", ()).into()
                } else {
                    ApiResult::failure("Failed to create account", ()).into()
                }
            }
        }
    } else {
        ApiResult::failure("Invalid Password", ()).into()
    }
}

pub async fn get_account_name(mut req: Request<()>) -> tide::Result {
    #[derive(Deserialize, Debug)]
    struct Login {
        username: String,
        password: String,
    }
    if let Ok(Login { username, password }) = dbg!(req.body_json().await) {
        if let Some(account) = dbg!(db_get_account(&username).await?) {
            if account.valid_password(&password) {
                ApiResult::success("Success", account.account_name).into()
            } else {
                ApiResult::failure("Invalid Password", ()).into()
            }
        } else {
            ApiResult::failure("Account not found", ()).into()
        }
    } else {
        ApiResult::failure("Invalid input", ()).into()
    }
}

pub async fn chpasswd(mut req: Request<()>) -> tide::Result {
    #[derive(Deserialize)]
    struct Input {
        username: String,
        password: String,
        new_password: String,
    }
    if let Ok(Input {
        username,
        password,
        new_password,
    }) = req.body_json().await
    {
        if let Some(account) = db_get_account(&username).await? {
            if account.valid_password(&password) {
                db_change_password(&username, &new_password).await?;
                ApiResult::success("Success", ()).into()
            } else {
                ApiResult::failure("Invalid password", ()).into()
            }
        } else {
            ApiResult::failure("Account not found", ()).into()
        }
    } else {
        ApiResult::failure("Invalid input", ()).into()
    }
}

pub async fn list_account(mut req: Request<()>) -> tide::Result {
    #[derive(Deserialize)]
    struct Input {
        username: String,
        password: String,
    }
    if let Ok(Input { username, password }) = req.body_json().await {
        if let Some(account) = db_get_account(&username).await? {
            if account.valid_password(&password) {
                let list = db_get_all_account(&username).await?;
                ApiResult::success("", list).into()
            } else {
                ApiResult::failure("Password incorrect", ()).into()
            }
        } else {
            ApiResult::failure("Account not found", ()).into()
        }
    } else {
        ApiResult::failure("Invalid Input", ()).into()
    }
}
pub async fn list_device(mut req: Request<()>) -> tide::Result {
    #[derive(Deserialize)]
    struct Input {
        username: String,
        password: String,
    }
    if let Ok(Input { username, password }) = req.body_json().await {
        if let Some(account) = db_get_account(&username).await? {
            if account.valid_password(&password) {
                #[derive(Serialize)]
                struct Device {
                    pub device_pubkey: String,
                    pub device_accepted: bool,
                    pub device_title: String,
                    pub device_local_ip: String,
                    pub device_schema: Value,
                }
                impl From<database::Device> for Device {
                    fn from(device: database::Device) -> Device {
                        Device {
                            device_pubkey: device.device_pubkey.to_base58(),
                            device_accepted: device.device_accepted,
                            device_title: device.device_title,
                            device_local_ip: device.device_local_ip,
                            device_schema: device.device_schema,
                        }
                    }
                }
                let devices = db_get_device_by_username(&username).await?;

                let devices: Vec<Device> = devices.into_iter().map(|d| d.into()).collect();
                ApiResult::success("", devices).into()
            } else {
                ApiResult::failure("Password incorrect", ()).into()
            }
        } else {
            ApiResult::failure("Account not found", ()).into()
        }
    } else {
        ApiResult::failure("Invalid Input", ()).into()
    }
}
pub async fn get_schema(mut req: Request<()>) -> tide::Result {
    #[derive(Deserialize)]
    struct Input {
        username: String,
        password: String,
        device: String,
    }
    if let Ok(Input {
        username,
        password,
        device,
    }) = req.body_json().await
    {
        if let Ok(pubkey) = device.from_base58() {
            if let Some(account) = db_get_account(&username).await? {
                if account.valid_password(&password) {
                    if let Some(device) = db_get_device(&username, &pubkey).await? {
                        ApiResult::success("", device.device_schema).into()
                    } else {
                        ApiResult::failure("Device not found", ()).into()
                    }
                } else {
                    ApiResult::failure("Password incorrect", ()).into()
                }
            } else {
                ApiResult::failure("Account not found", ()).into()
            }
        } else {
            ApiResult::failure("Invalid Input", ()).into()
        }
    } else {
        ApiResult::failure("Invalid Input", ()).into()
    }
}
pub async fn set_properties(mut req: Request<()>) -> tide::Result {
    #[derive(Deserialize)]
    struct Input {
        username: String,
        password: String,
        device: String,
        properties: BTreeMap<String, Value>,
    }

    if let Ok(Input {
        username,
        password,
        device,
        properties,
    }) = req.body_json().await
    {
        if let Ok(pubkey) = device.from_base58() {
            if let Some(account) = db_get_account(&username).await? {
                if account.valid_password(&password) {
                    crate::remote::set_wait(&pubkey, properties).await;
                    ApiResult::success("", ()).into()
                } else {
                    ApiResult::failure("Password incorrect", ()).into()
                }
            } else {
                ApiResult::failure("Account not found", ()).into()
            }
        } else {
            ApiResult::failure("Invalid Input", ()).into()
        }
    } else {
        ApiResult::failure("Invalid Input", ()).into()
    }
}
pub async fn get_properties(mut req: Request<()>) -> tide::Result {
    #[derive(Deserialize)]
    struct Input {
        username: String,
        password: String,
        device: String,
        property: String,
    }

    if let Ok(Input {
        username,
        password,
        device,
        property,
    }) = req.body_json().await
    {
        if let Ok(pubkey) = device.from_base58() {
            if let Some(account) = db_get_account(&username).await? {
                if account.valid_password(&password) {
                    if let Some(value) = db_get_property(&username, &pubkey, &property).await? {
                        ApiResult::success("", value).into()
                    } else {
                        ApiResult::failure("Property not found", ()).into()
                    }
                } else {
                    ApiResult::failure("Password incorrect", ()).into()
                }
            } else {
                ApiResult::failure("Account not found", ()).into()
            }
        } else {
            ApiResult::failure("Invalid Input", ()).into()
        }
    } else {
        ApiResult::failure("Invalid Input", ()).into()
    }
}
pub async fn get_local_ip(mut req: Request<()>) -> tide::Result {
    #[derive(Deserialize)]
    struct Input {
        username: String,
        password: String,
        device: String,
    }

    if let Ok(Input {
        username,
        password,
        device,
    }) = req.body_json().await
    {
        if let Ok(pubkey) = device.from_base58() {
            if let Some(account) = db_get_account(&username).await? {
                if account.valid_password(&password) {
                    if let Some(device) = db_get_device(&username, &pubkey).await? {
                        ApiResult::success("", device.device_local_ip).into()
                    } else {
                        ApiResult::failure("Device not found", ()).into()
                    }
                } else {
                    ApiResult::failure("Password incorrect", ()).into()
                }
            } else {
                ApiResult::failure("Account not found", ()).into()
            }
        } else {
            ApiResult::failure("Invalid Input", ()).into()
        }
    } else {
        ApiResult::failure("Invalid Input", ()).into()
    }
}

pub async fn accept_device(mut req: Request<()>) -> tide::Result {
    #[derive(Deserialize)]
    struct Input {
        username: String,
        password: String,
        device: String,
    }

    if let Ok(Input {
        username,
        password,
        device,
    }) = req.body_json().await
    {
        if let Ok(pubkey) = device.from_base58() {
            if let Some(account) = db_get_account(&username).await? {
                if account.valid_password(&password) {
                    if let Some(device) = db_get_device(&username, &pubkey).await? {
                        db_accept_device(&username, &pubkey).await?;
                        ApiResult::success("", device.device_local_ip).into()
                    } else {
                        ApiResult::failure("Device not found", ()).into()
                    }
                } else {
                    ApiResult::failure("Password incorrect", ()).into()
                }
            } else {
                ApiResult::failure("Account not found", ()).into()
            }
        } else {
            ApiResult::failure("Invalid Input", ()).into()
        }
    } else {
        ApiResult::failure("Invalid Input", ()).into()
    }
}
