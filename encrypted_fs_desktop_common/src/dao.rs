use diesel::{AsChangeset, Connection, delete, EqAll, insert_into, QueryDsl, QueryResult, RunQueryDsl, SelectableHelper, SqliteConnection, update};
use diesel::query_builder::QueryFragment;
use diesel::sqlite::Sqlite;

use crate::models::{NewVault, Vault};
use crate::schema::vaults::dsl::vaults;
use crate::schema::vaults::id;

pub struct VaultDao<'a>(&'a mut SqliteConnection);

impl<'a> VaultDao<'a> {
    pub fn new(conn: &'a mut SqliteConnection) -> Self {
        VaultDao(conn)
    }

    pub fn insert(&mut self, e: &NewVault) -> QueryResult<()> {
        insert_into(vaults)
            .values(e)
            .execute(self.0)?;

        Ok(())
    }

    pub fn delete(&mut self, id_v: i32) -> QueryResult<()> {
        delete(vaults
            .filter(id.eq_all(id_v)))
            .execute(self.0)?;

        Ok(())
    }

    pub fn get(&mut self, id_v: i32) -> QueryResult<Vault> {
        vaults.find(id_v)
            .select(Vault::as_select())
            .first(self.0)
    }

    pub fn update<V>(&mut self, id_v: i32, value: V) -> QueryResult<()>
        where V: AsChangeset<Target=vaults>, <V as AsChangeset>::Changeset: QueryFragment<Sqlite>
    {
        update(vaults.find(id_v))
            .set(value)
            .execute(self.0)?;

        Ok(())
    }

    pub fn get_all(&mut self, limit: Option<i64>) -> QueryResult<Vec<Vault>> {
        if let Some(limit) = limit {
            vaults.select(Vault::as_select())
                .limit(limit)
                .load(self.0)
        } else {
            vaults.select(Vault::as_select())
                .load(self.0)
        }
    }

    pub fn transaction<F>(&mut self, f: F) -> QueryResult<usize>
        where F: FnOnce(VaultDao) -> QueryResult<usize> {
        self.0.transaction(|conn| {
            f(VaultDao::new(conn))
        })
    }
}
