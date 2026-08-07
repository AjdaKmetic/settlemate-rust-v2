pub use sea_orm_migration::prelude::*;

mod m20260807_144809_create_users;
mod m20260807_153244_create_groups;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260807_144809_create_users::Migration),
            Box::new(m20260807_153244_create_groups::Migration),
        ]
    }
}