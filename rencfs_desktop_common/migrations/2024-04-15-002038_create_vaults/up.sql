CREATE TABLE vaults
(
    id          INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    name        VARCHAR NOT NULL UNIQUE,
    mount_point VARCHAR NOT NULL,
    data_dir    VARCHAR NOT NULL,
    locked      INTEGER NOT NULL default 1
)
