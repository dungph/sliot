mod api;
mod database;
mod remote;

#[async_std::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;
    database::migrate().await?;
    tide::log::start();

    let mut server = tide::new();

    server.at("/device/new").post(remote::new_device);
    server.at("/device/:device/schema").post(remote::put_schema);
    server
        .at("/device/:device/local_ip")
        .post(remote::put_local_ip);
    server.at("/device/:device/data/set").post(remote::put_data);
    server
        .at("/device/:device/data/wait")
        .post(remote::wait_data);

    server.at("/api/account/new").post(api::create_account);
    server.at("/api/account/name").post(api::get_account_name);
    server.at("/api/account/new_password").post(api::chpasswd);
    server.at("/api/list_device").post(api::list_device);
    server.at("/api/list_account").post(api::list_account);
    server.at("/api/device/local_ip").post(api::get_local_ip);
    server.at("/api/device/title/new").post(api::set_title);
    server.at("/api/device/schema").post(api::get_schema);
    server.at("/api/property/get").post(api::get_properties);
    server.at("/api/property/set").post(api::set_properties);

    server.listen("0.0.0.0:8080").await?;
    Ok(())
}
