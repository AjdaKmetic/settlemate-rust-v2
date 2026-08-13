pub use sea_orm_migration::prelude::*;

mod m20260807_144809_create_users;
mod m20260807_153244_create_groups;
mod m20260807_153839_create_group_members;
mod m20260807_154541_create_friendships;
mod m20260809_182239_create_expenses;
mod m20260809_195359_create_expense_splits;
mod m20260809_203112_create_sessions;
mod m20260813_012101_create_payments;

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
            Box::new(m20260809_195359_create_expense_splits::Migration),
            Box::new(m20260809_203112_create_sessions::Migration),
            Box::new(m20260813_012101_create_payments::Migration),
        ]
    }
}