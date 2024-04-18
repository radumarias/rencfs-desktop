use diesel::prelude::*;

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = crate::schema::vaults)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Vault {
    pub id: i32,
    pub name: String,
    pub mount_point: String,
    pub data_dir: String,
    pub locked: i32,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = crate::schema::vaults)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewVault {
    pub name: String,
    pub mount_point: String,
    pub data_dir: String,
}
