use crate::dashboard::UiReply;
use crate::DB_CONN;
use diesel::query_builder::QueryFragment;
use diesel::sqlite::Sqlite;
use diesel::{AsChangeset, QueryResult};
use rencfs_desktop_common::dao::VaultDao;
use rencfs_desktop_common::models::{NewVault, Vault};
use rencfs_desktop_common::schema::vaults::dsl::vaults;
use std::sync::mpsc::Sender;

pub(super) struct DbService {
    id: Option<i32>,
    tx_parent: Sender<UiReply>,
}

impl DbService {
    pub(super) fn new(id: Option<i32>, tx_parent: Sender<UiReply>) -> Self {
        Self { id, tx_parent }
    }

    pub(super) fn delete(&self) -> QueryResult<()> {
        let binding = DB_CONN.get().unwrap();
        let mut conn = binding.lock().unwrap();
        let mut dao = VaultDao::new(&mut conn);
        dao.delete(self.id.as_ref().unwrap().clone())
    }

    pub(super) fn update<V>(&self, v: V)
    where
        V: AsChangeset<Target = vaults>,
        <V as AsChangeset>::Changeset: QueryFragment<Sqlite>,
    {
        let binding = DB_CONN.get().unwrap();
        let mut conn = binding.lock().unwrap();
        let mut dao = VaultDao::new(&mut conn);
        dao.update(self.id.as_ref().unwrap().clone(), v).unwrap();
        self.tx_parent.send(UiReply::VaultUpdated(false)).unwrap();
    }

    pub(super) fn get_vault(&self) -> QueryResult<Vault> {
        let binding = DB_CONN.get().unwrap();
        let mut conn = binding.lock().unwrap();
        let mut dao = VaultDao::new(&mut conn);
        dao.get(self.id.as_ref().unwrap().clone())
    }

    pub(super) fn insert(&mut self, new_vault: NewVault) -> QueryResult<()> {
        let binding = DB_CONN.get().unwrap();
        let mut conn = binding.lock().unwrap();
        let mut dao = VaultDao::new(&mut conn);
        dao.insert(&new_vault)
    }
}
