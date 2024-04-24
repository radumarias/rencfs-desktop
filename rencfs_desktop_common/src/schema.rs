// @generated automatically by Diesel CLI.

diesel::table! {
    vaults (id) {
        id -> Integer,
        name -> Text,
        mount_point -> Text,
        data_dir -> Text,
        locked -> Integer,
    }
}
