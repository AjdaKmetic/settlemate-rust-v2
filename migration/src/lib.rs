pub use sea_orm_migration::prelude::*;

mod m20260807_144809_create_users;
mod m20260807_153244_create_groups;
mod m20260807_153839_create_group_members;
mod m20260807_154541_create_friendships;
mod m20260809_182239_create_expenses;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260807_144809_create_users::Migration),
            Box::new(m20260807_153244_create_groups::Migration),
            Box::new(m20260807_153839_create_group_members::Migration),
            Box::new(m20260807_154541_create_friendships::Migration),
            Box::new(m20260809_182239_create_expenses::Migration),
        ]
    }
}