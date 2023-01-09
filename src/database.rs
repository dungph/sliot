use anyhow::Result;
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use once_cell::sync::Lazy;
use serde::Serialize;
use serde_json::Value;
use sqlx::{query, query_as};

pub static DB: Lazy<sqlx::PgPool> = Lazy::new(|| {
    let url = std::env::var("DATABASE_URL").expect("set DATABASE_URL to your postgres uri");
    sqlx::PgPool::connect_lazy(&url).unwrap()
});

pub async fn migrate() -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(&*DB).await?;

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password("admin".as_bytes(), &salt)
        .unwrap()
        .to_string();
    query!(
        r#"
        insert into account(account_username, account_password, account_name)
        values ('admin', $1, 'Admin')
        on conflict do nothing;
        "#,
        password_hash
    )
    .execute(&*DB)
    .await?;
    Ok(())
}
#[derive(Serialize, Debug)]
pub struct Account {
    pub account_name: String,
    pub account_username: String,
    pub account_password: String,
}

impl Account {
    pub fn valid_password(&self, password: &str) -> bool {
        let parsed_hash = PasswordHash::new(&self.account_password).unwrap();
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok()
    }
}

#[derive(Serialize)]
pub struct Device {
    pub device_pubkey: Vec<u8>,
    pub device_accepted: bool,
    pub device_title: String,
    pub device_local_ip: String,
    pub device_schema: Value,
}
pub async fn db_get_property(
    username: &str,
    device: &[u8],
    property_id: &str,
) -> Result<Option<Value>> {
    Ok(sqlx::query!(
        r#"
                select property_value from property 
                join link_account_device 
                on link_account_device.device_pubkey = property.device_pubkey
                where account_username = $1 and property.device_pubkey = $2 and property_name = $3
            "#,
        username,
        device,
        property_id
    )
    .fetch_optional(&*DB)
    .await?
    .map(|o| o.property_value))
}
pub async fn db_get_device_by_username(username: &str) -> Result<Vec<Device>> {
    Ok(query_as!(
        Device,
        r#"
            select device.device_pubkey, device_accepted, device_title, device_local_ip, device_schema
            from device join link_account_device
            on link_account_device.device_pubkey = device.device_pubkey
            where account_username = $1
            "#,
        username
    )
    .fetch_all(&*DB)
    .await?)
}

pub async fn db_get_device(username: &str, pubkey: &[u8]) -> Result<Option<Device>> {
    Ok(query_as!(
        Device,
        r#"
            select device.device_pubkey, device_accepted, device_title, device_local_ip, device_schema
            from device join link_account_device
            on link_account_device.device_pubkey = device.device_pubkey
            where account_username = $1 and device.device_pubkey= $2
            "#,
        username,
        pubkey
    )
    .fetch_optional(&*DB)
    .await?)
}

pub async fn db_accept_device(username: &str, pubkey: &[u8]) -> Result<()> {
    query!(
        r#"
        update device 
        set device_accepted = true 
        where device_pubkey = $2 and 0 < (
            select count(*) from device
            join link_account_device
            on link_account_device.device_pubkey = device.device_pubkey
            where account_username = $1 and device.device_pubkey = $2
            )"#,
        username,
        pubkey
    )
    .execute(&*DB)
    .await?;
    Ok(())
}
pub async fn db_device_new_title(username: &str, pubkey: &[u8], title: &str) -> Result<()> {
    query!(
        r#"
        update device 
        set device_title = $3 
        where device_pubkey = $2 and 0 < (
            select count(*) from device
            join link_account_device
            on link_account_device.device_pubkey = device.device_pubkey
            where account_username = $1 and device.device_pubkey = $2
            )"#,
        username,
        pubkey,
        title
    )
    .execute(&*DB)
    .await?;
    Ok(())
}
pub async fn db_create_account(
    username: &str,
    password: &str,
    owner: &str,
    name: &str,
) -> Result<Account> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .unwrap()
        .to_string();

    query!(
        r#"
            insert into account (account_username, account_password, account_name)
            values ($1, $2, $3);
            "#,
        username,
        password_hash,
        name
    )
    .execute(&*DB)
    .await?;
    query!(
        r#"
            insert into link_account_account (account_username, derive_account_username)
            values ('admin', $1)
            on conflict do nothing;
            "#,
        username
    )
    .execute(&*DB)
    .await?;
    query!(
        r#"
            insert into link_account_account (account_username, derive_account_username)
            values ($2, $1)
            on conflict do nothing;
            "#,
        username,
        owner
    )
    .execute(&*DB)
    .await?;
    Ok(Account {
        account_name: username.to_string(),
        account_username: username.to_string(),
        account_password: password.to_string(),
    })
}
//pub async fn new_with_owner(username: &str, password: &str, owner: &str) -> Result<()> {

//}
pub async fn db_get_all_account(owner: &str) -> Result<Vec<Account>> {
    Ok(query_as!(Account,
            r#"
                select account.account_username, account.account_name, account.account_password from account
                join link_account_account
                on link_account_account.derive_account_username = account.account_username
                where link_account_account.account_username = $1
                "#,
            owner
        )
        .fetch_all(&*DB)
        .await?)
    //.into_iter()
    //.map(|o| Account {
    //    account_name: o.account_name,
    //    account_username: o.account_username,
    //    account_password: o.account_password,
    //})
    //.collect())
}
pub async fn db_get_account(username: &str) -> Result<Option<Account>> {
    Ok(query_as!(
        Account,
        r#"select * from account
            where account_username = $1"#,
        username
    )
    .fetch_optional(&*DB)
    .await?)
    //.map(|o| Account {
    //    account_name: o.account_name,
    //    account_username: o.account_username,
    //    account_password: o.account_password,
    //}))
}
pub async fn db_change_password(username: &str, password: &str) -> Result<()> {
    query!(
        r#"update account
            set account_password = $2
            where account_username = $1"#,
        username,
        password
    )
    .execute(&*DB)
    .await?;
    Ok(())
}
