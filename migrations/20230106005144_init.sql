-- Add migration script here
create table account (
    account_username                text primary key,
    account_password                text not null,
    account_name                    text not null

);

create table device (
    device_pubkey                   bytea primary key,
    device_accepted                 boolean not null default false,
    device_local_ip                 text not null,
    device_title                    text not null,
    device_schema                   json not null 
);

create table property (
    device_pubkey                   bytea references device on delete cascade,
    property_name                   text not null,
    property_value                  json not null,
    unique(device_pubkey, property_name)

);

create table link_account_account (
    account_username                text references account on delete cascade,
    derive_account_username         text references account(account_username) on delete cascade,
    unique(account_username, derive_account_username)
);

create table link_account_device (
    account_username                text references account on delete cascade,
    device_pubkey                   bytea references device on delete cascade,
    link_owner                      boolean not null default false,
    link_all_property               boolean not null default false,
    unique(account_username, device_pubkey)
);
